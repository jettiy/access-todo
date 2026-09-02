//! Sync engine: orchestrates fetch → merge → push against a Gist.
//!
//! Holds the last-seen ETag so subsequent fetches can be conditional.

use tokio::sync::Mutex;

use crate::error::Result;
use crate::gist::{FetchResult, GistClient};
use crate::merge::merge;
use crate::model::TodoDoc;
use crate::store::Store;

/// Owns a `GistClient` and a gist id, plus the current ETag.
pub struct SyncEngine {
    client: GistClient,
    gist_id: String,
    etag: Mutex<Option<String>>,
}

/// Maximum number of push retries after a conflict (412 Precondition Failed).
/// Each retry pulls the latest remote, merges, and reattempts the push.
const MAX_PUSH_RETRIES: usize = 3;

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
    pub async fn pull(&self, local: &mut Store, _actor: &str) -> Result<Option<TodoDoc>> {
        let etag_lock = self.etag.lock().await.clone();
        let fetched = self
            .client
            .fetch(&self.gist_id, etag_lock.as_deref())
            .await?;
        match fetched {
            FetchResult::NotModified { etag } => {
                // Cache hit: remote unchanged since our last fetch. Keep
                // local state as-is — no merge work, no timestamp churn.
                *self.etag.lock().await = etag;
                Ok(None)
            }
            FetchResult::Modified { doc: remote, etag: new_etag } => {
                // Build a local snapshot doc for merging. We use a temporary
                // store snapshot via clone+into_doc to carry both todos and
                // categories.
                let local_doc = local.clone().into_doc("local");
                let merged = merge(&local_doc, &remote);
                *local = Store::from_doc(merged.clone());
                *self.etag.lock().await = new_etag;
                Ok(Some(merged))
            }
        }
    }

    /// Push the current `local` store to the gist.
    ///
    /// Uses `If-Match` optimistic concurrency: if the remote moved on since
    /// our last fetch, the server returns 412 → [`CoreError::Conflict`].
    /// In that case we pull the latest remote, 3-way merge, and retry — up
    /// to [`MAX_PUSH_RETRIES`] times. If we still cannot converge, the
    /// final error is returned (so the API layer can mark `synced: false`).
    pub async fn push(&self, local: &mut Store, actor: &str) -> Result<()> {
        for _attempt in 0..MAX_PUSH_RETRIES {
            let doc = local.clone().into_doc(actor);
            let etag_lock = self.etag.lock().await.clone();
            match self
                .client
                .push(&self.gist_id, &doc, etag_lock.as_deref())
                .await
            {
                Ok(new_etag) => {
                    *self.etag.lock().await = new_etag;
                    return Ok(());
                }
                Err(crate::error::CoreError::Conflict) => {
                    // Remote moved on. Pull-merge into local and retry.
                    self.pull(local, actor).await?;
                    continue;
                }
                Err(other) => return Err(other),
            }
        }
        // Exhausted retries: surface the conflict so callers can flag it.
        Err(crate::error::CoreError::Conflict)
    }
}
