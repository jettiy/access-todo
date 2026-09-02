//! In-memory TODO store with CRUD operations and history tracking.
//!
//! All mutations record the acting agent in `created_by`/`updated_by`/
//! `completed_by` and append to the per-todo `history` array, which is
//! what makes agent activity auditable.

use chrono::Utc;

use crate::error::{CoreError, Result};
use crate::model::{new_cat_id, new_id, Category, HistoryEntry, Priority, Todo, TodoDoc};

/// Maximum number of history entries kept per todo. Older entries are
/// dropped on every push so a heavily-toggled todo can't grow unboundedly
/// (and bloat the Gist payload past GitHub's 1 MB file limit).
pub const MAX_HISTORY: usize = 50;

/// Append `entry` to `history`, then trim to the newest [`MAX_HISTORY`]
/// entries. Keeps the audit trail useful without letting it grow forever.
fn push_history(history: &mut Vec<HistoryEntry>, entry: HistoryEntry) {
    history.push(entry);
    let cutoff = history.len().saturating_sub(MAX_HISTORY);
    if cutoff > 0 {
        history.drain(0..cutoff);
    }
}

/// Input for creating a new todo.
#[derive(Debug, Clone, Default)]
pub struct TodoInput {
    pub title: String,
    pub note: Option<String>,
    pub priority: Priority,
    pub due_date: Option<String>,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
}

/// Per-field limits enforced by [`TodoInput::validate`] / [`TodoPatch::validate`].
pub const MAX_TITLE_LEN: usize = 200;
pub const MAX_NOTE_LEN: usize = 4000;

impl TodoInput {
    /// Validate the input, normalizing `title` (trim) in place. Returns
    /// the first violation as a human-readable string so callers can map
    /// it to a 422 body. Empty/whitespace titles, oversized fields, and
    /// malformed `due_date` (must be `YYYY-MM-DD`) are rejected.
    pub fn validate(&mut self) -> std::result::Result<(), String> {
        self.title = self.title.trim().to_string();
        if self.title.is_empty() {
            return Err("title must not be empty".into());
        }
        if self.title.chars().count() > MAX_TITLE_LEN {
            return Err(format!("title must be ≤ {MAX_TITLE_LEN} chars"));
        }
        if let Some(n) = self.note.as_ref() {
            if n.chars().count() > MAX_NOTE_LEN {
                return Err(format!("note must be ≤ {MAX_NOTE_LEN} chars"));
            }
        }
        if let Some(dd) = self.due_date.as_ref() {
            if !is_valid_yyyy_mm_dd(dd) {
                return Err("due_date must be YYYY-MM-DD".into());
            }
        }
        Ok(())
    }
}

/// Cheap `YYYY-MM-DD` shape check (does not validate the actual date —
/// `2026-13-45` would pass. We only need to keep obvious garbage out;
/// full calendar validation is the client's job).
fn is_valid_yyyy_mm_dd(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
}


/// Partial update for a todo. `None` means "leave unchanged"; for the
/// nullable `note`/`due_date`/`category_id` fields use `Some(None)` to clear.
#[derive(Debug, Clone, Default)]
pub struct TodoPatch {
    pub title: Option<String>,
    pub note: Option<Option<String>>,
    pub priority: Option<Priority>,
    pub due_date: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub category_id: Option<Option<String>>,
}

/// In-memory store of todos and categories. Cloneable so servers can snapshot safely.
#[derive(Debug, Clone, Default)]
pub struct Store {
    todos: Vec<Todo>,
    categories: Vec<Category>,
}

impl Store {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate over non-deleted todos (tombstones filtered out). All public
    /// read paths go through this so soft-deleted rows stay invisible to
    /// clients while still being carried in the doc for sync.
    fn iter_active(&self) -> impl Iterator<Item = &Todo> {
        self.todos.iter().filter(|t| t.deleted_at.is_none())
    }

    /// Return a snapshot of all todos (excluding tombstones).
    pub fn list(&self) -> Vec<Todo> {
        self.iter_active().cloned().collect()
    }

    /// Return all todos including tombstones — used by sync/merge so
    /// deletions propagate. Most callers want [`Store::list`] instead.
    pub fn list_with_tombstones(&self) -> Vec<Todo> {
        self.todos.clone()
    }

