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
fn complete_with_summary_writes_summary_field_only() {
    // summary가 Todo.summary 에만 기록되고, note 는 그대로 보존되어야 한다.
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    let t = s.complete_with_summary(&id, "main.lua 100줄 nil 체크 추가", "omp").unwrap();
    assert!(t.done);
    assert_eq!(t.summary.as_deref(), Some("main.lua 100줄 nil 체크 추가"));
    assert_eq!(t.note, None, "note must not be polluted by summary");
    assert_eq!(t.completed_by.as_deref(), Some("omp"));
    assert!(t.history.iter().any(|h| h.action.starts_with("completed:")));
}

#[test]
fn delete_marks_tombstone_and_hides_from_reads() {
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    let t = s.delete(&id, "omp").unwrap();
    assert!(t.deleted_at.is_some());
    assert_eq!(t.deleted_by.as_deref(), Some("omp"));

    // 공개 read 경로는 tombstone 을 숨긴다
    assert!(s.list().is_empty());
    assert!(s.get(&id).is_none());
    assert!(s.search(&id).is_empty());

    // tombstone 경로로는 여전히 접근 가능 (동기화용)
    assert!(s.get_with_tombstone(&id).is_some());
    assert_eq!(s.list_with_tombstones().len(), 1);

    // history 에도 deleted 기록
    let full = s.get_with_tombstone(&id).unwrap();
    assert!(full.history.iter().any(|h| h.action == "deleted" && h.by == "omp"));
}

#[test]
fn history_caps_at_max() {
    // MAX_HISTORY 초과로 push 해도 MAX_HISTORY 개만 유지되어야 한다.
    let mut s = user_store();
    let id = s.list()[0].id.clone();
    for i in 0..(todo_core::store::MAX_HISTORY + 20) {
        s.toggle(&id, &format!("agent-{i}")).unwrap();
    }
    let t = s.get_with_tombstone(&id).unwrap();
    assert_eq!(t.history.len(), todo_core::store::MAX_HISTORY);
    // 가장 최근 엔트리가 마지막에 있어야 함
    assert_eq!(t.history.last().unwrap().by, format!("agent-{}", todo_core::store::MAX_HISTORY + 19));
}

#[test]
fn prune_tombstones_drops_old_deletes_only() {
    use chrono::{Duration, Utc};
    use todo_core::model::{Priority, Todo, TodoDoc};

    // 30일 전에 삭제된 tombstone 과 방금 삭제된 tombstone 으로 Store 생성.
    let stale_cutoff = Utc::now() - Duration::days(30);
    let fresh_cutoff = Utc::now();

    let mk = |id: &str, deleted_at| Todo {
        id: id.into(),
        title: id.into(),
        note: None,
        summary: None,
        deleted_at,
        deleted_by: Some("u".into()),
        done: false,
        priority: Priority::Medium,
        due_date: None,
        tags: vec![],
        category_id: None,
        created_at: stale_cutoff,
        created_by: "u".into(),
        completed_at: None,
        completed_by: None,
        updated_at: deleted_at,
        updated_by: Some("u".into()),
        history: vec![],
    };

    let doc = TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "test".into(),
        todos: vec![
            mk("stale", Some(stale_cutoff)),
            mk("fresh", Some(fresh_cutoff)),
        ],
        categories: vec![],
    };
    let mut s = Store::from_doc(doc);

    // 7일 기준으로 prune → stale 만 제거
    let pruned = s.prune_tombstones(Duration::days(7));
    assert_eq!(pruned, 1, "only the 30-day-old tombstone should be pruned");
    assert!(s.get_with_tombstone("stale").is_none(), "stale tombstone must be gone");
    assert!(s.get_with_tombstone("fresh").is_some(), "fresh tombstone must remain");
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
