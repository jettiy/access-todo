//! Data model: `Todo`, `TodoDoc`, `Priority`, `HistoryEntry`.
//!
//! These types form the serialization contract shared by every client
//! (Rust servers, Tauri desktop widget, Android app). The shape is defined
//! in `shared/schema.json`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Priority level for a todo. Serializes as lowercase strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

/// A single entry in a todo's change history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub action: String,
    pub at: DateTime<Utc>,
    pub by: String,
}

/// A user-defined category for grouping todos within an agent's post-it.
/// Each category belongs to one agent and has a stable id so rename is O(1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    /// Owning agent name (hermes/omp/zcode/user).
    pub agent: String,
    pub name: String,
    /// Display order within the agent (lower = first).
    pub order: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// A single todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub done: bool,
    pub priority: Priority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

/// The top-level document stored in the Gist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoDoc {
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[serde(default)]
    pub todos: Vec<Todo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,
}

impl TodoDoc {
    /// Create an empty document owned by `actor`.
    pub fn new(actor: &str) -> Self {
        Self {
            version: "1.0".into(),
            updated_at: Utc::now(),
            updated_by: actor.into(),
            todos: vec![],
            categories: vec![],
        }
    }
}

/// Generate a fresh UUID v4 string for a new todo.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a category id (prefixed for readability in the JSON).
pub fn new_cat_id() -> String {
    format!("cat-{}", Uuid::new_v4())
}
