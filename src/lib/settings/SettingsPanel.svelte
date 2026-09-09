<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { settingsStore } from "../stores/settings";
  import { api } from "../api";
  import type { AppSettings, OperatingMode, Source } from "../types";

  const dispatch = createEventDispatcher();

  let sources: Source[] = [];
  let signedIn = false;
  let apiKeyInput = "";
  let authBusy = false;
  let authError = "";

  onMount(async () => {
    sources = await api.sources.list();
    signedIn = await api.ai.isSignedIn();
  });

  async function submitApiKey() {
    authError = "";
    authBusy = true;
    try {
      await api.ai.signInWithApiKey(apiKeyInput.trim());
      signedIn = true;
      apiKeyInput = "";
    } catch (e) {
      authError = String(e);
    } finally {
      authBusy = false;
    }
  }

  async function tryOAuth() {
    authError = "";
    authBusy = true;
    try {
      await api.ai.signInOAuth();
      signedIn = true;
    } catch (e) {
      // Expected until this app has an OpenAI-approved OAuth client_id —
      // see core::auth::sign_in_oauth. Point the user at the API-key path.
      authError =
        "„Sign in with ChatGPT“ ist für diese App noch nicht freigeschaltet — bitte einen API-Key verwenden.";
    } finally {
      authBusy = false;
    }
  }

  async function signOut() {
    await api.ai.signOut();
    signedIn = false;
  }

  function toggleInterest(id: string) {
    const interests = $settingsStore.interests.map((i) =>
      i.id === id ? { ...i, enabled: !i.enabled } : i
    );
    settingsStore.updateSettings({ interests });
  }

  async function removeSource(id: string) {
    await api.sources.remove(id);
    sources = sources.filter((s) => s.id !== id);
  }

  const MODES: { value: OperatingMode; label: string; hint: string }[] = [
    { value: "normal", label: "Normal", hint: "Regelmäßige Updates, volle Reaktionsfreude" },
    { value: "sparsam", label: "Sparsam", hint: "Längere Intervalle, weniger Hintergrundlast" },
    { value: "eco", label: "Eco", hint: "Minimale Ressourcen, Updates nur beim Öffnen" },
  ];

  // Inline `as SomeUnion` casts don't parse inside template attribute
  // expressions (those are plain JS, not TS) — so the narrowing has to
  // happen in named functions here instead.
  function setInfoFilter(value: string) {
    settingsStore.updateSettings({ infoFilter: value as AppSettings["infoFilter"] });
  }
  function setTheme(value: string) {
    settingsStore.updateSettings({ theme: value as AppSettings["theme"] });
  }
  function setCornerStyle(value: string) {
    settingsStore.updateSettings({ cornerStyle: value as AppSettings["cornerStyle"] });
  }
</script>

<div class="backdrop" on:click={() => dispatch("close")}
  on:keydown={(e) => e.key === "Escape" && dispatch("close")}
  role="button"
  tabindex="-1"
