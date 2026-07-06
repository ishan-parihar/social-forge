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
        // Semantic colors — use CSS variables so the theme toggle works.
        // The variables are defined in app.css under :root.dark and :root.light.
        surface: "var(--bg-card)",
        "surface-hover": "var(--bg-hover)",
        background: "var(--bg)",
        "background-input": "var(--bg-input)",
        line: "var(--border)",
        "line-hover": "var(--border-hover)",
        muted: "var(--text-muted)",
        "muted-dark": "var(--text-muted-dark)",
        content: "var(--text)",
        "content-secondary": "var(--text-secondary)",
      },
    },
  },
  plugins: [],
};
