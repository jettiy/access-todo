<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, type Todo, type Priority } from "./api";

  let todos: Todo[] = [];
  // 새 포스트잇 추가 폼
  let showAdd = false;
  let newTitle = "";
  let newNote = "";
  let newPriority: Priority = "medium";
  let newCategory: string = "공통"; // 에이전트 이름 또는 "공통"
  let newAgents: string = ""; // 어떤 에이전트와 할 일인지 자유 입력
  let synced = "로딩중";
  let errorMsg = "";
  let collapsed = false;
  // 색상 테마
  let colorIdx = 0;
  const colors = [
    { name: "노랑", bg: "#fff0a0", bgSoft: "#fff8cc", border: "#b89b00", text: "#5a4a00" },
    { name: "핑크", bg: "#ffd0e8", bgSoft: "#ffe5f2", border: "#c46b9c", text: "#7a2a5a" },
    { name: "민트", bg: "#c8f0e0", bgSoft: "#dcf5ec", border: "#3ca87e", text: "#1a5a44" },
    { name: "블루", bg: "#c8e0f5", bgSoft: "#dceaf5", border: "#3c7ab0", text: "#1a3a5a" },
  ];
  $: theme = colors[colorIdx];

  const appWindow = getCurrentWindow();

  // 헤더를 누르고 있으면 창 드래그 시작 (mousedown → startDragging)
  function startDrag() {
    appWindow.startDragging();
  }
  const pEmoji = (p: string) => (p === "high" ? "🔴" : p === "low" ? "🟢" : "🟡");

  // 에이전트/카테고리별 그룹화
  type Group = { key: string; label: string; emoji: string; todos: Todo[] };
  $: groups = buildGroups(todos);

  function buildGroups(ts: Todo[]): Group[] {
    const map = new Map<string, Todo[]>();
    for (const t of ts) {
      // tags에서 카테고리 추출 (에이전트 이름 또는 "공통"), 없으면 created_by 사용
      const agentTag = t.tags.find((tag) => tag.startsWith("agent:"));
      const cat = agentTag ? agentTag.replace("agent:", "") : t.created_by || "기타";
      if (!map.has(cat)) map.set(cat, []);
      map.get(cat)!.push(t);
    }
    // "공통"을 맨 앞으로, 나머지는 알파벳순
    const result: Group[] = [];
    if (map.has("공통")) {
      result.push({ key: "공통", label: "공통 할 일", emoji: "📌", todos: map.get("공통")! });
      map.delete("공통");
    }
    const agentEmoji: Record<string, string> = { hermes: "🤖", zcode: "⚡", claude: "🧠", user: "👤" };
    for (const [key, ts] of [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
      result.push({ key, label: key, emoji: agentEmoji[key] || "📋", todos: ts });
    }
    return result;
  }

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
    const tags = [`agent:${newCategory}`];
    if (newAgents.trim()) tags.push(newAgents.trim());
    await api.addRaw(newTitle.trim(), newNote.trim() || undefined, newPriority, undefined, tags);
    newTitle = "";
    newNote = "";
    newAgents = "";
    newPriority = "medium";
    newCategory = "공통";
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

  function cycleColor() {
    colorIdx = (colorIdx + 1) % colors.length;
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
    setInterval(refresh, 15000);
  });

  $: doneCount = todos.filter((t) => t.done).length;
</script>

