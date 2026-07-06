//! Shared application state: an Arc<Mutex<Store>>.

use std::sync::Arc;

use tokio::sync::Mutex;

use todo_core::store::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
