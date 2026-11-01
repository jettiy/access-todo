//! Shared application state.
//!
//! Holds the in-memory store, a crash-safe local file mirror, and — when
//! configured — a Gist sync engine. Data safety rule: every mutation is
//! persisted to the local file first; Gist is a sync/backup target, never
//! the only copy. A broadcast channel fans out "changed" events to SSE
//! subscribers so post-it windows refresh instantly.

use std::path::PathBuf;
use std::sync::Arc;

use agent_registry::Registry;
use tokio::sync::{broadcast, Mutex};

use todo_core::gist::GistClient;
use todo_core::storage::LocalStore;
use todo_core::store::Store;
use todo_core::sync::SyncEngine;

use crate::settings::Settings;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub sync: Arc<Option<SyncEngine>>,
    pub local: Arc<LocalStore>,
    pub tx: broadcast::Sender<String>,
    /// Pilot settings (enabled agents, knowledge level, sync mode).
    pub settings: Arc<Mutex<Settings>>,
    pub settings_path: PathBuf,
    /// Agent-tool detection/MCP registration, bound to the user's home.
    pub registry: Arc<Registry>,
    /// Path of the MCP server executable used in registrations.
    pub mcp_exe: PathBuf,
}

impl AppState {
    /// Empty in-memory store, no Gist sync (used by tests).
    /// Local persistence, settings and the agent registry all point at a
    /// temp directory so tests stay hermetic.
    pub fn new() -> Self {
        let tmp = std::env::temp_dir().join(format!(
            "access-api-test-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&tmp);
        Self::with_parts(
            None,
            LocalStore::new(tmp.join("data.json")),
            tmp.join("settings.json"),
            Registry::with_home(tmp.clone()),
            tmp.join("Access-MCP.exe"),
        )
    }

    /// Store backed by a Gist + local file. Reads GITHUB_TOKEN and
    /// TODO_GIST_ID from env; ACCESS_DATA_FILE overrides the data path.
    pub fn with_gist_from_env() -> Self {
        let token = std::env::var("GITHUB_TOKEN").ok();
        let gist_id = std::env::var("TODO_GIST_ID").ok();
        let engine = match (token, gist_id) {
            (Some(t), Some(id)) => Some(SyncEngine::new(GistClient::new(t), id)),
            _ => None,
        };
        let data_path = std::env::var("ACCESS_DATA_FILE").unwrap_or_else(|_| {
            default_data_path()
        });
        let settings_path = crate::settings::default_settings_path();
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let mcp_exe = exe_dir
            .clone()
            .map(|d| d.join("Access-MCP.exe"))
            .unwrap_or_else(|| PathBuf::from("Access-MCP.exe"));
        Self::with_parts(
            engine,
            LocalStore::new(data_path),
            settings_path,
            Registry::new(),
            mcp_exe,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn with_parts(
        engine: Option<SyncEngine>,
        local: LocalStore,
        settings_path: PathBuf,
        registry: Registry,
        mcp_exe: PathBuf,
    ) -> Self {
        let (tx, _rx) = broadcast::channel(16);
        let settings = Settings::load(&settings_path);
        Self {
            store: Arc::new(Mutex::new(Store::new())),
            sync: Arc::new(engine),
            local: Arc::new(local),
            tx,
            settings: Arc::new(Mutex::new(settings)),
            settings_path,
            registry: Arc::new(registry),
            mcp_exe,
        }
    }

    /// Startup recovery — guarantees no data loss:
    /// 1. load the local file into memory,
    /// 2. force-pull the Gist (ETag bypassed) and merge it in,
    /// 3. collapse duplicate categories,
    /// 4. persist the recovered state back to the local file.
    /// Returns (todo_count, category_count).
    pub async fn recover(&self) -> Result<(usize, usize), String> {
        let local_doc = self.local.load().map_err(|e| format!("local load: {e}"))?;
        let mut store = local_doc.map(Store::from_doc).unwrap_or_default();

        if let Some(engine) = self.sync.as_ref() {
            match engine.pull_full(&mut store, "startup").await {
                Ok(_) => {}
                Err(e) => eprintln!("warn: startup Gist pull failed ({e}); continuing from local file"),
            }
        }
        store.dedupe_categories();

        let counts = (store.list().len(), store.list_categories_all().len());
        let doc = store.clone().into_doc("recover");
        *self.store.lock().await = store;
        self.local.save(&doc).map_err(|e| format!("local save: {e}"))?;
        Ok(counts)
    }

    /// Persist the current store to the local file (crash-safe).
    pub async fn persist(&self, actor: &str) -> Result<(), String> {
        let doc = self.store.lock().await.clone().into_doc(actor);
        self.local.save(&doc).map_err(|e| e.to_string())
    }

    /// Full safe-write path used by every mutation handler:
    /// 1. persist locally (data is now durable),
    /// 2. push to Gist (best effort; local copy already safe),
    /// 3. notify SSE subscribers.
    pub async fn persist_and_push(&self, actor: &str) {
        if let Err(e) = self.persist(actor).await {
            eprintln!("warn: local persist failed: {e}");
        }
        if let Err(e) = self.push(actor).await {
            eprintln!("warn: gist push failed: {e}");
        }
        let _ = self.tx.send("changed".into());
    }

    /// Snapshot the current state into a timestamped backup file.
    pub async fn backup(&self) -> Result<String, String> {
        let doc = self.store.lock().await.clone().into_doc("backup");
        let path = self.local.backup(&doc).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    }

    /// Pull from Gist into the local store (no-op if not configured).
    pub async fn pull(&self, actor: &str) -> Result<(), String> {
        if let Some(engine) = self.sync.as_ref() {
            let mut s = self.store.lock().await;
            engine.pull(&mut s, actor).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Push the local store to the Gist (no-op if not configured).
    pub async fn push(&self, actor: &str) -> Result<(), String> {
        if let Some(engine) = self.sync.as_ref() {
            let s = self.store.lock().await;
            engine.push(&s, actor).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Notify SSE subscribers that something changed.
    pub fn broadcast_change(&self) {
        let _ = self.tx.send("changed".into());
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Default data file lives next to the API executable so it survives
/// restarts regardless of the working directory.
fn default_data_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("access-data.json")))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "access-data.json".into())
}
