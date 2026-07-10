# 게임 개발노트 툴 — 설계 스펙

> **날짜**: 2026-07-11
> **상위 프로젝트**: Access (에이전트 정리 가이드 시스템)
> **접근 방식**: 하이브리드 — 기존 `TodoDoc`에 `dev_notes` 배열 추가

---

## 1. 목적

게임 개발 과정에서 **무엇을 했고, 왜 했으며, 어떤 이슈가 있었는지**를 기록하고
유저(개발자 본인 + 협업자 + 커뮤니티)에게 보여주는 개발노트 툴.

### 핵심 가치
- **작업 로그 자동화** — 개발자가 일일이 적지 않아도, Access의 완료된 todo 데이터로 작업 로그가 자동 생성됨
- **게임 특화 분류** — 전투/인벤토리/퀘스트 등 게임 시스템별, 버전/패치 단위로 노트 정리
- **Access 통합** — 기존 Gist 동기화, 머지 로직, Tauri 프레임워크를 그대로 재사용

### 비목표 (YAGNI)
- Git 커밋/PR 연동 (명시적으로 제외됨)
- 웹 브라우저 조회 (데스크톱 위젯으로 충분)
- 마크다운 파일 내보내기 (필요시 별도 작업)
- 다중 게임 프로젝트 관리 (단일 게임에 집중)

---

## 2. 데이터 모델

### 2.1 새 엔티티: `DevNote`

`crates/core/src/model.rs`에 추가.

```rust
/// 개발노트의 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Worklog,     // 작업 로그 (자동 생성)
    Retrospect,  // 회고/메모 (수동)
    Issue,       // 이슈/버그 노트
    Playtest,    // 플레이테스트 노트
}

/// 게임 시스템 분류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameSystem {
    Combat,      // 전투
    Inventory,   // 인벤토리
    Quest,       // 퀘스트
    Skill,       // 스킬
    Ui,          // UI
    Audio,       // 오디오
    Map,         // 맵
    Ai,          // AI
    Save,        // 세이브
    Network,     // 네트워크
    Etc,         // 기타
}

/// 개발노트 엔티티
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevNote {
    pub id: String,
    pub note_type: NoteType,
    pub title: String,
    pub body: String,                       // 마크다운 본문 (긴 텍스트 가능)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<GameSystem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,            // 예: "v0.1-alpha", "v0.2-combat"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_todos: Vec<String>,          // 연결된 todo id들 (작업 로그용)
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
}
```

### 2.2 `TodoDoc` 확장

```rust
pub struct TodoDoc {
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[serde(default)]
    pub todos: Vec<Todo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dev_notes: Vec<DevNote>,            // ⭐ 새 필드
}
```

**하위 호환성**: `dev_notes`는 `skip_serializing_if = "Vec::is_empty"` + `default`이므로
빈 경우 직렬화에서 제외됨. 기존 Android 앱, 기존 Gist 데이터에 영향 없음.

### 2.3 ID 생성

```rust
pub fn new_note_id() -> String {
    format!("note-{}", Uuid::new_v4())  // 가독성을 위한 prefix
}
```

---

## 3. 작업 로그 자동 생성

### 3.1 Game System 태그 규칙

기존 `Todo` 구조체에는 `game_system` 필드가 없으므로, **tags 배열**에 규칙화된 태그로 게임 시스템을 표현한다:

```
tags: ["agent:zcode", "system:combat", "version:v0.2-alpha"]
```

- `system:<name>` — 게임 시스템 분류 (combat, inventory, quest, skill, ui, audio, map, ai, save, network, etc). 없으면 Etc로 분류
- `version:<ver>` — 버전 태그 (옵션). 없으면 미지정

### 3.2 생성 로직 (`crates/core/src/devnote.rs` 새 파일)

입력: 기준 날짜 (기본 오늘), 에이전트 필터 (옵션)

```
1. 완료된 todo 중 completed_at이 기준 날짜인 것들을 수집
2. 각 todo의 tags에서 "system:<name>" 추출 → 해당 시스템으로 분류 (없으면 Etc)
3. 각 game_system 그룹별로 bullet 리스트 생성:
   - bullet = "{todo.title}: {todo.note(summary)}"
4. "version:<ver>" 태그가 있으면 그룹 제목에 반영
5. DevNote(note_type=Worklog) 생성 후 dev_notes에 추가
```

