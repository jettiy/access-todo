# Desktop Post-it TODO List with Agent Integration

**Date:** 2026-07-07
**Status:** Approved
**Owner:** User

## 1. 개요

바탕화면에 떠 있는 포스트잇 형태의 TODO 리스트 위젯과, 이를 에이전트(ZCode/Claude Code via MCP, Nous Research Hermes Agent via REST)가 읽고 쓰고 체크할 수 있는 시스템. 데스크톱(Tauri)과 안드로이드(Kotlin/Compose) 양쪽에서 동일한 TODO 리스트를 확인·편집하며, GitHub Gist를 단일 진실 공급원(Single Source of Truth)으로 사용한다.

## 2. 목표 및 성공 기준

- 사용자가 바탕화면 포스트잇으로 오늘 할 일을 한눈에 확인할 수 있다.
- 에이전트가 TODO를 추가/수정/삭제/체크할 수 있고, 각 변경의 주체(`updated_by`)가 추적된다.
- 데스크톱 ↔ 안드로이드 간 체크 상태를 포함한 모든 변경이 동기화된다.
- 체크 이력이 `history`에 기록되어 "누가 언제 체크했는지" 알 수 있다.
- 각 TODO 항목에 메모(`note`)를 덧붙일 수 있다.

## 3. 비목표 (Out of Scope - 초기 버전)

- iOS 앱
- 실시간 푸시 (폴링 기반)
- 다중 사용자 / 공유 리스트
- 인증된 외부 공개 API (로컬/개인용)

## 4. 아키텍처

```
┌──────────────────────────────────────────────────────────────┐
│                GitHub Gist (Single Source of Truth)          │
│                 todos.json  (단일 비밀 Gist)                  │
└──────────┬───────────────────┬───────────────────┬──────────┘
           │                   │                   │
   Gist API            Gist API             Gist API
           │                   │                   │
┌──────────▼─────────┐ ┌──────▼─────────┐ ┌───────▼──────────┐
│  Desktop (Tauri)   │ │  Android App   │ │  Core Service    │
│  포스트잇 위젯     │ │  Kotlin/Compose│ │  (백그라운드)    │
└────────────────────┘ └─────────────────┘ │ ┌──────────────┐ │
                                           │ │ TODO Store   │ │
                                           │ │ + Gist Sync  │ │
                                           │ └──────┬───────┘ │
                                           └────────┼─────────┘
                                                    │
                              ┌─────────────────────┴──────────┐
                              ▼                                ▼
                      ┌───────────────┐              ┌────────────────┐
                      │  MCP Server   │              │   REST API     │
                      │  (stdio)      │              │ (HTTP :포트)   │
                      └───────┬───────┘              └────────┬───────┘
                              │                               │
                  ┌───────────┴────────────┐                  │
                  ▼                        ▼                  ▼
            ZCode/Claude              기타 MCP 클라      Hermes Agent
            Code 코딩 에이전트        (에이전트)         (HTTP 도구 호출)
```

**핵심 원칙:**
1. **Gist가 단일 진실 공급원** — 모든 클라이언트가 Gist에서 읽고 쓴다.
2. **Core Service가 관리자 역할** — 동기화 충돌 해결, 감사 로그, 에이전트 요청 검증을 중앙에서 처리. 항상 실행되는 백그라운드 서비스.
3. **에이전트는 사람과 동일한 권한** — 추가/수정/삭제/체크 모두 가능하되 `updated_by`로 추적.

## 5. 데이터 스키마

```json
{
  "version": "1.0",
  "updated_at": "2026-07-07T10:30:00Z",
  "updated_by": "user",
  "todos": [
    {
      "id": "uuid-v4",
      "title": "장보기",
      "note": "우유, 계란, 식빵",
      "done": false,
      "priority": "high|medium|low",
      "due_date": "2026-07-07",
      "tags": ["personal", "errand"],
      "created_at": "2026-07-07T09:00:00Z",
      "created_by": "user|zcode|hermes|claude",
      "completed_at": null,
      "completed_by": null,
      "history": [
        { "action": "created", "at": "...", "by": "user" },
        { "action": "checked", "at": "...", "by": "hermes" }
      ]
    }
  ]
}
```

