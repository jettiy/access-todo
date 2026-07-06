use chrono::Utc;
use todo_core::model::{Priority, Todo, TodoDoc};

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
