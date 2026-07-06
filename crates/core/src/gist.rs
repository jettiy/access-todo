//! GitHub Gist HTTP client with ETag-based optimistic concurrency.
//!
//! Stores and retrieves the `todos.json` file in a single secret gist.

use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, ETAG, IF_NONE_MATCH,
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

    /// Fetch the gist and parse `todos.json`. Returns the document and the
    /// ETag (if the server provided one) for subsequent conditional requests.
    pub async fn fetch(&self, gist_id: &str, etag: Option<&str>) -> Result<(TodoDoc, Option<String>)> {
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
        Ok((doc, etag_out))
    }

    /// Push (PATCH) the document to the gist. Returns the new ETag if any.
    pub async fn push(
        &self,
        gist_id: &str,
        doc: &TodoDoc,
        _etag: Option<&str>,
    ) -> Result<Option<String>> {
        let payload = serde_json::json!({
            "files": {
                "todos.json": {
                    "content": serde_json::to_string_pretty(doc)?
                }
            }
        });
        let resp = self
            .http
            .patch(format!("{}/gists/{}", self.base, gist_id))
            .headers(self.auth_headers(HeaderMap::new()))
            .json(&payload)
            .send()
            .await?;
        let etag_out = resp
            .headers()
            .get(ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(CoreError::GistHttp(format!("push {s}: {t}")));
        }
        Ok(etag_out)
    }
}