### 3.2 생성된 로그 예시

```markdown
# 2026-07-11 작업 로그
*총 5개 작업 완료 · 작성자: zcode*

## Combat (v0.2-alpha)
- 적 패턴 구현: 상태머신 추가하여 3가지 공격 패턴 구현
- 체력 시스템: HP와 실드 분리하여 데미지 흡력 구조 변경
- 데미지 계산: 방어력 공식 수정, 최소 데미지 보장

## UI
- 체력바 UI: 실드 게이지 별도 표시 추가

## Inventory
- 아이템 드롭: 적 사망 시 확률 기반 드롭 로직
```

### 3.3 트리거

1. **수동**: 위젯의 "⚡ 오늘 로그" 버튼 → `POST /devnotes/worklog/generate`
2. **완료 시 자동**: `complete_todo` 호출 시, 오늘 날짜의 Worklog가 없으면 생성, 있으면 업데이트 (기존 로그에 bullet 추가)
3. **옵션 - 하루 마감**: 밤 11시 자동 생성 (별도 스케줄러, MVP 이후)

### 3.4 중복 방지

- 같은 날짜 + 같은 agent의 Worklog는 1개만 존재
- 이미 linked_todos에 포함된 todo id는 중복 추가하지 않음
- 재실행 시 누락된 todo만 추가 (idempotent)

---

## 4. API 엔드포인트

`crates/api-server/src/routes.rs`에 추가. 기존 패턴 준수.

### 4.1 노트 CRUD

| 메서드 | 경로 | 설명 |
|--------|------|------|
| `GET` | `/devnotes` | 노트 목록. 쿼리: `?type=worklog&system=combat&version=v0.2&agent=zcode` |
| `GET` | `/devnotes/:id` | 노트 1개 조회 |
| `POST` | `/devnotes` | 노트 생성 (수동: 회고, 이슈, 플레이테스트) |
| `PATCH` | `/devnotes/:id` | 노트 수정 (제목, 본문, 시스템, 버전) |
| `DELETE` | `/devnotes/:id` | 노트 삭제 |

### 4.2 특수 엔드포인트

| 메서드 | 경로 | 설명 |
|--------|------|------|
| `POST` | `/devnotes/worklog/generate` | 작업 로그 자동 생성. Body: `{ "date": "2026-07-11", "agent": "zcode" }` |
| `GET` | `/devnotes/by-version/:ver` | 버전별 패치 노트 모음. 모든 type의 노트 중 version 일치하는 것 |
| `GET` | `/devnotes/timeline` | 타임라인 뷰. 최신순 모든 노트. 쿼리: `?limit=50` |

### 4.3 요청/응답 예시

**POST /devnotes** (수동 노트 생성):
```json
{
  "note_type": "retrospect",
  "title": "밸런스 회고",
  "body": "적 체력이 너무 높아서 초반 난이도 조정 필요...",
  "game_system": "combat",
  "version": "v0.2-alpha",
  "tags": ["balance", "agent:zcode"]
}
```

**POST /devnotes/worklog/generate** 응답:
```json
{
  "created": true,
  "note_id": "note-abc123",
  "todos_counted": 5,
  "systems": ["combat", "ui", "inventory"]
}
```

---

## 5. MCP 도구 (에이전트용)

`crates/mcp-server/`에 4개 도구 추가. 기존 9개 도구 패턴 준수.

| 도구 | 파라미터 | 설명 |
|------|---------|------|
| `list_devnotes` | `note_type?`, `game_system?`, `version?`, `agent?` | 노트 조회 (필터 지원) |
| `add_devnote` | `note_type`, `title`, `body`, `game_system?`, `version?`, `tags?` | 노트 추가 (에이전트가 회고/이슈 기록) |
| `update_devnote` | `id`, `title?`, `body?`, `game_system?`, `version?` | 노트 수정 (본문 업데이트 등) |
| `generate_worklog` | `date?`, `agent?` | 작업 로그 자동 생성 |

**에이전트 사용 시나리오**: 에이전트가 버그 수정 후 이슈 노트 작성
```
add_devnote(
  note_type="issue",
  title="적이 벽을 뚫음",
  body="NavMesh 갱신 지연으로 적이 벽 통과. fixed: 갱신 주기 0.5s→0.1s",
  game_system="ai",
  version="v0.2-alpha"
)
```

