//! In-memory TODO store with CRUD operations and history tracking.
//!
//! All mutations record the acting agent in `created_by`/`updated_by`/
//! `completed_by` and append to the per-todo `history` array, which is
//! what makes agent activity auditable.

use chrono::Utc;

use crate::error::{CoreError, Result};
use crate::model::{new_cat_id, new_id, new_note_id, Category, DevNote, GameSystem, HistoryEntry, NoteType, Priority, Todo, TodoDoc};

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

/// Input for creating a new dev note.
#[derive(Debug, Clone, Default)]
pub struct DevNoteInput {
    pub note_type: NoteType,
    pub title: String,
    pub body: String,
    pub game_system: Option<GameSystem>,
    pub version: Option<String>,
    pub tags: Vec<String>,
}

/// Partial update for a dev note. `None` means "leave unchanged"; for the
/// nullable `game_system`/`version` fields use `Some(None)` to clear.
#[derive(Debug, Clone, Default)]
pub struct DevNotePatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub game_system: Option<Option<GameSystem>>,
    pub version: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

/// Filter for listing dev notes.
#[derive(Debug, Clone, Default)]
pub struct NoteFilter {
    pub note_type: Option<NoteType>,
    pub game_system: Option<GameSystem>,
    pub version: Option<String>,
}

/// Result of worklog generation.
#[derive(Debug, Clone)]
pub struct WorklogResult {
    pub created: bool,
    pub note_id: String,
    pub todos_counted: usize,
    pub systems: Vec<String>,
}

/// In-memory store of todos, categories, and dev notes.
/// Cloneable so servers can snapshot safely.
#[derive(Debug, Clone, Default)]
pub struct Store {
    todos: Vec<Todo>,
    categories: Vec<Category>,
    dev_notes: Vec<DevNote>,
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

    // ── Dev Note CRUD ──

    /// List dev notes matching the filter (all if filter fields are None).
    pub fn list_devnotes(&self, f: NoteFilter) -> Vec<DevNote> {
        self.dev_notes
            .iter()
            .filter(|n| {
                if let Some(nt) = f.note_type {
                    if n.note_type != nt {
                        return false;
                    }
                }
                if let Some(gs) = f.game_system {
                    if n.game_system != Some(gs) {
                        return false;
                    }
                }
                if let Some(ref ver) = f.version {
                    if n.version.as_deref() != Some(ver.as_str()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Return a single dev note by id.
    pub fn get_devnote(&self, id: &str) -> Option<DevNote> {
        self.dev_notes.iter().find(|n| n.id == id).cloned()
    }

    /// Add a new dev note. Returns the created note.
    pub fn add_devnote(&mut self, i: DevNoteInput, actor: &str) -> DevNote {
        let now = Utc::now();
        let note = DevNote {
            id: new_note_id(),
            note_type: i.note_type,
            title: i.title,
            body: i.body,
            game_system: i.game_system,
            version: i.version,
            linked_todos: vec![],
            tags: i.tags,
            created_at: now,
            created_by: actor.into(),
            updated_at: None,
            updated_by: None,
        };
        self.dev_notes.push(note.clone());
        note
    }

    /// Apply a partial update to a dev note.
    pub fn update_devnote(&mut self, id: &str, p: DevNotePatch, actor: &str) -> Result<DevNote> {
        let now = Utc::now();
        let n = self
            .dev_notes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("devnote {id}")))?;
        if let Some(v) = p.title {
            n.title = v;
        }
        if let Some(v) = p.body {
            n.body = v;
        }
        if let Some(v) = p.game_system {
            n.game_system = v;
        }
        if let Some(v) = p.version {
            n.version = v;
        }
        if let Some(v) = p.tags {
            n.tags = v;
        }
        n.updated_at = Some(now);
        n.updated_by = Some(actor.into());
        Ok(n.clone())
    }

    /// Delete a dev note by id.
    pub fn delete_devnote(&mut self, id: &str) -> Result<()> {
        let before = self.dev_notes.len();
        self.dev_notes.retain(|n| n.id != id);
        if self.dev_notes.len() == before {
            return Err(CoreError::NotFound(format!("devnote {id}")));
        }
        Ok(())
    }

    /// Generate (or update) a Worklog note from completed todos on `date`.
    /// Idempotent: if a Worklog for this date already exists, missing todos
    /// are appended; no duplicates.
    pub fn generate_worklog(
        &mut self,
        date: &str,
        agent: Option<&str>,
        actor: &str,
    ) -> WorklogResult {
        // Collect completed todos for this date.
        let completed: Vec<Todo> = self
            .todos
            .iter()
            .filter(|t| {
                if !t.done {
                    return false;
                }
                let matches_date = t
                    .completed_at
                    .map(|dt| dt.format("%Y-%m-%d").to_string() == date)
                    .unwrap_or(false);
                if !matches_date {
                    return false;
                }
                if let Some(a) = agent {
                    return t.completed_by.as_deref() == Some(a) || t.created_by == a;
                }
                true
            })
            .cloned()
            .collect();

        // Check if a Worklog for this date already exists.
        let existing_idx = self.dev_notes.iter().position(|n| {
            n.note_type == NoteType::Worklog && n.title == format!("{date} 작업 로그")
        });

        let systems: Vec<String> = completed
            .iter()
            .filter_map(|t| {
                crate::devnote::parse_system_tag(&t.tags)
                    .map(|gs| format!("{:?}", gs).to_lowercase())
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let count = completed.len();

        match existing_idx {
            Some(idx) => {
                // Update existing: append any missing linked todos.
                let existing = &mut self.dev_notes[idx];
                for t in &completed {
                    if !existing.linked_todos.contains(&t.id) {
                        existing.linked_todos.push(t.id.clone());
                    }
                }
                // Regenerate body from all linked todos.
                let linked: Vec<Todo> = self
                    .todos
                    .iter()
                    .filter(|t| existing.linked_todos.contains(&t.id))
                    .cloned()
                    .collect();
                let new_note = crate::devnote::generate_worklog(&linked, date, agent);
                existing.body = new_note.body;
                existing.updated_at = Some(Utc::now());
                existing.updated_by = Some(actor.into());
                let note_id = existing.id.clone();
                WorklogResult {
                    created: false,
                    note_id,
                    todos_counted: linked.len(),
                    systems,
                }
            }
            None => {
                let note = crate::devnote::generate_worklog(&completed, date, agent);
                let note_id = note.id.clone();
                self.dev_notes.push(note);
                WorklogResult {
                    created: true,
                    note_id,
                    todos_counted: count,
                    systems,
                }
            }
        }
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
            dev_notes: self.dev_notes,
        }
    }

    /// Build a store from a `TodoDoc`.
    pub fn from_doc(doc: TodoDoc) -> Self {
        Self {
            todos: doc.todos,
            categories: doc.categories,
            dev_notes: doc.dev_notes,
        }
    }
}
