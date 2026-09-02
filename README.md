# Access — 에이전트 정리 가이드 시스템

에이전트들이 자유롭게 작업하지만, **무엇을 했는지, 어디까지 했는지, 다음에 뭘 해야 하는지**를 점검하고 정리하는 시스템입니다. 바탕화면 포스트잇 + GitHub Gist 동기화 + 안드로이드를 통해 모든 에이전트와 사용자가 같은 진행상황을 공유합니다.

> **Access의 역할**: 에이전트가 아무거나 할 수 있지만, 세션 시작/종료 시 점검하고 정리하여 **다음에 무엇을 해야 하는지** 모두에게 알려주는 "정리 계층"

```
ZCode (MCP) ─────┐
OMP   (MCP) ─────┤
Hermes (REST) ───┤→ API 서버 (:7878) → GitHub Gist → 포스트잇 + 안드로이드
Claude (MCP) ────┤
사용자 (위젯) ───┘
```

## ✨ 기능

- **4개 에이전트별 독립 포스트잇** — hermes/omp/zcode/user 각각 별도 창, 색상, 위치
- **카테고리** — 각 포스트잇 안에 프로젝트별 카테고리 (생성/rename/삭제/순서변경/접기)
- **에이전트 작업 추적** — 모든 변경이 `created_by`/`completed_by`/`history`로 기록
- **작업 요약 자동 기록** — 에이전트가 작업 완료 시 `complete_todo(summary="...")`로 무엇을 했는지 기록
- **GitHub Gist 동기화** — 단일 진실 공급원, `If-Match` ETag 기반 충돌 감지(412 시 pull→merge→재시도), 3-way 머지, **소프트 삭제(tombstone)** 로 삭제 동기화 보장
- **debounced push** — 여러 mutation이 1.5초 윈도우에서 한 번의 Gist PATCH로 통합 (rate limit 보호)
- **로컬 캐시 fallback** — `GITHUB_TOKEN` 없이 실행 시 플랫폼 데이터 디렉토리에 JSON 캐시 → 재시작 시 복원
- **검증 + RFC 7807 problem-details** — title/note/due_date 검증, 실패 시 `application/problem+json` 본문
- **안드로이드 앱** — Kotlin/Compose + Glance 홈 위젯
- **항상 위 토글** — 각 창마다 📌 버튼으로 always-on-top 켜기/끄기

## 🚀 빠른 시작

