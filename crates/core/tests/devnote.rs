use todo_core::devnote::{generate_worklog, parse_system_tag, parse_version_tag};
use todo_core::model::{GameSystem, NoteType};
use todo_core::store::{Store, TodoInput};

fn make_todo(title: &str, note: Option<&str>, tags: Vec<&str>) -> todo_core::model::Todo {
    let mut s = Store::new();
    let input = TodoInput {
        title: title.into(),
        note: note.map(String::from),
        priority: todo_core::model::Priority::Medium,
        due_date: None,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        category_id: None,
    };
    let t = s.add(input, "zcode");
    s.complete_with_summary(&t.id, "작업 완료", "zcode").unwrap();
    s.list().into_iter().next().unwrap()
}

#[test]
fn parse_system_tag_extracts_combat() {
    let tags = vec!["agent:zcode".into(), "system:combat".into()];
    assert_eq!(parse_system_tag(&tags), Some(GameSystem::Combat));
}

#[test]
fn parse_system_tag_returns_none_when_absent() {
    let tags = vec!["agent:zcode".into()];
    assert_eq!(parse_system_tag(&tags), None);
}

#[test]
fn parse_version_tag_extracts_version() {
    let tags = vec!["system:ui".into(), "version:v0.2-alpha".into()];
    assert_eq!(parse_version_tag(&tags), Some("v0.2-alpha".into()));
}

#[test]
fn generate_worklog_groups_by_system() {
    let todos = vec![
        make_todo("적 패턴 구현", Some("상태머신 추가"), vec!["system:combat"]),
        make_todo("체력바 UI", Some("실드 게이지 추가"), vec!["system:ui"]),
        make_todo("데미지 계산", Some("공식 수정"), vec!["system:combat"]),
    ];
    let note = generate_worklog(&todos, "2026-07-11", None);
    assert_eq!(note.note_type, NoteType::Worklog);
    assert!(note.body.contains("Combat"));
    assert!(note.body.contains("Ui"));
    assert!(note.body.contains("적 패턴 구현"));
    assert!(note.body.contains("데미지 계산"));
    assert!(note.body.contains("체력바 UI"));
    assert_eq!(note.linked_todos.len(), 3);
}

#[test]
fn generate_worklog_includes_version_in_header() {
    let todos = vec![
        make_todo("적 패턴", Some("구현"), vec!["system:combat", "version:v0.2-alpha"]),
    ];
    let note = generate_worklog(&todos, "2026-07-11", None);
    assert!(note.title.contains("2026-07-11"));
    assert!(note.body.contains("v0.2-alpha"));
}

#[test]
fn generate_worklog_without_system_goes_to_etc() {
    let todos = vec![
        make_todo("빌드 수정", Some("CI 설정"), vec![]),
    ];
    let note = generate_worklog(&todos, "2026-07-11", None);
    assert!(note.body.contains("Etc"));
}

#[test]
fn generate_worklog_title_has_date() {
    let todos = vec![make_todo("x", None, vec![])];
    let note = generate_worklog(&todos, "2026-07-11", None);
    assert_eq!(note.title, "2026-07-11 작업 로그");
}