---

## 6. 데스크톱 위젯 (Tauri + Svelte)

### 6.1 5번째 창 추가

기존 4개 창(hermes/omp/zcode/user)에 "개발노트" 창 추가.
`apps/desktop/src-tauri/tauri.conf.json`의 `windows` 배열에 항목 추가.

### 6.2 UI 구성

```
┌─────────────────────────────────────┐
│ 📝 게임 개발노트              📌 ✕  │  always-on-top 토글 (기존 패턴)
├─────────────────────────────────────┤
│ [전체] [작업로그] [회고] [이슈] [테스트]│  NoteType 탭 필터
├─────────────────────────────────────┤
│ 🔍 검색...    🎮 시스템▼  📦버전▼   │  필터 바
├─────────────────────────────────────┤
│ ┌─────────────────────────────────┐ │
│ │ 📋 2026-07-11 작업 로그         │ │  Worklog (자동 생성)
│ │ 🎮 Combat · v0.2-alpha          │ │
│ │ • 적 패턴 구현: 상태머신...     │ │
│ │ • 체력 시스템: HP/실드 분리     │ │
│ │ └── zcode · 5개 작업            │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │ 💭 밸런스 회고                  │ │  Retrospect (수동)
│ │ 🎮 Combat                       │ │
│ │ 적 체력이 너무 높아서...        │ │
│ └─────────────────────────────────┘ │
├─────────────────────────────────────┤
│     [+ 새 노트]  [⚡ 오늘 로그]     │  하단 액션 버튼
└─────────────────────────────────────┘
```

### 6.3 노트 카드 컴포넌트

각 노트는 카드 형태로 표시:
- **타입 아이콘**: 📋 Worklog / 💭 Retrospect / 🐛 Issue / 🎮 Playtest
- **메타 정보**: game_system, version, created_by, 날짜
- **본문 미리보기**: 최대 3줄, 클릭 시 확장
- **Worklog**: 편집 불가 (read-only), 내부 bullet만 표시
- **수동 노트**: 더블클릭 시 편집 모달

### 6.4 노트 에디터 모달

- 타입 선택 (회고/이슈/테스트 — Worklog는 자동만)
- 제목, 본문 (textarea, 마크다운)
- game_system 드롭다운
- version 입력 (자유 텍스트 + 이전 버전 제안)
- tags

### 6.5 프론트엔드 파일 변경

| 파일 | 변경 |
|------|------|
| `apps/desktop/src/api.ts` | `getDevNotes()`, `createDevNote()`, `updateDevNote()`, `deleteDevNote()`, `generateWorklog()` 추가 |
| `apps/desktop/src/App.svelte` | `windowType === 'devnotes'` 분기 추가, 기존 포스트잇 렌더링과 분리 |
| `apps/desktop/src/DevNoteWidget.svelte` | ⭐ 새 컴포넌트 — 노트 위젯 전체 |
| `apps/desktop/src/DevNoteCard.svelte` | ⭐ 새 컴포넌트 — 개별 노트 카드 |
| `apps/desktop/src/DevNoteEditor.svelte` | ⭐ 새 컴포넌트 — 노트 에디터 모달 |
| `apps/desktop/src-tauri/tauri.conf.json` | `windows` 배열에 개발노트 창 추가 |

---

## 7. Store 계층 변경 (`crates/core/src/store.rs`)

기존 `TodoStore`에 개발노트 메서드 추가:

```rust
impl TodoStore {
    // 기존 메서드들...

    // ⭐ 개발노트 메서드
    pub fn list_devnotes(&self, filter: NoteFilter) -> Vec<&DevNote>;
    pub fn get_devnote(&self, id: &str) -> Option<&DevNote>;
    pub fn add_devnote(&mut self, input: DevNoteInput, actor: &str) -> DevNote;
    pub fn update_devnote(&mut self, id: &str, patch: DevNotePatch, actor: &str) -> Result<DevNote>;
    pub fn delete_devnote(&mut self, id: &str, actor: &str) -> Result<()>;
    pub fn generate_worklog(&mut self, date: &str, agent: Option<&str>, actor: &str) -> WorklogResult;
}
```

