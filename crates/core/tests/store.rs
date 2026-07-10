use todo_core::model::Priority;
use todo_core::store::{DevNoteInput, DevNotePatch, NoteFilter, Store, TodoInput, TodoPatch};

fn user_store() -> Store {
    let mut s = Store::new();
    s.add(
        TodoInput {
            title: "A".into(),
            note: None,
            priority: Priority::Medium,
            due_date: Some("2099-01-01".into()),
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    s
}

#[test]
fn add_and_list() {
    let s = user_store();
    assert_eq!(s.list().len(), 1);
    assert_eq!(s.list()[0].title, "A");
}

#[test]
fn toggle_marks_done_with_actor() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    let t = s.toggle(&id, "hermes").unwrap();
    assert!(t.done);
    assert_eq!(t.completed_by.as_deref(), Some("hermes"));
    assert!(t
        .history
        .iter()
        .any(|h| h.action == "checked" && h.by == "hermes"));
}

#[test]
fn toggle_off_clears_completion() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    s.toggle(&id, "user").unwrap();
    let t = s.toggle(&id, "hermes").unwrap();
    assert!(!t.done);
    assert!(t.completed_at.is_none());
    assert!(t.history.iter().any(|h| h.action == "unchecked"));
}

#[test]
fn update_records_updated_by() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    let t = s
        .update(
            &id,
            TodoPatch {
                title: Some("B".into()),
                ..Default::default()
            },
            "zcode",
        )
        .unwrap();
    assert_eq!(t.title, "B");
    assert_eq!(t.updated_by.as_deref(), Some("zcode"));
}

#[test]
fn update_changes_category_id() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    // initially no category
    assert!(s.get(&id).unwrap().category_id.is_none());
    // set category via patch
    let t = s
        .update(
            &id,
            TodoPatch {
                category_id: Some(Some("cat-test-123".into())),
                ..Default::default()
            },
            "hermes",
        )
        .unwrap();
    assert_eq!(t.category_id.as_deref(), Some("cat-test-123"));
    // clear category via patch
    let t = s
        .update(
            &id,
            TodoPatch {
                category_id: Some(None),
                ..Default::default()
            },
            "hermes",
        )
        .unwrap();
    assert!(t.category_id.is_none());
}

#[test]
fn delete_removes() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    s.delete(&id, "user").unwrap();
    assert!(s.list().is_empty());
}

#[test]
fn search_matches_title_note_and_tag() {
    let mut s = Store::new();
    s.add(
        TodoInput {
            title: "Buy milk".into(),
            note: Some("from the corner shop".into()),
            priority: Priority::Low,
            due_date: None,
            tags: vec!["errand".into()],
            category_id: None,
        },
        "user",
    );
    assert_eq!(s.search("buy").len(), 1);
    assert_eq!(s.search("corner").len(), 1);
    assert_eq!(s.search("errand").len(), 1);
    assert_eq!(s.search("nothing").len(), 0);
}

