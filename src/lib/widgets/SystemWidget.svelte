<script lang="ts">
  import { systemStats } from "../stores/updateCycle";
  import { settingsStore } from "../stores/settings";
  import type { WidgetLayout } from "../types";

  export let widget: WidgetLayout;

  let showHidden = false;

  $: stats = $systemStats;
  $: hidden = new Set(widget.hiddenProcesses ?? []);
  $: visibleProcesses = (stats?.topProcesses ?? []).filter((p) => !hidden.has(p.name));
  $: hiddenProcesses = (stats?.topProcesses ?? []).filter((p) => hidden.has(p.name));

  // Backend already groups raw sensor labels into CPU/GPU/Mainboard/etc —
  // just sort hottest-first here.
  $: temps = Object.entries(stats?.tempsC ?? {}).sort((a, b) => b[1] - a[1]);

  $: gauges = stats
    ? [
        { label: "CPU", value: stats.cpuPercent },
        { label: "RAM", value: stats.ramPercent },
        ...(stats.gpuPercent !== null ? [{ label: "GPU", value: stats.gpuPercent }] : []),
        { label: "System", value: Math.min(stats.systemLoadPercent, 100) },
      ]
    : [];

  function hideProcess(name: string) {
    const current = widget.hiddenProcesses ?? [];
    updateWidget({ hiddenProcesses: [...current, name] });
  }

  function unhideProcess(name: string) {
    const current = widget.hiddenProcesses ?? [];
    updateWidget({ hiddenProcesses: current.filter((n) => n !== name) });
  }

  function updateWidget(patch: Partial<WidgetLayout>) {
    const layout = settingsStore.current().layout.map((w) =>
      w.id === widget.id ? { ...w, ...patch } : w
    );
    settingsStore.updateLayout(layout);
  }

  function gaugeColor(value: number): string {
    if (value >= 90) return "var(--color-bad)";
    if (value >= 70) return "var(--color-warn)";
    return "var(--color-accent)";
  }
</script>

{#if stats}
  <div class="system">
    <div class="gauges">
      {#each gauges as g (g.label)}
        <div class="gauge">
          <div
            class="ring"
            style="--pct: {Math.min(g.value, 100)}; --ring-color: {gaugeColor(g.value)};"
          >
            <span class="ring-value data-readout">{Math.round(g.value)}%</span>
          </div>
          <span class="gauge-label">{g.label}</span>
        </div>
      {/each}
    </div>

    <div class="secondary-metrics">
      <div class="metric">
        <span class="label">Frei</span>
        <span class="value data-readout">{stats.diskFreeGb.toFixed(0)} GB</span>
      </div>
      {#if stats.batteryPercent !== null}
        <div class="metric">
          <span class="label">Akku</span>
          <span class="value data-readout">{Math.round(stats.batteryPercent)}%</span>
        </div>
      {/if}
    </div>

    {#if temps.length > 0}
      <div class="temps">
        {#each temps as [label, temp]}
          <span class="temp-chip">
            <span class="temp-label">{label}</span>
            <span class="data-readout">{Math.round(temp)}°C</span>
          </span>
        {/each}
      </div>
    {/if}

    {#if visibleProcesses.length > 0}
      <div class="processes">
        <span class="section-label">Meiste Auslastung</span>
        {#each visibleProcesses as proc (proc.name)}
          <div class="process-row">
            <span class="process-name">{proc.name}</span>
            <span class="process-stat data-readout">{proc.cpuPercent.toFixed(0)}%</span>
            <button
              class="process-remove"
              on:click={() => hideProcess(proc.name)}
              title="Ausblenden"
              aria-label="{proc.name} ausblenden"
            >
              ×
            </button>
          </div>
        {/each}
      </div>
    {/if}

    {#if hiddenProcesses.length > 0}
      <button class="toggle-hidden" on:click={() => (showHidden = !showHidden)}>
        {showHidden ? "Ausgeblendete verbergen" : `${hiddenProcesses.length} ausgeblendet`}
      </button>
      {#if showHidden}
        <div class="processes">
          {#each hiddenProcesses as proc (proc.name)}
            <div class="process-row">
              <span class="process-name muted">{proc.name}</span>
              <button
                class="process-add"
                on:click={() => unhideProcess(proc.name)}
                title="Wieder anzeigen"
                aria-label="{proc.name} wieder anzeigen"
              >
                +
              </button>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
{:else}
  <p class="hint">Lade Systemdaten…</p>
{/if}

<style>
  .system {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
  }

  .gauges {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .gauge {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .ring {
    --size: 56px;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: conic-gradient(
      var(--ring-color) calc(var(--pct) * 1%),
      var(--color-border) calc(var(--pct) * 1%)
    );
    position: relative;
  }

  .ring::before {
    content: "";
    position: absolute;
    inset: 6px;
    border-radius: 50%;
    background: var(--color-surface);
  }

  .ring-value {
    position: relative;
    font-size: var(--text-xs);
    font-weight: 600;
  }

  .gauge-label {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .secondary-metrics {
    display: flex;
    gap: 18px;
  }

  .metric {
    display: flex;
    flex-direction: column;
  }

  .label {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .value {
    font-size: var(--text-lg);
  }

  .temps {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .temp-chip {
    display: flex;
    gap: 5px;
    align-items: baseline;
    font-size: var(--text-xs);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 2px 8px;
    color: var(--color-text-secondary);
  }

  .temp-label {
    color: var(--color-text-tertiary);
  }

  .section-label {
    display: block;
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    margin-bottom: 4px;
  }

  .processes {
    display: flex;
    flex-direction: column;
  }

  .process-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 0;
    border-top: 1px solid var(--color-border);
  }

  .process-row:first-child {
    border-top: none;
  }

  .process-name {
    flex: 1;
    font-size: var(--text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .process-name.muted {
    color: var(--color-text-tertiary);
  }

  .process-stat {
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
  }

  .process-remove,
  .process-add {
    width: 18px;
    height: 18px;
    line-height: 1;
    border-radius: 5px;
    border: 1px solid var(--color-border-strong);
    background: transparent;
    color: var(--color-text-tertiary);
    font-size: 0.7rem;
  }

  .process-remove:hover {
    color: var(--color-bad);
    border-color: var(--color-bad);
  }

  .process-add:hover {
    color: var(--color-good);
    border-color: var(--color-good);
  }

  .toggle-hidden {
    align-self: flex-start;
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    background: transparent;
    border: none;
    text-decoration: underline;
    padding: 0;
  }

  .toggle-hidden:hover {
    color: var(--color-text-secondary);
  }

  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }
</style>
