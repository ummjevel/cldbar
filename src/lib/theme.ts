export function applyTheme(theme: string) {
  if (theme === "system") {
    const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.dataset.theme = isDark ? "dark" : "light";
  } else {
    document.documentElement.dataset.theme = theme;
  }
}
