//! Agent identification from request headers.

use axum::http::HeaderMap;

/// Extract the acting agent from the `X-Agent` header (default: `"unknown"`).
pub fn agent_from_headers(h: &HeaderMap) -> String {
    h.get("X-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}
