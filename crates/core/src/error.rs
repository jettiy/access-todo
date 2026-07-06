//! Error types for the core crate.

use thiserror::Error;

/// All errors produced by the core crate.
#[derive(Debug, Error)]
pub enum CoreError {
    /// A todo with the given id was not found.
    #[error("todo not found: {0}")]
    NotFound(String),
    /// An HTTP-level error talking to the Gist API.
    #[error("gist HTTP error: {0}")]
    GistHttp(String),
    /// Optimistic concurrency conflict (ETag mismatch).
    #[error("gist conflict (ETag mismatch)")]
    Conflict,
    /// The document failed schema validation.
    #[error("schema validation error: {0}")]
    Schema(String),
    /// An underlying I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A JSON serialization/deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// An HTTP transport error from reqwest.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    /// An invalid header value was provided.
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, CoreError>;
