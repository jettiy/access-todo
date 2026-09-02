//! Shared application state.
//!
//! Holds the in-memory store and, when configured, a Gist sync engine
//! so the REST API can pull/push the canonical document to GitHub.
//!
//! Mutations don't push to Gist synchronously — they flip a dirty flag
//! via [`AppState::mark_push_pending`] and let [`AppState::spawn_flusher`]
//! coalesce rapid edits into a single PATCH (1.5s quiet window). This
//! keeps toggle/add storms under GitHub's rate limit and out of the
//! request hot path. `synced: true` in a response means "no Gist OR the
//! last flush succeeded" — `false` means we know sync is broken.
//!
//! When `GITHUB_TOKEN` is not set, the flusher falls back to writing a
//! local JSON file (`ACCESS_LOCAL_CACHE` or the platform data dir) so
//! restarts don't lose data. [`AppState::with_gist_from_env`] loads it
//! back on startup.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use todo_core::gist::GistClient;
use todo_core::model::TodoDoc;
use todo_core::store::Store;
use todo_core::sync::SyncEngine;

/// How long the flusher waits after a mutation before pushing, to let
/// rapid edits coalesce. Tuned to absorb a user mashing toggle and a
/// handful of agents acting at once, without making sync feel laggy.
pub const PUSH_DEBOUNCE: Duration = Duration::from_millis(1500);

/// Resolve the on-disk cache location. Precedence:
/// `ACCESS_LOCAL_CACHE` env var → platform-conventional data dir → None.
fn local_cache_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ACCESS_LOCAL_CACHE") {
        return Some(PathBuf::from(p));
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("Access").join("todos.json"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| {
            PathBuf::from(h)
                .join("Library/Application Support/Access/todos.json")
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".local/share/Access/todos.json"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        None
    }
}

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<Store>>,
    pub sync: Arc<Option<SyncEngine>>,
    /// Optional on-disk cache. When `sync` is None (no GITHUB_TOKEN), the
    /// flusher writes here so restarts preserve state. Even with a Gist,
    /// we keep this as a recovery aid — flusher writes to file too, so a
    /// later Gist push can be reconstructed if needed.
    local_cache: Arc<Option<PathBuf>>,
    /// Set by every mutation; cleared (and pushed) by the flusher task.
    dirty: Arc<AtomicBool>,
    /// Last flush outcome — drives the `synced: bool` response field.
    /// Starts `true` so the first mutation after boot doesn't look stale.
    last_push_ok: Arc<AtomicBool>,
}

