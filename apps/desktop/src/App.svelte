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
  let collapsedCats: Set<string> = new Set();
  let firstLoad = true;
  let flashIds: Set<string> = new Set();
  let newCount = 0;
  let hideDone = true; // 완료된 항목 기본 숨김

  const meta = AGENT_META[AGENT] || AGENT_META.user;
  const appWindow = getCurrentWindow();
  const pLabel = (p: string) => p === "high" ? "🔴 긴급" : p === "low" ? "🟢 여유" : "🟡 보통";

  async function refresh() {
    try {
      const r = await api.list();
      if (!firstLoad) {
        const oldIds = new Set(todos.map((t) => t.id));
        const incoming = r.todos.filter((t) => !oldIds.has(t.id) && t.created_by !== AGENT && !t.done);
        for (const t of incoming) {
          flashIds.add(t.id);
          setTimeout(() => { flashIds.delete(t.id); flashIds = flashIds; }, 4000);
        }
        if (incoming.length > 0) newCount += incoming.length;
      }
      firstLoad = false;
      todos = r.todos;
      categories = r.categories;
      synced = new Date().toLocaleTimeString("ko-KR");
      errorMsg = "";
      flashIds = flashIds;
    } catch (e) {
      errorMsg = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  function clearNewCount() { newCount = 0; }

  async function add() {
    if (!newTitle.trim()) return;
    await api.addRaw(newTitle.trim(), newNote.trim() || undefined, newPriority, undefined, [`agent:${AGENT}`], newCategoryId || undefined);
    newTitle = ""; newNote = ""; newPriority = "medium"; newCategoryId = ""; showAdd = false;
    await refresh();
  }
  async function addCategory() {
    if (!newCatName.trim()) return;
    await api.addCategory(newCatName.trim());
    newCatName = ""; showAddCat = false;
    await refresh();
  }
  async function startRename(cat: Category) { renamingId = cat.id; renameText = cat.name; }
  async function confirmRename() {
    if (renamingId && renameText.trim()) await api.renameCategory(renamingId, renameText.trim());
    renamingId = null; await refresh();
  }
  function cancelRename() { renamingId = null; renameText = ""; }
  async function deleteCategory(cat: Category) {
    const n = visibleTodos(cat.id).length;
    if (!confirm(`'${cat.name}' 삭제? ${n > 0 ? `${n}개 할 일은 미분류로 이동.` : ""}`)) return;
    await api.deleteCategory(cat.id); await refresh();
  }
  function toggleCatCollapse(catId: string) {
    if (collapsedCats.has(catId)) collapsedCats.delete(catId);
    else collapsedCats.add(catId);
    collapsedCats = collapsedCats;
  }
  async function moveCategory(idx: number, dir: -1 | 1) {
    const ni = idx + dir;
    if (ni < 0 || ni >= categories.length) return;
    const r = [...categories];
    [r[idx], r[ni]] = [r[ni], r[idx]];
    await api.reorderCategories(r.map((c) => c.id));
    await refresh();
  }
  async function toggle(id: string) { await api.toggle(id); await refresh(); }
  async function remove(id: string) { await api.del(id); await refresh(); }
  function toggleOnTop() { onTop = !onTop; appWindow.setAlwaysOnTop(onTop); }
  function minimize() { appWindow.minimize(); }
  async function close() {
    try { await fetch("http://127.0.0.1:7878/sync", { method: "POST", headers: { "X-Agent": AGENT } }); } catch {}
    await appWindow.close();
  }
  function toggleCollapse() { collapsed = !collapsed; }
  function startDrag() { appWindow.startDragging(); }
  function titleMousedown(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (t.closest(".window-controls") || t.closest("button")) return;
    startDrag();
  }

  function visibleTodos(catId: string | null): Todo[] {
    return todos.filter((t) => (t.category_id ?? null) === catId && (!hideDone || !t.done));
  }
  function doneTodos(catId: string | null): Todo[] {
    return todos.filter((t) => (t.category_id ?? null) === catId && t.done);
  }

  // "지금 이거 하세요" — 가장 우선순위 높은 미완료 할 일
  $: urgent = todos.filter((t) => !t.done).sort((a, b) => {
    const pr = (p: Priority) => p === "high" ? 0 : p === "medium" ? 1 : 2;
    return pr(a.priority) - pr(b.priority);
  }).slice(0, 3);

  $: doneCount = todos.filter((t) => t.done).length;
  $: totalCount = todos.length;
  $: pendingCount = totalCount - doneCount;
  $: progressPct = totalCount > 0 ? Math.round(doneCount * 100 / totalCount) : 0;

  let pollTimer: ReturnType<typeof setInterval>;
  onMount(() => {
    refresh();
    pollTimer = setInterval(refresh, 5000);
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", () => { if (!document.hidden) refresh(); });
  });
</script>

<main class:collapsed
  style="--bg:{meta.bg}; --bg-soft:{meta.bgSoft}; --border:{meta.border}; --text:{meta.text};">
  <header on:mousedown={titleMousedown}>
    <span class="title" on:mousedown={titleMousedown} on:click={clearNewCount}>{meta.emoji} {meta.title}</span>
    {#if newCount > 0}<span class="badge-new" on:click={clearNewCount}>🔴 {newCount}</span>{/if}
    <div class="window-controls" on:mousedown|stopPropagation>
      <button class="win-btn" on:click={toggleOnTop} title={onTop ? "항상 위 해제" : "항상 위"} class:active={onTop}>📌</button>
      <button class="win-btn" on:click={toggleCollapse} title="접기">{collapsed ? "▾" : "▴"}</button>
      <button class="win-btn" on:click={minimize} title="최소화">⚊</button>
      <button class="win-btn close-btn" on:click={close} title="닫기">✕</button>
    </div>
  </header>

  {#if !collapsed}
    {#if errorMsg}<p class="error">{errorMsg}</p>{/if}

    <!-- 진행률 바 -->
    <div class="progress-bar-wrap" title={`${doneCount}개 완료 / ${pendingCount}개 남음`}>
      <div class="progress-bar" style="width: {progressPct}%"></div>
      <span class="progress-text">{doneCount}/{totalCount} ({progressPct}%)</span>
    </div>

    <!-- "지금 이거 하세요" -->
    {#if urgent.length > 0}
      <div class="urgent-section">
        <div class="urgent-label">👉 지금 이거 하세요</div>
        {#each urgent as t (t.id)}
          <div class="urgent-item" class:flash={flashIds.has(t.id)}>
            <span class="prio-tag">{pLabel(t.priority)}</span>
            <span class="urgent-title">{t.title}</span>
            {#if t.note}<span class="urgent-note">{t.note}</span>{/if}
          </div>
        {/each}
      </div>
    {/if}

    <!-- 완료 항목 토글 -->
    <div class="done-toggle" on:click={() => (hideDone = !hideDone)} on:mousedown|stopPropagation>
      {hideDone ? "👁️ 완료 항목 보기" : "🙈 완료 항목 숨기기"}
      {#if doneCount > 0}({doneCount}개 완료){/if}
    </div>

    <div class="groups">
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
              <span class="cat-name" on:click={() => toggleCatCollapse(cat.id)} on:dblclick={() => startRename(cat)}>
                {collapsedCats.has(cat.id) ? "▸" : "▾"} 📂 {cat.name}
                <span class="cat-count">{visibleTodos(cat.id).length}</span>
              </span>
              <div class="cat-controls" on:mousedown|stopPropagation>
                <button class="cat-btn" on:click={() => startRename(cat)} title="이름 변경">✏️</button>
                <button class="cat-btn" on:click={() => moveCategory(idx, -1)} title="위로" disabled={idx === 0}>▲</button>
                <button class="cat-btn" on:click={() => moveCategory(idx, 1)} title="아래로" disabled={idx === categories.length - 1}>▼</button>
                <button class="cat-btn danger" on:click={() => deleteCategory(cat)} title="삭제">🗑️</button>
              </div>
            {/if}
          </div>
          {#if !collapsedCats.has(cat.id)}
          <ul>
            {#each visibleTodos(cat.id) as t (t.id)}
              <li class:done={t.done} class:flash={flashIds.has(t.id)}>
                <label>
                  <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
                  <span class="prio-dot prio-{t.priority}" title={pLabel(t.priority)}></span>
                  <span class="content">
                    <span class="ttl">{t.title}</span>
                    {#if t.note}<span class="note">{t.note.startsWith("✓") ? "✅" : "📝"} {t.note}</span>{/if}
                  </span>
                  <button class="del" on:click={() => remove(t.id)}>×</button>
                </label>
              </li>
            {:else}
              {#if doneTodos(cat.id).length > 0}
                <li class="empty-cat">전부 완료! 🎉 ({doneTodos(cat.id).length}개)</li>
              {:else}
                <li class="empty-cat">할 일 없음</li>
              {/if}
            {/each}
          </ul>
          {/if}
        </section>
      {/each}

      {#if visibleTodos(null).length > 0}
        <section class="cat-section">
          <div class="cat-header"><span class="cat-name">📦 미분류</span></div>
          <ul>
            {#each visibleTodos(null) as t (t.id)}
              <li class:done={t.done} class:flash={flashIds.has(t.id)}>
                <label>
                  <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
                  <span class="prio-dot prio-{t.priority}"></span>
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

    {#if showAdd}
      <div class="addform">
        <input placeholder="무엇을 해야 하나요?" bind:value={newTitle} on:keydown={(e) => e.key === "Enter" && add()} />
        <input placeholder="메모 (선택)" bind:value={newNote} />
        <div class="row">
          <label>폴더:</label>
          <select bind:value={newCategoryId}>
            <option value="">📦 미분류</option>
            {#each categories as cat (cat.id)}<option value={cat.id}>📂 {cat.name}</option>{/each}
          </select>
        </div>
        <div class="row">
          <label>중요도:</label>
          <select bind:value={newPriority}>
            <option value="high">🔴 긴급 (지금 해야 함)</option>
            <option value="medium">🟡 보통 (오늘 중)</option>
            <option value="low">🟢 여유 (시간 날 때)</option>
          </select>
        </div>
        <div class="addbtns">
          <button class="primary" on:click={add} disabled={!newTitle.trim()}>추가</button>
          <button on:click={() => (showAdd = false)}>취소</button>
        </div>
      </div>
    {/if}

    {#if showAddCat}
      <div class="addform">
        <input placeholder="폴더 이름" bind:value={newCatName} on:keydown={(e) => e.key === "Enter" && addCategory()} />
        <div class="addbtns">
          <button class="primary" on:click={addCategory} disabled={!newCatName.trim()}>폴더 추가</button>
          <button on:click={() => (showAddCat = false)}>취소</button>
        </div>
      </div>
    {/if}
  {/if}

  <footer on:mousedown|stopPropagation>
    {#if !collapsed}
      <div class="action-row">
        <button class="add-btn" on:click={() => (showAdd = !showAdd)}>+ 할 일</button>
        <button class="add-btn secondary" on:click={() => (showAddCat = !showAddCat)}>📂 폴더</button>
      </div>
    {/if}
    <span class="status">🔄 {synced} · 남음 {pendingCount}개</span>
  </footer>
</main>

<style>
  :global(body) { margin: 0; background: transparent; overflow: hidden; }
  main {
    background: var(--bg);
    border-radius: 6px;
    box-shadow: 0 8px 28px rgba(0,0,0,0.4);
    padding: 8px 10px;
    width: 100%; height: 100%;
    display: flex; flex-direction: column;
    -webkit-user-select: none; overflow: hidden; box-sizing: border-box;
    color: var(--text);
  }
  header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 4px; gap: 4px; cursor: move;
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

  /* 진행률 바 */
  .progress-bar-wrap {
    position: relative; height: 18px; background: rgba(0,0,0,0.1);
    border-radius: 9px; margin-bottom: 6px; overflow: hidden;
  }
  .progress-bar {
    height: 100%; background: var(--border); border-radius: 9px;
    transition: width 0.5s ease; min-width: 2px;
  }
  .progress-text {
    position: absolute; top: 0; left: 0; right: 0; text-align: center;
    font-size: 10px; font-weight: bold; line-height: 18px; color: var(--text);
  }

  /* "지금 이거 하세요" */
  .urgent-section {
    background: rgba(255,255,255,0.4); border-radius: 6px; padding: 6px;
    margin-bottom: 6px; border: 1px solid var(--border);
  }
  .urgent-label { font-size: 11px; font-weight: bold; margin-bottom: 4px; }
  .urgent-item {
    display: flex; align-items: flex-start; gap: 4px; padding: 3px 0;
    font-size: 12px; flex-wrap: wrap;
  }
  .urgent-title { font-weight: 600; flex: 1; min-width: 0; }
  .urgent-note { font-size: 10px; opacity: 0.7; width: 100%; padding-left: 20px; }
  .prio-tag { font-size: 9px; white-space: nowrap; }

  .done-toggle {
    font-size: 10px; text-align: center; padding: 3px; cursor: pointer;
    opacity: 0.6; border-radius: 4px; margin-bottom: 4px;
  }
  .done-toggle:hover { opacity: 1; background: rgba(0,0,0,0.05); }

  .groups { flex: 1; overflow-y: auto; }
  .cat-section { margin-bottom: 4px; }
  .cat-header {
    display: flex; align-items: center; justify-content: space-between;
    font-size: 12px; font-weight: 600;
    border-bottom: 1px solid var(--border); padding-bottom: 2px; margin: 4px 0 2px;
  }
  .cat-name { cursor: pointer; flex: 1; }
  .cat-count {
    font-size: 9px; font-weight: 400; opacity: 0.5;
    background: rgba(0,0,0,0.1); border-radius: 6px; padding: 0 5px; margin-left: 4px;
  }
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
  li.done .ttl { text-decoration: line-through; opacity: 0.4; }
  li.empty-cat { text-align: center; opacity: 0.4; font-size: 11px; padding: 4px 0; }
  label { display: flex; align-items: flex-start; gap: 5px; }
  .prio-dot {
    width: 8px; height: 8px; border-radius: 50%; margin-top: 4px; flex-shrink: 0;
  }
  .prio-high { background: #e81123; }
  .prio-medium { background: #f0a020; }
  .prio-low { background: #40b060; }
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
  .row label { font-size: 11px; min-width: 40px; }
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
  .badge-new {
    background: #e81123; color: white; font-size: 10px; font-weight: bold;
    padding: 1px 6px; border-radius: 8px; cursor: pointer; line-height: 1.5;
  }
  li.flash { animation: flashPulse 1.5s ease-out; }
  @keyframes flashPulse {
    0% { background: rgba(255, 215, 0, 0.8); transform: scale(1.02); }
    100% { background: transparent; transform: scale(1); }
  }
  .error { color: #c00; font-size: 11px; margin: 4px 0; }
</style>
