import { $ } from "./dom.ts";

const KEY = "deinon-theme";

type Theme = "dark" | "light";

function apply(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  $("themeIcon").textContent = theme === "dark" ? "light_mode" : "dark_mode";
}

export function loadTheme() {
  let saved: string | null = null;
  try {
    saved = localStorage.getItem(KEY);
  } catch {
    // Storage can be unavailable. The default theme is fine then.
  }
  apply(saved === "light" ? "light" : "dark");
}

export function toggleTheme() {
  const next: Theme = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
  apply(next);
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // The theme still applies for this session.
  }
}
