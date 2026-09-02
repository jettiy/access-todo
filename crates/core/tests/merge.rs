use chrono::Utc;
use todo_core::merge::merge;
use todo_core::model::{Priority, Todo, TodoDoc};

fn doc(todos: Vec<Todo>) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "x".into(),
        todos,
        categories: vec![],
    }
}

fn bare(id: &str, title: &str) -> Todo {
    Todo {
        id: id.into(),
        title: title.into(),
        note: None,
        summary: None,
        deleted_at: None,
        deleted_by: None,
        done: false,
        priority: Priority::Medium,
        due_date: None,
        tags: vec![],
        category_id: None,
        created_at: Utc::now(),
        created_by: "x".into(),
        completed_at: None,
        completed_by: None,
        updated_at: None,
        updated_by: None,
        history: vec![],
    }
}

#[test]
fn union_of_disjoint_ids() {
    let a = doc(vec![bare("1", "A")]);
    let b = doc(vec![bare("2", "B")]);
    let m = merge(&a, &b);
    let ids: Vec<_> = m.todos.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"1"));
    assert!(ids.contains(&"2"));
}

#[test]
fn same_id_latest_updated_at_wins() {
    let future = Utc::now() + chrono::Duration::days(1);
    let mut older = bare("1", "old");
    older.updated_at = None;
    let mut newer = bare("1", "new");
    newer.updated_at = Some(future);
    let a = doc(vec![older]);
    let b = doc(vec![newer]);
    let m = merge(&a, &b);
    assert_eq!(m.todos[0].title, "new");
}

#[test]
fn older_remote_keeps_local_version() {
    let future = Utc::now() + chrono::Duration::days(1);
    let mut local = bare("1", "local-newer");
    local.updated_at = Some(future);
    let remote = bare("1", "remote-older"); // updated_at = None
    let a = doc(vec![local]);
    let b = doc(vec![remote]);
    let m = merge(&a, &b);
    assert_eq!(m.todos[0].title, "local-newer");
}

#[test]
fn merge_is_symmetric_in_ids() {
    let a = doc(vec![bare("1", "A"), bare("3", "C")]);
    let b = doc(vec![bare("2", "B"), bare("3", "C-prime")]);
    let m = merge(&a, &b);
    assert_eq!(m.todos.len(), 3);
    // result is sorted by id for deterministic output
    let ids: Vec<_> = m.todos.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["1", "2", "3"]);
}
#[test]
fn delete_propagates_via_tombstone() {
    // 핵심 시나리오: A가 todo 1을 삭제(tombstone)하고, B는 여전히 로컬에
    // todo 1을 보관 중일 때, merge 결과는 tombstone 쪽이 이겨야 한다.
    let future = Utc::now() + chrono::Duration::seconds(1);
    let mut live = bare("1", "still-alive-on-B");
    live.updated_at = Some(Utc::now()); // B의 최근 수정

    let mut tombstone = bare("1", "deleted-on-A");
    tombstone.deleted_at = Some(future); // A의 더 최근 삭제

    let a = doc(vec![tombstone]);
    let b = doc(vec![live]);

    let m = merge(&a, &b);
    assert_eq!(m.todos.len(), 1, "tombstone should win, not duplicate");
    assert!(m.todos[0].deleted_at.is_some(), "deleted_at must survive merge");
}

#[test]
fn delete_loses_to_newer_edit_on_other_side() {
    // 반대 방향 검증: 삭제보다 더 최근의 수정이 있으면 수정이 이긴다
    // (사용자가 삭제 후 다른 기기에서 편집한 경우는 드물지만 정의되어야 함).
    let past = Utc::now() - chrono::Duration::seconds(10);
    let mut stale_tombstone = bare("1", "old-delete");
    stale_tombstone.deleted_at = Some(past);

    let mut fresh_edit = bare("1", "fresh-edit");
    fresh_edit.updated_at = Some(Utc::now());

    let a = doc(vec![stale_tombstone]);
    let b = doc(vec![fresh_edit]);
    let m = merge(&a, &b);
    assert_eq!(m.todos.len(), 1);
    assert!(m.todos[0].deleted_at.is_none(), "newer edit should beat stale tombstone");
    assert_eq!(m.todos[0].title, "fresh-edit");
}

#[test]
fn both_sides_delete_yields_one_tombstone() {
    let mut a_tomb = bare("1", "A");
    a_tomb.deleted_at = Some(Utc::now());
    let mut b_tomb = bare("1", "B");
    b_tomb.deleted_at = Some(Utc::now());

    let m = merge(&doc(vec![a_tomb]), &doc(vec![b_tomb]));
    assert_eq!(m.todos.len(), 1);
    assert!(m.todos[0].deleted_at.is_some());
}
