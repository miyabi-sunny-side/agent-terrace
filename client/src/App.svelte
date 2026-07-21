<script>
  import { onMount } from "svelte";

  import { fetchAgents, fetchScreen } from "./api.js";
  import { parseAnsi } from "./ansi.js";
  import LetterComposer from "./LetterComposer.svelte";
  import LettersView from "./LettersView.svelte";

  const SCREEN_INTERVAL = 1500;
  const REGISTRY_INTERVAL = 5000;

  let agents = $state([]);
  let selectedPane = $state(null);
  let activeView = $state("screen");
  let screen = $state("");
  let registryLoading = $state(true);
  let screenLoading = $state(false);
  let registryError = $state("");
  let screenError = $state("");
  let lastCapturedAt = $state(null);
  let lettersRefreshToken = $state(0);

  let selectedAgent = $derived(agents.find((agent) => agent.pane_id === selectedPane));
  let screenLines = $derived(parseAnsi(screen));

  function message(error, fallback) {
    if (error?.code === "pane_unavailable") return "この agent の pane は終了しました。";
    if (error?.code === "registry_unavailable") return "agent registry に接続できません。";
    return fallback;
  }

  async function refreshAgents() {
    try {
      agents = await fetchAgents();
      registryError = "";
      if (selectedPane && !agents.some((agent) => agent.pane_id === selectedPane)) closeAgent();
    } catch (error) {
      registryError = message(error, "agent 一覧を取得できませんでした。");
    } finally {
      registryLoading = false;
    }
  }

  async function refreshScreen(pane) {
    try {
      const result = await fetchScreen(pane);
      if (selectedPane !== pane) return;
      screen = result.content;
      screenError = "";
      lastCapturedAt = new Date();
    } catch (error) {
      if (selectedPane !== pane) return;
      screenError = message(error, "画面を取得できませんでした。");
    } finally {
      if (selectedPane === pane) screenLoading = false;
    }
  }

  function selectAgent(agent, updateHash = true) {
    if (selectedPane !== agent.pane_id) lettersRefreshToken = 0;
    selectedPane = agent.pane_id;
    activeView = "screen";
    screen = "";
    screenError = "";
    screenLoading = true;
    if (updateHash) updateLocation();
  }

  function closeAgent(updateHash = true) {
    selectedPane = null;
    activeView = "screen";
    screen = "";
    screenError = "";
    if (updateHash) history.pushState(null, "", location.pathname + location.search);
  }

  function syncHash() {
    const params = new URLSearchParams(location.hash.slice(1));
    const pane = params.get("agent");
    if (!pane) {
      if (selectedPane) closeAgent(false);
      return;
    }
    const agent = agents.find((item) => item.pane_id === pane);
    if (agent) {
      if (selectedPane !== pane) selectAgent(agent, false);
      activeView = params.get("view") === "letters" ? "letters" : "screen";
    }
  }

  function updateLocation() {
    if (!selectedPane) return;
    const params = new URLSearchParams({ agent: selectedPane, view: activeView });
    history.pushState(null, "", `#${params}`);
  }

  function selectView(view) {
    activeView = view;
    updateLocation();
  }

  function handleTabKey(event) {
    const views = ["screen", "letters"];
    const current = views.indexOf(activeView);
    let next;
    if (event.key === "ArrowRight") next = views[(current + 1) % views.length];
    if (event.key === "ArrowLeft") next = views[(current - 1 + views.length) % views.length];
    if (event.key === "Home") next = views[0];
    if (event.key === "End") next = views.at(-1);
    if (!next) return;
    event.preventDefault();
    selectView(next);
    document.getElementById(`tab-${next}`)?.focus();
  }

  $effect(() => {
    const pane = selectedPane;
    const view = activeView;
    if (!pane || view !== "screen") return;
    refreshScreen(pane);
    const timer = window.setInterval(() => refreshScreen(pane), SCREEN_INTERVAL);
    return () => window.clearInterval(timer);
  });

  onMount(() => {
    refreshAgents().then(syncHash);
    const registryTimer = window.setInterval(refreshAgents, REGISTRY_INTERVAL);
    window.addEventListener("popstate", syncHash);
    window.addEventListener("hashchange", syncHash);
    return () => {
      window.clearInterval(registryTimer);
      window.removeEventListener("popstate", syncHash);
      window.removeEventListener("hashchange", syncHash);
    };
  });
</script>

<svelte:head>
  <title>{selectedAgent ? `${selectedAgent.name} · agent terrace` : "agent terrace"}</title>
</svelte:head>

