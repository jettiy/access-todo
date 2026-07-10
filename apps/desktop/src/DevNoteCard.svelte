<script lang="ts">
  import type { DevNote } from "./api";

  export let note: DevNote;
  export let onEdit: (note: DevNote) => void;
  export let onDelete: (id: string) => void;

  const typeMeta: Record<string, { emoji: string; label: string; color: string }> = {
    worklog: { emoji: "📋", label: "작업 로그", color: "#3ca87e" },
    retrospect: { emoji: "💭", label: "회고", color: "#3c7ab0" },
    issue: { emoji: "🐛", label: "이슈", color: "#c46b3c" },
    playtest: { emoji: "🎮", label: "플레이테스트", color: "#7a2a9c" },
  };

  const systemLabels: Record<string, string> = {
    combat: "⚔️ 전투", inventory: "🎒 인벤토리", quest: "📜 퀘스트",
    skill: "✨ 스킬", ui: "🖥️ UI", audio: "🔊 오디오",
    map: "🗺️ 맵", ai: "🤖 AI", save: "💾 세이브",
    network: "🌐 네트워크", etc: "📦 기타",
  };

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
  <h3 class="title">{note.title}</h3>
  <div class="body">{note.body}</div>
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
  .title { margin: 4px 0; font-size: 14px; font-weight: 700; }
  .body {
    font-size: 12px; white-space: pre-wrap; line-height: 1.5;
    color: #444; max-height: 300px; overflow-y: auto;
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
