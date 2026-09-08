<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import type { WeatherSnapshot } from "../types";

  let snapshot: WeatherSnapshot | null = null;
  let failed = false;

  onMount(async () => {
    try {
      // TODO: replace with the user-configured or auto-detected location.
      snapshot = await api.weather.current(48.5372, 12.1522); // Landshut fallback
    } catch {
      failed = true;
    }
  });
</script>

{#if failed}
  <p class="hint">Wetterdaten nicht verfügbar.</p>
{:else if snapshot}
  <div class="weather">
    <div class="now">
      <span class="temp data-readout">{Math.round(snapshot.tempC)}°</span>
      <span class="condition">{snapshot.condition}</span>
    </div>
    <span class="location">{snapshot.locationName}</span>
  </div>
{:else}
  <p class="hint">Lade Wetter…</p>
{/if}

<style>
  .weather {
    display: flex;
    flex-direction: column;
    justify-content: center;
    height: 100%;
  }

  .now {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .temp {
    font-size: var(--text-2xl);
    font-weight: 500;
  }

  .condition {
    color: var(--color-text-secondary);
    font-size: var(--text-sm);
  }

  .location {
    margin-top: 2px;
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }
</style>
