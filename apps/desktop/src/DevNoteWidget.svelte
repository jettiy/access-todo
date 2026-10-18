<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { devnoteApi, type DevNote, type NoteType, type GameSystem } from "./api";
  import DevNoteCard from "./DevNoteCard.svelte";
  import DevNoteEditor from "./DevNoteEditor.svelte";

  let notes: DevNote[] = [];
  let activeTab: NoteType | "all" = "all";
  let systemFilter: GameSystem | "" = "";
  let versionFilter = "";
  let searchQuery = "";
  let showEditor = false;
  let editingNote: DevNote | null = null;
  let synced = "로딩중";
  let errorMsg = "";
  let generating = false;
  let allVersions: string[] = [];
  let collapsed = false;
  let onTop = false;

  const appWindow = getCurrentWindow();

  const tabs: { value: NoteType | "all"; label: string; emoji: string }[] = [
    { value: "all", label: "전체", emoji: "📋" },
    { value: "worklog", label: "작업로그", emoji: "📋" },
    { value: "retrospect", label: "회고", emoji: "💭" },
    { value: "issue", label: "이슈", emoji: "🐛" },
    { value: "playtest", label: "테스트", emoji: "🎮" },
  ];

  async function refresh() {
    try {
      const r = await devnoteApi.timeline();
      notes = r.devnotes;
      const vset = new Set<string>();
      notes.forEach((n) => { if (n.version) vset.add(n.version); });
      allVersions = Array.from(vset).sort().reverse();
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
    } catch (e) {
      errorMsg = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  $: filtered = notes
    .filter((n) => activeTab === "all" || n.note_type === activeTab)
    .filter((n) => !systemFilter || n.game_system === systemFilter)
    .filter((n) => !versionFilter || n.version === versionFilter)
    .filter(
      (n) =>
        !searchQuery ||
        n.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        n.body.toLowerCase().includes(searchQuery.toLowerCase())
    );

  function openNewEditor() {
    editingNote = null;
    showEditor = true;
  }
  function openEditEditor(note: DevNote) {
    editingNote = note;
    showEditor = true;
  }
  async function deleteNote(id: string) {
    if (!confirm("이 노트를 삭제할까요?")) return;
    await devnoteApi.del(id);
    await refresh();
  }
  async function generateToday() {
    generating = true;
    try {
      const r = await devnoteApi.generateWorklog();
      if (r.created) {
        alert(`작업 로그 생성! ${r.todos_counted}개 작업, 시스템: ${r.systems.join(", ")}`);
      } else {
        alert(`기존 로그 업데이트. ${r.todos_counted}개 작업 반영.`);
      }
      await refresh();
    } catch (e) {
      alert(`생성 실패: ${(e as Error).message}`);
    } finally {
      generating = false;
    }
  }
  function onSaved() {
    showEditor = false;
    refresh();
  }

  // ── 창 컨트롤 (포스트잇과 동일한 UX) ──
  function toggleOnTop() { onTop = !onTop; appWindow.setAlwaysOnTop(onTop); }
  function minimize() { appWindow.minimize(); }
  async function close() {
    try { await fetch("http://127.0.0.1:7878/sync", { method: "POST", headers: { "X-Agent": "devnotes" } }); } catch {}
    await appWindow.close();
  }
  function toggleCollapse() { collapsed = !collapsed; }
  function startDrag() { appWindow.startDragging(); }
  function titleMousedown(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t.closest(".window-controls") || t.closest("button")) return;
    startDrag();
  }

  onMount(() => {
    refresh();
    // 핀 기본 해제: lib.rs에서 always_on_top(true)로 뜨기 때문에 시작 시 한 번 끈다
    appWindow.setAlwaysOnTop(false);
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  });
</script>

<div class="widget" class:collapsed>
  <header class="top-bar" on:mousedown={titleMousedown}>
    <span class="title" on:mousedown={titleMousedown}>📝 게임 개발노트</span>
    <span class="synced">{synced}</span>
    <div class="window-controls" on:mousedown|stopPropagation>
      <button class="win-btn" on:click={toggleOnTop} title={onTop ? "항상 위 해제" : "항상 위"} class:active={onTop}>📌</button>
      <button class="win-btn" on:click={toggleCollapse} title="접기">{collapsed ? "▾" : "▴"}</button>
      <button class="win-btn" on:click={minimize} title="최소화">⚊</button>
      <button class="win-btn close-btn" on:click={close} title="닫기">✕</button>
    </div>
  </header>

  {#if !collapsed}
    {#if errorMsg}
      <div class="error">{errorMsg}</div>
    {/if}

    <div class="tabs">
      {#each tabs as tab}
        <button
          class="tab"
          class:active={activeTab === tab.value}
          on:click={() => (activeTab = tab.value)}
        >
          {tab.emoji} {tab.label}
        </button>
      {/each}
    </div>

    <div class="filter-bar">
      <input type="text" bind:value={searchQuery} placeholder="🔍 검색..." />
      <select bind:value={systemFilter}>
        <option value="">🎮 전체 시스템</option>
        <option value="combat">⚔️ 전투</option>
        <option value="inventory">🎒 인벤토리</option>
        <option value="quest">📜 퀘스트</option>
        <option value="skill">✨ 스킬</option>
        <option value="ui">🖥️ UI</option>
        <option value="audio">🔊 오디오</option>
        <option value="map">🗺️ 맵</option>
        <option value="ai">🤖 AI</option>
        <option value="save">💾 세이브</option>
        <option value="network">🌐 네트워크</option>
        <option value="etc">📦 기타</option>
      </select>
      <select bind:value={versionFilter}>
        <option value="">📦 전체 버전</option>
        {#each allVersions as v}
          <option value={v}>{v}</option>
        {/each}
      </select>
    </div>

    <div class="note-list">
      {#if filtered.length === 0}
        <div class="empty">노트가 없습니다. 새 노트를 추가하거나 작업 로그를 생성하세요.</div>
      {:else}
        {#each filtered as note (note.id)}
          <DevNoteCard {note} onEdit={openEditEditor} onDelete={deleteNote} />
        {/each}
      {/if}
    </div>

    <footer class="bottom-bar">
      <button class="btn-new" on:click={openNewEditor}>+ 새 노트</button>
      <button class="btn-worklog" on:click={generateToday} disabled={generating}>
        {generating ? "생성중..." : "⚡ 오늘 로그"}
      </button>
    </footer>
  {/if}
</div>

{#if showEditor}
  <DevNoteEditor existing={editingNote} on:saved={onSaved} on:cancel={() => (showEditor = false)} />
{/if}

<style>
  .widget {
    display: flex; flex-direction: column; height: 100vh;
    font-family: -apple-system, "Segoe UI", sans-serif; background: #fafafa;
  }
  .widget.collapsed { height: auto; }
  .top-bar {
    display: flex; justify-content: space-between; align-items: center; gap: 6px;
    padding: 10px 14px; background: #3ca87e; color: white; cursor: move;
  }
  .title { font-size: 14px; font-weight: 700; flex: 1; cursor: move; }
  .synced { font-size: 10px; opacity: 0.8; }
  .window-controls { display: flex; gap: 2px; }
  .win-btn {
    background: rgba(0,0,0,0.15); border: none; border-radius: 3px;
    width: 22px; height: 22px; cursor: pointer; font-size: 12px; line-height: 1;
    color: white; padding: 0; display: flex; align-items: center; justify-content: center;
  }
  .win-btn:hover { background: rgba(0,0,0,0.35); }
  .win-btn.active { background: rgba(0,0,0,0.45); font-weight: bold; }
  .close-btn:hover { background: #e81123; }
  .error { padding: 8px; background: #fee; color: #c00; font-size: 12px; }
  .tabs { display: flex; gap: 2px; padding: 6px 8px; background: white; border-bottom: 1px solid #eee; }
  .tab {
    flex: 1; padding: 6px 4px; border: none; background: none; cursor: pointer;
    font-size: 11px; border-radius: 6px; white-space: nowrap;
  }
  .tab.active { background: #dcf5ec; color: #1a5a44; font-weight: 600; }
  .filter-bar {
    display: flex; gap: 4px; padding: 6px 8px; background: white; border-bottom: 1px solid #eee;
  }
  .filter-bar input,
  .filter-bar select {
    flex: 1; padding: 4px 6px; font-size: 11px; border: 1px solid #ddd; border-radius: 4px;
  }
  .note-list { flex: 1; overflow-y: auto; padding: 8px; }
  .empty { text-align: center; color: #999; font-size: 12px; padding: 40px 20px; }
  .bottom-bar {
    display: flex; gap: 8px; padding: 10px 14px; background: white; border-top: 1px solid #eee;
  }
  .btn-new {
    flex: 1; padding: 10px; border: none; background: #3ca87e; color: white;
    border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 13px;
  }
  .btn-worklog {
    flex: 1; padding: 10px; border: none; background: #fff3cd; color: #856404;
    border-radius: 8px; cursor: pointer; font-weight: 600; font-size: 13px;
  }
  .btn-worklog:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-new:hover { opacity: 0.85; }
</style>
