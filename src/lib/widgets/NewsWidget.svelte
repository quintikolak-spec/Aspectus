<script lang="ts">
  import { newsCards, refreshNewsNow } from "../stores/updateCycle";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let refreshing = false;
  let refreshError = "";

  $: cards = [...$newsCards].sort((a, b) => b.importance - a.importance).slice(0, 6);

  function openArticle(url: string) {
    // Always the original link — the app never replaces the source page.
    openUrl(url);
  }

  async function manualRefresh() {
    refreshing = true;
    refreshError = "";
    try {
      await refreshNewsNow();
    } catch (e) {
      refreshError = String(e);
    } finally {
      refreshing = false;
    }
  }
</script>

<div class="news">
  <button class="refresh-btn" on:click={manualRefresh} disabled={refreshing}>
    {refreshing ? "Prüfe Quellen…" : "Jetzt aktualisieren"}
  </button>
  {#if refreshError}
    <p class="error">{refreshError}</p>
  {/if}
  {#if cards.length === 0 && !refreshing}
    <p class="hint">Noch keine neuen Meldungen.</p>
  {/if}
  {#each cards as card (card.id)}
    <button class="card" on:click={() => openArticle(card.originalUrl)}>
      <span class="category">{card.category}</span>
      <span class="headline">{card.headline}</span>
      <span class="summary">{card.summary}</span>
      <span class="source">{card.sourceTitle}</span>
    </button>
  {/each}
</div>

<style>
  .news {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
  }

  .card {
    text-align: left;
    background: transparent;
    border: none;
    border-top: 1px solid var(--color-border);
    padding: 10px 0 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    color: var(--color-text);
  }

  .card:first-child {
    border-top: none;
    padding-top: 0;
  }

  .category {
    font-size: var(--text-xs);
    color: var(--color-accent);
  }

  .headline {
    font-size: var(--text-base);
    font-weight: 600;
  }

  .summary {
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
  }

  .source {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }

  .refresh-btn {
    align-self: flex-start;
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
    background: transparent;
    border: 1px solid var(--color-border-strong);
    border-radius: 999px;
    padding: 4px 10px;
    margin-bottom: 4px;
  }

  .refresh-btn:hover {
    color: var(--color-text);
    border-color: var(--color-accent);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
  }

  .error {
    color: var(--color-bad);
    font-size: var(--text-xs);
    margin: 0;
  }
</style>
