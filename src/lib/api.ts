import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  NewsCard,
  NowPlaying,
  Source,
  SystemStats,
  WeatherSnapshot,
} from "./types";

/**
 * Every call here maps 1:1 to a #[tauri::command] in src-tauri/src/commands.rs.
 * Widgets never call `invoke` directly — they go through this module, so the
 * Rust command surface can change without touching widget components.
 */
export const api = {
  settings: {
    get: () => invoke<AppSettings>("get_settings"),
    save: (settings: AppSettings) => invoke<void>("save_settings", { settings }),
  },

  system: {
    stats: () => invoke<SystemStats>("get_system_stats"),
  },

  weather: {
    current: (lat: number, lon: number) =>
      invoke<WeatherSnapshot>("get_weather", { lat, lon }),
  },

  sources: {
    list: () => invoke<Source[]>("list_sources"),
    add: (url: string, category: string) =>
      invoke<Source>("add_source", { url, category }),
    remove: (id: string) => invoke<void>("remove_source", { id }),
    /** Triggers a manual "check now" for a single source, bypassing the interval. */
    refresh: (id: string) => invoke<NewsCard[]>("refresh_source", { id }),
  },

  news: {
    /** Cards already known/cached — cheap, no network, no AI call. */
    cached: () => invoke<NewsCard[]>("get_cached_news"),
    /** Runs the full pipeline: check → diff → AI summarize → categorize. */
    refreshAll: () => invoke<NewsCard[]>("refresh_all_sources"),
  },

  ai: {
    /** Practical default: paste an API key from platform.openai.com/api-keys. */
    signInWithApiKey: (key: string) => invoke<void>("openai_sign_in_with_api_key", { key }),
    /** "Sign in with ChatGPT" — opens the system browser (see core::auth for status). */
    signInOAuth: () => invoke<void>("openai_sign_in_oauth"),
    signOut: () => invoke<void>("openai_sign_out"),
    isSignedIn: () => invoke<boolean>("openai_is_signed_in"),
  },

  media: {
    /** Reads via MPRIS on Linux — picks up Spotify, and any browser tab
     *  playing audio through the Media Session API (Deezer, YouTube Music, ...).
     *  `preferred` is an MPRIS identity substring (e.g. "Spotify", "Brave")
     *  to control a specific app when several are active at once. */
    nowPlaying: (preferred?: string) => invoke<NowPlaying | null>("get_now_playing", { preferred }),
    playPause: (preferred?: string) => invoke<void>("media_play_pause", { preferred }),
    next: (preferred?: string) => invoke<void>("media_next", { preferred }),
    previous: (preferred?: string) => invoke<void>("media_previous", { preferred }),
    getVolume: (preferred?: string) => invoke<number | null>("media_get_volume", { preferred }),
    setVolume: (level: number, preferred?: string) =>
      invoke<void>("media_set_volume", { level, preferred }),
    listPlayers: () => invoke<string[]>("media_list_players"),
  },
};
