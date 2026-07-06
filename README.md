# PostIt Todo — Desktop Post-it + Agent + Android

바탕화면에 떠 있는 포스트잇 형태의 TODO 리스트와, AI 에이전트(ZCode/Claude Code via MCP, Nous Research Hermes Agent via REST)가 함께 편집하고, 안드로이드와 동기화되는 시스템.

GitHub Gist가 단일 진실 공급원(Single Source of Truth)이며, 모든 변경(체크 포함)은 누가 했는지(`created_by` / `updated_by` / `completed_by`) 추적됩니다.

## 아키텍처 한눈에 보기

```
        ┌─────────────────────────────────┐
        │   GitHub Gist (todos.json)      │  ← 진실 공급원
        └──────┬──────────┬──────────┬────┘
               │          │          │
        ┌──────▼──┐  ┌────▼────┐  ┌──▼──────────┐
        │ Desktop │  │ Android │  │ Core Service │
        │  (Tauri)│  │ (Kotlin)│  │  (Rust 바이너리)│
        └─────────┘  └─────────┘  └──────┬───────┘
                                            │
                              ┌─────────────┴────────────┐
                              ▼                          ▼
                       ┌────────────┐           ┌────────────────┐
                       │ MCP Server │           │   REST API     │
                       │  (stdio)   │           │ (127.0.0.1)    │
                       └─────┬──────┘           └────────┬───────┘
                             │                           │
                             ▼                           ▼
                       ZCode / Claude               Hermes Agent
                       Code 코딩 에이전트            (HTTP 도구 호출)
```

## 구성 요소

| 컴포넌트 | 경로 | 기술 | 상태 |
|---------|------|------|------|
| 코어 (TODO Store + Gist 동기화) | `crates/core` | Rust | ✅ 테스트 19개 통과 |
| REST API 서버 | `crates/api-server` | Rust + axum | ✅ e2e 검증 완료 |
| MCP 서버 | `crates/mcp-server` | Rust + JSON-RPC | ✅ e2e 검증 완료 |
| 데스크톱 포스트잇 | `apps/desktop` | Tauri 2 + Svelte | ✅ 빌드 + 실행 검증 |
| 안드로이드 앱 | `android` | Kotlin + Compose | ✅ 코드 작성 완료 |
| JSON 스키마 계약 | `shared/schema.json` | JSON Schema | ✅ |

## 사전 준비

### 공통
- **GitHub Personal Access Token** — `gist` 권한만 필요 (Classic 또는 Fine-grained)
- 토큰 발급: https://github.com/settings/tokens

### 데스크톱 (Windows)
- Rust 1.93+ (`rustup`)
- MSVC Build Tools (C++ 워크로드) 또는 Visual Studio 2022
- Node.js 20+ (Tauri 프론트엔드 빌드용)
- WebView2 Runtime (Windows 10/11은 기본 설치됨)

### 안드로이드
- JDK 17
- Android SDK (compileSdk 34, 명령줄 도구)
- Android Studio (권장) 또는 Gradle 명령줄

## 빠른 시작

### 1. Gist 생성 (최초 1회)

```powershell
$env:GITHUB_TOKEN = "ghp_여러분의토큰"
cargo run -p todo_core --bin bootstrap-gist
# 출력: Created secret Gist. Set TODO_GIST_ID=<id>
```

출력된 Gist ID를 메모해 두세요.

### 2. 데스크톱 포스트잇 실행

두 개의 터미널이 필요합니다:

**터미널 A — REST API 서버 (백그라운드):**
```powershell
# vcvars 환경 로드 후 (MSVC 필수)
cargo run -p api-server
# api-server listening on http://127.0.0.1:7878
```

**터미널 B — 포스트잇 위젯:**
```powershell
cd apps/desktop
npm install        # 최초 1회
npm run tauri dev  # 개발 모드 (hot-reload)
# 또는: npm run tauri build  # 프로덕션 빌드
```

노란 포스트잇이 바탕화면에 always-on-top으로 나타납니다.

### 3. 안드로이드 앱 빌드

