<script lang="ts">
  // TODO: wire up to a local ICS file or a calendar provider once section 4's
  // "optional integrierter Kalender" scope is decided. Kept independent of
  // other widgets per the modularity rule in section 17.
  const today = new Date().toLocaleDateString("de-DE", { day: "numeric", month: "long" });
</script>

<div class="calendar">
  <span class="today">{today}</span>
  <p class="hint">Keine Kalenderquelle verbunden.</p>
</div>

<style>
  .calendar {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .today {
    font-size: var(--text-lg);
    font-weight: 600;
  }
  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
    margin: 0;
  }
</style>
