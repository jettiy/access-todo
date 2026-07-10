# 게임 개발노트 툴 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Access 시스템에 게임 개발노트 기능을 추가 — 완료된 todo로 작업 로그를 자동 생성하고, 회고/이슈/플레이테스트 노트를 수동으로 기록하며, 게임 시스템·버전별로 분류하는 Tauri 위젯 제공.

**Architecture:** 하이브리드 접근 — 기존 `TodoDoc`에 `dev_notes: Vec<DevNote>` 배열을 추가. Gist 동기화, ETag, 3-way 머지를 그대로 재사용. 코어(store/merge) → API 라우트 → MCP 도구 → Svelte 위젯 순으로 구축.

**Tech Stack:** Rust (axum, serde, chrono, uuid), Svelte, TypeScript, Tauri

## Global Constraints

- Rust edition 2021, workspace 멤버: `crates/core`, `crates/api-server`, `crates/mcp-server`
- serde 직렬화 시 `dev_notes` 빈 배열은 `skip_serializing_if = "Vec::is_empty"` + `#[serde(default)]`로 기존 Gist/Android 호환 유지
- `NoteType`과 `GameSystem`은 `#[serde(rename_all = "lowercase")]`로 직렬화
- 모든 API 핸들러는 기존 패턴 준수: `State(s): State<AppState>`, `agent_from_headers(&h)`, `s.push(&actor).await`
- todo의 게임 시스템은 tags 배열의 `"system:<name>"` 규칙으로 표현 (기존 Todo 구조체 변경 없음)
- 버전 태그는 tags 배열의 `"version:<ver>"` 규칙
- API 서버는 localhost 전용 (`127.0.0.1:7878`)
- 테스트는 기존 패턴 준수: 코어는 `crates/core/tests/`, API는 `app_for_test()` + `oneshot`

---

## File Structure

| 파일 | 유형 | 책임 |
|------|------|------|
| `crates/core/src/model.rs` | 수정 | `DevNote`, `NoteType`, `GameSystem`, `new_note_id()` 추가; `TodoDoc.dev_notes` 필드 |
| `crates/core/src/devnote.rs` | 신규 | 작업 로그 자동 생성 로직 — todo 수집/그룹화/마크다운 생성 |
| `crates/core/src/store.rs` | 수정 | `Store`에 `dev_notes` 필드; devnote CRUD 메서드; `into_doc`/`from_doc` 업데이트 |
| `crates/core/src/merge.rs` | 수정 | `dev_notes` 배열 머지 로직 추가 |
| `crates/core/src/lib.rs` | 수정 | `pub mod devnote;` 선언 |
| `crates/core/tests/devnote.rs` | 신규 | 코어 단위 테스트 |
| `crates/core/tests/merge.rs` | 수정 | dev_notes 머지 테스트 추가 (기존 doc() 헬퍼에 dev_notes 필드) |
| `crates/api-server/src/routes.rs` | 수정 | 8개 devnote 엔드포인트 |
| `crates/api-server/tests/routes.rs` | 수정 | devnote API 테스트 추가 |
| `crates/mcp-server/src/tools.rs` | 수정 | 4개 devnote 도구 dispatch + catalog |
| `apps/desktop/src/api.ts` | 수정 | devnote 타입/함수 추가 |
| `apps/desktop/src/DevNoteWidget.svelte` | 신규 | 개발노트 메인 위젯 |
| `apps/desktop/src/DevNoteCard.svelte` | 신규 | 개별 노트 카드 |
| `apps/desktop/src/DevNoteEditor.svelte` | 신규 | 노트 에디터 모달 |
| `apps/desktop/src/App.svelte` | 수정 | devnotes 창 분기 처리 |
| `apps/desktop/src-tauri/src/lib.rs` | 수정 | 5번째 창(devnotes) 추가 |

---

## Task 1: 코어 데이터 모델 — DevNote, NoteType, GameSystem

**Files:**
- Modify: `crates/core/src/model.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/tests/model.rs`

**Interfaces:**
- Produces: `DevNote` struct, `NoteType` enum, `GameSystem` enum, `new_note_id()` function, `TodoDoc.dev_notes` field. These are consumed by Tasks 2-5.

- [ ] **Step 1: Write the failing test for DevNote serialization and backward compat**

Append to `crates/core/tests/model.rs`:

```rust
use todo_core::model::{DevNote, GameSystem, NoteType, TodoDoc};

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
        created_at: chrono::Utc::now(),
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
    // dev_notes should be absent when empty (backward compat)
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
        created_at: chrono::Utc::now(),
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p todo_core --test model`
Expected: FAIL — `DevNote`, `NoteType`, `GameSystem`, `new_note_id` not found, `TodoDoc` missing field.

- [ ] **Step 3: Add NoteType, GameSystem, DevNote, new_note_id to model.rs**

Append before the final closing of `crates/core/src/model.rs` (after `new_cat_id`):

```rust
/// Type of a dev note. Serializes as lowercase strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Worklog,
    Retrospect,
    Issue,
    Playtest,
}

/// Game system classification for notes. Serializes as lowercase strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameSystem {
    Combat,
    Inventory,
    Quest,
    Skill,
    Ui,
    Audio,
    Map,
    Ai,
    Save,
    Network,
    Etc,
}

/// A single dev note entry (worklog, retrospect, issue, or playtest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevNote {
    pub id: String,
    pub note_type: NoteType,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<GameSystem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_todos: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}

/// Generate a dev note id (prefixed for readability in the JSON).
pub fn new_note_id() -> String {
    format!("note-{}", Uuid::new_v4())
}
```

- [ ] **Step 4: Add dev_notes field to TodoDoc**