<main class:collapsed={collapsed} style="--bg:{theme.bg}; --bg-soft:{theme.bgSoft}; --border:{theme.border}; --text:{theme.text};">
  <!-- 헤더: mousedown으로 창 드래그, 우측 컨트롤 버튼들 -->
  <header on:mousedown={startDrag}>
    <span class="title">📒 오늘의 할 일</span>
    <div class="window-controls">
      <button class="win-btn" on:click={cycleColor} title="색상 변경">🎨</button>
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

    <!-- 에이전트/카테고리별 그룹 -->
    <div class="groups">
      {#each groups as g (g.key)}
        <section class="group">
          <h3 class="group-title">
            {g.emoji} {g.label}
            <span class="count">{g.todos.filter((t) => !t.done).length}/{g.todos.length}</span>
          </h3>
          <ul>
            {#each g.todos as t (t.id)}
              <li class:done={t.done}>
                <label>
                  <input type="checkbox" checked={t.done} on:change={() => toggle(t.id)} />
                  <span class="prio">{pEmoji(t.priority)}</span>
                  <span class="content">
                    <span class="ttl">{t.title}</span>
                    {#if t.note}<span class="note">📝 {t.note}</span>{/if}
                    {#if t.completed_by && t.completed_by !== "user"}
                      <span class="badge">✓ {t.completed_by}</span>
                    {/if}
                  </span>
                  <button class="del" on:click={() => remove(t.id)} title="삭제">×</button>
                </label>
              </li>
            {/each}
          </ul>
        </section>
      {:else}
        <p class="empty">할 일이 없어요. 아래 + 버튼을 눌러 추가하세요.</p>
      {/each}
    </div>

    <!-- 새 포스트잇 추가 폼 -->
    {#if showAdd}
      <div class="addform">
        <input placeholder="할 일 제목" bind:value={newTitle} on:keydown={(e) => e.key === "Enter" && add()} />
        <input placeholder="메모 (선택)" bind:value={newNote} />
        <div class="row">
          <label>분류:</label>
          <select bind:value={newCategory}>
            <option value="공통">📌 공통</option>
            <option value="hermes">🤖 hermes</option>
            <option value="zcode">⚡ zcode</option>
            <option value="claude">🧠 claude</option>
            <option value="user">👤 user</option>
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
        <input placeholder="추가 태그 (선택, 예: 긴급)" bind:value={newAgents} />
        <div class="addbtns">
          <button class="primary" on:click={add} disabled={!newTitle.trim()}>추가</button>
          <button on:click={() => (showAdd = false)}>취소</button>
        </div>
      </div>
    {/if}
  {/if}

  <footer>
    {#if !collapsed}
      <button class="add-btn" on:click={() => (showAdd = !showAdd)}>+ 포스트잇 추가</button>
    {/if}
    <span class="status">🔄 {synced} · {doneCount}/{todos.length} 완료 · {theme.name}</span>
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
    width: 24px; height: 24px; cursor: pointer; font-size: 13px; line-height: 1;
    color: var(--text); padding: 0; display: flex; align-items: center; justify-content: center;
  }
  .win-btn:hover { background: rgba(0,0,0,0.2); }
  .close-btn:hover { background: #e81123; color: white; }
  .groups { flex: 1; overflow-y: auto; }
  .group { margin-bottom: 8px; }
  .group-title {
    font-size: 12px; font-weight: 600; margin: 6px 0 3px;
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid var(--border); padding-bottom: 2px;
  }
  .count { font-size: 10px; opacity: 0.7; font-weight: 400; }
  ul { list-style: none; padding: 0; margin: 0; }
  li { padding: 4px 0; font-size: 13px; border-bottom: 1px dashed rgba(0,0,0,0.08); }
  li.done .ttl { text-decoration: line-through; opacity: 0.5; }
  label { display: flex; align-items: flex-start; gap: 5px; }
  .prio { line-height: 1.4; }
  .content { flex: 1; display: flex; flex-direction: column; }
  .ttl { line-height: 1.3; word-break: break-word; }
  .note { font-size: 11px; opacity: 0.7; margin-top: 1px; }
  .badge { font-size: 9px; background: rgba(255,255,255,0.5); padding: 0 4px; border-radius: 3px; width: fit-content; margin-top: 2px; }
  .del { background: none; border: none; color: var(--text); opacity: 0.4; cursor: pointer; font-size: 15px; padding: 0 2px; }
  .del:hover { opacity: 1; color: #c00; }
  .empty { text-align: center; opacity: 0.6; font-size: 13px; padding: 30px 0; }
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