<div class="app-shell" class:detail-open={selectedAgent}>
  <header class="lookout-header">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"></span>
      <div>
        <span class="eyebrow">SUNNY-SIDE LOOKOUT</span>
        <h1>agent terrace</h1>
      </div>
    </div>
    <span class="registry-count" aria-label={`${agents.length} agents registered`}>
      {agents.length}<span> agents</span>
    </span>
  </header>

  <main class="workspace">
    <aside class="agent-pane" aria-label="agent 一覧">
      <div class="pane-heading">
        <div>
          <span class="section-index">01</span>
          <h2>Registry</h2>
        </div>
        <button class="refresh-button" onclick={refreshAgents} aria-label="agent 一覧を更新">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" />
          </svg>
        </button>
      </div>

      {#if registryLoading}
        <div class="state-message"><span class="loader"></span>registry を確認中</div>
      {:else if registryError}
        <div class="error-banner" role="alert">{registryError}</div>
      {:else if agents.length === 0}
        <div class="state-message empty">待機中の agent はいません。</div>
      {:else}
        <div class="agent-list">
          {#each agents as agent, index (agent.pane_id)}
            <button
              class="agent-row"
              class:selected={agent.pane_id === selectedPane}
              onclick={() => selectAgent(agent)}
              aria-current={agent.pane_id === selectedPane ? "true" : undefined}
              style={`--row-index: ${index}`}
            >
              <span class:idle={agent.state === "idle"} class:busy={agent.state === "busy"} class="state-mark"></span>
              <span class="agent-copy">
                <span class="agent-title">
                  <strong>{agent.name}</strong>
                  <span class:idle={agent.state === "idle"} class:busy={agent.state === "busy"} class="state-label">{agent.state}</span>
                </span>
                <span class="agent-location">{agent.location} · {agent.pane_id}</span>
                <span class="agent-cwd" title={agent.cwd}>{agent.cwd}</span>
              </span>
              <svg class="chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 18 6-6-6-6" /></svg>
            </button>
          {/each}
        </div>
      {/if}
    </aside>

    <section class="screen-pane" aria-label={activeView === "letters" ? "agent への手紙" : "読み取り専用 agent 画面"}>
      {#if selectedAgent}
        <div class="screen-heading">
          <button class="back-button" onclick={() => closeAgent()} aria-label="agent 一覧に戻る">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
          </button>
          <div class="screen-title">
            <span class="section-index">02 / {activeView === "letters" ? "LETTER SHELF" : "READ ONLY"}</span>
            <h2>{selectedAgent.name}<span>{selectedAgent.pane_id}</span></h2>
          </div>
          <div class="capture-meta">
            <span class="live-dot"></span>
            {activeView === "letters" ? "2s refresh" : lastCapturedAt ? "1.5s refresh" : "connecting"}
          </div>
        </div>

        <div class="view-tabs" role="tablist" aria-label="agent detail view">
          <button
            id="tab-screen"
            role="tab"
            aria-selected={activeView === "screen"}
            aria-controls="panel-screen"
            tabindex={activeView === "screen" ? 0 : -1}
            class:active={activeView === "screen"}
            onclick={() => selectView("screen")}
            onkeydown={handleTabKey}>Screen</button
          >
          <button
            id="tab-letters"
            role="tab"
            aria-selected={activeView === "letters"}
            aria-controls="panel-letters"
            tabindex={activeView === "letters" ? 0 : -1}
            class:active={activeView === "letters"}
            onclick={() => selectView("letters")}
            onkeydown={handleTabKey}>Letters</button
          >
        </div>

        {#if activeView === "screen"}
          <div class="view-panel" id="panel-screen" role="tabpanel" aria-labelledby="tab-screen">
            {#if screenError}
              <div class="screen-error" role="alert">
                <span>{screenError}</span>
                <button onclick={() => refreshScreen(selectedAgent.pane_id)}>再試行</button>
              </div>
            {/if}

            <div class="terminal" aria-live="polite" aria-busy={screenLoading}>
              {#if screenLoading && !screen}
                <div class="terminal-empty"><span class="loader"></span>pane を撮影中</div>
              {:else if !screen}
                <div class="terminal-empty">pane に表示内容がありません。</div>
              {:else}
                <pre>{#each screenLines as line}{#each line as segment}<span class={segment.className} style={segment.style}>{segment.text}</span>{/each}{"\n"}{/each}</pre>
              {/if}
            </div>
          </div>
        {:else}
          <div class="view-panel" id="panel-letters" role="tabpanel" aria-labelledby="tab-letters">
            <LettersView agent={selectedAgent} refreshToken={lettersRefreshToken} />
          </div>
        {/if}

        {#key selectedPane}
          <LetterComposer agent={selectedAgent} onSent={() => (lettersRefreshToken += 1)} />
        {/key}
      {:else}
        <div class="unselected">
          <span class="section-index">02 / LOOKOUT</span>
          <div class="terrace-lines" aria-hidden="true"><i></i><i></i><i></i></div>
          <h2>外から、静かに眺める。</h2>
          <p>agent を選ぶと、tmux pane の現在の景色を読み取り専用で表示します。</p>
          <span class="read-only-note">capture-pane only · no attach · no input</span>
        </div>
      {/if}
    </section>
  </main>
</div>
