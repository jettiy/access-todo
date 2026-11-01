<script lang="ts">
  // Access 설정창 (P1) — 첫 실행 시 자동 오픈되는 온보딩 겸 상시 제어판.
  // 다크모드 카드 + 토글 스위치(사용자 제공 레퍼런스 이미지 스타일).
  // 토글 ON → POST /agents/:id/register (MCP 자동 등록),
  // OFF → unregister (MCP 등록을 백업 후 제거 — 연동도 끊김).

  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";
  import { agentsApi, settingsApi, type AgentRow, type SettingsInfo } from "./api";

  const LEVELS: { id: string; label: string; hint: string }[] = [
    { id: "beginner", label: "초보", hint: "쉬운 비유 위주 · 비전공자" },
    { id: "intermediate", label: "중급", hint: "키워드 + 가벼운 비유" },
    { id: "advanced", label: "고급", hint: "에이전트가 자율 판단" },
    { id: "auto", label: "자동", hint: "대화 흔적으로 추정 (기본)" },
  ];

  const EMOJI: Record<string, string> = {
    zcode: "⚡", "claude-code": "🤖", codex: "🧠", cline: "📟",
    "kilo-code": "⌨️", openclaw: "🦞", hermes: "📨", omp: "🛠️", dsh: "🐋",
  };

  let settings: SettingsInfo | null = null;
  let agents: AgentRow[] = [];
  let busy = "";            // 처리 중인 agent id (토글 비활성화용)
  let notice = "";
  let scannedAt = "";
  let customName = "";

  $: enabledCount = agents.filter((a) => a.enabled).length;

  function statusText(a: AgentRow): string {
    if (a.integration === "rest") return "REST 연동 · 활성";
    if (!a.installed) return "미설치";
    if (a.registered && a.enabled) return "설치됨 · 연결됨";
    if (a.registered) return "등록됨 · 비활성";
    if (a.enabled) return "설치됨 · 등록 대기";
    return "설치됨 · 미연결";
  }

  async function reload() {
    try {
      const [s, a] = await Promise.all([settingsApi.get(), agentsApi.list()]);
      settings = s;
      agents = a.agents;
      scannedAt = new Date().toLocaleTimeString("ko-KR");
      notice = "";
    } catch (e) {
      notice = `서버 연결 실패: ${(e as Error).message}`;
    }
  }

  async function setLevel(level: string) {
    if (!settings || settings.pilot_level === level) return;
    const r = await settingsApi.put({ pilot_level: level });
    settings = { ...settings, ...r.settings };
    notice = `지식상태: ${LEVELS.find((l) => l.id === level)?.label} — 다음 요약부터 적용됩니다`;
  }

  async function setSync(mode: string) {
    if (!settings || settings.sync_mode === mode) return;
    const r = await settingsApi.put({ sync_mode: mode });
    settings = { ...settings, ...r.settings };
  }

  // 토글 후 포스트잇 창을 즉시 표시/숨김 (활성한 에이전트만 포스트잇 — v1.2)
  function syncWindows() {
    const enabled = agents.filter((a) => a.enabled).map((a) => a.id);
    invoke("sync_agent_windows", { enabled }).catch(() => {});
  }

  async function toggleAgent(a: AgentRow) {
    if (busy) return;
    busy = a.id;
    try {
      const r = a.enabled ? await agentsApi.unregister(a.id) : await agentsApi.register(a.id);
      if (r.outcome === "error") {
        notice = `${a.display}: 실패 — ${r.error ?? "알 수 없는 오류"} (설정 파일이 바뀐 것 같으면 직접 확인 필요)`;
      } else if (r.outcome === "registered") {
        notice = `${a.display} MCP 등록 완료 — 실행 중인 도구는 재시작하면 연결됩니다`;
      } else if (r.outcome === "unregistered") {
        notice = `${a.display} 연동 해제 (백업 보관)`;
      }
      await reload();
      syncWindows();
    } catch (e) {
      notice = `${a.display}: ${(e as Error).message}`;
    } finally {
      busy = "";
    }
  }

  async function addCustom() {
    const id = customName.trim().toLowerCase().replace(/\s+/g, "-");
    if (!id) return;
    await agentsApi.register(id).catch((e) => (notice = String(e)));
    customName = "";
    await reload();
  }

  function close() { getCurrentWindow().close(); }

  onMount(reload);
</script>

