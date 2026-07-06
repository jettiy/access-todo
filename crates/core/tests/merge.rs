use chrono::Utc;
use todo_core::merge::merge;
use todo_core::model::{Priority, Todo, TodoDoc};

fn doc(todos: Vec<Todo>) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "x".into(),
        todos,
    }
}

fn bare(id: &str, title: &str) -> Todo {
    Todo {
        id: id.into(),
        title: title.into(),
        note: None,
        done: false,
        priority: Priority::Medium,
        due_date: None,
        tags: vec![],
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
