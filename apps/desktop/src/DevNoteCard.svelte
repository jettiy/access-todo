<script lang="ts">
  import type { DevNote } from "./api";

  export let note: DevNote;
  export let onEdit: (note: DevNote) => void;
  export let onDelete: (id: string) => void;
  export let showDev = false;

  const typeMeta: Record<string, { emoji: string; label: string; color: string }> = {
    worklog: { emoji: "📋", label: "작업 로그", color: "#3ca87e" },
    retrospect: { emoji: "💭", label: "회고", color: "#3c7ab0" },
    issue: { emoji: "🐛", label: "이슈", color: "#c46b3c" },
    playtest: { emoji: "🎮", label: "테스트", color: "#7a2a9c" },
  };

  const systemLabels: Record<string, string> = {
    combat: "⚔️ 전투", inventory: "🎒 인벤토리", quest: "📜 퀘스트",
    skill: "✨ 스킬", ui: "🖥️ UI", audio: "🔊 오디오",
    map: "🗺️ 맵", ai: "🤖 AI", save: "💾 세이브",
    network: "🌐 네트워크", etc: "📦 기타",
  };

  // ── 작업 단계 (파이프라인) ──
  const STAGES = ["계획", "조사", "개발", "검증", "완료"];
  const STAGE_FALLBACK: Record<string, number> = {
    issue: 2,      // 문제 발견 → 개발(수정) 단계 필요
    worklog: 2,    // 진행 중 기록 → 개발
    playtest: 3,   // 테스트 → 검증
    retrospect: 4, // 회고 → 완료
  };
  function detectStage(n: DevNote): number {
    const hay = `${n.title}\n${n.body}`;
    for (let i = 0; i < STAGES.length; i++) {
      const s = STAGES[i];
      if (hay.includes(`[${s}]`) || hay.includes(`단계: ${s}`) || hay.includes(`단계:${s}`)) return i;
    }
    return STAGE_FALLBACK[n.note_type] ?? 2;
  }
  $: stageIdx = detectStage(note);

  // ── 본문 3층 파싱: 첫 줄=한줄요약 / --- 구분선 앞=본문 / 뒤=개발 세부 ──
  function parseBody(body: string): { summary: string; main: string; dev: string } {
    const lines = (body || "").split("\n");
    const summary = (lines[0] || "").trim();
    const sepIdx = lines.findIndex((l) => /^(-{3,}|={3,})$/.test(l.trim()));
    if (sepIdx >= 0) {
      return {
        summary,
        main: lines.slice(1, sepIdx).join("\n").trim(),
        dev: lines.slice(sepIdx + 1).join("\n").trim(),
      };
    }
    return { summary, main: lines.slice(1).join("\n").trim(), dev: "" };
  }
  $: parsed = parseBody(note.body);

  $: meta = typeMeta[note.note_type] || typeMeta.retrospect;
  $: isWorklog = note.note_type === "worklog";
  $: dateStr = new Date(note.created_at).toLocaleDateString("ko-KR", { month: "short", day: "numeric" });
</script>

<div class="card" style="--accent: {meta.color}">
  <div class="header">
    <span class="type-badge">{meta.emoji} {meta.label}</span>
    {#if note.game_system}
      <span class="sys-badge">{systemLabels[note.game_system] || note.game_system}</span>
    {/if}
    {#if note.version}
      <span class="ver-badge">📦 {note.version}</span>
    {/if}
  </div>

  <!-- 작업 단계 표시기 -->
  <div class="stage-bar" title="작업 단계: {STAGES[stageIdx]}">
    {#each STAGES as s, i}
      <span class="stage-dot" class:done={i < stageIdx} class:current={i === stageIdx}></span>
    {/each}
    <span class="stage-label">{STAGES[stageIdx]} 단계</span>
  </div>

  <h3 class="title">{note.title}</h3>

  {#if parsed.summary}
    <div class="summary">💡 {parsed.summary}</div>
  {/if}

  {#if parsed.main}
    <div class="body">{parsed.main}</div>
  {/if}

  {#if parsed.dev}
    <button class="dev-toggle" on:click={() => (showDev = !showDev)}>
      {showDev ? "▾" : "▸"} 🔧 개발 세부 {showDev ? "접기" : "펼치기"}
    </button>
    {#if showDev}
      <div class="dev-detail">{parsed.dev}</div>
    {/if}
  {/if}

  <div class="footer">
    <span class="meta">✍️ {note.created_by} · {dateStr}</span>
    {#if !isWorklog}
      <div class="actions">
        <button class="btn-edit" on:click={() => onEdit(note)}>✏️ 수정</button>
        <button class="btn-del" on:click={() => onDelete(note.id)}>🗑️</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .card {
    background: #f8f9fa;
    border-left: 4px solid var(--accent);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 10px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
  }
  .header { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 6px; }
  .type-badge {
    background: var(--accent); color: white; font-size: 11px;
    padding: 2px 8px; border-radius: 10px; font-weight: 600;
  }
  .sys-badge,
  .ver-badge {
    background: #e0e0e0; color: #555; font-size: 11px;
    padding: 2px 8px; border-radius: 10px;
  }

  /* 단계 표시기 */
  .stage-bar {
    display: flex; align-items: center; gap: 4px;
    margin: 2px 0 6px;
  }
  .stage-dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: #ddd; border: 1px solid #ccc;
    flex-shrink: 0;
  }
  .stage-dot.done { background: #9fd8bd; border-color: #3ca87e; }
  .stage-dot.current { background: #3ca87e; border-color: #2a7a5e; box-shadow: 0 0 0 2px #dcf5ec; }
  .stage-label { font-size: 10px; font-weight: 700; color: #2a7a5e; margin-left: 4px; }

  .title { margin: 4px 0; font-size: 14px; font-weight: 700; }

  /* 한줄 요약 — 파란 박스 */
  .summary {
    background: #e3f0fd; border: 1px solid #b7d3f5; border-radius: 6px;
    padding: 6px 8px; font-size: 12.5px; font-weight: 600; color: #1a4a7a;
    margin-bottom: 6px; line-height: 1.4;
  }

  .body {
    font-size: 12px; white-space: pre-wrap; line-height: 1.5;
    color: #444; max-height: 300px; overflow-y: auto;
  }

  /* 개발 세부 — 토글 */
  .dev-toggle {
    background: none; border: 1px dashed #bbb; border-radius: 4px;
    font-size: 10.5px; color: #888; cursor: pointer; padding: 3px 8px;
    margin-top: 6px; width: 100%; text-align: left;
  }
  .dev-toggle:hover { background: #f0f0f0; color: #555; }
  .dev-detail {
    font-size: 10.5px; white-space: pre-wrap; line-height: 1.45;
    color: #777; background: #f2f2f2; border-radius: 4px;
    padding: 6px 8px; margin-top: 4px;
    font-family: Consolas, "Courier New", monospace;
    max-height: 220px; overflow-y: auto;
  }

  .footer {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 6px; border-top: 1px solid #eee;
  }
  .meta { font-size: 10px; color: #999; }
  .actions { display: flex; gap: 4px; }
  .btn-edit,
  .btn-del {
    border: none; background: none; cursor: pointer; font-size: 11px; padding: 2px 6px;
  }
  .btn-edit:hover { color: var(--accent); }
  .btn-del:hover { color: #c00; }
</style>
