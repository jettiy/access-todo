<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { devnoteApi, type DevNote, type NoteType, type GameSystem } from "./api";

  export let existing: DevNote | null = null;

  const dispatch = createEventDispatcher();

  let noteType: NoteType = existing?.note_type || "retrospect";
  let title = existing?.title || "";
  let body = existing?.body || "";
  let gameSystem: GameSystem | "" = existing?.game_system || "";
  let version = existing?.version || "";
  let saving = false;

  const typeOptions: { value: NoteType; label: string; emoji: string }[] = [
    { value: "retrospect", label: "회고", emoji: "💭" },
    { value: "issue", label: "이슈", emoji: "🐛" },
    { value: "playtest", label: "플레이테스트", emoji: "🎮" },
  ];

  const systemOptions: { value: GameSystem; label: string }[] = [
    { value: "combat", label: "⚔️ 전투" },
    { value: "inventory", label: "🎒 인벤토리" },
    { value: "quest", label: "📜 퀘스트" },
    { value: "skill", label: "✨ 스킬" },
    { value: "ui", label: "🖥️ UI" },
    { value: "audio", label: "🔊 오디오" },
    { value: "map", label: "🗺️ 맵" },
    { value: "ai", label: "🤖 AI" },
    { value: "save", label: "💾 세이브" },
    { value: "network", label: "🌐 네트워크" },
    { value: "etc", label: "📦 기타" },
  ];

  async function save() {
    if (!title.trim() || !body.trim()) return;
    saving = true;
    try {
      const data: { note_type: NoteType; title: string; body: string; game_system?: GameSystem; version?: string } = {
        note_type: noteType,
        title: title.trim(),
        body: body.trim(),
      };
      if (gameSystem) data.game_system = gameSystem;
      if (version.trim()) data.version = version.trim();

      if (existing) {
        await devnoteApi.update(existing.id, data);
      } else {
        await devnoteApi.create(data);
      }
      dispatch("saved");
    } catch (e) {
      alert(`저장 실패: ${(e as Error).message}`);
    } finally {
      saving = false;
    }
  }

  function close() {
    dispatch("cancel");
  }
</script>

<div class="overlay" on:click={close}>
  <div class="modal" on:click|stopPropagation>
    <h2>{existing ? "노트 수정" : "새 노트"}</h2>

    <label>타입</label>
    <div class="type-row">
      {#each typeOptions as opt}
        <button
          class="type-btn"
          class:active={noteType === opt.value}
          on:click={() => (noteType = opt.value)}
        >
          {opt.emoji} {opt.label}
        </button>
      {/each}
    </div>

    <label>제목</label>
    <input type="text" bind:value={title} placeholder="노트 제목" />

    <label>본문 (마크다운)</label>
    <textarea bind:value={body} rows="8" placeholder="무엇을 했나요? 무엇을 배웠나요?"></textarea>

    <div class="row">
      <div>
        <label>게임 시스템</label>
        <select bind:value={gameSystem}>
          <option value="">선택 안함</option>
          {#each systemOptions as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>
      <div>
        <label>버전</label>
        <input type="text" bind:value={version} placeholder="예: v0.2-alpha" />
      </div>
    </div>

    <div class="actions">
      <button class="btn-cancel" on:click={close}>취소</button>
      <button class="btn-save" on:click={save} disabled={saving || !title.trim() || !body.trim()}>
        {saving ? "저장중..." : "저장"}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.4); display: flex; align-items: center;
    justify-content: center; z-index: 100;
  }
  .modal {
    background: white; border-radius: 12px; padding: 20px;
    width: 90%; max-width: 500px; max-height: 85vh; overflow-y: auto;
  }
  h2 { margin: 0 0 16px; font-size: 18px; }
  label { display: block; font-size: 12px; font-weight: 600; color: #666; margin: 10px 0 4px; }
  input,
  textarea,
  select {
    width: 100%; padding: 8px; border: 1px solid #ddd; border-radius: 6px;
    font-size: 13px; box-sizing: border-box;
  }
  textarea { font-family: monospace; resize: vertical; }
  .type-row { display: flex; gap: 6px; margin-bottom: 4px; }
  .type-btn {
    flex: 1; padding: 8px; border: 2px solid #ddd; border-radius: 8px;
    background: white; cursor: pointer; font-size: 12px;
  }
  .type-btn.active { border-color: #3ca87e; background: #dcf5ec; }
  .row { display: flex; gap: 12px; }
  .row > div { flex: 1; }
  .actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 16px; }
  .btn-cancel { padding: 8px 16px; border: none; background: #eee; border-radius: 6px; cursor: pointer; }
  .btn-save {
    padding: 8px 16px; border: none; background: #3ca87e; color: white;
    border-radius: 6px; cursor: pointer; font-weight: 600;
  }
  .btn-save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
