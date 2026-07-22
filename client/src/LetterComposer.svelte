<script>
  import { onMount, tick } from "svelte";

  import { fetchSkills, sendLetter } from "./api.js";
  import { bodyBytes, MAX_BODY_BYTES } from "./letters.js";

  let { agent, onSent = () => {} } = $props();
  let expanded = $state(false);
  let skills = $state([]);
  let body = $state("");
  let selectedSkill = $state(null);
  let skillsOpen = $state(false);
  let sending = $state(false);
  let sendError = $state("");
  let skillError = $state("");
  let sentNotice = $state("");

  let byteCount = $derived(bodyBytes(body));
  let canSend = $derived(body.trim().length > 0 && byteCount <= MAX_BODY_BYTES && !sending);

  function errorMessage(reason, fallback) {
    if (reason?.code === "letter_delivery_failed") return "手紙を届けられませんでした。本文は残してあります。";
    if (reason?.code === "unknown_agent") return "この agent は退出しました。";
    return fallback;
  }

  async function openComposer() {
    expanded = true;
    await tick();
    document.getElementById("letter-body")?.focus();
  }

  async function closeComposer() {
    document.getElementById("letter-launcher")?.focus();
    expanded = false;
    skillsOpen = false;
  }

  function toggleComposer() {
    if (expanded) void closeComposer();
    else void openComposer();
  }

  async function submitLetter(event) {
    event.preventDefault();
    if (!canSend) return;
    sending = true;
    sendError = "";
    sentNotice = "";
    try {
      const result = await sendLetter({
        agent: agent.pane_id,
        skill: selectedSkill,
        body,
      });
      body = "";
      selectedSkill = null;
      skillsOpen = false;
      sentNotice = result.status === "queued" ? `手紙 #${result.id} を預けました。` : `手紙 #${result.id} を届けました。`;
      onSent(result);
    } catch (reason) {
      sendError = errorMessage(reason, "手紙を届けられませんでした。本文は残してあります。");
    } finally {
      sending = false;
    }
  }

  async function chooseSkill(skill) {
    selectedSkill = skill;
    skillsOpen = false;
    await tick();
    document.getElementById("skill-trigger")?.focus();
  }

  async function toggleSkills() {
    skillsOpen = !skillsOpen;
    if (skillsOpen) {
      await tick();
      document.getElementById("skill-none")?.focus();
    }
  }

  async function handleKeydown(event) {
    if (event.key === "Escape" && expanded) await closeComposer();
  }

  async function handleSkillMenuKey(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      skillsOpen = false;
      await tick();
      document.getElementById("skill-trigger")?.focus();
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const items = [...event.currentTarget.querySelectorAll('[role="menuitemradio"]')];
    const current = items.indexOf(document.activeElement);
    let next = event.key === "ArrowUp" ? current - 1 : current + 1;
    if (event.key === "Home") next = 0;
    if (event.key === "End") next = items.length - 1;
    items[(next + items.length) % items.length]?.focus();
  }

  onMount(() => {
    fetchSkills(agent.pane_id)
      .then((values) => {
        skills = values;
        skillError = "";
      })
      .catch(() => {
        skills = [];
        skillError = "skill 一覧を取得できませんでした。";
      });
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<footer class:expanded class="letter-dock">
  <div class="letter-dock-tab">
    <button
      id="letter-launcher"
      class="letter-launcher"
      type="button"
      aria-expanded={expanded}
      aria-controls="letter-composer-panel"
      onclick={toggleComposer}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16v12H4z" /><path d="m4 7 8 6 8-6" /></svg>
      手紙
      <svg class="dock-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m7 15 5-5 5 5" /></svg>
    </button>
  </div>

  <div id="letter-composer-panel" class="letter-dock-panel" role="region" aria-label="手紙を書く" aria-hidden={!expanded} inert={!expanded}>
    <form class="composer" onsubmit={submitLetter}>
      <div class="composer-heading">
        <div>
          <span class="section-index">LETTER COMPOSER</span>
          <strong>{agent.name}へ届ける</strong>
        </div>
        <button class="composer-close" type="button" onclick={closeComposer} aria-label="手紙を閉じる">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
        </button>
      </div>
      {#if sendError}<div class="composer-message error" role="alert">{sendError}</div>{/if}
      {#if skillError}<div class="composer-message error" role="status">{skillError}</div>{/if}
      {#if sentNotice}<div class="composer-message success" role="status">{sentNotice}</div>{/if}
      <label for="letter-body">本文</label>
      <textarea
        id="letter-body"
        bind:value={body}
        rows="3"
        placeholder={`${agent.name} に作業指示を送る`}
        aria-describedby="letter-count"
        aria-invalid={byteCount > MAX_BODY_BYTES}
        disabled={sending}
      ></textarea>
      <div class="composer-actions">
        <button
          type="button"
          id="skill-trigger"
          class="skill-button"
          class:active={selectedSkill}
          aria-haspopup="menu"
          aria-controls="skill-menu"
          aria-expanded={skillsOpen}
          onclick={toggleSkills}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4" /></svg>
          {selectedSkill ?? "Skill"}
        </button>
        <span id="letter-count" class:over-limit={byteCount > MAX_BODY_BYTES} class="body-count">
          {byteCount.toLocaleString()} / {MAX_BODY_BYTES.toLocaleString()} bytes
        </span>
        <button type="submit" class="send-button" disabled={!canSend}>
          {sending ? "送信中" : "届ける"}
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 14-7-4 14-3-6-7-1Z" /><path d="m12 13 7-8" /></svg>
        </button>
      </div>
      {#if skillsOpen}
        <div id="skill-menu" class="skill-menu" role="menu" aria-label="skillを選択" tabindex="-1" onkeydown={handleSkillMenuKey}>
          <span aria-hidden="true">SKILL</span>
          <button
            type="button"
            id="skill-none"
            role="menuitemradio"
            aria-checked={selectedSkill === null}
            class:selected={selectedSkill === null}
            onclick={() => chooseSkill(null)}>なし</button
          >
          {#each skills as skill}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={selectedSkill === skill}
              class:selected={selectedSkill === skill}
              onclick={() => chooseSkill(skill)}>{skill}</button
            >
          {/each}
        </div>
      {/if}
    </form>
  </div>
</footer>
