//! In-memory TODO store with CRUD operations and history tracking.
//!
//! All mutations record the acting agent in `created_by`/`updated_by`/
//! `completed_by` and append to the per-todo `history` array, which is
//! what makes agent activity auditable.

use chrono::Utc;

use crate::error::{CoreError, Result};
use crate::model::{new_cat_id, new_id, Category, HistoryEntry, Priority, Todo, TodoDoc};

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

/// Partial update for a todo. `None` means "leave unchanged"; for the
/// nullable `note`/`due_date` fields use `Some(None)` to clear.
#[derive(Debug, Clone, Default)]
pub struct TodoPatch {
    pub title: Option<String>,
    pub note: Option<Option<String>>,
    pub priority: Option<Priority>,
    pub due_date: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
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

    /// Return a snapshot of all todos.
    pub fn list(&self) -> Vec<Todo> {
        self.todos.clone()
    }

    /// Return todos due today (local timezone) and not yet done.
    /// `due_date` is a user-facing YYYY-MM-DD string interpreted in the
    /// machine's local timezone, so we compare against the local date.
    pub fn list_today(&self) -> Vec<Todo> {
        let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
        self.todos
            .iter()
            .filter(|t| t.due_date.as_deref() == Some(&today) && !t.done)
            .cloned()
            .collect()
    }

    /// Return a single todo by id, if present.
    pub fn get(&self, id: &str) -> Option<Todo> {
        self.todos.iter().find(|t| t.id == id).cloned()
    }

    /// Add a new todo. Records `created` history entry.
    pub fn add(&mut self, i: TodoInput, actor: &str) -> Todo {
        let now = Utc::now();
        let todo = Todo {
            id: new_id(),
            title: i.title,
            note: i.note,
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
        t.updated_at = Some(now);
        t.updated_by = Some(actor.into());
        t.history.push(HistoryEntry {
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
        t.history.push(HistoryEntry {
            action: action.into(),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Mark a todo done and record a work summary (what was done).
    /// The summary replaces the note with a "✓ <summary>" prefix and is
    /// appended to history so the post-it shows what changed.
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
        // 요약을 note에 기록 (기존 note 보존)
        let prev_note = t.note.take().unwrap_or_default();
        let combined = if prev_note.is_empty() {
            format!("✓ {summary}")
        } else {
            format!("{prev_note}\n✓ {summary}")
        };
        t.note = Some(combined);
        t.history.push(HistoryEntry {
            action: format!("completed: {summary}"),
            at: now,
            by: actor.into(),
        });
        Ok(t.clone())
    }

    /// Delete a todo by id.
    pub fn delete(&mut self, id: &str, _actor: &str) -> Result<()> {
        let before = self.todos.len();
        self.todos.retain(|t| t.id != id);
        if self.todos.len() == before {
            return Err(CoreError::NotFound(id.into()));
        }
        Ok(())
    }

    /// Case-insensitive search across title, note, and tags.
    pub fn search(&self, q: &str) -> Vec<Todo> {
        let ql = q.to_lowercase();
        self.todos
            .iter()
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
