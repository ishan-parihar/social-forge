/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  theme: {
    extend: {
      colors: {
        brand: {
          50: "#eef2ff",
          100: "#e0e7ff",
          200: "#c7d2fe",
          300: "#a5b4fc",
          400: "#818cf8",
          500: "#6366f1",
          600: "#4f46e5",
          700: "#4338ca",
          800: "#3730a3",
          900: "#312e81",
        },
        // Semantic colors — single source of truth for the dark theme.
        // Usage: bg-surface, text-muted, border-line, etc.
        surface: "#131720",
        "surface-hover": "#1a1f2e",
        background: "#0b0e14",
        "background-input": "#0d1117",
        line: "#1e2435",
        "line-hover": "#2a3045",
        muted: "#6b7280",
        "muted-dark": "#4b5563",
        content: "#e8edf5",
        "content-secondary": "#d1d5db",
      },
    },
  },
  plugins: [],
};