<main>
  <header>
    <span class="title">⚙ Access 설정</span>
    <span class="header-actions">
      <span class="count">{enabledCount}개 활성</span>
      <button class="ghost" on:click={reload} title={scannedAt ? `마지막 스캔 ${scannedAt}` : ""}>다시 스캔</button>
      <button class="ghost" on:click={close}>✕</button>
    </span>
  </header>

  {#if notice}<div class="notice">{notice}</div>{/if}

  {#if settings}
    <section class="panel">
      <h2>파일럿 지식상태 <span class="sub">에이전트 요약의 난이도</span></h2>
      <div class="segments">
        {#each LEVELS as l}
          <button
            class="segment"
            class:active={settings.pilot_level === l.id}
            title={l.hint}
            on:click={() => setLevel(l.id)}
          >{l.label}</button>
        {/each}
      </div>
      <p class="hint">{LEVELS.find((l) => l.id === settings?.pilot_level)?.hint ?? ""} · 요약은 최대 3줄</p>
    </section>

    <section class="panel">
      <h2>동기화</h2>
      <div class="segments">
        <button class="segment" class:active={settings.sync_mode === "gist"} on:click={() => setSync("gist")}>GitHub Gist</button>
        <button class="segment" class:active={settings.sync_mode === "local"} on:click={() => setSync("local")}>로컬 전용</button>
      </div>
      <p class="hint">
        현재 서버: {settings.gist_sync ? "Gist 동기화 활성" : "로컬 전용 모드(토큰 불필요)"}
        {#if !settings.gist_sync && settings.sync_mode === "gist"} — GITHUB_TOKEN/TODO_GIST_ID 설정 후 서버 재시작 시 활성화{/if}
      </p>
    </section>

    <section class="panel grow">
      <h2>에이전트 <span class="sub">활성한 에이전트만 포스트잇·연동됩니다</span></h2>
      <div class="cards">
        {#each agents as a (a.id)}
          <div class="card" class:off={!a.enabled}>
            <div class="card-icon">{EMOJI[a.id] ?? "🔌"}</div>
            <div class="card-body">
              <div class="card-name">
                {a.display}
                {#if a.experimental}<span class="tag" title="설정 경로가 바뀔 수 있음">실험</span>{/if}
                {#if a.integration === "rest"}<span class="tag">REST</span>{/if}
              </div>
              <div class="card-status" class:ok={a.enabled}>{statusText(a)}</div>
            </div>
            <label class="switch" class:disabled={busy === a.id}>
              <input type="checkbox" checked={a.enabled} on:change={() => toggleAgent(a)} />
              <span class="slider"></span>
            </label>
          </div>
        {/each}
      </div>
      <div class="custom-row">
        <input placeholder="커스텀 에이전트 이름 (예: hermes)" bind:value={customName} on:keydown={(e) => e.key === "Enter" && addCustom()} />
        <button class="ghost" on:click={addCustom}>추가</button>
      </div>
      <p class="hint">OFF 시 MCP 등록을 백업 후 제거합니다. 도구 업데이트로 연결이 풀려도 Access가 자동으로 복구합니다(6시간 주기).</p>
    </section>
  {:else}
    <div class="loading">{notice || "불러오는 중..."}</div>
  {/if}
</main>

<style>
  :global(body) { margin: 0; background: #16161e; }
  main {
    display: flex; flex-direction: column; gap: 12px;
    height: 100vh; box-sizing: border-box; padding: 14px 16px;
    background: #16161e; color: #e6e6ef;
    font-family: "Segoe UI", "Malgun Gothic", sans-serif; font-size: 14px;
    overflow-y: auto;
  }
  header { display: flex; align-items: center; justify-content: space-between; }
  .title { font-size: 18px; font-weight: 700; }
  .header-actions { display: flex; align-items: center; gap: 8px; }
  .count { color: #9a9ab0; font-size: 12px; }

  .notice {
    background: #2a2a3a; border: 1px solid #44445c; border-radius: 8px;
    padding: 8px 10px; font-size: 12.5px; color: #c9c9e8;
  }

  .panel {
    background: #1e1e2a; border: 1px solid #2c2c3e; border-radius: 12px;
    padding: 12px 14px;
  }
  .panel.grow { flex: 1; }
  h2 { margin: 0 0 10px; font-size: 14px; }
  h2 .sub { font-weight: 400; color: #8888a0; font-size: 12px; margin-left: 6px; }
  .hint { color: #7c7c96; font-size: 12px; margin: 8px 0 0; }

  .segments { display: flex; gap: 6px; }
  .segment {
    flex: 1; padding: 8px 0; border: 1px solid #33334a; border-radius: 8px;
    background: #23232f; color: #b6b6cc; cursor: pointer; font-size: 13px;
  }
  .segment:hover { border-color: #4a4a6a; }
  .segment.active { background: #2f4a8a; border-color: #5b7fd4; color: #fff; font-weight: 600; }

  .cards { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .card {
    display: flex; align-items: center; gap: 10px;
    background: #23232f; border: 1px solid #33334a; border-radius: 10px;
    padding: 10px 12px;
  }
  .card.off { opacity: 0.55; }
  .card-icon { font-size: 22px; }
  .card-body { flex: 1; min-width: 0; }
  .card-name { font-weight: 600; display: flex; align-items: center; gap: 6px; }
  .card-status { color: #8f8fa8; font-size: 12px; margin-top: 2px; }
  .card-status.ok { color: #7bd88f; }
  .tag {
    font-size: 10px; color: #a8a8c8; border: 1px solid #3c3c54;
    border-radius: 4px; padding: 0 4px;
  }

  .switch { position: relative; width: 44px; height: 24px; flex-shrink: 0; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider {
    position: absolute; inset: 0; border-radius: 24px; cursor: pointer;
    background: #3a3a4e; transition: 0.15s;
  }
  .slider::before {
    content: ""; position: absolute; width: 18px; height: 18px; border-radius: 50%;
    left: 3px; top: 3px; background: #cfcfe0; transition: 0.15s;
  }
  .switch input:checked + .slider { background: #3f7a4e; }
  .switch input:checked + .slider::before { transform: translateX(20px); background: #fff; }
  .switch.disabled { opacity: 0.5; pointer-events: none; }

  .custom-row { display: flex; gap: 8px; margin-top: 10px; }
  .custom-row input {
    flex: 1; background: #23232f; border: 1px solid #33334a; border-radius: 8px;
    color: #e6e6ef; padding: 8px 10px; font-size: 13px; outline: none;
  }
  .custom-row input:focus { border-color: #5b7fd4; }

  .ghost {
    background: #26263a; color: #c6c6de; border: 1px solid #3a3a52;
    border-radius: 8px; padding: 6px 12px; cursor: pointer; font-size: 12.5px;
  }
  .ghost:hover { background: #2e2e46; }

  .loading { color: #8888a0; text-align: center; padding: 40px 0; }
</style>