#[test]
fn list_today_filters_unmatched() {
    let mut s = Store::new();
    let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
    s.add(
        TodoInput {
            title: "today item".into(),
            note: None,
            priority: Priority::High,
            due_date: Some(today),
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    s.add(
        TodoInput {
            title: "other item".into(),
            note: None,
            priority: Priority::Low,
            due_date: Some("1999-01-01".into()),
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    let todays = s.list_today();
    assert_eq!(todays.len(), 1);
    assert_eq!(todays[0].title, "today item");
}

// ── Dev Note tests ──

fn note_store_with_todo() -> (Store, String) {
    let mut s = Store::new();
    let t = s.add(
        TodoInput {
            title: "적 패턴".into(),
            note: None,
            priority: Priority::Medium,
            due_date: None,
            tags: vec!["system:combat".into(), "version:v0.2".into()],
            category_id: None,
        },
        "zcode",
    );
    s.complete_with_summary(&t.id, "상태머신 추가", "zcode").unwrap();
    let id = s.list()[0].id.clone();
    (s, id)
}

#[test]
fn add_devnote_and_list() {
    let mut s = Store::new();
    let note = s.add_devnote(
        DevNoteInput {
            note_type: todo_core::model::NoteType::Retrospect,
            title: "밸런스 회고".into(),
            body: "적 체력 너무 높음".into(),
            game_system: Some(todo_core::model::GameSystem::Combat),
            version: Some("v0.2".into()),
            tags: vec!["agent:zcode".into()],
        },
        "zcode",
    );
    assert!(note.id.starts_with("note-"));
    assert_eq!(note.created_by, "zcode");

    let listed = s.list_devnotes(NoteFilter::default());
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "밸런스 회고");
}

#[test]
fn update_devnote_body() {
    let mut s = Store::new();
    let n = s.add_devnote(
        DevNoteInput {
            note_type: todo_core::model::NoteType::Issue,
            title: "버그".into(),
            body: "원본".into(),
            game_system: None,
            version: None,
            tags: vec![],
        },
        "zcode",
    );
    let updated = s.update_devnote(
        &n.id,
        DevNotePatch {
            body: Some("수정됨".into()),
            ..Default::default()
        },
        "hermes",
    ).unwrap();
    assert_eq!(updated.body, "수정됨");
    assert_eq!(updated.updated_by.as_deref(), Some("hermes"));
}

#[test]
fn delete_devnote() {
    let mut s = Store::new();
    let n = s.add_devnote(
        DevNoteInput {
            note_type: todo_core::model::NoteType::Retrospect,
            title: "x".into(),
            body: "".into(),
            game_system: None,
            version: None,
            tags: vec![],
        },
        "zcode",
    );
    s.delete_devnote(&n.id).unwrap();
    assert_eq!(s.list_devnotes(NoteFilter::default()).len(), 0);
}

#[test]
fn generate_worklog_from_completed_todos() {
    let (mut s, _todo_id) = note_store_with_todo();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let result = s.generate_worklog(&today, None, "zcode");
    assert!(result.created);
    assert_eq!(result.todos_counted, 1);
    assert!(result.systems.contains(&"combat".to_string()));

    let notes = s.list_devnotes(NoteFilter {
        note_type: Some(todo_core::model::NoteType::Worklog),
        ..Default::default()
    });
    assert_eq!(notes.len(), 1);
    assert!(notes[0].body.contains("Combat"));
}

#[test]
fn generate_worklog_is_idempotent() {
    let (mut s, _todo_id) = note_store_with_todo();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let r1 = s.generate_worklog(&today, None, "zcode");
    assert!(r1.created);
    let r2 = s.generate_worklog(&today, None, "zcode");
    assert!(!r2.created);
    assert_eq!(r2.todos_counted, 1);
    let notes = s.list_devnotes(NoteFilter {
        note_type: Some(todo_core::model::NoteType::Worklog),
        ..Default::default()
    });
    assert_eq!(notes.len(), 1);
}

#[test]
fn filter_devnotes_by_system() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: todo_core::model::NoteType::Issue, title: "a".into(), body: "".into(),
        game_system: Some(todo_core::model::GameSystem::Combat), version: None, tags: vec![],
    }, "zcode");
    s.add_devnote(DevNoteInput {
        note_type: todo_core::model::NoteType::Issue, title: "b".into(), body: "".into(),
        game_system: Some(todo_core::model::GameSystem::Ui), version: None, tags: vec![],
    }, "zcode");
    let combat_only = s.list_devnotes(NoteFilter {
        game_system: Some(todo_core::model::GameSystem::Combat),
        ..Default::default()
    });
    assert_eq!(combat_only.len(), 1);
    assert_eq!(combat_only[0].title, "a");
}

#[test]
fn into_doc_carries_dev_notes() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: todo_core::model::NoteType::Retrospect, title: "x".into(), body: "".into(),
        game_system: None, version: None, tags: vec![],
    }, "zcode");
    let doc = s.into_doc("zcode");
    assert_eq!(doc.dev_notes.len(), 1);
}

#[test]
fn from_doc_restores_dev_notes() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: todo_core::model::NoteType::Retrospect, title: "x".into(), body: "".into(),
        game_system: None, version: None, tags: vec![],
    }, "zcode");
    let doc = s.clone().into_doc("zcode");
    let s2 = Store::from_doc(doc);
    assert_eq!(s2.list_devnotes(NoteFilter::default()).len(), 1);
}
