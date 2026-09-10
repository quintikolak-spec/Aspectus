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

    <div class="details">
      {#if snapshot.feelsLikeC !== null}
        <span class="detail">
          <span class="detail-label">Gefühlt</span>
          <span class="data-readout">{Math.round(snapshot.feelsLikeC)}°</span>
        </span>
      {/if}
      {#if snapshot.humidityPercent !== null}
        <span class="detail">
          <span class="detail-label">Luftfeuchte</span>
          <span class="data-readout">{Math.round(snapshot.humidityPercent)}%</span>
        </span>
      {/if}
      {#if snapshot.windSpeedKmh !== null}
        <span class="detail">
          <span class="detail-label">Wind</span>
          <span class="data-readout">{Math.round(snapshot.windSpeedKmh)} km/h</span>
        </span>
      {/if}
    </div>

    {#if snapshot.forecast.length > 0}
      <div class="forecast">
        {#each snapshot.forecast as day}
          <div class="forecast-day">
            <span class="forecast-label">
              {new Date(day.day).toLocaleDateString("de-DE", { weekday: "short" })}
            </span>
            <span class="forecast-temps data-readout">
              {Math.round(day.high)}° / {Math.round(day.low)}°
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{:else}
  <p class="hint">Lade Wetter…</p>
{/if}

<style>
  .weather {
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
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

  .details {
    display: flex;
    gap: 14px;
    margin-top: 8px;
  }

  .detail {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .detail-label {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .forecast {
    display: flex;
    gap: 12px;
    margin-top: 8px;
  }

  .forecast-day {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .forecast-label {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    text-transform: capitalize;
  }

  .forecast-temps {
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
  }

  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }
</style>
