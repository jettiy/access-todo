# AGENTS.md — Access 에이전트 연동 가이드

> 이 파일은 모든 AI 에이전트(ZCode, OMP, Hermes, Claude Code)가
> Access TODO 시스템과 연동하는 방법을 설명합니다.
> 에이전트는 이 파일을 읽고 자동으로 연동 규칙을 이해해야 합니다.

## Access란?

Access는 바탕화면 포스트잇 + 에이전트 연동 + 안드로이드 동기화 TODO 시스템입니다.
모든 에이전트가 같은 할 일 리스트를 공유하고, 작업 내역을 추적합니다.

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

## 자동 시작/종료

- 에이전트 게이트웨이 시작 → Access 자동 시작 (wrapper 스크립트)
- 모든 에이전트 종료 → Access 자동 종료 (Access-감시.ps1)
