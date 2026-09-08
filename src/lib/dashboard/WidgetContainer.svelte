<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { WidgetLayout } from "../types";
  import ClockWidget from "../widgets/ClockWidget.svelte";
  import WeatherWidget from "../widgets/WeatherWidget.svelte";
  import NewsWidget from "../widgets/NewsWidget.svelte";
  import SystemWidget from "../widgets/SystemWidget.svelte";
  import CalendarWidget from "../widgets/CalendarWidget.svelte";
  import MediaWidget from "../widgets/MediaWidget.svelte";
  import SourceWidget from "../widgets/SourceWidget.svelte";

  export let widget: WidgetLayout;

  const dispatch = createEventDispatcher();

  const COMPONENTS = {
    clock: ClockWidget,
    weather: WeatherWidget,
    news: NewsWidget,
    system: SystemWidget,
    calendar: CalendarWidget,
    media: MediaWidget,
    source: SourceWidget,
  } as const;

  // svelte:component with heterogeneous prop shapes across the mapped
  // components doesn't type-check cleanly — narrowing to `any` here is
  // the standard workaround; each widget still validates its own props
  // internally.
  $: Component = COMPONENTS[widget.kind] as any;
</script>

<div class="widget surface">
  <div class="controls">
    <button on:click={() => dispatch("shrink")} title="Verkleinern" aria-label="Verkleinern">–</button>
    <button on:click={() => dispatch("grow")} title="Vergrößern" aria-label="Vergrößern">+</button>
    <button on:click={() => dispatch("remove")} title="Entfernen" aria-label="Entfernen">×</button>
  </div>
  <div class="content">
    <svelte:component this={Component} {widget} />
  </div>
</div>

<style>
  .widget {
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative;
    padding: var(--card-padding);
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .controls {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .widget:hover .controls,
  .widget:focus-within .controls {
    opacity: 1;
  }

  .controls button {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    border: 1px solid var(--color-border-strong);
    background: var(--color-surface-raised);
    color: var(--color-text-secondary);
    font-size: 0.75rem;
    line-height: 1;
  }

  .controls button:hover {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