### 사전 준비
- Rust 1.93+ + MSVC Build Tools
- Node.js 20+ (Tauri 프론트엔드)
- GitHub Personal Access Token (`gist` 스코프) — [발급](https://github.com/settings/tokens/new?scopes=gist)

### 빌드 및 실행

```bash
# 1. Gist 생성 (최초 1회)
export GITHUB_TOKEN=ghp_여러분의토큰
cargo run -p todo_core --bin bootstrap-gist --release
# → "Set TODO_GIST_ID=xxxx" 출력

# 2. API 서버 실행
export TODO_GIST_ID=위에서_출력된_ID
cargo run -p api-server --release

# 3. 포스트잇 위젯 실행 (다른 터미널)
cd apps/desktop
npm install && npm run build
cd src-tauri && cargo run --release --features custom-protocol
```

```bash
cargo test --workspace  # core 32 + API 13 + MCP 4 = 총 49개 테스트
```

## 📁 프로젝트 구조

```
access/
├── crates/
│   ├── core/           # TODO Store + Category + Gist 동기화 + 3-way 머지
│   ├── api-server/     # REST API (axum, localhost:7878)
│   └── mcp-server/     # MCP 서버 (12개 도구, REST API 전달)
├── apps/
│   └── desktop/        # Tauri 포스트잇 (4개 창, Svelte)
├── android/            # Kotlin/Compose 안드로이드 앱
└── shared/schema.json  # JSON 스키마 계약
```

## 🔌 에이전트 연동

### ZCode / Claude Code / OMP (MCP)

`~/.zcode/cli/config.json` (또는 각 에이전트의 MCP 설정)에 추가:

```json
{
  "mcp": {
    "servers": {
      "access-todo": {
        "type": "stdio",
        "command": "/path/to/Access-MCP.exe"
      }
    }
  }
}
```

**사용 가능한 도구 (12개):**

| 도구 | 설명 |
|------|------|
| `list_todos` | 에이전트별 할 일 + 카테고리 조회 |
| `get_today_todos` | 오늘 할 일 |
| `get_todo` | 단일 할 일 상세 조회 (id) |
| `add_todo` | 할 일 추가 (`category_id` 선택) |
| `add_todos_batch` | 여러 할 일 일괄 등록 (온보딩) |
| `update_todo` | 할 일 수정 |
| `toggle_todo` | 체크 토글 |
| `complete_todo` | **체크 + 작업 요약 기록** ★ |
| `delete_todo` | 할 일 삭제 |
| `search_todos` | 키워드 검색 |
| `create_category` | 카테고리 생성 |
| `access_review` | **진행상황 종합 요약** ★ |

### Hermes Agent (REST)

```bash
# 할 일 추가
curl -X POST -H "X-Agent: hermes" -H "Content-Type: application/json" \
  -d '{"title":"코드 리뷰","tags":["agent:hermes"]}' \
  http://127.0.0.1:7878/todos

# 완료 + 요약
curl -X POST -H "X-Agent: hermes" -H "Content-Type: application/json" \
  -d '{"summary":"인증 모듈 수정, 3개 테스트 추가"}' \
  http://127.0.0.1:7878/todos/{id}/complete
```

### 카테고리

```bash
# 생성
curl -X POST -H "X-Agent: zcode" -H "Content-Type: application/json" \
  -d '{"agent":"zcode","name":"버그 수정"}' http://127.0.0.1:7878/categories

# 이름 변경
curl -X PATCH -H "X-Agent: zcode" -H "Content-Type: application/json" \
  -d '{"name":"긴급 버그"}' http://127.0.0.1:7878/categories/{id}

# 삭제 (할 일은 미분류로 이동)
curl -X DELETE -H "X-Agent: zcode" http://127.0.0.1:7878/categories/{id}
```

## 📱 안드로이드 앱

```bash
cd android
./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

첫 실행 시 GitHub Token과 Gist ID를 입력하면 같은 TODO 리스트를 공유합니다.

## 🏗️ 아키텍처

**데이터 모델 (Gist의 `todos.json`):**
```json
{
  "version": "1.0",
  "categories": [
    {"id": "cat-uuid", "agent": "zcode", "name": "버그 수정", "order": 0}
  ],
  "todos": [
    {
      "id": "uuid", "title": "...", "done": false,
      "category_id": "cat-uuid", "tags": ["agent:zcode"],
      "created_by": "zcode", "completed_by": null,
      "history": [{"action": "created", "by": "zcode"}]
    }
  ]
}
```

- **삭제 동기화:** tombstone(`deleted_at`) 기반. 삭제 시 vec에서 제거하지 않고 마킹 → 머지 시 가장 최신 타임스탬프가 이겨 다른 replica에 삭제가 전파됨. `prune_tombstones(Duration)` 로 일정 기간 후 실제 제거.
- **동시성/읽기 확장성:** `Arc<RwLock<Store>>` 로 다중 읽기 동시 실행. mutation은 `mark_push_pending()` 으로 동기 I/O 없이 즉시 반환, 백그라운드 flusher가 debounced push.
- **동기화:** `If-Match` ETag 기반 optimistic concurrency. push 시 412 Precondition Failed가 뜨면 자동으로 pull→3-way 머지 후 재시도 (최대 3회). 동시 수정 시 마지막 컨픽트는 `synced: false`로 클라이언트에 노출.
- **요약 분리:** `complete_todo(summary=...)` 는 `summary` 필드에만 기록. 사용자 `note`는 보존.

## 📄 라이선스

MIT
