import { derived } from "svelte/store";
import { settingsStore } from "./settings";

const CORNER_RADIUS: Record<string, string> = {
  sharp: "0px",
  soft: "10px",
  round: "22px",
};

/**
 * Applies the user's personalization choices (section 15) to the document
 * root as CSS variables, so every widget picks them up automatically
 * without each widget re-implementing theme logic.
 */
export const appliedTheme = derived(settingsStore, ($settings) => {
  if (typeof document !== "undefined") {
    const root = document.documentElement;
    root.setAttribute(
      "data-theme",
      $settings.theme === "system" ? systemPrefersDark() : $settings.theme
    );
    root.style.setProperty("--radius", CORNER_RADIUS[$settings.cornerStyle] ?? "10px");
    root.style.setProperty("--color-accent", $settings.accentColor);
  }
  return $settings;
});

function systemPrefersDark(): "dark" | "light" {
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
