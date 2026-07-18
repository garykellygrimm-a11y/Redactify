// Theme handling: respect the OS on first launch, then honor the user's
// explicit choice. The <html data-theme> attribute drives every token.

const STORAGE_KEY = "redactify-theme";
type Theme = "light" | "dark";

export function initTheme(): Theme {
  const stored = window.localStorage.getItem(STORAGE_KEY) as Theme | null;
  const theme =
    stored ??
    (window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light");
  document.documentElement.dataset.theme = theme;
  return theme;
}

export function toggleTheme(): Theme {
  const next: Theme =
    document.documentElement.dataset.theme === "dark" ? "light" : "dark";
  document.documentElement.dataset.theme = next;
  window.localStorage.setItem(STORAGE_KEY, next);
  return next;
}
