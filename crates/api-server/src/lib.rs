//! api-server: REST HTTP server (localhost-only) exposing the TODO store.
//!
//! Every mutation reads the `X-Agent` header and records it in the
//! resulting todo's audit fields, so caller attribution is preserved.

mod auth;
mod routes;
mod state;

pub use routes::router;
pub use state::AppState;

/// Build a router backed by a fresh empty store (for tests).
pub fn app_for_test() -> axum::Router {
    routes::router(AppState::new())
}
