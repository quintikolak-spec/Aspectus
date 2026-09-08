import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  NewsCard,
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
};
