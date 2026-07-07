# AGENTS.md — Access 에이전트 연동 가이드

> 이 파일은 모든 AI 에이전트(ZCode, OMP, Hermes, Claude Code)가
> Access TODO 시스템과 연동하는 방법을 설명합니다.
> 에이전트는 이 파일을 읽고 자동으로 연동 규칙을 이해해야 합니다.

## Access란?

Access는 **"에이전트 정리 가이드 시스템"**입니다.

에이전트들은 자유롭게 코드를 작성하고, 파일을 수정하고, 실험할 수 있습니다.
하지만 그 과정에서 **무엇을 했는지, 어디까지 했는지, 다음에 뭘 해야 하는지**를
스스로 점검하고 Access에 정리하는 것이 핵심입니다.

### 🔴 Access 핵심 원칙 (정리 규격)

**모든 에이전트는 세션 시작/종료 시 다음을 수행해야 합니다:**

#### 1. 세션 시작 시: 점검
```
① list_todos(agent="<본인>") 로 현재 할 일 확인
② 완료된 항목(complete_todo summary) 검토 — 이전 세션에서 무엇을 했는지 파악
③ 미완료 항목 우선순위 확인 — 오늘 뭘 해야 하는지 파악
④ 새로 발견한 작업이 있으면 add_todo로 등록
```

#### 2. 작업 중: 기록
```
① 각 작업을 완료할 때마다 complete_todo(id, summary="무엇을 했는지 구체적으로")
② 새 버그/기능 발견 시 add_todo로 즉시 등록
③ 다른 에이전트에게 넘겨야 할 작업은 tags=["agent:<상대에이전트>"]로 지정
```

#### 3. 세션 종료 시: 정리 요약
```
① access_review 호출 — 전체 진행상황 요약 생성
② 남은 할 일의 우선순위 재조정
③ 사용자에게 "어디까지 했고, 앞으로 뭘 해야 하는지" 보고
```

### 왜 이 규격이 필요한가?

에이전트가 아무거나 자유롭게 할 수 있지만, **정리하지 않으면**:
- 다른 에이전트가 무엇이 진행 중인지 모름 → 중복 작업, 충돌
- 사용자가 진행 상황을 파악할 수 없음 → 혼란
- 세션이 바뀌면 맥락이 사라짐 → 처음부터 다시 파악

Access는 이 **"정리 계층"**을 담당합니다.

## 연결 방식

| 에이전트 | 방식 | 설정 파일 |
|---------|------|---------|
| ZCode | MCP | `~/.zcode/cli/config.json` → `mcp.servers.access-todo` |
| Claude Code | MCP | 동일 config |
| OMP | MCP | `~/.omp/agent/mcp.json` → `mcpServers.access-todo` |
| Hermes | REST (curl) | skill: `skills/access-todo/SKILL.md` |

**API 서버**: `http://127.0.0.1:7878` (Access시작.bat 실행 시 자동 시작)

## 🔴 핵심 규칙: 작업 완료 시 요약과 함께 체크

### 1단계: 변경사항 분석
```
- git diff 또는 변경된 파일 목록 확인
- 핵심 변경사항을 1-2문장으로 요약
```

### 2단계: complete_todo 호출 (toggle_todo가 아님!)
```
complete_todo(id="<할일id>", summary="<작업 요약>", agent="<본인>")
```

**summary 작성 요령:**
- ✅ `job_select.lua 51줄 nil 체크 추가, main.lua 1867줄 H 초기화`
- ✅ `백엔드 시그널 룰 엔진 POST /rules 구현, 테스트 3개 추가`
- ❌ `작업 완료` (모호)
- ❌ `코드 수정함` (불명확)

### 3단계: 사용자 알림
```
"✓ '<할일제목>' 완료: <요약>"
```

## MCP 도구 (9개)

| 도구 | 설명 | 필수 인수 |
|------|------|---------|
| `list_todos` | 에이전트별 할 일 + 카테고리 | `agent` |
| `get_today_todos` | 오늘 할 일 | - |
| `add_todo` | 할 일 추가 | `title`, `agent`, `tags` |
| `complete_todo` | 체크 + 요약 ★ | `id`, `summary`, `agent` |
| `toggle_todo` | 단순 체크 (요약 없음) | `id`, `agent` |
| `update_todo` | 할 일 수정 | `id` |
| `delete_todo` | 삭제 | `id` |
| `search_todos` | 검색 | `q` |

## REST API (Hermes용)

모든 요청에 `X-Agent: <에이전트이름>` 헤더 필수.

```
GET    /todos?agent=<이름>          할 일 + 카테고리
POST   /todos                       추가 {title, priority, tags, category_id}
PATCH  /todos/:id                   수정
POST   /todos/:id/toggle            체크
POST   /todos/:id/complete          완료+요약 {summary}
DELETE /todos/:id                   삭제
GET    /todos/search?q=             검색

POST   /categories                  카테고리 생성 {agent, name}
PATCH  /categories/:id              이름 변경 {name}
DELETE /categories/:id              삭제
POST   /categories/reorder          순서 변경 {agent, ordered_ids}
```

## 카테고리

할 일의 `tags`에 `agent:<이름>`을 넣으면 해당 에이전트 포스트잇에 표시.
`category_id`로 카테고리 지정 (선택, 없으면 미분류).

## "내 할 일" 자동화

`user` 에이전트 창은 사용자 본인의 할 일입니다.
에이전트가 사용자를 위해 할 일을 추가할 때:
```
add_todo(title="...", agent="user", tags=["agent:user"], category_id="...")
```

## 🆕 프로젝트 온보딩 (자동 할 일 등록)

새 프로젝트를 만나면 다음 절차를 따르세요:

### 1단계: rules.json 확인
프로젝트 루트의 `.access/rules.json`을 읽습니다. 없으면 README의 TODO 섹션이나 `TODO:`/`FIXME:` 주석을 스캔합니다.

### 2단계: 카테고리 생성
```
create_category(agent="zcode", name="버그 수정")
create_category(agent="zcode", name="기능 개발")
```

### 3단계: 할 일 일괄 등록
```
add_todos_batch(agent="zcode", todos=[
  {title: "nil 버그 수정", priority: "high", category_id: "<id>"},
  {title: "차트 기능 추가", priority: "medium", category_id: "<id>"}
])
```

### 4단계: 알림
"N개 카테고리, M개 할 일을 등록했습니다"라고 사용자에게 알림.

### .access/rules.json 표준 포맷

프로젝트 루트에 `.access/rules.json`을 두면 에이전트가 자동으로 읽습니다:

```json
{
  "project": { "id": "k-stonks", "name": "K-STONKS-V2" },
  "categories": ["버그 수정", "기능 개발"],
  "starter_todos": [
    {
      "title": "nil 버그 수정",
      "priority": "high",
      "category": "버그 수정",
      "note": "job_select.lua 51줄"
    },
    {
      "title": "차트 기능 추가",
      "priority": "medium",
      "category": "기능 개발"
    }
  ]
}
```

에이전트가 이 파일을 읽고:
1. `categories` 배열로 카테고리 생성
2. `starter_todos`의 `category` 이름을 ID로 매핑
3. `add_todos_batch`로 한 번에 등록

## 자동 시작/종료

- 에이전트 게이트웨이 시작 → Access 자동 시작 (wrapper 스크립트)
- 모든 에이전트 종료 → Access 자동 종료 (Access-감시.ps1)
