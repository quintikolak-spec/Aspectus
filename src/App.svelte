<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { settingsStore } from "./lib/stores/settings";
  import { appliedTheme } from "./lib/stores/theme";
  import { startUpdateCycles, stopUpdateCycles } from "./lib/stores/updateCycle";
  import Dashboard from "./lib/dashboard/Dashboard.svelte";
  import SettingsPanel from "./lib/settings/SettingsPanel.svelte";

  let ready = false;
  let settingsOpen = false;

  onMount(async () => {
    await settingsStore.load();
    startUpdateCycles();
    ready = true;
  });

  onDestroy(() => stopUpdateCycles());

  // Referencing the derived store is what makes it apply the CSS vars.
  $: void $appliedTheme;
</script>

{#if ready}
  <Dashboard />
  <button class="settings-btn" on:click={() => (settingsOpen = true)} aria-label="Einstellungen">
    ⚙
  </button>
  {#if settingsOpen}
    <SettingsPanel on:close={() => (settingsOpen = false)} />
  {/if}
{:else}
  <div class="boot">
    <span class="data-readout">Lade Dashboard…</span>
  </div>
{/if}

<style>
  .boot {
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-secondary);
    font-size: var(--text-sm);
  }

  .settings-btn {
    position: fixed;
    left: 24px;
    bottom: 24px;
    width: 40px;
    height: 40px;
    border-radius: 999px;
    border: 1px solid var(--color-border-strong);
    background: var(--color-surface-raised);
    color: var(--color-text-secondary);
  }

  .settings-btn:hover {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
