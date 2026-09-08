import { writable, get } from "svelte/store";
import { api } from "../api";
import { settingsStore } from "./settings";
import type { NewsCard, SystemStats } from "../types";

/**
 * Update intervals per operating mode (section 11). These govern *local*
 * widgets like the system monitor; the news pipeline has its own,
 * separate cadence below since it involves network + AI cost (section 13).
 */
const SYSTEM_INTERVAL_MS: Record<string, number> = {
  normal: 2_000,
  sparsam: 8_000,
  eco: 30_000,
};

const NEWS_INTERVAL_MS: Record<string, number> = {
  normal: 10 * 60_000, // 10 min
  sparsam: 30 * 60_000, // 30 min
  eco: 2 * 60 * 60_000, // 2 h, and only while the dashboard is open
};

export const systemStats = writable<SystemStats | null>(null);
export const newsCards = writable<NewsCard[]>([]);

let systemTimer: ReturnType<typeof setInterval> | undefined;
let newsTimer: ReturnType<typeof setInterval> | undefined;

export function startUpdateCycles() {
  stopUpdateCycles();

  const mode = get(settingsStore).mode;

  const tickSystem = () =>
    api.system
      .stats()
      .then((s) => systemStats.set(s))
      .catch(() => {
        /* system stats are best-effort; keep last known value on failure */
      });

  const tickNews = () =>
    api.news
      .refreshAll()
      .then((cards) => newsCards.set(cards))
      .catch((err) => console.error("News refresh failed", err));

  // Load from cache immediately (no network, no AI) so the dashboard is
  // never empty while waiting for the first interval to elapse.
  api.news.cached().then((cards) => newsCards.set(cards));
  tickSystem();

  systemTimer = setInterval(tickSystem, SYSTEM_INTERVAL_MS[mode] ?? SYSTEM_INTERVAL_MS.normal);

  if (mode !== "eco") {
    tickNews(); // sofortiger erster Check statt bis zu 10/30 Minuten zu warten
    newsTimer = setInterval(tickNews, NEWS_INTERVAL_MS[mode] ?? NEWS_INTERVAL_MS.normal);
  } else {
    // Eco Mode: only refresh news when the dashboard is actually opened,
    // not on a background timer at all (section 11).
    tickNews();
  }
}

export function stopUpdateCycles() {
  clearInterval(systemTimer);
  clearInterval(newsTimer);
}

export function refreshNewsNow() {
  return api.news.refreshAll().then((cards) => newsCards.set(cards));
}
