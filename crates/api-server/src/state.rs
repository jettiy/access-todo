//! Shared application state.
//!
//! Holds the in-memory store and, when configured, a Gist sync engine
//! so the REST API can pull/push the canonical document to GitHub.

use std::sync::Arc;

use tokio::sync::Mutex;

use todo_core::gist::GistClient;
use todo_core::store::Store;
use todo_core::sync::SyncEngine;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub sync: Arc<Option<SyncEngine>>,
}

impl AppState {
    /// Empty in-memory store, no Gist sync (used by tests).
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::new())),
            sync: Arc::new(None),
        }
    }

    /// Store backed by a Gist. Reads GITHUB_TOKEN and TODO_GIST_ID from env.
    pub fn with_gist_from_env() -> Self {
        let token = std::env::var("GITHUB_TOKEN").ok();
        let gist_id = std::env::var("TODO_GIST_ID").ok();
        let engine = match (token, gist_id) {
            (Some(t), Some(id)) => Some(SyncEngine::new(GistClient::new(t), id)),
            _ => None,
        };
        Self {
            store: Arc::new(Mutex::new(Store::new())),
            sync: Arc::new(engine),
        }
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
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
