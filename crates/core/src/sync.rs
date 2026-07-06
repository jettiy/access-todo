//! Sync engine: orchestrates fetch → merge → push against a Gist.
//!
//! Holds the last-seen ETag so subsequent fetches can be conditional.

use tokio::sync::Mutex;

use crate::error::Result;
use crate::gist::GistClient;
use crate::merge::merge;
use crate::model::TodoDoc;
use crate::store::Store;

/// Owns a `GistClient` and a gist id, plus the current ETag.
pub struct SyncEngine {
    client: GistClient,
    gist_id: String,
    etag: Mutex<Option<String>>,
}

impl SyncEngine {
    /// Create a new engine for the given gist id.
    pub fn new(client: GistClient, gist_id: String) -> Self {
        Self {
            client,
            gist_id,
            etag: Mutex::new(None),
        }
    }

    /// Pull remote, merge into `local`, store the merged result back into
    /// `local`, and return the merged document.
    pub async fn pull(&self, local: &mut Store, _actor: &str) -> Result<TodoDoc> {
        let etag_lock = self.etag.lock().await.clone();
        let (remote, new_etag) = self.client.fetch(&self.gist_id, etag_lock.as_deref()).await?;

        let local_todos = local.clone().list();
        let local_doc = TodoDoc {
            version: "1.0".into(),
            updated_at: remote.updated_at,
            updated_by: "local".into(),
            todos: local_todos,
        };
        let merged = merge(&local_doc, &remote);
        *local = Store::from_doc(merged.clone());
        *self.etag.lock().await = new_etag;
        Ok(merged)
    }

    /// Push the current `local` store to the gist.
    pub async fn push(&self, local: &Store, actor: &str) -> Result<()> {
        let doc = local.clone().into_doc(actor);
        let etag_lock = self.etag.lock().await.clone();
        let new_etag = self.client.push(&self.gist_id, &doc, etag_lock.as_deref()).await?;
        *self.etag.lock().await = new_etag;
        Ok(())
    }
}
