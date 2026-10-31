# 개선제안: Tauri 프록시 전환 — "Failed to fetch" 근본 해결 (2026-10-18)

작성: 관제 Hermes · 배경 실측 동봉 · 수행 후보: heco-dev · 상태: 제안(준석님 승인 대기)

## 배경 (2026-10-18 실측)

- 각 포스트잇 창이 `App.svelte`에서 자기 전용 EventSource(`/events`)를 열어 서버 변경을 수신한다.
- WebView2(Chromium)는 **호스트당 동시 연결 6개 제한**(HTTP/1.1). 7창 × SSE(+재접속 재시도) = 순간 12연결 실측 → 소켓 풀 고갈.
- 고갈 상태에서 mutation 발생 → 각 창의 재조회 fetch가 소켓을 못 얻어 **"Failed to fetch"** 고착.
- 임시 완료: SSE 연결 소유자를 hermes 창 1개로 제한 + 나머지 5초 폴링(연결 12→1 실측 확인). 단, 창 추가 시 재발 소지.

## 목표

프론트의 7878 직접 HTTP 호출을 제거해 **창 수와 무관하게 소켓 고갈이 원천 불가능**한 구조로 전환.

## 설계

1. **데이터 호출 프록시**: 프론트 `api.ts`의 fetch를 Tauri `invoke('api_call', { method, path, body })`로 대체.
   Rust 백엔드가 reqwest로 `http://127.0.0.1:7878` 호출 후 JSON 반환 (브라우저 소켓 풀 미사용).
2. **단일 SSE 소유**: Rust가 `/events`를 1개만 구독 → 수신 시 `app.emit("todos-changed", payload)`.
   프론트는 `listen("todos-changed")`로 수신 (Tauri 이벤트는 TCP 소켓 아님).
3. **폴링 제거**: 2가 완료되면 5초 폴링 타이머 제거(이벤트가 진실의 원천).

## Definition of Done

1. 창 7개 기동 상태에서 `netstat`로 7878 브라우저 소켓(WebView2 소속) **0개** 확인.
2. mutation → 전 창 UI 반영 지연 **1초 미만** (수동 체크 + 로그).
3. 기존 기능 회귀 없음: todo CRUD·카테고리·devnotes·백업 전 동작 확인.
4. 창 10개 스트레스(중복 기동 시뮬)에서도 Failed to fetch 0건.
5. REST API·Access-MCP·Gist 동기화는 **무변경** — 외부 에이전트(omp/codex/dsh/hermes) 영향 없음.

## 마이그레이션 순서

1. Rust `api_call` 커맨드 + SSE 단일 구독·emit 먼저 추가 (프론트 미변경, 병행 기간).
2. api.ts를 feature flag(`VITE_USE_PROXY`)로 invoke 경로 전환 → 기본 off로 배포.
3. 7창 실측(연결 수·반영 지연) → 기본 on 전환 → 폴링 제거.
4. 각 단계 커밋 분리. 배포는 반드시 `cargo build --release --features custom-protocol`.

## 주의

- `crates/mcp-server/src/main.rs` 등 타 에이전트 진행 중 변경과 충돌 없음(데스크톱 앱 영역만).
- CSP의 connect-src는 invoke 경로에서 불필요 — 정리 시 기존 http://127.0.0.1:7878 허용은 유지(폴백용).
