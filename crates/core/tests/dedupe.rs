//! Merge + dedupe integration tests: the failure modes seen in production
//! (duplicate categories after sync, stale-ETag 304 wipes, concurrent edits).

use chrono::Utc;
use todo_core::merge::merge;
use todo_core::model::{Category, Priority, Todo, TodoDoc};
use todo_core::store::{Store, TodoInput};

fn doc(todos: Vec<Todo>, categories: Vec<Category>) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "test".into(),
        todos,
        categories,
        dev_notes: vec![],
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
        category_id: None,
        created_at: Utc::now(),
        created_by: "test".into(),
        completed_at: None,
        completed_by: None,
        updated_at: None,
        updated_by: None,
        history: vec![],
    }
}

fn cat(id: &str, agent: &str, name: &str) -> Category {
    Category {
        id: id.into(),
        agent: agent.into(),
        name: name.into(),
        order: 0,
        updated_at: Some(Utc::now()),
    }
}

#[test]
fn merge_unions_same_name_categories_from_both_sides() {
    // Local and remote each created "버그 수정" for zcode with different ids
    // (classic duplicate storm). Merge unions them; dedupe collapses them.
    let mut local = doc(
        vec![bare("t1", "todo pointing at local cat")],
        vec![cat("cat-local", "zcode", "버그 수정")],
    );
    local.todos[0].category_id = Some("cat-local".into());

    let mut remote = doc(
        vec![bare("t2", "todo pointing at remote cat")],
        vec![cat("cat-remote", "zcode", "버그 수정")],
    );
    remote.todos[0].category_id = Some("cat-remote".into());

    let merged = merge(&local, &remote);
    assert_eq!(merged.categories.len(), 2, "merge unions by id first");

    // dedupe must keep exactly one and remap todos
    let mut store = Store::from_doc(merged);
    store.dedupe_categories();
    let all = store.list_categories_all();
    assert_eq!(all.len(), 1, "same (agent,name) collapses to one");
    let todos = store.list();
    assert_eq!(todos.len(), 2);
    let ids: std::collections::HashSet<_> = todos.iter().filter_map(|t| t.category_id.clone()).collect();
    assert_eq!(ids.len(), 1, "all todos remapped to the surviving id");
    assert_eq!(ids.iter().next().unwrap(), &all[0].id);
}

#[test]
fn dedupe_keeps_the_id_most_todos_reference() {
    // cat-b has 2 refs, cat-a has 1 → cat-b wins regardless of order.
    let mut local = doc(
        vec![bare("t1", "a1"), bare("t2", "b1"), bare("t3", "b2")],
        vec![cat("cat-a", "hermes", "백엔드"), cat("cat-b", "hermes", "백엔드")],
    );
    local.todos[0].category_id = Some("cat-a".into());
    local.todos[1].category_id = Some("cat-b".into());
    local.todos[2].category_id = Some("cat-b".into());

    let mut store = Store::from_doc(local);
    store.dedupe_categories();
    let all = store.list_categories_all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "cat-b", "most-referenced id survives");
    for t in store.list() {
        assert_eq!(t.category_id.as_deref(), Some("cat-b"));
    }
}

#[test]
fn dedupe_is_noop_when_already_unique() {
    let local = doc(
        vec![],
        vec![cat("c1", "zcode", "A"), cat("c2", "zcode", "B"), cat("c3", "omp", "A")],
    );
    let mut store = Store::from_doc(local);
    store.dedupe_categories();
    assert_eq!(store.list_categories_all().len(), 3);
    // order re-normalized per agent: zcode → 0,1 / omp → 0
    let z: Vec<_> = store.list_categories("zcode");
    assert_eq!(z.iter().map(|c| c.order).collect::<Vec<_>>(), vec![0, 1]);
    let o = store.list_categories("omp");
    assert_eq!(o[0].order, 0);
}

#[test]
fn add_category_returns_existing_instead_of_duplicating() {
    let mut s = Store::new();
    let first = s.add_category("zcode", "버그 수정");
    let second = s.add_category("zcode", "버그 수정");
    assert_eq!(first.id, second.id, "same (agent,name) must not create a second category");
    assert_eq!(s.list_categories_all().len(), 1);
    // different agent with same name is fine
    let other = s.add_category("omp", "버그 수정");
    assert_ne!(other.id, first.id);
    assert_eq!(s.list_categories_all().len(), 2);
}

#[test]
fn merge_latest_update_wins_for_concurrent_edits() {
    let future = Utc::now() + chrono::Duration::hours(1);
    let mut a = bare("1", "local edit");
    a.updated_at = None;
    let mut b = bare("1", "remote edit");
    b.updated_at = Some(future);
    let merged = merge(&doc(vec![a], vec![]), &doc(vec![b], vec![]));
    assert_eq!(merged.todos[0].title, "remote edit");
}

#[test]
fn local_store_roundtrip_and_backup() {
    use todo_core::storage::LocalStore;
    let dir = std::env::temp_dir().join(format!("acc-dedupe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let store = LocalStore::new(dir.join("data.json"));
    assert!(store.load().unwrap().is_none());

    let mut d = TodoDoc::new("t");
    d.categories.push(cat("c1", "zcode", "폴더"));
    store.save(&d).unwrap();

    let back = store.backup(&d).unwrap();
    assert!(back.exists(), "backup file created");
    let loaded = store.load().unwrap().unwrap();
    assert_eq!(loaded.categories.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn add_todo_with_category_via_store() {
    let mut s = Store::new();
    let c = s.add_category("zcode", "버그 수정");
    let t = s.add(
        TodoInput {
            title: "nil 버그".into(),
            note: None,
            priority: Priority::High,
            due_date: None,
            tags: vec![],
            category_id: Some(c.id.clone()),
        },
        "zcode",
    );
    assert_eq!(t.category_id.as_deref(), Some(c.id.as_str()));
    assert_eq!(s.list_categories("zcode").len(), 1);
}
