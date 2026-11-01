---
name: access-todo
description: Coordinate work across all coding agents (Claude Code, Codex, ZCode, OpenClaw, Cline, custom agents) and the pilot on one shared TODO board with desktop post-it notes. Use it to check what previous sessions left unfinished, register new tasks, complete tasks with a pilot-friendly summary, and surface decisions only the human can make. Works through the Access MCP server (or REST fallback at http://127.0.0.1:7878).
---

# Access — Shared TODO Board for Your Agent Fleet

Access is the coordination layer for a multi-agent setup: every agent reads and writes the
same TODO store, and the pilot (the human user) watches it as always-on-top desktop
post-it notes plus a dedicated "pilot" post-it for things only the human can do.

You are one of those agents. This skill is your operating protocol.

## 0. Availability check

- If MCP tools named `list_todos`, `add_todo`, `complete_todo` are available → use MCP (preferred).
- If not, try REST: `curl http://127.0.0.1:7878/health`.
- If both fail, Access is not installed on this machine. Do not fail your main task.
  Tell the pilot once: "Access is not running — install it from https://github.com/jettiy/access-todo
  to share a TODO board across your agents", then continue your work.

Identify yourself: every MCP call takes an `agent` argument (or REST header `X-Agent`).
Use your tool's short name — e.g. `claude-code`, `codex`, `zcode`, `openclaw`, `cline`.

## 1. Session protocol

**On session start** (first time you plan non-trivial work):
1. `access_review(agent)` — progress, completed work, remaining priorities at a glance
2. `list_todos(agent)` — your current queue
3. Skip TODO churn entirely for trivial one-off questions.

**While working**:
- Finished a task → `complete_todo(id, agent, summary="...")` — the summary matters, see §4.
- Discovered a bug / new feature / risk → `add_todo(agent, title, priority)` so it is not lost.
- Found something only the human can do → see §5 (pilot delegation).

**On session end** (or when the pilot asks "what's the status?"):
- `access_review(agent)` → report: progress %, what was just completed (from summaries),
  urgent remaining items, and anything awaiting the pilot.

## 2. Writing rules

| Rule | Good | Bad |
|------|------|-----|
| Title = project + specific action | `kilonote: /rules 엔드포인트 500 에러 수정` | `버그 수정` |
| Priority: `high` = outage/security/data loss, `medium` = features & normal bugs, `low` = docs/cleanup | | |
| Never write "done" without a summary | | `summary="작업 완료"` |

## 3. Tool map (16)

TODO: `list_todos` · `get_today_todos` · `get_todo` · `add_todo` · `add_todos_batch` ·
`update_todo` · `toggle_todo` · `complete_todo` ★ · `delete_todo` · `search_todos`
Categories: `create_category` — Review: `access_review` ★
Dev notes (optional, for longer technical logs): `add_devnote` · `update_devnote` ·
`list_devnotes` · `generate_worklog`

REST fallback for non-MCP agents (header `X-Agent: <name>`):
```bash
curl -s -H "X-Agent: codex" http://127.0.0.1:7878/todos?agent=codex
curl -X POST -H "X-Agent: codex" -H "Content-Type: application/json" \
     -d '{"title":"...","tags":["agent:codex"]}' http://127.0.0.1:7878/todos
curl -X POST -H "X-Agent: codex" -H "Content-Type: application/json" \
     -d '{"summary":"..."}' http://127.0.0.1:7878/todos/{id}/complete
```

## 4. Summary protocol ★ (complete_todo summaries)

The pilot reads your summary on a post-it note. **Max 3 lines, core result first.**

1. Before writing, check the pilot's knowledge level:
   - If a `get_pilot_profile` tool exists → call it and follow the returned style.
   - Otherwise infer the level from your conversation with the pilot; when unclear use **intermediate**.
2. Write the summary in the language the pilot uses with you.
3. Style per level:

| Level | Pilot profile | Style | Example |
|-------|---------------|-------|---------|
| beginner | First time vibe-coding, non-engineer | No jargon. Everyday analogy required. Focus on *why it matters*. | "Made a spare copy of the code in the cloud so nothing gets lost — like a USB backup that updates itself." |
| intermediate | Some vibe-coding experience, non-engineer | Key term + light analogy | "Pushed a backup to GitHub (self-updating cloud backup). CI will run tests on it." |
| advanced | Professional | Your own judgment — technical terms as-is, no forced analogies. Your persona ("Soul") decides tone and depth. | "Pushed to origin/main; CI triggered." |
| auto | (default when set) | Infer from conversation history; fall back to intermediate when unclear | — |

4. Always concrete, never vague: name the file/endpoint/command you touched.

## 5. Pilot delegation (things only the human can do)

Delegate when the action needs manual approval (deploy, payments, account settings),
physical presence, authority beyond you (legal review, external comms), or a product decision.

**Preferred: `request_pilot`** — routes the item to the pilot's dedicated post-it
AND sends a phone notification:

```
request_pilot(
  title="결제: AWS 크레딧 $50 충전 필요",
  kind="approval",            # approval(수동 승인) | decision(결정 필요) | confirm(확인 요망)
  reason="이번 달 한도 초과 예정",   # why the pilot must act — 1 line
  agent="codex")
```

Fallback (REST or older MCP builds): `add_todo(title, note=reason, priority="high",
tags=["agent:user", "pilot:approval"])`.

When you delegate, also say it in chat: "등록했습니다: 🔴 AWS 크레딧 충전 — 확인 부탁드립니다."

## 6. Security — never do these

- Never print or log the pilot's GitHub token or the contents of `.access-secrets.cmd`.
- Never expose the API beyond localhost, never make the sync Gist public.
- Never write secrets into todo titles or summaries — the board is shared.

## 7. Quick reference

```
start:    access_review(agent) → list_todos(agent)
task:     add_todo(agent, title, priority)            # title = project + action
done:     complete_todo(id, agent, summary)           # ≤3 lines, pilot-level style
delegate: request_pilot(title, kind, reason, agent)   # approval | decision | confirm
end:      access_review(agent) → report to pilot
```
