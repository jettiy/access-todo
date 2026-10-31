# Agensi 등록 가이드 — Access 스킬 무료 리스팅 (2026-11-01)

목적: `skills/access-todo/SKILL.md`를 Agensi(agensi.io)에 **무료**로 등록해 홍보 퍼널 확보.
판매자 몫 70% 조건은 유료 판매 시에만 관련 있음 — 우리 전략은 무료 배포(제품기획 v1.1 5번).

## 등록 전 확인 (2026-11-01 시장조사 기준)

- Agensi: 스킬 마켓플레이스. 3개월 만에 스킬 2,000+, Trustpilot 4★(17건)
- 크리에이터: **신원 인증 필요**, 스킬은 게시 전 24–48시간 검수
- 스킬 시세 $3–10이지만 우리는 **Free**로 등록 (repo 유입이 목적)
- 유료 스킬 구매는 에이전트가 MCP로 승인하는 구조 — 무료 스킬도 같은 경로로 설치됨

## 준비물 체크리스트

- [x] 스킬 패키지: repo `skills/access-todo/` (SKILL.md, frontmatter 검증 완료)
- [x] 공개 repo: https://github.com/jettiy/access-todo (README 영문 병기 완료)
- [ ] Agensi 판매자 계정 (GitHub 계정으로 가입 + 신원 인증)
- [ ] 등록 후 라이브 링크를 이 문서에 기록

## 등록 절차 (실제 UI는 표시 시점 기준으로 조정)

1. https://www.agensi.io → 가입/로그인 (GitHub 계정 권장)
2. 크리에이터 온보딩: 신원 인증 (24–48시간 검수 전에 스킬 게시 검수가 별도로 있을 수 있음)
3. "Create skill / Add skill" 메뉴 → 아래 카피 붙여넣기
4. 스킬 콘텐츠: repo의 `skills/access-todo/SKILL.md` 원문 (마켓이 폴더 업로드/본문 붙여넣기 중 무엇을 요구하든 SKILL.md 본문이 그대로 들어가게)
5. 가격: **Free**
6. 제출 → 검수(24–48시간) → 라이브 링크 확보 → 아래에 기록

## 리스팅 카피 (붙여넣기용, English)

**Title:**
```
Access — Shared TODO Board for Your Agent Fleet
```

**Short description / tagline:**
```
One TODO board for Claude Code, Codex, ZCode, OpenClaw, Cline and custom agents — with pilot-ready work summaries.
```

**Full description:**
```
You run several coding agents. Each one works hard, and each one forgets.

Access gives every agent one shared TODO board, and gives you the big picture as
desktop post-it notes: one per agent, plus a dedicated post-it for decisions only
you can make.

This skill teaches your agents the Access operating protocol:
• Session start/end reviews — pick up exactly where the last session stopped
• Task registration and completion with concrete summaries (max 3 lines)
• Summaries styled to YOUR knowledge level: beginner (everyday analogies),
  intermediate (key terms + light analogies), advanced (agent's own judgment),
  or auto (agent infers from your conversations)
• Pilot delegation — human-only actions routed to your personal post-it
• REST fallback for agents without MCP support

Requirements: the Access MCP server (free & open source, MIT) running on localhost.
Get it: https://github.com/jettiy/access-todo
If Access is not installed, your agent will tell you exactly once and keep working.
```

**Category:** developer tools / agent skills / productivity (마켓 분류에 맞는 것 선택)
**Repo link:** https://github.com/jettiy/access-todo
**Price:** Free

## 병행 채널

| 채널 | 조건 | 우선순위 |
|------|------|---------|
| Agensi | 무료 등록, 검수 24–48h | 1순위 (이메일 발신처) |
| SkillHQ (skillhq.dev) | 구매자 수수료 없음, 판매자 15% (무료 등록도 가능한지 확인) | 2순위 |
| GitHub README/스타 | 근본 퍼널 | 상시 |

## 유입 추적

- GitHub Insights → Traffic (레퍼러별 조회수) 주 1회 기록
- 전환 지표: 스킬 설치 수(Agensi 제공 시), repo 스타, 포스트잇 활성 사용자
- 4주간 데이터 모은 뒤 P1(온보딩 마법사) 투자 강도 재검토 (제품기획 P3 트리거와 연결)

## 라이브 링크

- (등록 후 기입) Agensi: ______
- (등록 후 기입) SkillHQ: ______
