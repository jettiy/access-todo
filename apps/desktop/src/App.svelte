<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, type Todo, type Priority } from "./api";

  let todos: Todo[] = [];
  let newTitle = "";
  let newNote = "";
  let newPriority: Priority = "medium";
  let showAdd = false;
  let synced = "로딩중";
  let errorMsg = "";
  let collapsed = false;

  const appWindow = getCurrentWindow();
  const pEmoji = (p: string) => (p === "high" ? "🔴" : p === "low" ? "🟢" : "🟡");

  async function refresh() {
    try {
      const r = await api.today();
      todos = r.todos;
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
    } catch (e) {
      errorMsg = `서버에 연결할 수 없어요: ${(e as Error).message}`;
    }
  }

  async function add() {
    if (!newTitle.trim()) return;
    await api.add(newTitle.trim(), newNote.trim() || undefined, newPriority);
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

  function minimize() {
    appWindow.minimize();
  }

  function close() {
    appWindow.close();
  }

  function toggleCollapse() {
    collapsed = !collapsed;
  }

  onMount(() => {
    refresh();
    setInterval(refresh, 30000);
  });

  $: doneCount = todos.filter((t) => t.done).length;
</script>

<main class:collapsed>
  <!-- 헤더: 드래그로 이동 가능, 우측에 닫기/최소화 버튼 -->
  <header data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>📒 오늘의 할 일</span>
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
              {#if t.completed_by && t.completed_by !== "user"}
                <span class="badge">🤖 {t.completed_by}</span>
              {/if}
            </span>
            <button class="del" on:click={() => remove(t.id)} title="삭제">×</button>
          </label>
        </li>
      {:else}
        <li class="empty">오늘 할 일이 없어요 🎉<br /><span class="hint">(포스트잇을 더블클릭하면 접혀요)</span></li>
      {/each}
    </ul>

    {#if showAdd}
      <div class="addform">
        <input placeholder="제목" bind:value={newTitle} on:keydown={(e) => e.key === "Enter" && add()} />
        <input placeholder="메모 (선택)" bind:value={newNote} on:keydown={(e) => e.key === "Enter" && add()} />
        <select bind:value={newPriority}>
          <option value="high">🔴 높음</option>
          <option value="medium">🟡 보통</option>
          <option value="low">🟢 낮음</option>
        </select>
        <div class="addbtns">
          <button class="primary" on:click={add}>추가</button>
          <button on:click={() => (showAdd = false)}>취소</button>
        </div>
      </div>
    {:else}
      <button class="addtoggle" on:click={() => (showAdd = true)}>+ 새 할 일</button>
    {/if}

    <footer>
      <span>🔄 {synced}</span>
      <span>{doneCount}/{todos.length} 완료</span>
    </footer>
  {/if}
</main>

<svelte:window on:dblclick={toggleCollapse} />

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overflow: hidden;
  }

  main {
    background: #fff0a0;
    border-radius: 6px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
    padding: 8px 10px 10px;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    -webkit-user-select: none;
    overflow: hidden;
    box-sizing: border-box;
  }

  main.collapsed {
    padding-bottom: 8px;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
    cursor: default;
    gap: 4px;
  }

  .title {
    font-weight: 700;
    font-size: 14px;
    flex: 1;
    cursor: default;
  }

  .window-controls {
    display: flex;
    gap: 2px;
  }

  .win-btn {
    background: rgba(0, 0, 0, 0.08);
    border: none;
    border-radius: 3px;
    width: 22px;
    height: 22px;
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
    color: #5a4a00;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .win-btn:hover {
    background: rgba(0, 0, 0, 0.18);
  }

  .close-btn:hover {
    background: #e81123;
    color: white;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    flex: 1;
    overflow-y: auto;
  }

  li {
    padding: 5px 2px;
    border-bottom: 1px dashed rgba(0, 0, 0, 0.12);
    font-size: 13px;
  }

  li.empty {
    text-align: center;
    color: #777;
    border: none;
    padding: 20px 0;
    line-height: 1.6;
  }

  .hint {
    font-size: 10px;
    color: #aaa;
  }

  li.done .ttl {
    text-decoration: line-through;
    color: #888;
  }

  label {
    display: flex;
    align-items: flex-start;
    gap: 5px;
  }

  .prio {
    line-height: 1.5;
  }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .ttl {
    line-height: 1.4;
    word-break: break-word;
  }

  .note {
    font-size: 11px;
    color: #5a4a00;
    margin-top: 2px;
  }

  .badge {
    display: inline-block;
    width: fit-content;
    margin-top: 2px;
    font-size: 10px;
    background: rgba(255, 255, 255, 0.6);
    padding: 1px 5px;
    border-radius: 3px;
  }

  .del {
    background: none;
    border: none;
    color: #a55;
    cursor: pointer;
    font-size: 15px;
    line-height: 1;
    padding: 0 2px;
  }

  .del:hover {
    color: #c00;
  }

  .addtoggle {
    margin-top: 6px;
    padding: 6px;
    background: rgba(255, 255, 255, 0.4);
    border: 1px dashed #b89b00;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    color: #5a4a00;
  }

  .addtoggle:hover {
    background: rgba(255, 255, 255, 0.6);
  }

  .addform {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 6px;
    padding: 6px;
    background: rgba(255, 255, 255, 0.5);
    border-radius: 4px;
  }

  .addform input,
  .addform select {
    padding: 4px 6px;
    border: 1px solid #c9a;
    border-radius: 3px;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.85);
  }

  .addbtns {
    display: flex;
    gap: 4px;
  }

  .addbtns button {
    flex: 1;
    padding: 4px;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    background: rgba(0, 0, 0, 0.08);
    color: #5a4a00;
  }

  .addbtns .primary {
    background: #b89b00;
    color: white;
  }

  footer {
    margin-top: 6px;
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: #5a4a00;
  }

  .error {
    color: #b00;
    font-size: 11px;
    margin: 4px 0;
  }
</style>
