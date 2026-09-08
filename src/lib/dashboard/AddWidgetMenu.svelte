<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { settingsStore } from "../stores/settings";
  import { api } from "../api";
  import type { WidgetKind } from "../types";

  const dispatch = createEventDispatcher();

  const BASE_WIDGETS: { kind: WidgetKind; label: string }[] = [
    { kind: "clock", label: "Uhr" },
    { kind: "weather", label: "Wetter" },
    { kind: "news", label: "Nachrichten" },
    { kind: "system", label: "System" },
    { kind: "calendar", label: "Kalender" },
    { kind: "media", label: "Medien" },
  ];

  let mode: "menu" | "add-source" = "menu";
  let sourceUrl = "";
  let sourceCategory = "Tech";
  let submitting = false;
  let error = "";

  function addBaseWidget(kind: WidgetKind) {
    const layout = settingsStore.current().layout;
    settingsStore.updateLayout([
      ...layout,
      { id: `${kind}-${Date.now()}`, kind, x: 0, y: 0, w: 2, h: 1 },
    ]);
    dispatch("close");
  }

  async function submitSource() {
    error = "";
    if (!sourceUrl.trim()) return;
    submitting = true;
    try {
      const source = await api.sources.add(sourceUrl.trim(), sourceCategory);
      const layout = settingsStore.current().layout;
      settingsStore.updateLayout([
        ...layout,
        { id: `source-${source.id}`, kind: "source", x: 0, y: 0, w: 2, h: 2, sourceId: source.id },
      ]);
      dispatch("close");
    } catch (e) {
      error = "Diese Quelle konnte nicht zuverlässig ausgelesen werden.";
    } finally {
      submitting = false;
    }
  }
</script>

<div class="backdrop" on:click={() => dispatch("close")}
  on:keydown={(e) => e.key === "Escape" && dispatch("close")}
  role="button"
  tabindex="-1"
>
  <div class="menu surface" on:click|stopPropagation role="presentation">
    {#if mode === "menu"}
      <h2>Widget hinzufügen</h2>
      <div class="options">
        {#each BASE_WIDGETS as w}
          <button class="option" on:click={() => addBaseWidget(w.kind)}>{w.label}</button>
        {/each}
        <button class="option accent" on:click={() => (mode = "add-source")}>
          + Eigene Quelle
        </button>
      </div>
    {:else}
      <h2>Quelle hinzufügen</h2>
      <label class="field">
        <span>Link</span>
        <input type="url" placeholder="https://…" bind:value={sourceUrl} />
      </label>
      <label class="field">
        <span>Kategorie</span>
        <select bind:value={sourceCategory}>
          <option>Tech</option>
          <option>Gaming</option>
          <option>Minecraft</option>
          <option>Linux</option>
          <option>Hardware</option>
          <option>KI</option>
          <option>FPV</option>
          <option>Finanzen</option>
          <option>Sonstiges</option>
        </select>
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button on:click={() => (mode = "menu")}>Zurück</button>
        <button class="primary" disabled={submitting} on:click={submitSource}>
          {submitting ? "Prüfe…" : "Hinzufügen"}
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }

  .menu {
    width: 320px;
    padding: 20px;
  }

  h2 {
    margin: 0 0 14px;
    font-size: var(--text-lg);
    font-weight: 600;
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .option {
    text-align: left;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text);
    font-size: var(--text-sm);
  }

  .option:hover {
    background: var(--color-surface-raised);
  }

  .option.accent {
    color: var(--color-accent);
  }

  .field {
    display: block;
    margin-bottom: 12px;
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
  }

  .field span {
    display: block;
    margin-bottom: 4px;
  }

  .field input,
  .field select {
    width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--color-border-strong);
    background: var(--color-bg);
    color: var(--color-text);
    font-size: var(--text-sm);
  }

  .error {
    color: var(--color-bad);
    font-size: var(--text-xs);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }

  .actions .primary {
    background: var(--color-accent);
    color: #14171c;
    border: none;
    padding: 8px 14px;
    border-radius: 8px;
    font-weight: 600;
  }
</style>
