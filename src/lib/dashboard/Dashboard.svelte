<script lang="ts">
  import { settingsStore } from "../stores/settings";
  import WidgetContainer from "./WidgetContainer.svelte";
  import AddWidgetMenu from "./AddWidgetMenu.svelte";

  let draggedId: string | null = null;
  let menuOpen = false;

  $: layout = $settingsStore.layout;

  function onDragStart(id: string) {
    draggedId = id;
  }

  function onDrop(targetId: string) {
    if (!draggedId || draggedId === targetId) return;
    const current = [...layout];
    const fromIdx = current.findIndex((w) => w.id === draggedId);
    const toIdx = current.findIndex((w) => w.id === targetId);
    if (fromIdx === -1 || toIdx === -1) return;

    // Swap grid positions rather than array order, so "moved" reads as
    // an actual placement change (section 3: verschoben werden können).
    const a = current[fromIdx];
    const b = current[toIdx];
    [a.x, b.x] = [b.x, a.x];
    [a.y, b.y] = [b.y, a.y];

    settingsStore.updateLayout(current);
    draggedId = null;
  }

  function removeWidget(id: string) {
    settingsStore.updateLayout(layout.filter((w) => w.id !== id));
  }

  function resizeWidget(id: string, dw: number, dh: number) {
    const next = layout.map((w) =>
      w.id === id
        ? { ...w, w: Math.max(1, Math.min(4, w.w + dw)), h: Math.max(1, Math.min(3, w.h + dh)) }
        : w
    );
    settingsStore.updateLayout(next);
  }
</script>

<div class="dashboard">
  <div class="grid">
    {#each layout as widget (widget.id)}
      <div
        class="cell"
        role="group"
        aria-label="{widget.kind} widget"
        style="grid-column: span {widget.w}; grid-row: span {widget.h};"
        draggable="true"
        on:dragstart={() => onDragStart(widget.id)}
        on:dragover|preventDefault
        on:drop={() => onDrop(widget.id)}
      >
        <WidgetContainer
          {widget}
          on:remove={() => removeWidget(widget.id)}
          on:grow={() => resizeWidget(widget.id, 1, 0)}
          on:shrink={() => resizeWidget(widget.id, -1, 0)}
        />
      </div>
    {/each}
  </div>

  <button class="add-btn" on:click={() => (menuOpen = true)} aria-label="Widget hinzufügen">
    +
  </button>

  {#if menuOpen}
    <AddWidgetMenu on:close={() => (menuOpen = false)} />
  {/if}
</div>

<style>
  .dashboard {
    height: 100vh;
    padding: var(--widget-gap);
    overflow-y: auto;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    grid-auto-rows: 140px;
    gap: var(--widget-gap);
  }

  .cell {
    min-width: 0;
    min-height: 0;
  }

  .cell[draggable="true"] {
    cursor: grab;
  }

  .add-btn {
    position: fixed;
    right: 24px;
    bottom: 24px;
    width: 44px;
    height: 44px;
    border-radius: 999px;
    border: 1px solid var(--color-border-strong);
    background: var(--color-surface-raised);
    color: var(--color-text);
    font-size: 1.25rem;
    line-height: 1;
    box-shadow: none;
    transition: transform var(--transition-fast), border-color var(--transition-fast);
  }

  .add-btn:hover {
    border-color: var(--color-accent);
    transform: translateY(-1px);
  }
</style>
