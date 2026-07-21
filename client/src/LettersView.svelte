<script>
  import { onMount, untrack } from "svelte";

  import { fetchLetters } from "./api.js";
  import { createRefreshQueue, eventsForPane, latestEventId, mergeEvents } from "./letters.js";

  const LETTER_INTERVAL = 2000;

  let { agent, refreshToken = 0 } = $props();
  let events = $state([]);
  let loading = $state(true);
  let error = $state("");
  let seenRefreshToken = $state(untrack(() => refreshToken));

  let visibleEvents = $derived(eventsForPane(events, agent.pane_id));

  function errorMessage(reason, fallback) {
    if (reason?.code === "letter_history_unavailable") return "手紙箱に接続できません。";
    return fallback;
  }

  const refreshLetters = createRefreshQueue(async () => {
    try {
      const after = latestEventId(events);
      const result = await fetchLetters({ after, limit: 500 });
      events = mergeEvents(events, result.events);
      error = "";
    } catch (reason) {
      error = errorMessage(reason, "手紙箱を更新できませんでした。");
    } finally {
      loading = false;
    }
  });

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
    refreshLetters();
    const timer = window.setInterval(refreshLetters, LETTER_INTERVAL);
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    if (refreshToken === seenRefreshToken) return;
    seenRefreshToken = refreshToken;
    void refreshLetters(true);
  });
</script>

<div class="letters-view">
  <div class="letters-scroll" aria-live="polite" aria-busy={loading}>
    {#if error}
      <div class="letter-error" role="alert">
        <span>{error}</span>
        <button type="button" onclick={() => refreshLetters(true)}>再試行</button>
      </div>
    {/if}

    {#if loading}
      <div class="letters-empty"><span class="loader"></span>手紙箱を確認中</div>
    {:else if visibleEvents.length === 0}
      <div class="letters-empty">
        <span class="empty-rule" aria-hidden="true"></span>
        <strong>まだ手紙はありません。</strong>
        <span>{agent.name} への最初の指示は、下の「手紙」から届けられます。</span>
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
</div>
