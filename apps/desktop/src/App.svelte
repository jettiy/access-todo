<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, AGENT, AGENT_META, type Todo, type Priority } from "./api";

  let todos: Todo[] = [];
  let showAdd = false;
  let newTitle = "";
  let newNote = "";
  let newPriority: Priority = "medium";
  let synced = "로딩중";
  let errorMsg = "";
  let collapsed = false;

  const meta = AGENT_META[AGENT] || AGENT_META.user;
  const appWindow = getCurrentWindow();
  const pEmoji = (p: string) => (p === "high" ? "🔴" : p === "low" ? "🟢" : "🟡");

  async function refresh() {
    try {
      const r = await api.list();
      todos = r.todos;
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
    } catch (e) {
      errorMsg = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  async function add() {
    if (!newTitle.trim()) return;
    await api.addRaw(newTitle.trim(), newNote.trim() || undefined, newPriority, undefined, [`agent:${AGENT}`]);
    newTitle = "";
    newNote = "";
    newPriority = "medium";
    showAdd = false;
    await refresh();
  }

  async function toggle(id: string) {
    await api.toggle(id);
    await refresh();
  }

  async function remove(id: string) {
    await api.del(id);
    await refresh();
  }

  function minimize() { appWindow.minimize(); }
  function close() { appWindow.close(); }
  function toggleCollapse() { collapsed = !collapsed; }
  function startDrag() { appWindow.startDragging(); }

  onMount(() => {
    refresh();
    setInterval(refresh, 15000);
  });

  $: doneCount = todos.filter((t) => t.done).length;
</script>

<main class:collapsed
  style="--bg:{meta.bg}; --bg-soft:{meta.bgSoft}; --border:{meta.border}; --text:{meta.text};">
  <header on:mousedown={startDrag}>
    <span class="title">{meta.emoji} {meta.title}</span>
    <div class="window-controls">
      <button class="win-btn" on:click={toggleCollapse} title={collapsed ? "펼치기" : "접기"}>
        {collapsed ? "▾" : "▴"}
      </button>
      <button class="win-btn" on:click={minimize} title="최소화">⚊</button>
      <button class="win-btn close-btn" on:click={close} title="닫기">✕</button>
    </div>
  </header>

  {#if !collapsed}
    {#if errorMsg}
      <p class="error">{errorMsg}</p>
    {/if}

    <ul>
      {#each todos as t (t.id)}
        <li class:done={t.done}>
          <label>
            <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
            <span class="prio">{pEmoji(t.priority)}</span>
            <span class="content">
              <span class="ttl">{t.title}</span>
              {#if t.note}<span class="note">📝 {t.note}</span>{/if}
              {#if t.completed_by && t.completed_by !== AGENT}
                <span class="badge">✓ {t.completed_by}</span>
              {/if}
            </span>
            <button class="del" on:click={() => remove(t.id)} title="삭제">×</button>
          </label>
        </li>
      {:else}
        <li class="empty">할 일이 없어요</li>
      {/each}
    </ul>

    {#if showAdd}
      <div class="addform">
        <input placeholder="할 일 제목" bind:value={newTitle} on:keydown={(e) => e.key === "Enter" && add()} />
        <input placeholder="메모 (선택)" bind:value={newNote} />
        <div class="row">
          <label>우선순위:</label>
          <select bind:value={newPriority}>
            <option value="high">🔴 높음</option>
            <option value="medium">🟡 보통</option>
            <option value="low">🟢 낮음</option>
          </select>
        </div>
        <div class="addbtns">
          <button class="primary" on:click={add} disabled={!newTitle.trim()}>추가</button>
          <button on:click={() => (showAdd = false)}>취소</button>
        </div>
      </div>
    {/if}
  {/if}

  <footer>
    {#if !collapsed}
      <button class="add-btn" on:click={() => (showAdd = !showAdd)}>+ 추가</button>
    {/if}
    <span class="status">🔄 {synced} · {doneCount}/{todos.length}</span>
  </footer>
</main>

<style>
  :global(body) { margin: 0; background: transparent; overflow: hidden; }
  main {
    background: var(--bg);
    border-radius: 6px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.4);
    padding: 8px 10px;
    width: 100%; height: 100%;
    display: flex; flex-direction: column;
    -webkit-user-select: none; overflow: hidden; box-sizing: border-box;
    color: var(--text);
  }
  header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 6px; gap: 4px; cursor: move;
  }
  .title { font-weight: 700; font-size: 14px; flex: 1; cursor: move; }
  .window-controls { display: flex; gap: 2px; }
  .win-btn {
    background: rgba(0,0,0,0.08); border: none; border-radius: 3px;
    width: 22px; height: 22px; cursor: pointer; font-size: 12px; line-height: 1;
    color: var(--text); padding: 0; display: flex; align-items: center; justify-content: center;
  }
  .win-btn:hover { background: rgba(0,0,0,0.2); }
  .close-btn:hover { background: #e81123; color: white; }
  ul { list-style: none; padding: 0; margin: 0; flex: 1; overflow-y: auto; }
  li { padding: 4px 0; font-size: 13px; border-bottom: 1px dashed rgba(0,0,0,0.08); }
  li.done .ttl { text-decoration: line-through; opacity: 0.5; }
  li.empty { text-align: center; opacity: 0.6; font-size: 13px; padding: 30px 0; }
  label { display: flex; align-items: flex-start; gap: 5px; }
  .prio { line-height: 1.4; }
  .content { flex: 1; display: flex; flex-direction: column; }
  .ttl { line-height: 1.3; word-break: break-word; }
  .note { font-size: 11px; opacity: 0.7; margin-top: 1px; }
  .badge { font-size: 9px; background: rgba(255,255,255,0.5); padding: 0 4px; border-radius: 3px; width: fit-content; margin-top: 2px; }
  .del { background: none; border: none; color: var(--text); opacity: 0.4; cursor: pointer; font-size: 15px; padding: 0 2px; }
  .del:hover { opacity: 1; color: #c00; }
  .addform {
    display: flex; flex-direction: column; gap: 4px; margin: 6px 0;
    padding: 8px; background: var(--bg-soft); border-radius: 4px;
  }
  .addform input, .addform select {
    padding: 5px 7px; border: 1px solid var(--border); border-radius: 3px;
    font-size: 12px; background: rgba(255,255,255,0.7); color: var(--text);
  }
  .row { display: flex; align-items: center; gap: 6px; }
  .row label { font-size: 11px; min-width: 50px; }
  .row select { flex: 1; }
  .addbtns { display: flex; gap: 4px; }
  .addbtns button {
    flex: 1; padding: 5px; border: none; border-radius: 3px; cursor: pointer; font-size: 12px;
    background: rgba(0,0,0,0.08); color: var(--text);
  }
  .addbtns .primary { background: var(--border); color: white; }
  .add-btn {
    background: var(--border); color: white; border: none; border-radius: 4px;
    padding: 6px; cursor: pointer; font-size: 12px; font-weight: 600; width: 100%; margin-bottom: 4px;
  }
  .add-btn:hover { opacity: 0.85; }
  footer { display: flex; flex-direction: column; gap: 4px; }
  .status { font-size: 10px; opacity: 0.7; text-align: center; }
  .error { color: #c00; font-size: 11px; margin: 4px 0; }
</style>
