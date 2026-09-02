//! GitHub Gist HTTP client with ETag-based optimistic concurrency.
//!
//! Stores and retrieves the `todos.json` file in a single secret gist.

use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, ETAG, IF_MATCH, IF_NONE_MATCH,
};
use serde::Deserialize;

use crate::error::{CoreError, Result};
use crate::model::TodoDoc;

const GITHUB_BASE: &str = "https://api.github.com";

#[derive(Deserialize)]
struct GistResp {
    files: std::collections::HashMap<String, GistFile>,
}

#[derive(Deserialize)]
struct GistFile {
    content: String,
}

/// Outcome of a conditional fetch.
///
/// `NotModified` means the server returned 304 against our last ETag —
/// callers should keep their local state untouched and skip merging.
pub enum FetchResult {
    /// The remote document changed; here is the new state (and ETag, if any).
    Modified { doc: TodoDoc, etag: Option<String> },
    /// The remote is unchanged since the supplied ETag. No body to merge.
    NotModified { etag: Option<String> },
}

/// HTTP client for a GitHub Gist containing `todos.json`.
pub struct GistClient {
    token: String,
    base: String,
    http: reqwest::Client,
}

impl GistClient {
    /// Create a client targeting the real GitHub API.
    pub fn new(token: String) -> Self {
        Self::with_base(token, GITHUB_BASE.into())
    }

    /// Create a client with a custom API base (used for tests).
    pub fn with_base(token: String, base: String) -> Self {
        // GitHub API requires a User-Agent header on all requests.
        let http = reqwest::Client::builder()
            .user_agent("desktop-todo-agents/0.1 (https://github.com/jettiy)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            token,
            base,
            http,
        }
    }

    fn auth_headers(&self, extra: HeaderMap) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Ok(auth) = HeaderValue::from_str(&format!("Bearer {}", self.token)) {
            h.insert(AUTHORIZATION, auth);
        }
        h.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        h.extend(extra);
        h
    }

    /// Fetch the gist and parse `todos.json`. Returns a [`FetchResult`] so
    /// the caller can distinguish a real change from a cache hit (304) and
    /// skip the merge work in the latter case. `etag` enables conditional
    /// requests via `If-None-Match`.
    pub async fn fetch(
        &self,
        gist_id: &str,
        etag: Option<&str>,
    ) -> Result<FetchResult> {
        let mut extra = HeaderMap::new();
        if let Some(e) = etag {
            extra.insert(IF_NONE_MATCH, HeaderValue::from_str(e)?);
        }
        let resp = self
            .http
            .get(format!("{}/gists/{}", self.base, gist_id))
            .headers(self.auth_headers(extra))
            .send()
            .await?;
        let etag_out = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(FetchResult::NotModified { etag: etag_out });
        }
        let body = resp
            .json::<GistResp>()
            .await
            .map_err(|e| CoreError::GistHttp(format!("{}: {e}", status)))?;
        let content = body
            .files
            .get("todos.json")
            .ok_or_else(|| CoreError::GistHttp("todos.json missing".into()))?
            .content
            .clone();
        let doc: TodoDoc = serde_json::from_str(&content)?;
        Ok(FetchResult::Modified { doc, etag: etag_out })
    }

    /// Push (PATCH) the document to the gist with optimistic-concurrency
    /// control: when an `etag` is supplied it is sent as `If-Match`, so a
    /// concurrent remote change yields `412 Precondition Failed` →
    /// [`CoreError::Conflict`] instead of silently clobbering the remote.
    /// Returns the new ETag if any.
    pub async fn push(
        &self,
        gist_id: &str,
        doc: &TodoDoc,
        etag: Option<&str>,
    ) -> Result<Option<String>> {
        let payload = serde_json::json!({
            "files": {
                "todos.json": {
                    "content": serde_json::to_string_pretty(doc)?
                }
            }
        });
        let mut extra = HeaderMap::new();
        if let Some(e) = etag {
            // If-Match lets the server reject the write when the remote has
            // moved on since we last fetched. Empty string is never a valid
            // etag — guard against it so we don't send a header that always
            // fails.
            if !e.is_empty() {
                extra.insert(IF_MATCH, HeaderValue::from_str(e)?);
            }
        }
        let resp = self
            .http
            .patch(format!("{}/gists/{}", self.base, gist_id))
            .headers(self.auth_headers(extra))
            .json(&payload)
            .send()
            .await?;
        let etag_out = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let status = resp.status();
        if status == reqwest::StatusCode::PRECONDITION_FAILED {
            // Drain the body so the connection can be reused.
            let _ = resp.text().await;
            return Err(CoreError::Conflict);
        }
        if !status.is_success() {
            let t = resp.text().await.unwrap_or_default();
            return Err(CoreError::GistHttp(format!("push {status}: {t}")));
        }
        Ok(etag_out)
    }
}
