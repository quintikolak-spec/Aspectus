<script lang="ts">
  // TODO: wire up to MPRIS (Linux) / SMTC (Windows) via a Rust command,
  // e.g. `get_now_playing` + `media_control(action)`.
  let playing = false;
</script>

<div class="media">
  <span class="track">Kein Titel wird wiedergegeben</span>
  <div class="transport">
    <button aria-label="Vorheriger Titel">⏮</button>
    <button aria-label={playing ? "Pause" : "Abspielen"} on:click={() => (playing = !playing)}>
      {playing ? "⏸" : "▶"}
    </button>
    <button aria-label="Nächster Titel">⏭</button>
  </div>
</div>

<style>
  .media {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
    height: 100%;
  }
  .track {
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
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
</style>
