<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import { settingsStore } from "../stores/settings";
  import type { NowPlaying, WidgetLayout } from "../types";

  export let widget: WidgetLayout;

  let nowPlaying: NowPlaying | null = null;
  let available = true; // false if MPRIS itself isn't reachable at all
  let volume: number | null = null;
  let adjustingVolume = false;
  let players: string[] = [];
  let timer: ReturnType<typeof setInterval>;

  $: preferred = widget.preferredMediaPlayer;

  async function poll() {
    try {
      players = await api.media.listPlayers();
      nowPlaying = await api.media.nowPlaying(preferred);
      available = true;
      if (!adjustingVolume) {
        const v = await api.media.getVolume(preferred);
        if (v !== null) volume = v;
      }
    } catch {
      available = false;
    }
  }

  onMount(() => {
    poll();
    timer = setInterval(poll, 2000);
  });
  onDestroy(() => clearInterval(timer));

  async function playPause() {
    await api.media.playPause(preferred);
    poll();
  }
  async function next() {
    await api.media.next(preferred);
    poll();
  }
  async function previous() {
    await api.media.previous(preferred);
    poll();
  }

  function onVolumeChange(e: Event) {
    const level = Number((e.currentTarget as HTMLInputElement).value) / 100;
    volume = level;
    api.media.setVolume(level, preferred);
    adjustingVolume = false;
  }

  function onPlayerChange(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value;
    const layout = settingsStore.current().layout.map((w) =>
      w.id === widget.id ? { ...w, preferredMediaPlayer: value || undefined } : w
    );
    settingsStore.updateLayout(layout);
  }
</script>

<div class="media">
  {#if players.length > 1}
    <select class="player-select" value={preferred ?? ""} on:change={onPlayerChange}>
      <option value="">Automatisch</option>
      {#each players as p}
        <option value={p}>{p}</option>
      {/each}
    </select>
  {/if}

  {#if !available}
    <span class="track muted">Medien-Steuerung nicht verfügbar.</span>
  {:else if nowPlaying}
    <div class="info">
      <span class="track">{nowPlaying.title}</span>
      {#if nowPlaying.artist}
        <span class="artist">{nowPlaying.artist}</span>
      {/if}
      <span class="source">{nowPlaying.source}</span>
    </div>
  {:else}
    <span class="track muted">Kein Titel wird wiedergegeben</span>
  {/if}

  <div class="transport">
    <button on:click={previous} aria-label="Vorheriger Titel" disabled={!available}>⏮</button>
    <button
      on:click={playPause}
      aria-label={nowPlaying?.isPlaying ? "Pause" : "Abspielen"}
      disabled={!available}
    >
      {nowPlaying?.isPlaying ? "⏸" : "▶"}
    </button>
    <button on:click={next} aria-label="Nächster Titel" disabled={!available}>⏭</button>
  </div>

  {#if volume !== null}
    <div class="volume">
      <span class="volume-icon">🔊</span>
      <input
        type="range"
        min="0"
        max="100"
        value={Math.round(volume * 100)}
        on:pointerdown={() => (adjustingVolume = true)}
        on:change={onVolumeChange}
        aria-label="Lautstärke"
      />
    </div>
  {/if}
</div>

<style>
  .media {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
    height: 100%;
  }
  .player-select {
    align-self: flex-start;
    font-size: var(--text-xs);
    background: var(--color-bg);
    color: var(--color-text-secondary);
    border: 1px solid var(--color-border-strong);
    border-radius: 6px;
    padding: 2px 6px;
    appearance: none;
    -webkit-appearance: none;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .track {
    font-size: var(--text-sm);
    color: var(--color-text);
  }
  .track.muted {
    color: var(--color-text-secondary);
  }
  .artist {
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
  }
  .source {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }
  .transport {
    display: flex;
    gap: 10px;
  }
  .transport button {
    background: none;
    border: none;
    color: var(--color-text);
    font-size: 1rem;
  }
  .transport button:disabled {
    color: var(--color-text-tertiary);
    opacity: 0.5;
  }
  .volume {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .volume-icon {
    font-size: 0.8rem;
  }
  .volume input[type="range"] {
    flex: 1;
    accent-color: var(--color-accent);
  }
</style>
