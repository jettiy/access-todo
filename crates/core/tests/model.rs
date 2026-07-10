use chrono::Utc;
use todo_core::model::{DevNote, GameSystem, NoteType, Priority, Todo, TodoDoc};

#[test]
fn serde_roundtrip_todo() {
    let todo = Todo {
        id: "11111111-1111-1111-1111-111111111111".into(),
        title: "장보기".into(),
        note: Some("우유".into()),
        done: false,
        priority: Priority::High,
        due_date: Some("2026-07-07".into()),
        tags: vec!["errand".into()],
        category_id: None,
        created_at: Utc::now(),
        created_by: "user".into(),
        completed_at: None,
        completed_by: None,
        updated_at: None,
        updated_by: None,
        history: vec![],
    };
    let json = serde_json::to_string(&todo).unwrap();
    let back: Todo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.title, "장보기");
    assert!(matches!(back.priority, Priority::High));
}

#[test]
fn doc_has_version_one() {
    let doc = TodoDoc::new("user");
    let json = serde_json::to_string(&doc).unwrap();
    assert!(json.contains("\"1.0\""));
    assert!(json.contains("\"todos\":[]"));
}

#[test]
fn priority_serializes_lowercase() {
    let p = serde_json::to_string(&Priority::High).unwrap();
    assert_eq!(p, "\"high\"");
    let back: Priority = serde_json::from_str("\"medium\"").unwrap();
    assert!(matches!(back, Priority::Medium));
}

#[test]
fn devnote_serializes_with_lowercase_enums() {
    let note = DevNote {
        id: "note-test".into(),
        note_type: NoteType::Worklog,
        title: "테스트".into(),
        body: "본문".into(),
        game_system: Some(GameSystem::Combat),
        version: Some("v0.1".into()),
        linked_todos: vec!["todo-1".into()],
        tags: vec!["agent:zcode".into()],
        created_at: Utc::now(),
        created_by: "zcode".into(),
        updated_at: None,
        updated_by: None,
    };
    let json = serde_json::to_string(&note).unwrap();
    assert!(json.contains("\"note_type\":\"worklog\""));
    assert!(json.contains("\"game_system\":\"combat\""));
}

#[test]
fn empty_dev_notes_omits_field_in_doc() {
    let doc = TodoDoc::new("zcode");
    let json = serde_json::to_string(&doc).unwrap();
    assert!(!json.contains("dev_notes"));
}

#[test]
fn devnote_round_trip() {
    let note = DevNote {
        id: "note-x".into(),
        note_type: NoteType::Issue,
        title: "버그".into(),
        body: "벽 뚫림".into(),
        game_system: Some(GameSystem::Ai),
        version: None,
        linked_todos: vec![],
        tags: vec![],
        created_at: Utc::now(),
        created_by: "hermes".into(),
        updated_at: None,
        updated_by: None,
    };
    let json = serde_json::to_string(&note).unwrap();
    let back: DevNote = serde_json::from_str(&json).unwrap();
    assert_eq!(back.note_type, NoteType::Issue);
    assert_eq!(back.game_system, Some(GameSystem::Ai));
}

#[test]
fn new_note_id_has_prefix() {
    let id = todo_core::model::new_note_id();
    assert!(id.starts_with("note-"));
}