`generate_worklog` 내부 로직:
1. 해당 날짜의 완료된 todo 수집
2. 기존 Worklog 노트 있는지 확인 (같은 날짜 + 같은 agent)
3. 있으면 업데이트 (누락된 todo만 추가), 없으면 새로 생성
4. game_system별 그룹화하여 body 마크다운 생성

---

## 8. Gist 동기화 호환성

### 기존 머지 로직 (`crates/core/src/merge.rs`)

현재 3-way 머지는 `todos`와 `categories` 배열을 다룸.
`dev_notes`도 동일한 원리로 머지 가능:

- **id 기준 식별**: 각 DevNote는 `note-<uuid>` id로 식별
- **필드 단위 머지**: todo와 동일하게 id가 같으면 최신 updated_at 기준으로 선택
- **삭제 처리**: 한쪽에서 삭제된 노트는 다른쪽에서 유지되어도 삭제 (todo와 동일)

머지 로직 확장 필요: `merge.rs`에 `dev_notes` 배열 처리 추가.

---

## 9. JSON 스키마 업데이트 (`shared/schema.json`)

`DevNote` 정의 추가 및 `TodoDoc`의 properties에 `dev_notes` 추가.

---

## 10. 빌드 순서 (구현 단계)

1. **코어 데이터 모델** — `model.rs`에 `DevNote`, `NoteType`, `GameSystem` 추가
2. **Store 메서드** — `store.rs`에 devnote CRUD + `generate_worklog`
3. **작업 로그 생성 로직** — `devnote.rs` 신규 파일, 그룹화 + 마크다운 생성
4. **머지 로직 확장** — `merge.rs`에 `dev_notes` 처리
5. **API 라우트** — `routes.rs`에 8개 엔드포인트
6. **MCP 도구** — `mcp-server`에 4개 도구
7. **프론트엔드** — `DevNoteWidget.svelte` 등 신규 컴포넌트
8. **Tauri 창 설정** — `tauri.conf.json`에 5번째 창
9. **JSON 스키마** — `shared/schema.json` 업데이트
10. **테스트** — 코어 단위 테스트 + API 통합 테스트

---

## 11. 테스트 계획

### 코어 테스트 (`crates/core/tests/devnote.rs`)
- DevNote 직렬화/역직렬화 (빈 dev_notes 호환성)
- `generate_worklog`: todo 5개 → 올바른 그룹화 + body 생성
- 중복 방지: 같은 날짜 재생성 시 누락된 것만 추가
- 머지: 양쪽에서 노트 추가/수정 시 올바른 병합

### API 테스트
- POST /devnotes → 201 + 올바른 id
- POST /devnotes/worklog/generate → 완료된 todo 기반 로그 생성
- GET /devnotes?type=issue → 필터 동작
- GET /devnotes/by-version/v0.2-alpha → 버전 필터

---

## 12. 파일 변경 요약

| 파일 | 유형 | 변경 내용 |
|------|------|----------|
| `crates/core/src/model.rs` | 수정 | `DevNote`, `NoteType`, `GameSystem`, `new_note_id()` 추가, `TodoDoc.dev_notes` |
| `crates/core/src/store.rs` | 수정 | devnote CRUD + `generate_worklog` |
| `crates/core/src/devnote.rs` | ⭐ 신규 | 작업 로그 생성 로직 (그룹화, 마크다운) |
| `crates/core/src/merge.rs` | 수정 | `dev_notes` 배열 머지 |
| `crates/core/src/lib.rs` | 수정 | `pub mod devnote;` |
| `crates/api-server/src/routes.rs` | 수정 | 8개 엔드포인트 |
| `crates/mcp-server/src/` | 수정 | 4개 도구 |
| `apps/desktop/src/api.ts` | 수정 | devnote API 함수 |
| `apps/desktop/src/App.svelte` | 수정 | devnotes 창 분기 |
| `apps/desktop/src/DevNoteWidget.svelte` | ⭐ 신규 | 메인 위젯 |
| `apps/desktop/src/DevNoteCard.svelte` | ⭐ 신규 | 노트 카드 |
| `apps/desktop/src/DevNoteEditor.svelte` | ⭐ 신규 | 에디터 모달 |
| `apps/desktop/src-tauri/tauri.conf.json` | 수정 | 창 추가 |
| `shared/schema.json` | 수정 | DevNote 정의 |
| `crates/core/tests/devnote.rs` | ⭐ 신규 | 테스트 |
