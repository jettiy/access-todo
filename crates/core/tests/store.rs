use todo_core::model::Priority;
use todo_core::store::{Store, TodoInput, TodoPatch};

fn user_store() -> Store {
    let mut s = Store::new();
    s.add(
        TodoInput {
            title: "A".into(),
            note: None,
            priority: Priority::Medium,
            due_date: Some("2099-01-01".into()),
            tags: vec![],
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
    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();
    s.add(
        TodoInput {
            title: "today item".into(),
            note: None,
            priority: Priority::High,
            due_date: Some(today),
            tags: vec![],
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
        },
        "user",
    );
    let todays = s.list_today();
    assert_eq!(todays.len(), 1);
    assert_eq!(todays[0].title, "today item");
}