    /// Return todos due today (local timezone) and not yet done.
    /// `due_date` is a user-facing YYYY-MM-DD string interpreted in the
    /// machine's local timezone, so we compare against the local date.
    pub fn list_today(&self) -> Vec<Todo> {
        let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        self.iter_active()
            .filter(|t| t.due_date.as_deref() == Some(&today) && !t.done)
            .cloned()
            .collect()
    }

    /// Return a single todo by id, if present and not deleted.
    pub fn get(&self, id: &str) -> Option<Todo> {
        self.iter_active().find(|t| t.id == id).cloned()
    }

    /// Return a todo by id even if soft-deleted (for tombstone-aware code).
    pub fn get_with_tombstone(&self, id: &str) -> Option<Todo> {
        self.todos.iter().find(|t| t.id == id).cloned()
    }

    /// Add a new todo. Records `created` history entry.
    pub fn add(&mut self, i: TodoInput, actor: &str) -> Todo {
        let now = Utc::now();
        let todo = Todo {
            id: new_id(),
            title: i.title,
            note: i.note,
            summary: None,
            deleted_at: None,
            deleted_by: None,
            done: false,
            priority: i.priority,
            due_date: i.due_date,
            tags: i.tags,
            category_id: i.category_id,
            created_at: now,
            created_by: actor.into(),
            completed_at: None,
            completed_by: None,
            updated_at: None,
            updated_by: None,
            history: vec![HistoryEntry {
                action: "created".into(),
                at: now,
                by: actor.into(),
            }],
        };
        self.todos.push(todo.clone());
        todo
    }

    // ── Category CRUD ──

    /// Create a new category for an agent. Returns the created category.
    pub fn add_category(&mut self, agent: &str, name: &str) -> Category {
        let order = self.categories.iter()
            .filter(|c| c.agent == agent)
            .count() as i32;
        let cat = Category {
            id: new_cat_id(),
            agent: agent.into(),
            name: name.into(),
            order,
            updated_at: Some(Utc::now()),
        };
        self.categories.push(cat.clone());
        cat
    }

