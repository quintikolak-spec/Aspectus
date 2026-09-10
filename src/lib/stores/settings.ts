import { writable, get } from "svelte/store";
import { api } from "../api";
import type { AppSettings } from "../types";

const DEFAULT_SETTINGS: AppSettings = {
  mode: "normal",
  theme: "dark",
  cornerStyle: "soft",
  accentColor: "#e8a33d",
  interests: [
    { id: "gaming", label: "Gaming", enabled: true },
    { id: "minecraft", label: "Minecraft", enabled: true },
    { id: "linux", label: "Linux", enabled: true },
    { id: "hardware", label: "Hardware", enabled: true },
    { id: "ki", label: "KI", enabled: true },
    { id: "fpv", label: "FPV", enabled: true },
    { id: "it", label: "IT", enabled: true },
    { id: "sport", label: "Sport", enabled: false },
    { id: "promis", label: "Prominente", enabled: false },
    { id: "politik", label: "Politik", enabled: false },
  ],
  infoFilter: "relevant",
  layout: [
    { id: "clock-1", kind: "clock", x: 0, y: 0, w: 2, h: 1 },
    { id: "weather-1", kind: "weather", x: 2, y: 0, w: 2, h: 2 },
    { id: "news-1", kind: "news", x: 0, y: 1, w: 4, h: 2 },
    { id: "system-1", kind: "system", x: 0, y: 3, w: 4, h: 1 },
  ],
  launchOnStartup: true,
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<AppSettings>(DEFAULT_SETTINGS);

  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  function scheduleSave(current: AppSettings) {
    // Debounce writes so dragging a widget doesn't hammer disk I/O.
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      api.settings.save(current).catch((err) => {
        console.error("Failed to persist settings", err);
      });
    }, 400);
  }

  return {
    subscribe,
    async load() {
      try {
        const loaded = await api.settings.get();
        set(loaded);
      } catch {
        // First run: no settings file yet, fall back to defaults silently.
        set(DEFAULT_SETTINGS);
      }
    },
    updateSettings(patch: Partial<AppSettings>) {
      update((current) => {
        const next = { ...current, ...patch };
        scheduleSave(next);
        return next;
      });
    },
    updateLayout(layout: AppSettings["layout"]) {
      update((current) => {
        const next = { ...current, layout };
        scheduleSave(next);
        return next;
      });
    },
    current() {
      return get({ subscribe });
    },
  };
}

export const settingsStore = createSettingsStore();
