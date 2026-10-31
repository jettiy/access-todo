# Access — Shared TODO Board for Your Agent Fleet

**에이전트 함대를 위한 공유 TODO 보드 — 모든 에이전트와 파일럿(사용자)이 같은 진행상황을 공유합니다.**

![License](https://img.shields.io/badge/license-MIT-blue) ![Platform](https://img.shields.io/badge/platform-Windows-success) ![Stack](https://img.shields.io/badge/Rust-Tauri-orange)

You run several coding agents — Claude Code, Codex, ZCode, OpenClaw, Cline, custom agents.
Each one works hard, and each one forgets. **Access** is the coordination layer: every agent
reads and writes one shared TODO store, and you watch it as always-on-top desktop post-it
notes — one per agent, plus a dedicated **pilot post-it** for decisions only you can make.
Agents record what they did as short, plain-language summaries written for *your* knowledge
level (beginner → advanced), so the board stays readable at a glance.

```
ZCode (MCP) ─────┐
Codex (MCP) ─────┤
Claude (MCP) ────┼──→ API server (:7878) ──→ GitHub Gist ──→ post-its + Android
Hermes (REST) ───┤        (3-way merge, SSE)
Pilot (widget) ──┘
```

## ✨ Features

- **Per-agent post-it notes** — separate window, color and position per agent, always-on-top 📌
- **Pilot post-it & delegation** — agents route human-only decisions via `tags:["agent:user"]`
- **Work summaries at your level** — agents summarize completed work in ≤3 lines, styled
  for your knowledge level (beginner / intermediate / advanced / auto)
- **Categories** — per-project grouping, create/rename/reorder
- **GitHub Gist sync** — single source of truth, ETag-based optimistic concurrency, 3-way merge
- **Local-only mode** — no token, no cloud: everything stays in one JSON file *(wizard: in progress)*
- **Android app** — same TODO list on your phone (Kotlin/Compose + Glance widget)
- **16 MCP tools** — full CRUD, review, dev-notes and worklog generation

## 🚀 Roadmap

Access is **100% free and open source (MIT) — no paid tier is planned for this project.**
Commercial follow-ups, if any, will be separate products decided by usage signals.

| Phase | Deliverable | Status |
|-------|-------------|--------|
| P0 | Agent skill (`skills/access-todo`) + free marketplace listing | ✅ this release |
| P1 | First-run settings window: auto-detect agents → toggle to register MCP; connection self-healing after agent updates | 🚧 |
| P2 | Pilot post-it + `request_pilot` tool + Android push | ⏳ |
| P3 | Growth & maintenance — stays free forever | ⏳ |

Support wave (by ecosystem usage ranking): Hermes → Claude Code → Kilo Code → pi → Codex → Cline → Freebuff → omp → OpenClaw → DeepSeek Harness — details in [제품기획_v2_온보딩과상업화.md](docs/제품기획_v2_온보딩과상업화.md).

## 📦 Install

### Option A — Agent skill (any skills-compatible agent)

Give every agent the Access operating protocol (session protocol, summary levels, delegation):

```bash
git clone https://github.com/jettiy/access-todo.git
# Claude Code
cp -r access-todo/skills/access-todo ~/.claude/skills/
# ZCode
cp -r access-todo/skills/access-todo ~/.zcode/skills/
# OpenClaw / other SKILL.md-compatible agents: copy into that agent's skills directory
```

The skill alone does nothing without the MCP server — keep going.

### Option B — Build from source (installer ships with P1)

Prerequisites: Rust 1.93+ (MSVC), Node.js 20+, GitHub PAT with `gist` scope.

```bash
# 1) bootstrap your sync Gist (once)
export GITHUB_TOKEN=ghp_yourtoken
cargo run -p todo_core --bin bootstrap-gist --release   # → prints TODO_GIST_ID=...

# 2) API server (localhost:7878)
export TODO_GIST_ID=that_id
cargo run -p api-server --release

# 3) post-it widget (separate terminal)
cd apps/desktop && npm install && npm run build
cd src-tauri && cargo run --release --features custom-protocol

# 4) tests
cargo test --workspace
```

### Connect your agents (MCP)

Add to the agent's MCP config — command is the built `Access-MCP.exe` absolute path:

```jsonc
// Claude Code (~/.claude.json → mcpServers), ZCode (~/.zcode/cli/config.json → mcp.servers)
"access-todo": { "command": "C:\\path\\to\\Access-MCP.exe" }
```

```toml
# Codex (~/.codex/config.toml)
[mcp_servers.access-todo]
command = "C:/path/to/Access-MCP.exe"
```

Non-MCP agents (e.g. Hermes) use plain REST with an `X-Agent: <name>` header —
see [AGENTS.md](AGENTS.md) for the full protocol, including security rules.

## 🇰🇷 한국어 요약

여러 에이전트(Claude Code·Codex·ZCode·OpenClaw·Cline·자체 에이전트)가 **같은 TODO 저장소**를
쓰고, 진행상황을 바탕화면 포스트잇(에이전트별 + 파일럿 전용)으로 공유하는 시스템입니다.
에이전트가 작업을 마치면 파일럿의 지식 수준(초보/중급/고급/자동)에 맞춘 **3줄 이내 요약**을
기록합니다. Gist 동기화로 안드로이드와도 공유되며, 로컬 전용 모드도 지원 예정입니다.
에이전트 프로토콜 전문은 [AGENTS.md](AGENTS.md), 제품 로드맵은
[docs/제품기획_v2_온보딩과상업화.md](docs/제품기획_v2_온보딩과상업화.md)를 보세요.

## 📁 Structure

```
crates/core        # TODO store + categories + Gist sync + 3-way merge
crates/api-server  # REST API (axum, localhost:7878) + SSE
crates/mcp-server  # MCP server (16 tools)
apps/desktop       # Tauri post-it widget (Svelte)
android/           # Kotlin/Compose app + home widget
skills/access-todo # Agent skill: the Access operating protocol
shared/schema.json # JSON schema contract
```

## 📄 License

MIT