    /// Rename a category by id (O(1), no todo changes needed).
    pub fn rename_category(&mut self, id: &str, name: &str) -> Result<Category> {
        let c = self.categories.iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("category {id}")))?;
        c.name = name.into();
        c.updated_at = Some(Utc::now());
        Ok(c.clone())
    }

    /// Delete a category by id. Todos referencing it get category_id cleared
    /// (moved to "uncategorized") rather than deleted.
    pub fn delete_category(&mut self, id: &str) -> Result<()> {
        let before = self.categories.len();
        self.categories.retain(|c| c.id != id);
        if self.categories.len() == before {
            return Err(CoreError::NotFound(format!("category {id}")));
        }
        // Clear category_id on todos that referenced this category.
        for t in self.todos.iter_mut() {
            if t.category_id.as_deref() == Some(id) {
                t.category_id = None;
            }
        }
        Ok(())
    }

    /// Reorder categories for an agent. `ordered_ids` is the desired order.
    pub fn reorder_categories(&mut self, agent: &str, ordered_ids: &[String]) -> Result<()> {
        let now = Utc::now();
        for (i, id) in ordered_ids.iter().enumerate() {
            let c = self.categories.iter_mut()
                .find(|c| c.id == *id && c.agent == agent)
                .ok_or_else(|| CoreError::NotFound(format!("category {id}")))?;
            c.order = i as i32;
            c.updated_at = Some(now);
        }
        Ok(())
    }

    /// List categories for an agent, ordered by `order` field.
    pub fn list_categories(&self, agent: &str) -> Vec<Category> {
        let mut cats: Vec<Category> = self.categories.iter()
            .filter(|c| c.agent == agent)
            .cloned()
            .collect();
        cats.sort_by_key(|c| c.order);
        cats
    }

    /// Apply a partial update to a todo. Records `updated` history entry.
    pub fn update(&mut self, id: &str, p: TodoPatch, actor: &str) -> Result<Todo> {
        let now = Utc::now();
        let t = self
            .todos
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| CoreError::NotFound(id.into()))?;
        if let Some(v) = p.title {
            t.title = v;
        }
        if let Some(v) = p.note {
            t.note = v;
        }
        if let Some(v) = p.priority {
            t.priority = v;
        }
        if let Some(v) = p.due_date {
            t.due_date = v;
        }
        if let Some(v) = p.tags {
            t.tags = v;
        }
        if let Some(v) = p.category_id {
            t.category_id = v;
        }
        t.updated_at = Some(now);
        t.updated_by = Some(actor.into());
        push_history(&mut t.history, HistoryEntry {
            action: "updated".into(),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Toggle the done state. Sets/clears `completed_at`/`completed_by`.
    pub fn toggle(&mut self, id: &str, actor: &str) -> Result<Todo> {
        let now = Utc::now();
        let t = self
            .todos
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| CoreError::NotFound(id.into()))?;
        t.done = !t.done;
        let action = if t.done { "checked" } else { "unchecked" };
        if t.done {
            t.completed_at = Some(now);
            t.completed_by = Some(actor.into());
        } else {
            t.completed_at = None;
            t.completed_by = None;
        }
        t.updated_at = Some(now);
        t.updated_by = Some(actor.into());
        push_history(&mut t.history, HistoryEntry {
            action: action.into(),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Mark a todo done and record a work summary (what was done).
    ///
    /// The summary is written to the dedicated `summary` field so the
    /// user's `note` is preserved verbatim, and the same text is appended
    /// to `history` for audit. Re-completing overwrites the previous
    /// summary (latest completion wins).
    pub fn complete_with_summary(
        &mut self,
        id: &str,
        summary: &str,
        actor: &str,
    ) -> Result<Todo> {
        let now = Utc::now();
        let t = self
            .todos
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| CoreError::NotFound(id.into()))?;
        t.done = true;
        t.completed_at = Some(now);
        t.completed_by = Some(actor.into());
        t.updated_at = Some(now);
        t.updated_by = Some(actor.into());
        t.summary = Some(summary.into());
        push_history(&mut t.history, HistoryEntry {
            action: format!("completed: {summary}"),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Soft-delete a todo by id. The row is kept in the doc as a tombstone
    /// (with `deleted_at` / `deleted_by` set) so the deletion can propagate
    /// to other replicas via merge. [`Store::list`] / [`Store::get`] hide
    /// it from clients; use [`Store::prune_tombstones`] to actually free
    /// the memory after the deletion has had time to sync.
    pub fn delete(&mut self, id: &str, actor: &str) -> Result<Todo> {
        let now = Utc::now();
        let t = self
            .todos
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| CoreError::NotFound(id.into()))?;
        t.deleted_at = Some(now);
        t.deleted_by = Some(actor.into());
        t.updated_at = Some(now);
        t.updated_by = Some(actor.into());
        push_history(&mut t.history, HistoryEntry {
            action: "deleted".into(),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Permanently drop tombstones older than `older_than`. Returns the
    /// number of rows pruned. Safe to call periodically once you're
    /// confident every replica has pulled past the deletion.
    pub fn prune_tombstones(&mut self, older_than: chrono::Duration) -> usize {
        let cutoff = Utc::now() - older_than;
        let before = self.todos.len();
        self.todos.retain(|t| {
            // keep if not deleted, or if deleted but still warmer than cutoff
            t.deleted_at.map_or(true, |d| d > cutoff)
        });
        before - self.todos.len()
    }

    /// Case-insensitive search across title, note, and tags (tombstones
    /// excluded).
    pub fn search(&self, q: &str) -> Vec<Todo> {
        let ql = q.to_lowercase();
        self.iter_active()
            .filter(|t| {
                t.title.to_lowercase().contains(&ql)
                    || t.note
                        .as_deref()
                        .map(|n| n.to_lowercase().contains(&ql))
                        .unwrap_or(false)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&ql))
            })
            .cloned()
            .collect()
    }

    /// Consume the store into a `TodoDoc` stamped with `actor`.
    pub fn into_doc(self, actor: &str) -> TodoDoc {
        TodoDoc {
            version: "1.0".into(),
            updated_at: Utc::now(),
            updated_by: actor.into(),
            todos: self.todos,
            categories: self.categories,
        }
    }

    /// Build a store from a `TodoDoc`.
    pub fn from_doc(doc: TodoDoc) -> Self {
        Self {
            todos: doc.todos,
            categories: doc.categories,
        }
    }
}
