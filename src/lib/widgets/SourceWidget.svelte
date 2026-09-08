<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { newsCards } from "../stores/updateCycle";
  import type { WidgetLayout } from "../types";

  export let widget: WidgetLayout;

  $: latest = $newsCards
    .filter((c) => c.sourceId === widget.sourceId)
    .sort((a, b) => (b.publishedAt ?? "").localeCompare(a.publishedAt ?? ""))[0];
</script>

{#if latest}
  <button class="card" on:click={() => openUrl(latest.originalUrl)}>
    <span class="source">{latest.sourceTitle}</span>
    <span class="headline">{latest.headline}</span>
    <span class="summary">{latest.summary}</span>
  </button>
{:else}
  <p class="hint">Noch keine Inhalte für diese Quelle.</p>
{/if}

<style>
  .card {
    text-align: left;
    background: transparent;
    border: none;
    color: var(--color-text);
    display: flex;
    flex-direction: column;
    gap: 4px;
    height: 100%;
    width: 100%;
  }
  .source {
    font-size: var(--text-xs);
    color: var(--color-accent);
  }
  .headline {
    font-weight: 600;
  }
  .summary {
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
  }
  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }
</style>