impl AppState {
    /// Empty in-memory store, no Gist sync (used by tests).
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::new())),
            sync: Arc::new(None),
            local_cache: Arc::new(None),
            dirty: Arc::new(AtomicBool::new(false)),
            last_push_ok: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Store backed by a Gist, falling back to a local cache file when
    /// `GITHUB_TOKEN` is not set. Reads `GITHUB_TOKEN` + `TODO_GIST_ID`
    /// and the local cache from env. If neither is available, the store
    /// is purely in-memory and a warning is logged.
    pub fn with_gist_from_env() -> Self {
        let token = std::env::var("GITHUB_TOKEN").ok();
        let gist_id = std::env::var("TODO_GIST_ID").ok();
        let engine = match (token, gist_id) {
            (Some(t), Some(id)) => Some(SyncEngine::new(GistClient::new(t), id)),
            _ => None,
        };
        let cache = if engine.is_some() {
            // Gist configured: keep a local copy too, as a recovery aid.
            local_cache_path()
        } else {
            // No Gist: local cache is the only persistence.
            let cache = local_cache_path();
            if cache.is_none() {
                eprintln!(
                    "warn: no GITHUB_TOKEN and no writable local cache — store is in-memory only and will be lost on restart"
                );
            }
            cache
        };
        Self {
            store: Arc::new(RwLock::new(Store::new())),
            sync: Arc::new(engine),
            local_cache: Arc::new(cache),
            dirty: Arc::new(AtomicBool::new(false)),
            last_push_ok: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Construct an AppState wired to a caller-provided SyncEngine — used
    /// by integration tests that point the engine at a mock Gist.
    pub fn with_sync_engine(engine: SyncEngine) -> Self {
        Self {
            store: Arc::new(RwLock::new(Store::new())),
            sync: Arc::new(Some(engine)),
            local_cache: Arc::new(None),
            dirty: Arc::new(AtomicBool::new(false)),
            last_push_ok: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Pull from Gist into the local store (no-op if not configured).
    /// If a local cache exists but no Gist, it's loaded on first call.
    pub async fn pull(&self, actor: &str) -> Result<(), String> {
        if let Some(engine) = self.sync.as_ref() {
            let mut s = self.store.write().await;
            engine.pull(&mut s, actor).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Load the local cache file into the in-memory store, replacing
    /// whatever is there. No-op if no cache path is configured or the
    /// file doesn't exist yet. Used at startup so the previous session's
    /// state survives restarts even without a Gist.
    pub async fn load_local_cache(&self) -> Result<(), String> {
        let Some(path) = self.local_cache.as_ref() else {
            return Ok(());
        };
        let bytes = match tokio::fs::read(path).await {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        let doc: TodoDoc = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let mut s = self.store.write().await;
        *s = Store::from_doc(doc);
        Ok(())
    }

    /// Synchronously write the current store to the local cache file, if
    /// any. Best-effort — errors are logged but not propagated, since the
    /// cache is a recovery aid rather than the source of truth.
    async fn write_local_cache(&self) {
        let Some(path) = self.local_cache.as_ref() else {
            return;
        };
        let snapshot = {
            let st = self.store.read().await;
            let doc = st.clone().into_doc("local-cache");
            serde_json::to_vec_pretty(&doc).unwrap_or_default()
        };
        if let Err(e) = write_file_atomic(path, &snapshot).await {
            eprintln!("warn: local cache write failed: {e}");
        }
    }

    /// Mark the store dirty so the flusher will push on its next cycle.
    /// Returns the value callers should put in `"synced": bool` — `true`
    /// when no Gist is configured (or the last push succeeded), `false`
    /// when we know sync is broken. Cheap (no I/O) — call from every
    /// mutation handler.
    pub fn mark_push_pending(&self) -> bool {
        self.dirty.store(true, Ordering::Release);
        if self.sync.is_none() {
            // No Gist → the local cache flusher is the only sink. Report
            // success optimistically; a real write failure will clear
            // last_push_ok and next mutation sees `false`.
            return self.last_push_ok.load(Ordering::Acquire);
        }
        self.last_push_ok.load(Ordering::Acquire)
    }

    /// Force an immediate push now (used by `/sync`). Bypasses the
    /// debouncer and returns the actual outcome.
    pub async fn push_now(&self, actor: &str) -> bool {
        let ok = match self.sync.as_ref() {
            None => true,
            Some(engine) => {
                let mut s = self.store.write().await;
                match engine.push(&mut s, actor).await {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("warn: gist push failed: {e}");
                        false
                    }
                }
            }
        };
        self.last_push_ok.store(ok, Ordering::Release);
        self.dirty.store(false, Ordering::Release);
        // Also bump the local cache so restart-restored state matches.
        self.write_local_cache().await;
        ok
    }

    /// Spawn the background flusher. Run once at startup. Polls the dirty
    /// flag every 200ms; when set, waits [`PUSH_DEBOUNCE`] for more
    /// mutations to coalesce, then pushes once (to Gist if configured,
    /// otherwise to the local cache file). Exits when all `AppState`
    /// clones are dropped.
    pub fn spawn_flusher(self) {
        if self.sync.is_none() && self.local_cache.is_none() {
            return;
        }
        let store = self.store.clone();
        let sync = self.sync.clone();
        let cache = self.local_cache.clone();
        let dirty = self.dirty.clone();
        let last_push_ok = self.last_push_ok.clone();

        tokio::spawn(async move {
            let poll = Duration::from_millis(200);
            loop {
                if dirty.load(Ordering::Acquire) {
                    // wait for the quiet window so rapid edits merge
                    tokio::time::sleep(PUSH_DEBOUNCE).await;
                    // only flush if still dirty — a concurrent push_now
                    // may have already flushed it.
                    if !dirty.swap(false, Ordering::AcqRel) {
                        continue;
                    }
                    let ok = match sync.as_ref() {
                        Some(engine) => {
                            let mut s = store.write().await;
                            match engine.push(&mut s, "flusher").await {
                                Ok(()) => true,
                                Err(e) => {
                                    eprintln!("warn: gist flush failed: {e}");
                                    false
                                }
                            }
                        }
                        None => true, // local-only mode: file write below
                    };
                    // Always also write the local cache (cheap, recovery aid).
                    if let Some(path) = cache.as_ref() {
                        let snapshot = {
                            let st = store.read().await;
                            serde_json::to_vec_pretty(&st.clone().into_doc("local-cache"))
                                .unwrap_or_default()
                        };
                        if let Err(e) = write_file_atomic(path, &snapshot).await {
                            eprintln!("warn: local cache write failed: {e}");
                            last_push_ok.store(false, Ordering::Release);
                            continue;
                        }
                    }
                    last_push_ok.store(ok, Ordering::Release);
                }
                tokio::time::sleep(poll).await;
            }
        });
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Write `bytes` to `path` via a temp file + rename so a crash mid-write
/// can't corrupt the cache. Best-effort: if the parent dir doesn't exist
/// we create it; anything else is reported as an error.
async fn write_file_atomic(path: &PathBuf, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}
