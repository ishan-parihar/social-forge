// Theme store — dark/light mode toggle.
// Stores preference in localStorage, defaults to dark.
// Applies by toggling .dark / .light class on documentElement.

import { browser } from '$app/environment';

const STORAGE_KEY = 'social-forge-theme';

type Theme = 'dark' | 'light';

function getInitial(): Theme {
  if (!browser) return 'dark';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === 'light' || stored === 'dark') return stored;
  return 'dark';
}

class ThemeStore {
  value = $state<Theme>(getInitial());

  set(theme: Theme) {
    this.value = theme;
    if (browser) {
      localStorage.setItem(STORAGE_KEY, theme);
      document.documentElement.classList.remove('dark', 'light');
      document.documentElement.classList.add(theme);
    }
  }

  toggle() {
    this.set(this.value === 'dark' ? 'light' : 'dark');
  }

  init() {
    if (browser) {
      document.documentElement.classList.remove('dark', 'light');
      document.documentElement.classList.add(this.value);
    }
  }
}

export const theme = new ThemeStore();
