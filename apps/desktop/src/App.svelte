<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, AGENT, AGENT_META, type Todo, type Priority, type Category } from "./api";

  let todos: Todo[] = [];
  let categories: Category[] = [];
  let showAdd = false;
  let newTitle = "";
  let newNote = "";
  let newPriority: Priority = "medium";
  let newCategoryId = "";
  let showAddCat = false;
  let newCatName = "";
  let renamingId: string | null = null;
  let renameText = "";
  let synced = "로딩중";
  let errorMsg = "";
  let collapsed = false;
  let onTop = true;

  const meta = AGENT_META[AGENT] || AGENT_META.user;
  const appWindow = getCurrentWindow();
  const pEmoji = (p: string) => (p === "high" ? "🔴" : p === "low" ? "🟢" : "🟡");

  async function refresh() {
    try {
      const r = await api.list();
      todos = r.todos;
      categories = r.categories;
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
    } catch (e) {
      errorMsg = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  async function add() {
    if (!newTitle.trim()) return;
    await api.addRaw(newTitle.trim(), newNote.trim() || undefined, newPriority, undefined, [`agent:${AGENT}`], newCategoryId || undefined);
    newTitle = "";
    newNote = "";
    newPriority = "medium";
    newCategoryId = "";
    showAdd = false;
    await refresh();
  }

  async function addCategory() {
    if (!newCatName.trim()) return;
    await api.addCategory(newCatName.trim());
    newCatName = "";
    showAddCat = false;
    await refresh();
  }

  async function startRename(cat: Category) {
    renamingId = cat.id;
    renameText = cat.name;
  }

  async function confirmRename() {
    if (renamingId && renameText.trim()) {
      await api.renameCategory(renamingId, renameText.trim());
    }
    renamingId = null;
    await refresh();
  }

  function cancelRename() {
    renamingId = null;
    renameText = "";
  }

  async function deleteCategory(cat: Category) {
    const todoCount = todosByCategory(cat.id).length;
    const msg = todoCount > 0
      ? `'${cat.name}' 삭제? ${todoCount}개 할 일은 미분류로 이동합니다.`
      : `'${cat.name}' 삭제?`;
    if (!confirm(msg)) return;
    await api.deleteCategory(cat.id);
    await refresh();
  }

  async function moveCategory(idx: number, dir: -1 | 1) {
    const newIdx = idx + dir;
    if (newIdx < 0 || newIdx >= categories.length) return;
    const reordered = [...categories];
    [reordered[idx], reordered[newIdx]] = [reordered[newIdx], reordered[idx]];
    await api.reorderCategories(reordered.map((c) => c.id));
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

  function toggleOnTop() { onTop = !onTop; appWindow.setAlwaysOnTop(onTop); }
  function minimize() { appWindow.minimize(); }
  function close() { appWindow.close(); }
  function toggleCollapse() { collapsed = !collapsed; }
  function startDrag() { appWindow.startDragging(); }
  function titleMousedown(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest(".window-controls") || target.closest("button")) return;
    startDrag();
  }

  // todos를 category_id별로 그룹화. null/undefined = "미분류"
  function todosByCategory(catId: string | null): Todo[] {
    return todos.filter((t) => (t.category_id ?? null) === catId);
  }

  onMount(() => {
    refresh();
    setInterval(refresh, 15000);
  });

  $: doneCount = todos.filter((t) => t.done).length;
</script>

<main class:collapsed
  style="--bg:{meta.bg}; --bg-soft:{meta.bgSoft}; --border:{meta.border}; --text:{meta.text};">
  <header on:mousedown={titleMousedown}>
    <span class="title" on:mousedown={titleMousedown}>{meta.emoji} {meta.title}</span>
    <div class="window-controls" on:mousedown|stopPropagation>
      <button class="win-btn" on:click={toggleOnTop} title={onTop ? "항상 위 해제" : "항상 위 설정"} class:active={onTop}>📌</button>
      <button class="win-btn" on:click={toggleCollapse} title={collapsed ? "펼치기" : "접기"}>{collapsed ? "▾" : "▴"}</button>
      <button class="win-btn" on:click={minimize} title="최소화">⚊</button>
      <button class="win-btn close-btn" on:click={close} title="닫기">✕</button>
    </div>
  </header>

  {#if !collapsed}
    {#if errorMsg}<p class="error">{errorMsg}</p>{/if}

    <div class="groups">
      <!-- 정의된 카테고리들 -->
      {#each categories as cat, idx (cat.id)}
        <section class="cat-section">
          <div class="cat-header">
            {#if renamingId === cat.id}
              <input class="rename-input" bind:value={renameText} on:keydown={(e) => e.key === "Enter" && confirmRename() || e.key === "Escape" && cancelRename()} />
              <div class="cat-controls" on:mousedown|stopPropagation>
                <button class="cat-btn confirm" on:click={confirmRename} title="확인">✓</button>
                <button class="cat-btn" on:click={cancelRename} title="취소">✕</button>
              </div>
            {:else}
              <span class="cat-name" on:dblclick={() => startRename(cat)}>📂 {cat.name}</span>
              <div class="cat-controls" on:mousedown|stopPropagation>
                <button class="cat-btn" on:click={() => startRename(cat)} title="이름 변경">✏️</button>
                <button class="cat-btn" on:click={() => moveCategory(idx, -1)} title="위로" disabled={idx === 0}>▲</button>
                <button class="cat-btn" on:click={() => moveCategory(idx, 1)} title="아래로" disabled={idx === categories.length - 1}>▼</button>
                <button class="cat-btn danger" on:click={() => deleteCategory(cat)} title="삭제">🗑️</button>
              </div>
            {/if}
          </div>
          <ul>
            {#each todosByCategory(cat.id) as t (t.id)}
              <li class:done={t.done}>
                <label>
                  <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
                  <span class="prio">{pEmoji(t.priority)}</span>
                  <span class="content">
                    <span class="ttl">{t.title}</span>
                    {#if t.note}<span class="note">{t.note.startsWith("✓") ? "✅" : "📝"} {t.note}</span>{/if}
                  </span>
                  <button class="del" on:click={() => remove(t.id)}>×</button>
                </label>
              </li>
            {:else}
              <li class="empty-cat">비어있음</li>
            {/each}
          </ul>
        </section>
      {/each}

      <!-- 미분류 (category_id가 없는 할 일들) -->
      {#if todosByCategory(null).length > 0}
        <section class="cat-section">
          <div class="cat-header"><span class="cat-name">📦 미분류</span></div>
          <ul>
            {#each todosByCategory(null) as t (t.id)}
              <li class:done={t.done}>
                <label>
                  <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
                  <span class="prio">{pEmoji(t.priority)}</span>
                  <span class="content">
                    <span class="ttl">{t.title}</span>
                    {#if t.note}<span class="note">{t.note.startsWith("✓") ? "✅" : "📝"} {t.note}</span>{/if}
                  </span>
                  <button class="del" on:click={() => remove(t.id)}>×</button>
                </label>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>

    <!-- 새 할 일 추가 폼 -->
    {#if showAdd}
      <div class="addform">
        <input placeholder="할 일 제목" bind:value={newTitle} on:keydown={(e) => e.key === "Enter" && add()} />
        <input placeholder="메모 (선택)" bind:value={newNote} />
        <div class="row">
          <label>카테고리:</label>
          <select bind:value={newCategoryId}>
            <option value="">📦 미분류</option>
            {#each categories as cat (cat.id)}
              <option value={cat.id}>📂 {cat.name}</option>
            {/each}
          </select>
        </div>
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

    <!-- 새 카테고리 추가 폼 -->
    {#if showAddCat}
      <div class="addform">
        <input placeholder="카테고리 이름" bind:value={newCatName} on:keydown={(e) => e.key === "Enter" && addCategory()} />
        <div class="addbtns">
          <button class="primary" on:click={addCategory} disabled={!newCatName.trim()}>카테고리 추가</button>
          <button on:click={() => (showAddCat = false)}>취소</button>
        </div>
      </div>
    {/if}
  {/if}

  <footer on:mousedown|stopPropagation>
    {#if !collapsed}
      <div class="action-row">
        <button class="add-btn" on:click={() => (showAdd = !showAdd)}>+ 할 일</button>
        <button class="add-btn secondary" on:click={() => (showAddCat = !showAddCat)}>📂 카테고리</button>
      </div>
    {/if}
    <span class="status">🔄 {synced} · {doneCount}/{todos.length} · 📂{categories.length}</span>
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
  .win-btn.active { background: rgba(0,0,0,0.25); font-weight: bold; }
  .close-btn:hover { background: #e81123; color: white; }
  .groups { flex: 1; overflow-y: auto; }
  .cat-section { margin-bottom: 6px; }
  .cat-header {
    display: flex; align-items: center; justify-content: space-between;
    font-size: 12px; font-weight: 600;
    border-bottom: 1px solid var(--border); padding-bottom: 2px; margin: 4px 0 2px;
  }
  .cat-name { cursor: pointer; }
  .cat-controls { display: flex; gap: 1px; }
  .cat-btn {
    background: none; border: none; cursor: pointer; font-size: 11px;
    color: var(--text); opacity: 0.5; padding: 0 2px; line-height: 1;
  }
  .cat-btn:hover { opacity: 1; }
  .cat-btn:disabled { opacity: 0.2; cursor: default; }
  .cat-btn.confirm { color: green; opacity: 0.8; }
  .cat-btn.danger:hover { color: #c00; }
  .rename-input {
    font-size: 12px; font-weight: 600; border: 1px solid var(--border);
    border-radius: 3px; padding: 1px 4px; background: white; color: var(--text); flex: 1;
  }
  ul { list-style: none; padding: 0; margin: 0; }
  li { padding: 3px 0; font-size: 13px; border-bottom: 1px dashed rgba(0,0,0,0.06); }
  li.done .ttl { text-decoration: line-through; opacity: 0.5; }
  li.empty-cat { text-align: center; opacity: 0.4; font-size: 11px; padding: 4px 0; }
  label { display: flex; align-items: flex-start; gap: 5px; }
  .prio { line-height: 1.4; }
  .content { flex: 1; display: flex; flex-direction: column; }
  .ttl { line-height: 1.3; word-break: break-word; }
  .note { font-size: 11px; opacity: 0.7; margin-top: 1px; }
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
  .action-row { display: flex; gap: 4px; margin-bottom: 4px; }
  .add-btn {
    background: var(--border); color: white; border: none; border-radius: 4px;
    padding: 6px; cursor: pointer; font-size: 12px; font-weight: 600; flex: 1;
  }
  .add-btn.secondary { background: rgba(0,0,0,0.15); }
  .add-btn:hover { opacity: 0.85; }
  footer { display: flex; flex-direction: column; gap: 4px; }
  .status { font-size: 10px; opacity: 0.7; text-align: center; }
  .error { color: #c00; font-size: 11px; margin: 4px 0; }
</style>