**필드 설명:**
- `id`: UUID v4. 항목 식별 및 동기화 머지의 기준.
- `title`: 할 일 제목 (필수).
- `note`: 항목별 메모 (선택). TODO 항목별 메모 필드 요구사항 충족.
- `done`: 체크 상태.
- `priority`: high / medium / low.
- `due_date`: YYYY-MM-DD. "오늘 할 일" 필터에 사용.
- `tags`: 자유 태그 배열.
- `created_at` / `created_by`: 생성 메타데이터.
- `completed_at` / `completed_by`: 체크 시 자동 기록. "누가 체크했는지" 추적.
- `history`: 모든 변경 이력 배열. 체크 이력 포함.

JSON Schema는 `shared/schema.json`에 별도 정의하여 모든 클라이언트 계약으로 사용.

## 6. 에이전트 인터페이스

MCP 도구명과 REST 경로를 1:1로 대응시킨다. 모든 호출에는 에이전트 식별자가 필수.

| 동작 | MCP 도구 | REST 엔드포인트 | 본문/파라미터 |
|------|---------|---------------|--------------|
| 할 일 전체 조회 | `list_todos` | `GET /todos` | `?done=&due_date=` |
| 오늘 할 일 | `get_today_todos` | `GET /todos/today` | — |
| 단일 조회 | `get_todo` | `GET /todos/:id` | — |
| 추가 | `add_todo` | `POST /todos` | title, note?, priority?, due_date?, tags? |
| 수정 | `update_todo` | `PATCH /todos/:id` | title?, note?, priority?, due_date?, tags? |
| 체크 토글 | `toggle_todo` | `POST /todos/:id/toggle` | — |
| 삭제 | `delete_todo` | `DELETE /todos/:id` | — |
| 검색 | `search_todos` | `GET /todos/search` | `?q=` |

**에이전트 식별:**
- MCP: 클라이언트 컨텍스트에서 전달 (예: `agent` 파라미터)
- REST: `X-Agent` 헤더 (예: `X-Agent: hermes`)

식별자는 `created_by` / `updated_by` / `completed_by` / `history[].by` 에 자동 기록된다.

## 7. 동기화 전략 (Gist)

- **저장소:** 비밀 Gist 1개, 파일명 `todos.json`.
- **동시성 제어:** Gist ETag 헤더로 낙관적 동시성. 로컬 캐시 ETag와 비교.
- **폴링 간격:**
  - Core Service: 30초
  - Desktop 위젯: Core Service 경유 (위젯이 직접 Gist 호출 안 함)
  - Android: 30초 (포그라운드), 30분 (WorkManager 백그라운드)
  - 에이전트 요청 시: 즉시 fetch 후 처리
- **충돌 해결:** 항목 ID 기준 3-way 머지.
  - 같은 ID를 다른 클라이언트가 동시 수정 → `updated_at` 타임스탬프 우선.
  - 서로 다른 필드 변경은 병합 보존.
  - 삭제 vs 수정 충돌 → 수정 우선 (삭제는 명시적 toggle 필드로 처리).
- **감사:** 모든 변경이 `history[]`에 추가됨.

## 8. 데스크톱 포스트잇 위젯 (Tauri)

**형태:** 단일 큰 포스트잇 (바탕화면 한쪽에 고정, 내부 스크롤).

**윈도우 속성:**
- always-on-top
- borderless (프레임 없음)
- semi-transparent
- border-radius, 노란 포스트잇 배경, 그림자
- 드래그로 위치 이동, 더블클릭으로 접기/펼치기

**레이아웃:**
```
┌────────────────────────────────┐
│ 📒 오늘의 할 일        − □ ×   │
├────────────────────────────────┤
│ ☐ 🔴 프로젝트 기획서 작성      │
│      📝 클라이언트 피드백 반영  │
│ ☐ 🟡 장보기                    │
│      📝 우유, 계란             │
│ ☑ 🟢 운동 30분  (🤖 hermes)   │  ← 에이전트 체크 뱃지
│ ──────────────────────────────│
│ [+ 새 할 일 추가]              │
│ 🔄 동기화됨 12:30 · 5/8 완료   │
└────────────────────────────────┘
```

