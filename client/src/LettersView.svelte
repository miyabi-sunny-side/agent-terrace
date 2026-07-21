<script>
  import { onMount, tick } from "svelte";

  import { fetchLetters, fetchSkills, sendLetter } from "./api.js";
  import {
    bodyBytes,
    eventsForPane,
    latestEventId,
    MAX_BODY_BYTES,
    mergeEvents,
  } from "./letters.js";

  const LETTER_INTERVAL = 2000;

  let { agent } = $props();
  let events = $state([]);
  let skills = $state([]);
  let body = $state("");
  let selectedSkill = $state(null);
  let skillsOpen = $state(false);
  let loading = $state(true);
  let refreshing = $state(false);
  let refreshQueued = false;
  let sending = $state(false);
  let error = $state("");
  let sendError = $state("");
  let skillError = $state("");
  let sentNotice = $state("");

  let visibleEvents = $derived(eventsForPane(events, agent.pane_id));
  let byteCount = $derived(bodyBytes(body));
  let canSend = $derived(body.trim().length > 0 && byteCount <= MAX_BODY_BYTES && !sending);

  function errorMessage(reason, fallback) {
    if (reason?.code === "letter_history_unavailable") return "手紙箱に接続できません。";
    if (reason?.code === "letter_delivery_failed") return "手紙を届けられませんでした。本文は残してあります。";
    if (reason?.code === "unknown_agent") return "この agent は退出しました。";
    return fallback;
  }

  async function refreshLetters(force = false) {
    if (refreshing) {
      if (force) refreshQueued = true;
      return;
    }
    refreshing = true;
    try {
      const after = latestEventId(events);
      const result = await fetchLetters({ after, limit: 500 });
      events = mergeEvents(events, result.events);
      error = "";
    } catch (reason) {
      error = errorMessage(reason, "手紙箱を更新できませんでした。");
    } finally {
      refreshing = false;
      loading = false;
      if (refreshQueued) {
        refreshQueued = false;
        void refreshLetters();
      }
    }
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
      await refreshLetters(true);
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

  async function handleSkillMenuKey(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      skillsOpen = false;
      await tick();
      document.getElementById("skill-trigger")?.focus();
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const items = [...event.currentTarget.querySelectorAll('[role="menuitemradio"]')];
    const current = items.indexOf(document.activeElement);
    let next = event.key === "ArrowUp" ? current - 1 : current + 1;
    if (event.key === "Home") next = 0;
    if (event.key === "End") next = items.length - 1;
    items[(next + items.length) % items.length]?.focus();
  }

  function formatTime(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return new Intl.DateTimeFormat("ja-JP", {
      month: "numeric",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date);
  }

  onMount(() => {
    fetchSkills()
      .then((values) => {
        skills = values;
        skillError = "";
      })
      .catch(() => {
        skills = [];
        skillError = "skill 一覧を取得できませんでした。";
      });
    refreshLetters();
    const timer = window.setInterval(refreshLetters, LETTER_INTERVAL);
    return () => window.clearInterval(timer);
  });
</script>

<div class="letters-view">
  <div class="letters-scroll" aria-live="polite" aria-busy={loading}>
    {#if error}
      <div class="letter-error" role="alert">
        <span>{error}</span>
        <button type="button" onclick={refreshLetters}>再試行</button>
      </div>
    {/if}

    {#if loading}
      <div class="letters-empty"><span class="loader"></span>手紙箱を確認中</div>
    {:else if visibleEvents.length === 0}
      <div class="letters-empty">
        <span class="empty-rule" aria-hidden="true"></span>
        <strong>まだ手紙はありません。</strong>
        <span>{agent.name} への最初の指示を、下の欄から届けられます。</span>
      </div>
    {:else}
      <ol class="letter-list">
        {#each visibleEvents as letter (letter.id)}
          <li class:incoming={letter.direction === "in"} class="letter-card">
            <div class="letter-meta">
              <span class="letter-direction">
                {letter.direction === "out" ? "YOU →" : "← REPLY"} {letter.target_name}
              </span>
              <time datetime={letter.created_at}>{formatTime(letter.created_at)}</time>
            </div>
            {#if letter.skill}<span class="skill-chip">{letter.skill}</span>{/if}
            <p>{letter.body}</p>
            <span class="letter-id">#{letter.id}{letter.reply_to !== null ? ` · reply to #${letter.reply_to}` : ""}</span>
          </li>
        {/each}
      </ol>
    {/if}
  </div>

  <form class="composer" onsubmit={submitLetter}>
    {#if sendError}<div class="composer-message error" role="alert">{sendError}</div>{/if}
    {#if skillError}<div class="composer-message error" role="status">{skillError}</div>{/if}
    {#if sentNotice}<div class="composer-message success" role="status">{sentNotice}</div>{/if}
    <label for="letter-body">手紙</label>
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