In `crates/core/src/model.rs`, modify the `TodoDoc` struct. Add `dev_notes` field with skip-when-empty:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoDoc {
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[serde(default)]
    pub todos: Vec<Todo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dev_notes: Vec<DevNote>,
}
```

Update `TodoDoc::new`:

```rust
impl TodoDoc {
    pub fn new(actor: &str) -> Self {
        Self {
            version: "1.0".into(),
            updated_at: Utc::now(),
            updated_by: actor.into(),
            todos: vec![],
            categories: vec![],
            dev_notes: vec![],
        }
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p todo_core --test model`
Expected: PASS

- [ ] **Step 6: Add pub mod devnote to lib.rs**

In `crates/core/src/lib.rs`, add the module declaration (the file will be created in Task 2, but declare it now — use `#[allow(dead_code)]` is not needed since we create the file before running):

```rust
pub mod devnote;
```

Wait — we must create the file or compilation fails. Create a placeholder `crates/core/src/devnote.rs` with just a doc comment:

```rust
//! Dev note worklog generation logic (implemented in Task 2).
```

- [ ] **Step 7: Fix any compilation errors in merge.rs and store.rs from TodoDoc change**

The `merge.rs` constructs `TodoDoc` without `dev_notes` field, and `store.rs` `into_doc` does too. Fix both now (full logic in later tasks, but they must compile).

In `crates/core/src/merge.rs`, the `merge` function's return `TodoDoc { ... }` — add `dev_notes: vec![]` for now:

```rust
TodoDoc {
    version: "1.0".into(),
    updated_at: Utc::now(),
    updated_by: "merge".into(),
    todos,
    categories,
    dev_notes: vec![],
}
```

In `crates/core/src/store.rs`, the `into_doc` method — add `dev_notes` field. Since store doesn't have dev_notes yet (Task 3), use empty for now:

```rust
pub fn into_doc(self, actor: &str) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: actor.into(),
        todos: self.todos,
        categories: self.categories,
        dev_notes: vec![],
    }
}
```

Also fix `crates/core/tests/merge.rs` — the `doc()` helper constructs TodoDoc without dev_notes:

```rust
fn doc(todos: Vec<Todo>) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "x".into(),
        todos,
        categories: vec![],
        dev_notes: vec![],
    }
}
```

- [ ] **Step 8: Run all core tests**

Run: `cargo test -p todo_core`
Expected: all PASS

- [ ] **Step 9: Commit**

```bash
git add crates/core/src/model.rs crates/core/src/lib.rs crates/core/src/devnote.rs crates/core/src/store.rs crates/core/src/merge.rs crates/core/tests/model.rs crates/core/tests/merge.rs
git commit -m "feat(core): DevNote/NoteType/GameSystem 데이터 모델 + TodoDoc.dev_notes 필드"
```

---

## Task 2: 작업 로그 자동 생성 로직

**Files:**
- Create: `crates/core/src/devnote.rs` (replace placeholder)
- Test: `crates/core/tests/devnote.rs` (new file)

**Interfaces:**
- Consumes: `DevNote`, `NoteType`, `GameSystem`, `new_note_id` from Task 1; `Todo` from existing model
- Produces: `generate_worklog(todos: &[Todo], date: &str, agent: Option<&str>) -> DevNote`, `parse_system_tag(tags: &[String]) -> Option<GameSystem>`, `parse_version_tag(tags: &[String]) -> Option<String>`

- [ ] **Step 1: Write the failing test**

Create `crates/core/tests/devnote.rs`:

```rust
use todo_core::devnote::{generate_worklog, parse_system_tag, parse_version_tag};
use todo_core::model::{GameSystem, NoteType, Priority, Todo};
use todo_core::store::{Store, TodoInput};

fn make_todo(title: &str, note: Option<&str>, tags: Vec<&str>) -> Todo {
    let mut s = Store::new();
    let mut input = TodoInput {
        title: title.into(),
        note: note.map(String::from),
        priority: Priority::Medium,
        due_date: None,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        category_id: None,
    };
    let t = s.add(input, "zcode");
    // Mark as completed
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p todo_core --test devnote`
Expected: FAIL — `generate_worklog`, `parse_system_tag`, `parse_version_tag` not found.

- [ ] **Step 3: Implement devnote.rs**

Replace `crates/core/src/devnote.rs` content:

```rust
//! Worklog generation: collects completed todos, groups by game system,
//! and renders a markdown body for a DevNote.

use std::collections::BTreeMap;

use crate::model::{DevNote, GameSystem, NoteType, Todo, new_note_id};

/// Extract the game system from a todo's `"system:<name>"` tag.
pub fn parse_system_tag(tags: &[String]) -> Option<GameSystem> {
    for tag in tags {
        if let Some(rest) = tag.strip_prefix("system:") {
            return match rest {
                "combat" => Some(GameSystem::Combat),
                "inventory" => Some(GameSystem::Inventory),
                "quest" => Some(GameSystem::Quest),
                "skill" => Some(GameSystem::Skill),
                "ui" => Some(GameSystem::Ui),
                "audio" => Some(GameSystem::Audio),
                "map" => Some(GameSystem::Map),
                "ai" => Some(GameSystem::Ai),
                "save" => Some(GameSystem::Save),
                "network" => Some(GameSystem::Network),
                "etc" => Some(GameSystem::Etc),
                _ => Some(GameSystem::Etc),
            };
        }
    }
    None
}

/// Extract the version from a todo's `"version:<ver>"` tag.
pub fn parse_version_tag(tags: &[String]) -> Option<String> {
    for tag in tags {
        if let Some(rest) = tag.strip_prefix("version:") {
            return Some(rest.to_string());
        }
    }
    None
}

/// Generate a Worklog DevNote from a slice of completed todos.
///
/// Todos are grouped by game system (parsed from their `system:<name>` tag,
/// defaulting to `Etc`). The body is markdown with `## SystemName` headers
/// and `- title: summary` bullets.
pub fn generate_worklog(todos: &[Todo], date: &str, _agent: Option<&str>) -> DevNote {
    // Group todos by game system.
    let mut groups: BTreeMap<GameSystem, Vec<&Todo>> = BTreeMap::new();
    let mut versions: Vec<String> = Vec::new();

    for t in todos {
        let sys = parse_system_tag(&t.tags).unwrap_or(GameSystem::Etc);
        groups.entry(sys).or_default().push(t);
        if let Some(v) = parse_version_tag(&t.tags) {
            if !versions.contains(&v) {
                versions.push(v);
            }
        }
    }

    // Build markdown body.
    let mut body = String::new();
    let count = todos.len();
    body.push_str(&format!("*총 {count}개 작업 완료*\n\n"));

    for (sys, items) in &groups {
        let sys_name = system_display_name(sys);
        body.push_str(&format!("## {sys_name}\n"));
        for t in items {
            let summary = t.note.as_deref().unwrap_or("").trim_start_matches("✓ ");
            if summary.is_empty() {
                body.push_str(&format!("- {}\n", t.title));
            } else {
                body.push_str(&format!("- {}: {}\n", t.title, summary));
            }
        }
        body.push('\n');
    }

    if !versions.is_empty() {
        body.push_str(&format!("**버전**: {}", versions.join(", ")));
    }

    let linked: Vec<String> = todos.iter().map(|t| t.id.clone()).collect();

    DevNote {
        id: new_note_id(),
        note_type: NoteType::Worklog,
        title: format!("{date} 작업 로그"),
        body,
        game_system: None,
        version: versions.first().cloned(),
        linked_todos: linked,
        tags: vec![],
        created_at: chrono::Utc::now(),
        created_by: "system".into(),
        updated_at: None,
        updated_by: None,
    }
}

fn system_display_name(sys: &GameSystem) -> &'static str {
    match sys {
        GameSystem::Combat => "Combat (전투)",
        GameSystem::Inventory => "Inventory (인벤토리)",
        GameSystem::Quest => "Quest (퀘스트)",
        GameSystem::Skill => "Skill (스킬)",
        GameSystem::Ui => "Ui (UI)",
        GameSystem::Audio => "Audio (오디오)",
        GameSystem::Map => "Map (맵)",
        GameSystem::Ai => "Ai (AI)",
        GameSystem::Save => "Save (세이브)",
        GameSystem::Network => "Network (네트워크)",
        GameSystem::Etc => "Etc (기타)",
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p todo_core --test devnote`
Expected: PASS

- [ ] **Step 5: Run all core tests**

Run: `cargo test -p todo_core`
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/devnote.rs crates/core/tests/devnote.rs
git commit -m "feat(core): 작업 로그 자동 생성 — system/version 태그 파싱 + 마크다운 생성"
```

---

## Task 3: Store devnote CRUD + worklog 생성 메서드

**Files:**
- Modify: `crates/core/src/store.rs`
- Modify: `crates/core/tests/store.rs`

**Interfaces:**
- Consumes: `DevNote`, `NoteType`, `GameSystem`, `new_note_id` from Task 1; `generate_worklog` from Task 2
- Produces: `Store` with `dev_notes` field, `DevNoteInput`, `DevNotePatch`, `NoteFilter`, `WorklogResult`, methods: `list_devnotes()`, `get_devnote()`, `add_devnote()`, `update_devnote()`, `delete_devnote()`, `generate_worklog()`. Updated `into_doc()`/`from_doc()`.

- [ ] **Step 1: Write the failing test**

Append to `crates/core/tests/store.rs`:

```rust
use todo_core::devnote::generate_worklog;
use todo_core::model::{DevNote, GameSystem, NoteType};
use todo_core::store::{DevNoteInput, DevNotePatch, NoteFilter};

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
            note_type: NoteType::Retrospect,
            title: "밸런스 회고".into(),
            body: "적 체력 너무 높음".into(),
            game_system: Some(GameSystem::Combat),
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
            note_type: NoteType::Issue,
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
            note_type: NoteType::Retrospect,
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
    let result = s.generate_worklog("2026-07-11", None, "zcode");
    assert!(result.created);
    assert_eq!(result.todos_counted, 1);
    assert!(result.systems.contains(&"combat".to_string()));

    let notes = s.list_devnotes(NoteFilter {
        note_type: Some(NoteType::Worklog),
        ..Default::default()
    });
    assert_eq!(notes.len(), 1);
    assert!(notes[0].body.contains("Combat"));
}

#[test]
fn generate_worklog_is_idempotent() {
    let (mut s, _todo_id) = note_store_with_todo();
    let r1 = s.generate_worklog("2026-07-11", None, "zcode");
    assert!(r1.created);
    let r2 = s.generate_worklog("2026-07-11", None, "zcode");
    assert!(!r2.created); // already exists, not re-created
    assert_eq!(r2.todos_counted, 1); // still counts the same todo
    // only one worklog note
    let notes = s.list_devnotes(NoteFilter {
        note_type: Some(NoteType::Worklog),
        ..Default::default()
    });
    assert_eq!(notes.len(), 1);
}

#[test]
fn filter_devnotes_by_system() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: NoteType::Issue, title: "a".into(), body: "".into(),
        game_system: Some(GameSystem::Combat), version: None, tags: vec![],
    }, "zcode");
    s.add_devnote(DevNoteInput {
        note_type: NoteType::Issue, title: "b".into(), body: "".into(),
        game_system: Some(GameSystem::Ui), version: None, tags: vec![],
    }, "zcode");
    let combat_only = s.list_devnotes(NoteFilter {
        game_system: Some(GameSystem::Combat),
        ..Default::default()
    });
    assert_eq!(combat_only.len(), 1);
    assert_eq!(combat_only[0].title, "a");
}

#[test]
fn into_doc_carries_dev_notes() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: NoteType::Retrospect, title: "x".into(), body: "".into(),
        game_system: None, version: None, tags: vec![],
    }, "zcode");
    let doc = s.into_doc("zcode");
    assert_eq!(doc.dev_notes.len(), 1);
}

#[test]
fn from_doc_restores_dev_notes() {
    let mut s = Store::new();
    s.add_devnote(DevNoteInput {
        note_type: NoteType::Retrospect, title: "x".into(), body: "".into(),
        game_system: None, version: None, tags: vec![],
    }, "zcode");
    let doc = s.clone().into_doc("zcode");
    let s2 = Store::from_doc(doc);
    assert_eq!(s2.list_devnotes(NoteFilter::default()).len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p todo_core --test store`
Expected: FAIL — `DevNoteInput`, `DevNotePatch`, `NoteFilter` not found.

- [ ] **Step 3: Add dev_notes field and structs to store.rs**

In `crates/core/src/store.rs`, first update the imports and add new structs. Add to the top imports section:

```rust
use crate::model::{new_cat_id, new_id, new_note_id, Category, DevNote, GameSystem, HistoryEntry, NoteType, Priority, Todo, TodoDoc};
```

Add `dev_notes` to the `Store` struct:

```rust
#[derive(Debug, Clone, Default)]
pub struct Store {
    todos: Vec<Todo>,
    categories: Vec<Category>,
    dev_notes: Vec<DevNote>,
}
```

Update `into_doc`:

```rust
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
```

Update `from_doc`:

```rust
pub fn from_doc(doc: TodoDoc) -> Self {
    Self {
        todos: doc.todos,
        categories: doc.categories,
        dev_notes: doc.dev_notes,
    }
}
```

Now add the input/patch/filter structs and WorklogResult after the existing `TodoPatch` definition:

```rust
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

/// Partial update for a dev note. `None` means "leave unchanged".
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
```

- [ ] **Step 4: Add devnote CRUD methods to Store impl**

Add these methods inside the `impl Store` block (after the existing `search` method, before `into_doc`):

```rust
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
                crate::devnote::parse_system_tag(&t.tags).map(|gs| format!("{:?}", gs).to_lowercase())
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
                let mut new_note = crate::devnote::generate_worklog(&linked, date, agent);
                existing.body = new_note.body.clone();
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p todo_core --test store`
Expected: PASS

- [ ] **Step 6: Run all core tests**

Run: `cargo test -p todo_core`
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/store.rs crates/core/tests/store.rs
git commit -m "feat(core): Store devnote CRUD + generate_worklog (idempotent) + into_doc/from_doc"
```

---

## Task 4: dev_notes 3-way 머지 로직

**Files:**
- Modify: `crates/core/src/merge.rs`
- Modify: `crates/core/tests/merge.rs`

**Interfaces:**
- Consumes: `DevNote` from Task 1
- Produces: `merge()` now handles `dev_notes` array (same id-based union + latest-updated_at-wins as todos/categories)

- [ ] **Step 1: Write the failing test**

Append to `crates/core/tests/merge.rs`:

```rust
use todo_core::model::{DevNote, NoteType};

fn note(id: &str, title: &str) -> DevNote {
    DevNote {
        id: id.into(),
        note_type: NoteType::Retrospect,
        title: title.into(),
        body: "".into(),
        game_system: None,
        version: None,
        linked_todos: vec![],
        tags: vec![],
        created_at: Utc::now(),
        created_by: "x".into(),
        updated_at: None,
        updated_by: None,
    }
}

fn doc_with_notes(notes: Vec<DevNote>) -> TodoDoc {
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "x".into(),
        todos: vec![],
        categories: vec![],
        dev_notes: notes,
    }
}

#[test]
fn merge_devnotes_union_disjoint_ids() {
    let a = doc_with_notes(vec![note("note-1", "A")]);
    let b = doc_with_notes(vec![note("note-2", "B")]);
    let m = merge(&a, &b);
    assert_eq!(m.dev_notes.len(), 2);
}

#[test]
fn merge_devnotes_latest_updated_at_wins() {
    let future = Utc::now() + chrono::Duration::days(1);
    let mut older = note("note-1", "old");
    older.updated_at = None;
    let mut newer = note("note-1", "new");
    newer.updated_at = Some(future);
    let a = doc_with_notes(vec![older]);
    let b = doc_with_notes(vec![newer]);
    let m = merge(&a, &b);
    assert_eq!(m.dev_notes[0].title, "new");
}

#[test]
fn merge_devnotes_empty_preserves_notes() {
    let a = doc_with_notes(vec![note("note-1", "A")]);
    let b = doc_with_notes(vec![]);
    let m = merge(&a, &b);
    assert_eq!(m.dev_notes.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p todo_core --test merge`
Expected: FAIL — merge doesn't carry dev_notes, or dev_notes field missing in returned doc.

- [ ] **Step 3: Add dev_notes merge logic to merge.rs**

In `crates/core/src/merge.rs`, update imports and add the merge logic. First update the import line:

```rust
use crate::model::{Category, DevNote, Todo, TodoDoc};
```

Add a timestamp helper for DevNote after the existing `cat_ts` function:

```rust
/// Effective change timestamp for a dev note: `updated_at` if set, else
/// `created_at`, else epoch.
fn note_ts(n: &DevNote) -> DateTime<Utc> {
    n.updated_at.or(Some(n.created_at)).unwrap_or(DateTime::UNIX_EPOCH)
}
```

In the `merge` function, after the categories merge block and before the final `TodoDoc { ... }` return, add:

```rust
    // Merge dev_notes by id (latest updated_at wins, union of disjoint ids).
    let mut nmap: HashMap<String, DevNote> = HashMap::new();
    for n in &local.dev_notes {
        nmap.insert(n.id.clone(), n.clone());
    }
    for n in &remote.dev_notes {
        match nmap.get(&n.id) {
            Some(existing) => {
                if note_ts(n) >= note_ts(existing) {
                    nmap.insert(n.id.clone(), n.clone());
                }
            }
            None => {
                nmap.insert(n.id.clone(), n.clone());
            }
        }
    }
    let mut dev_notes: Vec<DevNote> = nmap.into_values().collect();
    dev_notes.sort_by(|a, b| a.id.cmp(&b.id));
```

Update the final `TodoDoc` return to include `dev_notes`:

```rust
    TodoDoc {
        version: "1.0".into(),
        updated_at: Utc::now(),
        updated_by: "merge".into(),
        todos,
        categories,
        dev_notes,
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p todo_core --test merge`
Expected: PASS

- [ ] **Step 5: Run all core tests + full workspace build**

Run: `cargo test --workspace`
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add crates/core/src/merge.rs crates/core/tests/merge.rs
git commit -m "feat(core): dev_notes 3-way 머지 — id 기반 union + latest-updated_at-wins"
```

---

## Task 5: API 엔드포인트 (8개 devnote 라우트)

**Files:**
- Modify: `crates/api-server/src/routes.rs`
- Modify: `crates/api-server/tests/routes.rs`

**Interfaces:**
- Consumes: `Store` devnote methods from Task 3, `DevNoteInput`, `DevNotePatch`, `NoteFilter`, `WorklogResult`
- Produces: 8 HTTP endpoints under `/devnotes`. Follows existing handler pattern (`State(s)`, `agent_from_headers`, `s.push(&actor)`).

- [ ] **Step 1: Write the failing test**

Append to `crates/api-server/tests/routes.rs`:

```rust
#[tokio::test]
async fn devnote_create_and_list() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"회고","body":"적 체력 높음","game_system":"combat","version":"v0.2"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["title"], "회고");
    assert_eq!(v["note_type"], "retrospect");
    let id = v["id"].as_str().unwrap().to_string();

    // list
    let resp = app
        .clone()
        .oneshot(req("GET", "/devnotes", ""))
        .await
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn devnote_update_body() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"issue","title":"버그","body":"원본"}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/devnotes/{}", id);
    let resp = app
        .oneshot(req("PATCH", &uri, r#"{"body":"수정됨"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["body"], "수정됨");
}

#[tokio::test]
async fn devnote_delete() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"x","body":""}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/devnotes/{}", id);
    let resp = app.clone().oneshot(req("DELETE", &uri, "")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app.oneshot(req("GET", "/devnotes", "")).await.unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn worklog_generate_from_completed_todo() {
    let app = app_for_test();
    // add a todo with system:combat tag
    let resp = app
        .clone()
        .oneshot(req("POST", "/todos", r#"{"title":"적 패턴","tags":["system:combat","version:v0.2"]}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    // complete it
    let complete_uri = format!("/todos/{}/complete", id);
    let _ = app
        .clone()
        .oneshot(req("POST", &complete_uri, r#"{"summary":"상태머신 추가"}"#))
        .await
        .unwrap();
    // generate worklog
    let today = chrono::Local::now().date_naive().format("%Y-%m-%d").to_string();
    let body = format!(r#"{{"date":"{}"}}"#, today);
    let resp = app
        .oneshot(req("POST", "/devnotes/worklog/generate", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["created"], true);
    assert_eq!(v["todos_counted"], 1);
}

#[tokio::test]
async fn devnotes_filter_by_type() {
    let app = app_for_test();
    app.clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"r","body":""}"#))
        .await
        .unwrap();
    app.clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"issue","title":"i","body":""}"#))
        .await
        .unwrap();
    let resp = app
        .oneshot(req("GET", "/devnotes?type=issue", ""))
        .await
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 1);
    assert_eq!(v["devnotes"][0]["title"], "i");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p api-server --test routes`
Expected: FAIL — `/devnotes` routes not found (404).

- [ ] **Step 3: Add devnote route handlers and request types to routes.rs**

In `crates/api-server/src/routes.rs`, update imports at the top:

```rust
use todo_core::model::{GameSystem, NoteType};
use todo_core::store::{DevNoteInput, DevNotePatch, NoteFilter};
```

Add request/response structs after the existing `ListQ` struct:

```rust
#[derive(Deserialize)]
pub struct NewDevNote {
    pub note_type: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub game_system: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct PatchDevNote {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub game_system: Option<Option<String>>,
    #[serde(default)]
    pub version: Option<Option<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct WorklogBody {
    pub date: Option<String>,
    pub agent: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct DevNoteListQ {
    #[serde(default)]
    pub note_type: Option<String>,
    #[serde(default)]
    pub game_system: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
}

fn parse_note_type(s: &str) -> NoteType {
    match s {
        "worklog" => NoteType::Worklog,
        "retrospect" => NoteType::Retrospect,
        "issue" => NoteType::Issue,
        "playtest" => NoteType::Playtest,
        _ => NoteType::Retrospect,
    }
}

fn parse_game_system(s: &str) -> GameSystem {
    match s {
        "combat" => GameSystem::Combat,
        "inventory" => GameSystem::Inventory,
        "quest" => GameSystem::Quest,
        "skill" => GameSystem::Skill,
        "ui" => GameSystem::Ui,
        "audio" => GameSystem::Audio,
        "map" => GameSystem::Map,
        "ai" => GameSystem::Ai,
        "save" => GameSystem::Save,
        "network" => GameSystem::Network,
        _ => GameSystem::Etc,
    }
}
```

- [ ] **Step 4: Add handler functions**

Add these handler functions at the end of `crates/api-server/src/routes.rs` (before the final closing of the file, after the `remove` function):

```rust
// ── Dev Notes ──

async fn list_devnotes(
    State(s): State<AppState>,
    q: Query<DevNoteListQ>,
) -> Json<serde_json::Value> {
    let st = s.store.lock().await;
    let filter = NoteFilter {
        note_type: q.note_type.as_deref().map(parse_note_type),
        game_system: q.game_system.as_deref().map(parse_game_system),
        version: q.version.clone(),
    };
    let mut notes = st.list_devnotes(filter);
    // agent filter
    if let Some(ref agent) = q.agent {
        let tag = format!("agent:{agent}");
        notes.retain(|n| n.tags.iter().any(|t| t == &tag) || n.created_by == *agent);
    }
    Json(serde_json::json!({ "devnotes": notes }))
}

async fn one_devnote(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let st = s.store.lock().await;
    match st.get_devnote(&id) {
        Some(n) => Json(serde_json::to_value(n).unwrap_or(serde_json::Value::Null)),
        None => Json(serde_json::Value::Null),
    }
}

async fn add_devnote(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<NewDevNote>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let note = {
        let mut st = s.store.lock().await;
        st.add_devnote(
            DevNoteInput {
                note_type: parse_note_type(&b.note_type),
                title: b.title,
                body: b.body,
                game_system: b.game_system.as_deref().map(parse_game_system),
                version: b.version,
                tags: b.tags.unwrap_or_default(),
            },
            &actor,
        )
    };
    if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
    Ok(Json(serde_json::to_value(note).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}

async fn update_devnote(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
    Json(b): Json<PatchDevNote>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.lock().await;
        let patch = DevNotePatch {
            title: b.title,
            body: b.body,
            game_system: b.game_system.map(|opt| opt.map(|s| parse_game_system(&s))),
            version: b.version,
            tags: b.tags,
        };
        st.update_devnote(&id, patch, &actor)
    };
    match result {
        Ok(n) => {
            if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
            Ok(Json(serde_json::to_value(n).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn delete_devnote(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.lock().await;
        st.delete_devnote(&id)
    };
    match result {
        Ok(()) => {
            if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
            Ok(Json(serde_json::json!({ "deleted": id })))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn generate_worklog_handler(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<WorklogBody>,
) -> Json<serde_json::Value> {
    let actor = agent_from_headers(&h);
    let date = b.date.unwrap_or_else(|| {
        chrono::Local::now().date_naive().format("%Y-%m-%d").to_string()
    });
    let result = {
        let mut st = s.store.lock().await;
        st.generate_worklog(&date, b.agent.as_deref(), &actor)
    };
    if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
    Json(serde_json::json!({
        "created": result.created,
        "note_id": result.note_id,
        "todos_counted": result.todos_counted,
        "systems": result.systems,
    }))
}

async fn devnotes_by_version(
    State(s): State<AppState>,
    Path(ver): Path<String>,
) -> Json<serde_json::Value> {
    let st = s.store.lock().await;
    let notes: Vec<_> = st
        .list_devnotes(NoteFilter::default())
        .into_iter()
        .filter(|n| n.version.as_deref() == Some(ver.as_str()))
        .collect();
    Json(serde_json::json!({ "devnotes": notes, "version": ver }))
}

async fn devnotes_timeline(
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    let st = s.store.lock().await;
    let mut notes = st.list_devnotes(NoteFilter::default());
    notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(serde_json::json!({ "devnotes": notes }))
}
```

- [ ] **Step 5: Register routes in the router function**

In the `router` function in `crates/api-server/src/routes.rs`, add the devnote routes before the `.layer(cors)` line:

```rust
        .route("/devnotes", get(list_devnotes).post(add_devnote))
        .route("/devnotes/timeline", get(devnotes_timeline))
        .route("/devnotes/by-version/:ver", get(devnotes_by_version))
        .route("/devnotes/worklog/generate", post(generate_worklog_handler))
        .route("/devnotes/:id", get(one_devnote).patch(update_devnote).delete(delete_devnote))
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p api-server --test routes`
Expected: PASS

- [ ] **Step 7: Run full workspace test**

Run: `cargo test --workspace`
Expected: all PASS

- [ ] **Step 8: Commit**

```bash
git add crates/api-server/src/routes.rs crates/api-server/tests/routes.rs
git commit -m "feat(api): 8개 devnote 엔드포인트 — CRUD + worklog/generate + by-version + timeline"
```

---

## Task 6: MCP 도구 4개 (list/add/update_devnote, generate_worklog)

**Files:**
- Modify: `crates/mcp-server/src/tools.rs`
- Modify: `crates/mcp-server/tests/tools.rs`

**Interfaces:**
- Consumes: REST API endpoints from Task 5
- Produces: 4 MCP tool handlers in `dispatch()` + 4 catalog entries in `tool_catalog()`

- [ ] **Step 1: Read the existing MCP test file to understand the pattern**

```bash
cat crates/mcp-server/tests/tools.rs
```

- [ ] **Step 2: Write the failing test**

Append to `crates/mcp-server/tests/tools.rs` (follow the existing mock server pattern in that file):

```rust
#[tokio::test]
async fn list_devnotes_forwards_to_api() {
    let mock = mock_server(vec![]).await; // reuse existing helper or create one
    std::env::set_var("ACCESS_API_BASE", &mock.base);
    let result = dispatch((), ToolCall {
        name: "list_devnotes".into(),
        arguments: serde_json::json!({ "agent": "zcode" }),
    }).await.unwrap();
    std::env::remove_var("ACCESS_API_BASE");
    assert!(result["devnotes"].is_array());
}
```

Note: if the test file already has a `mock_server` helper, reuse it. If not, add a minimal mock:

```rust
async fn mock_devnote_server() -> WireMock {
    // Use wiremock or a simple HTTP mock — follow the existing pattern in tools.rs tests
    // If no HTTP mock library is available, test dispatch logic by checking
    // that the correct URL is constructed (integration test against app_for_test).
}
```

If the existing tests use a real mock server pattern, follow it. If they don't test HTTP forwarding (only catalog), then add catalog tests instead:

```rust
#[test]
fn catalog_includes_devnote_tools() {
    let catalog = tool_catalog();
    let names: Vec<&str> = catalog.iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"list_devnotes"));
    assert!(names.contains(&"add_devnote"));
    assert!(names.contains(&"update_devnote"));
    assert!(names.contains(&"generate_worklog"));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p mcp-server`
Expected: FAIL — devnote tools not in catalog / not in dispatch.

- [ ] **Step 4: Add dispatch handlers for 4 devnote tools**

In `crates/mcp-server/src/tools.rs`, add inside the `dispatch` function's `match` block, before the `other =>` arm:

```rust
        "list_devnotes" => {
            let mut url = format!("{base}/devnotes?");
            let mut first = true;
            if let Some(nt) = call.arguments.get("note_type").and_then(|v| v.as_str()) {
                url.push_str("note_type="); url.push_str(nt); first = false;
            }
            if let Some(gs) = call.arguments.get("game_system").and_then(|v| v.as_str()) {
                if !first { url.push('&'); }
                url.push_str("game_system="); url.push_str(gs); first = false;
            }
            if let Some(ver) = call.arguments.get("version").and_then(|v| v.as_str()) {
                if !first { url.push('&'); }
                url.push_str("version="); url.push_str(ver); first = false;
            }
            if !first { url.push('&'); }
            url.push_str("agent="); url.push_str(&agent);
            let r = client
                .get(&url)
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "add_devnote" => {
            let note_type = call.arguments["note_type"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("note_type required"))?;
            let title = call.arguments["title"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("title required"))?;
            let mut body = serde_json::json!({
                "note_type": note_type,
                "title": title,
                "body": call.arguments.get("body").and_then(|v| v.as_str()).unwrap_or(""),
            });
            if let Some(gs) = call.arguments.get("game_system").and_then(|v| v.as_str()) {
                body["game_system"] = Value::String(gs.into());
            }
            if let Some(ver) = call.arguments.get("version").and_then(|v| v.as_str()) {
                body["version"] = Value::String(ver.into());
            }
            body["tags"] = Value::Array(vec![Value::String(format!("agent:{agent}"))]);
            let r = client
                .post(format!("{base}/devnotes"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "update_devnote" => {
            let id = id_of(&call.arguments)?;
            let r = client
                .patch(format!("{base}/devnotes/{id}"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&call.arguments)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "generate_worklog" => {
            let mut body = serde_json::json!({});
            if let Some(date) = call.arguments.get("date").and_then(|v| v.as_str()) {
                body["date"] = Value::String(date.into());
            }
            if let Some(wl_agent) = call.arguments.get("agent").and_then(|v| v.as_str()) {
                body["agent"] = Value::String(wl_agent.into());
            }
            let r = client
                .post(format!("{base}/devnotes/worklog/generate"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
```

- [ ] **Step 5: Add 4 entries to tool_catalog()**

In `crates/mcp-server/src/tools.rs`, add to the `tool_catalog()` vec, before the closing `]`:

```rust
        serde_json::json!({
            "name": "list_devnotes",
            "description": "개발노트를 조회합니다. note_type, game_system, version으로 필터링 가능.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "note_type": { "type": "string", "enum": ["worklog", "retrospect", "issue", "playtest"] },
                    "game_system": { "type": "string", "enum": ["combat", "inventory", "quest", "skill", "ui", "audio", "map", "ai", "save", "network", "etc"] },
                    "version": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }
        }),
        serde_json::json!({
            "name": "add_devnote",
            "description": "개발노트를 추가합니다 (회고, 이슈, 플레이테스트). 에이전트가 작업 후 기록용.",
            "inputSchema": {
                "type": "object",
                "required": ["note_type", "title", "body"],
                "properties": {
                    "note_type": { "type": "string", "enum": ["retrospect", "issue", "playtest"], "description": "노트 타입 (worklog는 generate_worklog로 자동 생성)" },
                    "title": { "type": "string" },
                    "body": { "type": "string", "description": "마크다운 본문" },
                    "game_system": { "type": "string", "enum": ["combat", "inventory", "quest", "skill", "ui", "audio", "map", "ai", "save", "network", "etc"] },
                    "version": { "type": "string", "description": "예: v0.2-alpha" },
                    "agent": { "type": "string" }
                }
            }
        }),
        serde_json::json!({
            "name": "update_devnote",
            "description": "개발노트를 수정합니다.",
            "inputSchema": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "body": { "type": "string" },
                    "game_system": { "type": ["string", "null"] },
                    "version": { "type": ["string", "null"] },
                    "agent": { "type": "string" }
                }
            }
        }),
        serde_json::json!({
            "name": "generate_worklog",
            "description": "완료된 todo로부터 작업 로그를 자동 생성합니다. 게임 시스템별로 그룹화된 마크다운 노트가 만들어집니다.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "date": { "type": "string", "description": "기준 날짜 YYYY-MM-DD (기본: 오늘)" },
                    "agent": { "type": "string", "description": "특정 에이전트의 작업만 (선택)" }
                }
            }
        }),
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p mcp-server`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/mcp-server/src/tools.rs crates/mcp-server/tests/tools.rs
git commit -m "feat(mcp): 4개 devnote 도구 — list/add/update_devnote + generate_worklog"
```

---

## Task 7: 프론트엔드 api.ts — devnote 타입/함수

**Files:**
- Modify: `apps/desktop/src/api.ts`

**Interfaces:**
- Consumes: REST API endpoints from Task 5
- Produces: `DevNote` interface, `NoteType`, `GameSystem` types, `devnoteApi` object with methods. Consumed by Svelte components in Tasks 8-10.

- [ ] **Step 1: Add DevNote types and API methods**

In `apps/desktop/src/api.ts`, add the types after the existing `Todo` interface:

```typescript
export type NoteType = "worklog" | "retrospect" | "issue" | "playtest";
export type GameSystem = "combat" | "inventory" | "quest" | "skill" | "ui" | "audio" | "map" | "ai" | "save" | "network" | "etc";

export interface DevNote {
  id: string;
  note_type: NoteType;
  title: string;
  body: string;
  game_system?: GameSystem | null;
  version?: string | null;
  linked_todos: string[];
  tags: string[];
  created_at: string;
  created_by: string;
  updated_at?: string | null;
  updated_by?: string | null;
}

export interface WorklogResult {
  created: boolean;
  note_id: string;
  todos_counted: number;
  systems: string[];
}
```

Add the devnote API object after the existing `api` object:

```typescript
export const devnoteApi = {
  list: (params?: { note_type?: NoteType; game_system?: GameSystem; version?: string }) => {
    const qs = new URLSearchParams();
    if (params?.note_type) qs.set("note_type", params.note_type);
    if (params?.game_system) qs.set("game_system", params.game_system);
    if (params?.version) qs.set("version", params.version);
    const query = qs.toString();
    return req<{ devnotes: DevNote[] }>(`/devnotes${query ? "?" + query : ""}`);
  },
  timeline: () => req<{ devnotes: DevNote[] }>("/devnotes/timeline"),
  byVersion: (ver: string) => req<{ devnotes: DevNote[]; version: string }>(`/devnotes/by-version/${ver}`),
  create: (data: { note_type: NoteType; title: string; body: string; game_system?: GameSystem; version?: string; tags?: string[] }) =>
    req<DevNote>("/devnotes", { method: "POST", body: JSON.stringify(data) }),
  update: (id: string, data: { title?: string; body?: string; game_system?: GameSystem | null; version?: string | null }) =>
    req<DevNote>(`/devnotes/${id}`, { method: "PATCH", body: JSON.stringify(data) }),
  del: (id: string) => req<{ deleted: string }>(`/devnotes/${id}`, { method: "DELETE" }),
  generateWorklog: (date?: string) =>
    req<WorklogResult>("/devnotes/worklog/generate", { method: "POST", body: JSON.stringify({ date }) }),
};
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/api.ts
git commit -m "feat(frontend): DevNote 타입 + devnoteApi 함수 (list/timeline/create/update/del/generateWorklog)"
```

---

## Task 8: DevNoteCard.svelte — 노트 카드 컴포넌트

**Files:**
- Create: `apps/desktop/src/DevNoteCard.svelte`

**Interfaces:**
- Consumes: `DevNote` type from api.ts
- Produces: A card UI component that displays a single note with type icon, system badge, version badge, and body preview.

- [ ] **Step 1: Create DevNoteCard.svelte**

```svelte
<script lang="ts">
  import type { DevNote, GameSystem } from "./api";

  export let note: DevNote;
  export let onEdit: (note: DevNote) => void;
  export let onDelete: (id: string) => void;

  const typeMeta: Record<string, { emoji: string; label: string; color: string }> = {
    worklog: { emoji: "📋", label: "작업 로그", color: "#3ca87e" },
    retrospect: { emoji: "💭", label: "회고", color: "#3c7ab0" },
    issue: { emoji: "🐛", label: "이슈", color: "#c46b3c" },
    playtest: { emoji: "🎮", label: "플레이테스트", color: "#7a2a9c" },
  };

  const systemLabels: Record<string, string> = {
    combat: "⚔️ 전투", inventory: "🎒 인벤토리", quest: "📜 퀘스트",
    skill: "✨ 스킬", ui: "🖥️ UI", audio: "🔊 오디오",
    map: "🗺️ 맵", ai: "🤖 AI", save: "💾 세이브",
    network: "🌐 네트워크", etc: "📦 기타",
  };

  $: meta = typeMeta[note.note_type] || typeMeta.retrospect;
  $: isWorklog = note.note_type === "worklog";
  $: dateStr = new Date(note.created_at).toLocaleDateString("ko-KR", { month: "short", day: "numeric" });
</script>

<div class="card" style="--accent: {meta.color}">
  <div class="header">
    <span class="type-badge">{meta.emoji} {meta.label}</span>
    {#if note.game_system}
      <span class="sys-badge">{systemLabels[note.game_system] || note.game_system}</span>
    {/if}
    {#if note.version}
      <span class="ver-badge">📦 {note.version}</span>
    {/if}
  </div>
  <h3 class="title">{note.title}</h3>
  <div class="body">{note.body}</div>
  <div class="footer">
    <span class="meta">✍️ {note.created_by} · {dateStr}</span>
    {#if !isWorklog}
      <div class="actions">
        <button class="btn-edit" on:click={() => onEdit(note)}>✏️ 수정</button>
        <button class="btn-del" on:click={() => onDelete(note.id)}>🗑️</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg-soft, #f8f9fa);
    border-left: 4px solid var(--accent);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 10px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.08);
  }
  .header { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 6px; }
  .type-badge {
    background: var(--accent); color: white; font-size: 11px;
    padding: 2px 8px; border-radius: 10px; font-weight: 600;
  }
  .sys-badge, .ver-badge {
    background: #e0e0e0; color: #555; font-size: 11px;
    padding: 2px 8px; border-radius: 10px;
  }
  .title { margin: 4px 0; font-size: 14px; font-weight: 700; }
  .body {
    font-size: 12px; white-space: pre-wrap; line-height: 1.5;
    color: #444; max-height: 300px; overflow-y: auto;
  }
  .footer {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 6px; border-top: 1px solid #eee;
  }
  .meta { font-size: 10px; color: #999; }
  .actions { display: flex; gap: 4px; }
  .btn-edit, .btn-del {
    border: none; background: none; cursor: pointer; font-size: 11px; padding: 2px 6px;
  }
  .btn-edit:hover { color: var(--accent); }
  .btn-del:hover { color: #c00; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/DevNoteCard.svelte
git commit -m "feat(frontend): DevNoteCard 컴포넌트 — 타입/시스템/버전 배지 + 본문 미리보기"
```

---

## Task 9: DevNoteEditor.svelte — 노트 에디터 모달

**Files:**
- Create: `apps/desktop/src/DevNoteEditor.svelte`

**Interfaces:**
- Consumes: `DevNote`, `NoteType`, `GameSystem`, `devnoteApi` from api.ts
- Produces: Modal form for creating/editing notes (retrospect/issue/playtest only).

- [ ] **Step 1: Create DevNoteEditor.svelte**

```svelte
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { devnoteApi, type DevNote, type NoteType, type GameSystem } from "./api";

  export let existing: DevNote | null = null;

  const dispatch = createEventDispatcher();

  let noteType: NoteType = existing?.note_type || "retrospect";
  let title = existing?.title || "";
  let body = existing?.body || "";
  let gameSystem: GameSystem | "" = existing?.game_system || "";
  let version = existing?.version || "";
  let saving = false;

  const typeOptions: { value: NoteType; label: string; emoji: string }[] = [
    { value: "retrospect", label: "회고", emoji: "💭" },
    { value: "issue", label: "이슈", emoji: "🐛" },
    { value: "playtest", label: "플레이테스트", emoji: "🎮" },
  ];

  const systemOptions: { value: GameSystem; label: string }[] = [
    { value: "combat", label: "⚔️ 전투" },
    { value: "inventory", label: "🎒 인벤토리" },
    { value: "quest", label: "📜 퀘스트" },
    { value: "skill", label: "✨ 스킬" },
    { value: "ui", label: "🖥️ UI" },
    { value: "audio", label: "🔊 오디오" },
    { value: "map", label: "🗺️ 맵" },
    { value: "ai", label: "🤖 AI" },
    { value: "save", label: "💾 세이브" },
    { value: "network", label: "🌐 네트워크" },
    { value: "etc", label: "📦 기타" },
  ];

  async function save() {
    if (!title.trim() || !body.trim()) return;
    saving = true;
    try {
      const data: any = {
        note_type: noteType,
        title: title.trim(),
        body: body.trim(),
      };
      if (gameSystem) data.game_system = gameSystem;
      if (version.trim()) data.version = version.trim();

      if (existing) {
        await devnoteApi.update(existing.id, data);
      } else {
        await devnoteApi.create(data);
      }
      dispatch("saved");
    } catch (e) {
      alert(`저장 실패: ${(e as Error).message}`);
    } finally {
      saving = false;
    }
  }

  function close() {
    dispatch("cancel");
  }
</script>

<div class="overlay" on:click={close}>
  <div class="modal" on:click|stopPropagation>
    <h2>{existing ? "노트 수정" : "새 노트"}</h2>

    <label>타입</label>
    <div class="type-row">
      {#each typeOptions as opt}
        <button
          class="type-btn"
          class:active={noteType === opt.value}
          on:click={() => (noteType = opt.value)}
        >
          {opt.emoji} {opt.label}
        </button>
      {/each}
    </div>

    <label>제목</label>
    <input type="text" bind:value={title} placeholder="노트 제목" />

    <label>본문 (마크다운)</label>
    <textarea bind:value={body} rows="8" placeholder="무엇을 했나요? 무엇을 배웠나요?"></textarea>

    <div class="row">
      <div>
        <label>게임 시스템</label>
        <select bind:value={gameSystem}>
          <option value="">선택 안함</option>
          {#each systemOptions as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div>
        <label>버전</label>
        <input type="text" bind:value={version} placeholder="예: v0.2-alpha" />
      </div>
    </div>

    <div class="actions">
      <button class="btn-cancel" on:click={close}>취소</button>
      <button class="btn-save" on:click={save} disabled={saving || !title.trim() || !body.trim()}>
        {saving ? "저장중..." : "저장"}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.4); display: flex; align-items: center;
    justify-content: center; z-index: 100;
  }
  .modal {
    background: white; border-radius: 12px; padding: 20px;
    width: 90%; max-width: 500px; max-height: 85vh; overflow-y: auto;
  }
  h2 { margin: 0 0 16px; font-size: 18px; }
  label { display: block; font-size: 12px; font-weight: 600; color: #666; margin: 10px 0 4px; }
  input, textarea, select {
    width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 6px;
    font-size: 13px; box-sizing: border-box;
  }
  textarea { font-family: monospace; resize: vertical; }
  .type-row { display: flex; gap: 6px; margin-bottom: 4px; }
  .type-btn {
    flex: 1; padding: 8px; border: 2px solid #ddd; border-radius: 8px;
    background: white; cursor: pointer; font-size: 12px;
  }
  .type-btn.active { border-color: #3ca87e; background: #dcf5ec; }
  .row { display: flex; gap: 12px; }
  .row > div { flex: 1; }
  .actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
  .btn-cancel { padding: 8px 16px; border: none; background: #eee; border-radius: 6px; cursor: pointer; }
  .btn-save {
    padding: 8px 16px; border: none; background: #3ca87e; color: white;
    border-radius: 6px; cursor: pointer; font-weight: 600;
  }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/DevNoteEditor.svelte
git commit -m "feat(frontend): DevNoteEditor 모달 — 타입/제목/본문/시스템/버전 입력"
```

---

## Task 10: DevNoteWidget.svelte — 메인 위젯

**Files:**
- Create: `apps/desktop/src/DevNoteWidget.svelte`

**Interfaces:**
- Consumes: `devnoteApi`, `DevNote`, `NoteType`, `GameSystem` from api.ts; `DevNoteCard` from Task 8; `DevNoteEditor` from Task 9
- Produces: Full widget UI with tab filters, system/version filter bar, note list, and action buttons.

- [ ] **Step 1: Create DevNoteWidget.svelte**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { devnoteApi, type DevNote, type NoteType, type GameSystem } from "./api";
  import DevNoteCard from "./DevNoteCard.svelte";
  import DevNoteEditor from "./DevNoteEditor.svelte";

  let notes: DevNote[] = [];
  let activeTab: NoteType | "all" = "all";
  let systemFilter: GameSystem | "" = "";
  let versionFilter = "";
  let searchQuery = "";
  let showEditor = false;
  let editingNote: DevNote | null = null;
  let synced = "로딩중";
  let errorMsg = "";
  let generating = false;
  let allVersions: string[] = [];

  const tabs: { value: NoteType | "all"; label: string; emoji: string }[] = [
    { value: "all", label: "전체", emoji: "📋" },
    { value: "worklog", label: "작업로그", emoji: "📋" },
    { value: "retrospect", label: "회고", emoji: "💭" },
    { value: "issue", label: "이슈", emoji: "🐛" },
    { value: "playtest", label: "테스트", emoji: "🎮" },
  ];

  async function refresh() {
    try {
      const r = await devnoteApi.timeline();
      notes = r.devnotes;
      // collect unique versions
      const vset = new Set<string>();
      notes.forEach((n) => { if (n.version) vset.add(n.version); });
      allVersions = Array.from(vset).sort().reverse();
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
    } catch (e) {
      errorMsg = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  $: filtered = notes
    .filter((n) => activeTab === "all" || n.note_type === activeTab)
    .filter((n) => !systemFilter || n.game_system === systemFilter)
    .filter((n) => !versionFilter || n.version === versionFilter)
    .filter(
      (n) =>
        !searchQuery ||
        n.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        n.body.toLowerCase().includes(searchQuery.toLowerCase())
    );

  function openNewEditor() {
    editingNote = null;
    showEditor = true;
  }
  function openEditEditor(note: DevNote) {
    editingNote = note;
    showEditor = true;
  }
  async function deleteNote(id: string) {
    if (!confirm("이 노트를 삭제할까요?")) return;
    await devnoteApi.del(id);
    await refresh();
  }
  async function generateToday() {
    generating = true;
    try {
      const r = await devnoteApi.generateWorklog();
      if (r.created) {
        alert(`작업 로그 생성! ${r.todos_counted}개 작업, 시스템: ${r.systems.join(", ")}`);
      } else {
        alert(`기존 로그 업데이트. ${r.todos_counted}개 작업 반영.`);
      }
      await refresh();
    } catch (e) {
      alert(`생성 실패: ${(e as Error).message}`);
    } finally {
      generating = false;
    }
  }
  function onSaved() {
    showEditor = false;
    refresh();
  }

  onMount(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  });
</script>

<div class="widget">
  <header class="top-bar">
    <span class="title">📝 게임 개발노트</span>
    <span class="synced">{synced}</span>
  </header>

  {#if errorMsg}
    <div class="error">{errorMsg}</div>
  {/if}

  <div class="tabs">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeTab === tab.value}
        on:click={() => (activeTab = tab.value)}
      >
        {tab.emoji} {tab.label}
      </button>
    {/each}
  </div>

  <div class="filter-bar">
    <input type="text" bind:value={searchQuery} placeholder="🔍 검색..." />
    <select bind:value={systemFilter}>
      <option value="">🎮 전체 시스템</option>
      <option value="combat">⚔️ 전투</option>
      <option value="inventory">🎒 인벤토리</option>
      <option value="quest">📜 퀘스트</option>
      <option value="skill">✨ 스킬</option>
      <option value="ui">🖥️ UI</option>
      <option value="audio">🔊 오디오</option>
      <option value="map">🗺️ 맵</option>
      <option value="ai">🤖 AI</option>
      <option value="save">💾 세이브</option>
      <option value="network">🌐 네트워크</option>
      <option value="etc">📦 기타</option>
    </select>
    <select bind:value={versionFilter}>
      <option value="">📦 전체 버전</option>
      {#each allVersions as v}
        <option value={v}>{v}</option>
      {/each}
    </select>
  </div>

  <div class="note-list">
    {#if filtered.length === 0}
      <div class="empty">노트가 없습니다. 새 노트를 추가하거나 작업 로그를 생성하세요.</div>
    {:else}
      {#each filtered as note (note.id)}
        <DevNoteCard {note} onEdit={openEditEditor} onDelete={deleteNote} />
      {/each}
    {/if}
  </div>

  <footer class="bottom-bar">
    <button class="btn-new" on:click={openNewEditor}>+ 새 노트</button>
    <button class="btn-worklog" on:click={generateToday} disabled={generating}>
      {generating ? "생성중..." : "⚡ 오늘 로그"}
    </button>
  </footer>
</div>

{#if showEditor}
  <DevNoteEditor existing={editingNote} on:saved={onSaved} on:cancel={() => (showEditor = false)} />
{/if}

<style>
  .widget {
    display: flex; flex-direction: column; height: 100vh;
    font-family: -apple-system, "Segoe UI", sans-serif; background: #fafafa;
  }
  .top-bar {
    display: flex; justify-content: space-between; align-items: center;
    padding: 10px 14px; background: #3ca87e; color: white;
  }
  .title { font-size: 14px; font-weight: 700; }
  .synced { font-size: 10px; opacity: 0.8; }
  .error { padding: 8px; background: #fee; color: #c00; font-size: 12px; }
  .tabs { display: flex; gap: 2px; padding: 6px 8px; background: white; border-bottom: 1px solid #eee; }
  .tab {
    flex: 1; padding: 6px 4px; border: none; background: none; cursor: pointer;
    font-size: 11px; border-radius: 6px; white-space: nowrap;
  }
  .tab.active { background: #dcf5ec; color: #1a5a44; font-weight: 600; }
  .filter-bar {
    display: flex; gap: 4px; padding: 6px 8px; background: white; border-bottom: 1px solid #eee;
  }
  .filter-bar input, .filter-bar select {
    flex: 1; padding: 4px 6px; font-size: 11px; border: 1px solid #ddd; border-radius: 4px;
  }
  .note-list { flex: 1; overflow-y: auto; padding: 8px; }
  .empty { text-align: center; color: #999; font-size: 12px; padding: 40px 20px; }
  .bottom-bar {
    display: flex; gap: 8px; padding: 10px 14px; background: white; border-top: 1px solid #eee;
  }
  .btn-new {
    flex: 1; padding: 10px; border: none; background: #3ca87e; color: white;
    border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 13px;
  }
  .btn-worklog {
    flex: 1; padding: 10px; border: none; background: #fff3cd; color: #856404;
    border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 13px;
  }
  .btn-worklog:disabled, .btn-new:hover { opacity: 0.85; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/DevNoteWidget.svelte
git commit -m "feat(frontend): DevNoteWidget 메인 위젯 — 탭/필터/검색 + 작업로그 생성"
```

---

## Task 11: App.svelte 분기 + Tauri 창 추가

**Files:**
- Modify: `apps/desktop/src/main.ts`
- Modify: `apps/desktop/src/App.svelte`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `DevNoteWidget` from Task 10; existing `getWindowAgent()` from api.ts
- Produces: When the Tauri window label is `devnotes`, render `DevNoteWidget` instead of the post-it App.

- [ ] **Step 1: Read main.ts to understand the entry point**

```bash
cat apps/desktop/src/main.ts
```

- [ ] **Step 2: Modify main.ts to branch by window type**

The `main.ts` currently mounts `App`. Branch based on window label. Replace `apps/desktop/src/main.ts` content (preserving any existing imports/styles):

```typescript
import "./app.css";
import App from "./App.svelte";
import DevNoteWidget from "./DevNoteWidget.svelte";
import { getWindowAgent } from "./api";

const agent = getWindowAgent();

// devnotes 창이면 별도 위젯 렌더링
const app = agent === "devnotes" ? new DevNoteWidget({ target: document.body }) : new App({ target: document.body });

export default app;
```

- [ ] **Step 3: Add devnotes window to Tauri lib.rs**

In `apps/desktop/src-tauri/src/lib.rs`, add `devnotes` to the `AGENT_WINDOWS` const array. Add this entry as the 5th element:

```rust
const AGENT_WINDOWS: &[(&str, &str, f64, f64, &str)] = &[
    ("hermes", "🤖 Hermes", 50.0, 50.0, "#ffd0e8"),
    ("omp", "🛠️ OMP", 390.0, 50.0, "#c8e0f5"),
    ("zcode", "⚡ ZCode", 50.0, 560.0, "#c8f0e0"),
    ("user", "👤 내 할 일", 390.0, 560.0, "#fff0a0"),
    ("devnotes", "📝 게임 개발노트", 730.0, 50.0, "#c8f0e0"),
];
```

- [ ] **Step 4: Verify frontend builds**

Run: `cd apps/desktop && npm run build`
Expected: build succeeds

- [ ] **Step 5: Verify Rust compiles (Tauri side)**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/main.ts apps/desktop/src-tauri/src/lib.rs
git commit -m "feat: 5번째 창 '게임 개발노트' 추가 + devnotes 분기 렌더링"
```

---

## Task 12: JSON 스키마 업데이트 + 통합 테스트

**Files:**
- Modify: `shared/schema.json`

**Interfaces:**
- Consumes: `DevNote`, `NoteType`, `GameSystem` definitions
- Produces: Updated `schema.json` with `DevNote` definition and `dev_notes` on `TodoDoc`.

- [ ] **Step 1: Update schema.json**

In `shared/schema.json`, add `dev_notes` to the top-level `properties` and add the `DevNote` definition to `$defs`. The `properties` section should include:

```json
"dev_notes": {
  "type": "array",
  "items": { "$ref": "#/$defs/DevNote" }
}
```

(Not added to `required` — it's optional for backward compat.)

Add to `$defs`:

```json
"DevNote": {
  "type": "object",
  "required": ["id", "note_type", "title", "body", "created_at", "created_by"],
  "properties": {
    "id": { "type": "string" },
    "note_type": { "enum": ["worklog", "retrospect", "issue", "playtest"] },
    "title": { "type": "string" },
    "body": { "type": "string" },
    "game_system": { "enum": ["combat", "inventory", "quest", "skill", "ui", "audio", "map", "ai", "save", "network", "etc"] },
    "version": { "type": ["string", "null"] },
    "linked_todos": { "type": "array", "items": { "type": "string" } },
    "tags": { "type": "array", "items": { "type": "string" } },
    "created_at": { "type": "string", "format": "date-time" },
    "created_by": { "type": "string" },
    "updated_at": { "type": ["string", "null"] },
    "updated_by": { "type": ["string", "null"] }
  }
}
```

- [ ] **Step 2: Run full workspace test**

Run: `cargo test --workspace`
Expected: all PASS

- [ ] **Step 3: Run frontend build**

Run: `cd apps/desktop && npm run build`
Expected: build succeeds

- [ ] **Step 4: Commit**

```bash
git add shared/schema.json
git commit -m "feat: JSON 스키마에 DevNote 정의 추가 + dev_notes 배열"
```

---

## Task 13: 최종 통합 검증 + 빌드

**Files:** None (verification only)

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test --workspace`
Expected: all PASS (core: model + store + devnote + merge; api: routes; mcp: tools)

- [ ] **Step 2: Build the full workspace in release**

Run: `cargo build --workspace --release`
Expected: compiles successfully

- [ ] **Step 3: Build the desktop frontend**

Run: `cd apps/desktop && npm run build`
Expected: build succeeds

- [ ] **Step 4: Verify Tauri check**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: no errors

- [ ] **Step 5: Final commit (if any uncommitted changes)**

```bash
git add -A
git commit -m "feat: 게임 개발노트 툴 완성 — 코어/API/MCP/위젯 통합" || echo "nothing to commit"
```

- [ ] **Step 6: Report completion**

Confirm:
- Core data model: `DevNote`, `NoteType`, `GameSystem` ✓
- Worklog auto-generation: `generate_worklog()` + idempotent store method ✓
- 3-way merge for `dev_notes` ✓
- 8 API endpoints under `/devnotes` ✓
- 4 MCP tools: `list_devnotes`, `add_devnote`, `update_devnote`, `generate_worklog` ✓
- Frontend: `DevNoteWidget`, `DevNoteCard`, `DevNoteEditor` + 5th Tauri window ✓
- JSON schema updated ✓