**프론트엔드:** Svelte (가벼움 우선) 또는 Vanilla JS.
**데이터 흐름:** 위젯 → Core Service HTTP API → Gist. (위젯이 Gist 직접 호출 안 함)

**색상 테마:** 클래식 옐로우(기본) / 핑크 / 블루 (설정 가능).

## 9. 안드로이드 앱 (Kotlin + Compose)

**아키텍처:** 단일 Activity, Jetpack Compose, Material 3.

**화면:**
- 메인: 오늘/내일/완료됨 섹션으로 그룹핑된 리스트
- 편집: TODO 추가/수정 폼
- 설정: Gist 토큰, Gist ID, 동기화 간격

**Gist 통신:** Retrofit + OkHttp로 Gist API 직제 호출 (Core Service 경유 안 함 — 모바일은 밖에서도 쓰므로).

**충돌 해결 (안드로이드 내장):** Core Service를 거치지 않으므로 안드로이드 앱 내에 동일한 ID 기준 3-way 머지 로직을 Kotlin으로 포함. `shared/schema.json`과 머지 알고리즘은 Rust/Kotlin 양쪽에서 동일 동작 보장 (스키마로 계약 고정, 알고리즘은 명세화하여 언어별 재구현).

**백그라운드 동기화:**
- WorkManager 주기적 동기화 (30분)
- 앱 포그라운드 시 30초 폴링
- 수동 새로고침 풀투리프레시

**알림:**
- 오늘 마감 할 일 (오전 9시 1회)
- 에이전트가 새 할 일 추가 시 (Gist 변경 감지 시)

**홈화면 위젯:** Glance 기반. 바탕화면에서 오늘 할 일 미리보기 + 체크.

## 10. 프로젝트 구조 (모노레포)

```
desktop-todo-agents/
├── crates/
│   ├── core/              # TODO Store + Gist Sync + 스키마 (Rust)
│   ├── mcp-server/        # MCP stdio 서버
│   └── api-server/        # REST HTTP 서버 (axum)
├── apps/
│   └── desktop/           # Tauri 포스트잇 위젯
├── android/               # Kotlin/Compose 안드로이드 앱
├── docs/
│   └── superpowers/specs/
│       └── 2026-07-07-desktop-todo-agents-design.md
├── shared/
│   └── schema.json        # TODO JSON 스키마 (모든 클라이언트 계약)
└── Cargo.toml             # 워크스페이스 루트
```

## 11. 보안

- GitHub Personal Access Token은 로컬 파일(`~/.config/desktop-todo-agents/config.toml`)에 저장, 권한 600.
- 안드로이드: Android Keystore + EncryptedSharedPreferences.
- REST API는 localhost 전용 바인딩 (외부 노출 안 함). 선택적으로 단일 bearer token 인증.
- Gist는 비밀(secret) Gist 사용.

## 12. 에러 처리

- Gist API 실패: 로컬 큐에 적재, 재시도 (지수 백오프, 최대 5회).
- 스키마 검증 실패: 해당 변경 거부 + 에이전트에 명확한 에러 메시지.
- 동기화 충돌: 자동 머지 시도 → 실패 시 양쪽 변경 보존 + `conflict: true` 플래그.

## 13. 테스팅 전략

- `crates/core`: 단위 테스트 (스토어 조작, 스키마 검증, 머지 로직).
- `crates/mcp-server` / `api-server`: 인터페이스 계약 테스트 (동일 입력 → 동일 결과).
- 동기화: 가짜 Gist API(Mock)로 충돌 시나리오 테스트.
- 안드로이드: Compose UI 테스트 + 에뮬레이터 통합 테스트.
- JSON 스키마: `shared/schema.json`으로 모든 클라이언트 직렬화/역직렬화 검증.

## 14. 구현 순서 (초기 버전)

1. `shared/schema.json` + `crates/core` (스토어, 스키마, Gist 클라이언트)
2. `crates/api-server` (REST) + `crates/mcp-server` (MCP)
3. `apps/desktop` (Tauri 포스트잇)
4. `android/` (Kotlin/Compose 앱 + 홈 위젯)
5. 통합 테스트 + 문서
