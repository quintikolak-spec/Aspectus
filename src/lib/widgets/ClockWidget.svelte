<script lang="ts">
  import { onDestroy, onMount } from "svelte";

  let now = new Date();
  let timer: ReturnType<typeof setInterval>;

  onMount(() => {
    timer = setInterval(() => (now = new Date()), 1000);
  });
  onDestroy(() => clearInterval(timer));

  $: time = now.toLocaleTimeString("de-DE", { hour: "2-digit", minute: "2-digit" });
  $: date = now.toLocaleDateString("de-DE", {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
</script>

<div class="clock">
  <span class="time data-readout">{time}</span>
  <span class="date">{date}</span>
</div>

<style>
  .clock {
    display: flex;
    flex-direction: column;
    justify-content: center;
    height: 100%;
  }

  .time {
    font-size: var(--text-2xl);
    font-weight: 500;
  }

  .date {
    color: var(--color-text-secondary);
    font-size: var(--text-sm);
    margin-top: 2px;
  }
</style>