>
  <div class="panel surface" on:click|stopPropagation role="presentation">
    <h2>Einstellungen</h2>

    <section>
      <h3>Modus</h3>
      <div class="modes">
        {#each MODES as m}
          <button
            class="mode"
            class:active={$settingsStore.mode === m.value}
            on:click={() => settingsStore.updateSettings({ mode: m.value })}
          >
            <span class="mode-label">{m.label}</span>
            <span class="mode-hint">{m.hint}</span>
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h3>Meine Interessen</h3>
      <div class="chips">
        {#each $settingsStore.interests as interest}
          <button
            class="chip"
            class:active={interest.enabled}
            on:click={() => toggleInterest(interest.id)}
          >
            {interest.label}
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h3>Informationsfilter</h3>
      <select
        value={$settingsStore.infoFilter}
        on:change={(e) => setInfoFilter(e.currentTarget.value)}
      >
        <option value="all">Alles anzeigen</option>
        <option value="relevant">Nur relevante Informationen</option>
        <option value="important">Nur wichtige Informationen</option>
        <option value="critical">Nur sehr wichtige Informationen</option>
      </select>
    </section>

    <section>
      <h3>Erscheinungsbild</h3>
      <div class="row">
        <label>
          Theme
          <select
            value={$settingsStore.theme}
            on:change={(e) => setTheme(e.currentTarget.value)}
          >
            <option value="dark">Dunkel</option>
            <option value="light">Hell</option>
            <option value="system">System</option>
          </select>
        </label>
        <label>
          Ecken
          <select
            value={$settingsStore.cornerStyle}
            on:change={(e) => setCornerStyle(e.currentTarget.value)}
          >
            <option value="sharp">Eckig</option>
            <option value="soft">Leicht abgerundet</option>
            <option value="round">Komplett rund</option>
          </select>
        </label>
        <label>
          Akzentfarbe
          <input
            type="color"
            value={$settingsStore.accentColor}
            on:input={(e) => settingsStore.updateSettings({ accentColor: e.currentTarget.value })}
          />
        </label>
      </div>
    </section>

    <section>
      <h3>KI-Konto</h3>
      {#if signedIn}
        <div class="account-row">
          <span class="account-status">✓ Verbunden</span>
          <button class="remove" on:click={signOut}>Abmelden</button>
        </div>
      {:else}
        <p class="hint">
          Für KI-Zusammenfassungen wird ein OpenAI-Zugang benötigt. Am schnellsten geht das
          mit einem API-Key von platform.openai.com/api-keys.
        </p>
        <div class="row" style="margin-bottom:8px;">
          <input
            type="password"
            placeholder="sk-..."
            bind:value={apiKeyInput}
            style="flex:1; background: var(--color-bg); color: var(--color-text); border: 1px solid var(--color-border-strong); border-radius: 8px; padding: 8px;"
          />
          <button class="primary-btn" disabled={authBusy || !apiKeyInput} on:click={submitApiKey}>
            Verbinden
          </button>
        </div>
        <button class="oauth-btn" disabled={authBusy} on:click={tryOAuth}>
          Mit ChatGPT-Konto anmelden
        </button>
        {#if authError}<p class="error">{authError}</p>{/if}
      {/if}
    </section>

    <section>
      <h3>Meine Quellen</h3>
      {#if sources.length === 0}
        <p class="hint">Noch keine Quellen hinzugefügt.</p>
      {/if}
      <ul class="sources">
        {#each sources as source}
          <li>
            <div>
              <span class="source-title">{source.title || source.url}</span>
              <span class="source-meta">
                {source.category} · {source.healthy ? "OK" : "Fehler beim Abruf"}
              </span>
            </div>
            <button class="remove" on:click={() => removeSource(source.id)}>Entfernen</button>
          </li>
        {/each}
      </ul>
    </section>

    <label class="row startup">
      <input
        type="checkbox"
        checked={$settingsStore.launchOnStartup}
        on:change={(e) =>
          settingsStore.updateSettings({ launchOnStartup: e.currentTarget.checked })}
      />
      Beim Systemstart automatisch starten
    </label>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }

  .panel {
    width: 460px;
    max-height: 80vh;
    overflow-y: auto;
    padding: 22px;
  }

  h2 {
    margin: 0 0 16px;
  }

  h3 {
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
    margin: 0 0 8px;
    font-weight: 600;
  }

  section {
    margin-bottom: 18px;
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .mode {
    text-align: left;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius);
    padding: 10px;
    background: transparent;
    color: var(--color-text);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .mode.active {
    border-color: var(--color-accent);
    background: var(--color-accent-dim);
  }

  .mode-label {
    font-weight: 600;
    font-size: var(--text-sm);
  }

  .mode-hint {
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    border: 1px solid var(--color-border-strong);
    border-radius: 999px;
    padding: 6px 12px;
    background: transparent;
    color: var(--color-text-secondary);
    font-size: var(--text-xs);
  }

  .chip.active {
    color: var(--color-bg);
    background: var(--color-accent);
    border-color: var(--color-accent);
  }

  .row {
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .row label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--text-xs);
    color: var(--color-text-secondary);
  }

  select,
  input[type="color"] {
    background: var(--color-bg);
    color: var(--color-text);
    border: 1px solid var(--color-border-strong);
    border-radius: 8px;
    padding: 6px 8px;
  }

  select {
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 8'%3E%3Cpath fill='%238b93a1' d='M1 1l5 5 5-5'/%3E%3C/svg%3E");
    background-size: 9px 6px;
    background-repeat: no-repeat;
    background-position: right 10px center;
    padding-right: 28px;
  }

  .sources {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .sources li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--color-border);
    padding-top: 6px;
  }

  .source-title {
    display: block;
    font-size: var(--text-sm);
  }

  .source-meta {
    display: block;
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
  }

  .remove {
    background: none;
    border: none;
    color: var(--color-bad);
    font-size: var(--text-xs);
  }

  .startup {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: var(--text-sm);
  }

  .hint {
    color: var(--color-text-tertiary);
    font-size: var(--text-sm);
  }

  .account-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .account-status {
    color: var(--color-good);
    font-size: var(--text-sm);
    font-weight: 600;
  }

  .primary-btn {
    background: var(--color-accent);
    color: #14171c;
    border: none;
    padding: 8px 14px;
    border-radius: 8px;
    font-weight: 600;
    white-space: nowrap;
  }

  .primary-btn:disabled {
    opacity: 0.5;
  }

  .oauth-btn {
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    border: 1px solid var(--color-border-strong);
    background: transparent;
    color: var(--color-text-secondary);
    font-size: var(--text-sm);
  }

  .oauth-btn:hover {
    color: var(--color-text);
    border-color: var(--color-accent);
  }
</style>
