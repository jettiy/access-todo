//! 3-way merge of two `TodoDoc`s by todo id.
//!
//! Used by the sync engine to reconcile local and remote Gist state.
//! Conflict resolution: same id, latest `updated_at` (or `created_at`
//! fallback) wins. Disjoint ids are unioned. Output is sorted by id for
//! deterministic comparisons.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::model::{Todo, TodoDoc};

/// Effective change timestamp for a todo: `updated_at` if set, else
/// `created_at`, else the Unix epoch (oldest possible).
fn ts(t: &Todo) -> DateTime<Utc> {
    t.updated_at.or(Some(t.created_at)).unwrap_or(DateTime::UNIX_EPOCH)
}

/// Merge two documents into a fresh document.
///
/// The merged document's `updated_by` is `"merge"` and `updated_at` is now;
/// clients should overwrite these when persisting.
pub fn merge(local: &TodoDoc, remote: &TodoDoc) -> TodoDoc {
    let mut map: HashMap<String, Todo> = HashMap::new();
    for t in &local.todos {
        map.insert(t.id.clone(), t.clone());
    }
    for t in &remote.todos {
        match map.get(&t.id) {
            Some(existing) => {
                if ts(t) >= ts(existing) {
                    map.insert(t.id.clone(), t.clone());
                }
            }
            None => {
                map.insert(t.id.clone(), t.clone());
            }
        }
    }
    let mut todos: Vec<Todo> = map.into_values().collect();
    todos.sort_by(|a, b| a.id.cmp(&b.id));
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "merge".into(),
        todos,
    }
}
