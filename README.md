# Access — 에이전트 연동 포스트잇 TODO 시스템

바탕화면에 떠 있는 포스트잇 형태의 TODO 리스트와 AI 에이전트(ZCode, OMP, Hermes, Claude Code)가 함께 편집하고, GitHub Gist로 동기화되며, 안드로이드에서도 확인할 수 있는 시스템입니다.

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
- **GitHub Gist 동기화** — 단일 진실 공급원, ETag 기반 충돌 제어, 3-way 머지
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

### 테스트
```bash
cargo test --workspace  # 코어 19개 + API + MCP 테스트
```

## 📁 프로젝트 구조

```
access/
├── crates/
│   ├── core/           # TODO Store + Category + Gist 동기화 + 3-way 머지
│   ├── api-server/     # REST API (axum, localhost:7878)
│   └── mcp-server/     # MCP 서버 (9개 도구, REST API 전달)
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

**사용 가능한 도구 (9개):**

| 도구 | 설명 |
|------|------|
| `list_todos` | 에이전트별 할 일 + 카테고리 조회 |
| `get_today_todos` | 오늘 할 일 |
| `add_todo` | 할 일 추가 (`category_id` 선택) |
| `update_todo` | 할 일 수정 |
| `toggle_todo` | 체크 토글 |
| `complete_todo` | **체크 + 작업 요약 기록** ★ |
| `delete_todo` | 할 일 삭제 |
| `search_todos` | 키워드 검색 |

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

**동기화:** Gist ETag 기반 낙관적 동시성, 항목 ID 기준 3-way 머지

## 📄 라이선스

MIT
