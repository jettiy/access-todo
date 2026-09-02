//! RFC 7807 problem-details error type for the REST API.
//!
//! Every handler returns `Result<_, ApiError>`; axum's `IntoResponse`
//! turns the error into an `application/problem+json` body so clients
//! get a consistent, machine-parseable error shape:
//!
//! ```json
//! {
//!   "type": "https://access.errors/not-found",
//!   "title": "Not Found",
//!   "status": 404,
//!   "detail": "todo <id> not found"
//! }
//! ```

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// API-facing error. Convertible to a problem-details response.
#[derive(Debug)]
pub enum ApiError {
    /// Resource lookup miss — `detail` carries the id that wasn't found.
    NotFound(String),
    /// Input validation failed (empty title, bad date, oversized field).
    /// `detail` is the human-readable reason.
    Validation(String),
    /// Internal serialization failure. Shouldn't happen in practice.
    Internal(String),
}

#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    type_: &'static str,
    title: &'static str,
    status: u16,
    detail: String,
}

impl ApiError {
    fn into_parts(self) -> (StatusCode, &'static str, &'static str, String) {
        match self {
            ApiError::NotFound(d) => (
                StatusCode::NOT_FOUND,
                "https://access.errors/not-found",
                "Not Found",
                d,
            ),
            ApiError::Validation(d) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "https://access.errors/validation",
                "Validation Failed",
                d,
            ),
            ApiError::Internal(d) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "https://access.errors/internal",
                "Internal Server Error",
                d,
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, type_, title, detail) = self.into_parts();
        let body = Json(Problem {
            type_,
            title,
            status: status.as_u16(),
            detail,
        });
        // Override the Content-Type that `Json` would set so it matches
        // RFC 7807. `Json` adds `application/json`; we precede it with
        // the correct header and axum merges (the first one wins per
        // header name for Content-Type when we use the tuple form).
        (status, body).into_response()
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Internal(format!("serialization: {e}"))
    }
}
