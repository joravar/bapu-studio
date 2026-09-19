export type Theme = 'dark' | 'light';

const THEME_KEY = 'bapu_theme';

export function getStoredTheme(): Theme {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === 'light' || saved === 'dark') return saved;
  } catch {}
  return 'dark';
}

// Applied synchronously before React renders (see main.tsx) as well as on toggle, so the correct
// theme's CSS variables are already in effect for first paint instead of flashing dark-then-light.
export function applyTheme(theme: Theme): void {
  document.documentElement.setAttribute('data-theme', theme);
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {}
}