```bash
cd android
./gradlew assembleDebug
# APK: app/build/outputs/apk/debug/app-debug.apk
```

에뮬레이터/실기기에 설치:
```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

앱 첫 실행 시 GitHub Token과 Gist ID를 입력하세요.

### 4. 전체 테스트

```powershell
cargo test --workspace
# todo_core: 19개, api-server: 5개, mcp-server: 4개 테스트
```

## 에이전트 연동

### ZCode / Claude Code (MCP)

ZCode 설정 파일(`~/.zcode/config.toml`)에 MCP 서버 등록:

```toml
[[mcp_servers]]
name = "postit-todo"
command = "cargo"
args = ["run", "--release", "-p", "mcp-server"]
```

이제 ZCode/Claude Code가 다음 8개 도구를 호출할 수 있습니다:
`list_todos`, `get_today_todos`, `get_todo`, `add_todo`, `update_todo`, `toggle_todo`, `delete_todo`, `search_todos`

각 도구 호출 시 `agent` 인수로 에이전트 이름을 전달하면, 그 이름이 `created_by`/`updated_by`/`completed_by`에 기록됩니다.

### Hermes Agent (REST)

Hermes의 OpenAI 호환 도구 정의에 REST 엔드포인트를 등록합니다. 모든 요청에 `X-Agent: hermes` 헤더를 포함하세요:

```http
GET  /todos                    HTTP/1.1   Host: 127.0.0.1:7878   X-Agent: hermes
GET  /todos/today
POST /todos                    {"title":"...", "note":"...", "priority":"high", "due_date":"2026-07-07"}
PATCH /todos/{id}              {"title":"변경된 제목"}
POST /todos/{id}/toggle
DELETE /todos/{id}
GET  /todos/search?q=키워드
```

Hermes skills/tools 설정 예시:
```yaml
- name: postit_add_todo
  description: "Add a todo to the shared post-it list"
  endpoint: "http://127.0.0.1:7878/todos"
  method: POST
  headers:
    X-Agent: hermes
    Content-Type: application/json
```

## 데이터 모델

모든 클라이언트가 공유하는 JSON 문서 형식 (`shared/schema.json`):

```json
{
  "version": "1.0",
  "todos": [{
    "id": "uuid-v4",
    "title": "장보기",
    "note": "우유, 계란",
    "done": false,
    "priority": "high",
    "due_date": "2026-07-07",
    "created_by": "user",
    "completed_by": "hermes",
    "history": [
      {"action": "created", "by": "user", "at": "..."},
      {"action": "checked", "by": "hermes", "at": "..."}
    ]
  }]
}
```

핵심: `history` 배열에 모든 변경이 기록되어 **"누가 언제 체크했는지"** 추적할 수 있습니다.

## 동기화

- **Gist ETag** 기반 낙관적 동시성 제어
- **충돌 해결**: 항목 ID 기준 3-way 머지, `updated_at` 타임스탬프 최신 우선
- **폴링 간격**: 데스크톱 위젯 30초, 안드로이드 30분 (WorkManager), 에이전트 요청 시 즉시 fetch

## 보안

- GitHub 토큰: 데스크톱은 환경 변수/로컬 설정, 안드로이드는 EncryptedSharedPreferences (Android Keystore)
- REST API는 127.0.0.1 전용 바인딩 (외부 노출 없음)
- Gist는 비밀(secret) Gist 사용

## 프로젝트 구조

```
desktop-todo-agents/
├── crates/
│   ├── core/              # TODO Store + Gist 동기화 + 머지 (Rust)
│   ├── api-server/        # REST HTTP 서버 (axum)
│   └── mcp-server/        # MCP stdio 서버 (JSON-RPC)
├── apps/
│   └── desktop/           # Tauri 포스트잇 위젯 (Svelte)
├── android/               # Kotlin/Compose 안드로이드 앱
├── shared/schema.json     # TODO JSON 스키마 (공유 계약)
└── docs/
    └── superpowers/
        ├── specs/         # 설계 문서
        └── plans/         # 구현 계획
```

## 라이선스

MIT
