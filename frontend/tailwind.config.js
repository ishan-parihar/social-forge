/** @type {import('tailwindcss').Config} */

// v25 F1 — semantic token layer.
//
// Rule: app-chrome colors come from `colors.*` below, never from a raw hex or
// a dark-tuned Tailwind palette step. Every value resolves to a CSS variable
// declared in `src/app.css`, so switching `.dark` / `.light` on <html> is the
// whole theme mechanism. Run `pnpm check:tokens` to enforce it.
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
        //
        // v25 F4: every entry is a `rgb(var(--x-rgb) / <alpha-value>)` triplet
        // rather than a bare `var(--x)`. Tailwind 3 can only synthesise an
        // opacity modifier (`bg-surface/50`) from the triplet form — given a
        // bare `var(--surface)` it silently emits NOTHING, so the class is a
        // no-op and the element renders with no background at all. That had
        // already broken ~60 call sites (bars, hovers, washes) across
        // composer/calendar/kanban/channels/analytics, including the
        // analytics channel bars, which rendered as empty tracks in BOTH
        // themes and read as "no data" rather than "broken class". Fixing it
        // here repairs every call site without editing the component that
        // asked for the alpha. The `-rgb` triplets live in app.css next to
        // their hex, so the two cannot drift.
        surface: "rgb(var(--bg-card-rgb) / <alpha-value>)",
        "surface-hover": "rgb(var(--bg-hover-rgb) / <alpha-value>)",
        background: "rgb(var(--bg-rgb) / <alpha-value>)",
        "background-input": "rgb(var(--bg-input-rgb) / <alpha-value>)",
        line: "rgb(var(--border-rgb) / <alpha-value>)",
        "line-hover": "rgb(var(--border-hover-rgb) / <alpha-value>)",
        muted: "rgb(var(--text-muted-rgb) / <alpha-value>)",
        content: "rgb(var(--text-rgb) / <alpha-value>)",
        "content-secondary": "rgb(var(--text-secondary-rgb) / <alpha-value>)",
        // v25 F1: `muted-dark` renamed to `faint`. It was always the faintest
        // text tier (it flips value with the theme), so the old name misled.
        // The alias stays one release so no call site breaks.
        faint: "rgb(var(--text-faint-rgb) / <alpha-value>)",
        "muted-dark": "rgb(var(--text-faint-rgb) / <alpha-value>)",
        // v25 F1: accent is split into FILL and FOREGROUND, because a single
        // brand value cannot clear 4.5:1 as text in both themes.
        //   accent-fill*       → backgrounds (buttons, bar fills)
        //   accent/accent-strong → text, borders, rings (AA in both themes)
        accent: "rgb(var(--accent-rgb) / <alpha-value>)",
        "accent-strong": "rgb(var(--accent-strong-rgb) / <alpha-value>)",
        "accent-fill": "rgb(var(--brand-rgb) / <alpha-value>)",
        "accent-fill-hover": "rgb(var(--brand-hover-rgb) / <alpha-value>)",
        "accent-fg": "rgb(var(--brand-fg-rgb) / <alpha-value>)",
        "accent-soft": "var(--brand-soft)",
        // v22 Phase 3: semantic status colors (success/warning/error/info).
        // Exposed as Tailwind colors so components can use `bg-success/20`,
        // `text-error`, `border-warning`, etc. The CSS variables are
        // defined per-theme in app.css.
        success: "rgb(var(--success-rgb) / <alpha-value>)",
        warning: "rgb(var(--warning-rgb) / <alpha-value>)",
        error: "rgb(var(--error-rgb) / <alpha-value>)",
        info: "rgb(var(--info-rgb) / <alpha-value>)",
        // v25 F1: metric colors. Engagement numbers keep one meaning app-wide,
        // so the ramp is declared once instead of per call site.
        "viz-like": "rgb(var(--viz-like-rgb) / <alpha-value>)",
        "viz-comment": "rgb(var(--viz-comment-rgb) / <alpha-value>)",
        "viz-share": "rgb(var(--viz-share-rgb) / <alpha-value>)",
        "viz-view": "rgb(var(--viz-view-rgb) / <alpha-value>)",
        "viz-impression": "rgb(var(--viz-impression-rgb) / <alpha-value>)",
        "viz-click": "rgb(var(--viz-click-rgb) / <alpha-value>)",
        // v25 F1: categorical chip hues (TargetPicker, ChannelCard, RSS status).
        "hue-blue": "rgb(var(--hue-blue-rgb) / <alpha-value>)",
        "hue-violet": "rgb(var(--hue-violet-rgb) / <alpha-value>)",
        "hue-teal": "rgb(var(--hue-teal-rgb) / <alpha-value>)",
        "hue-cyan": "rgb(var(--hue-cyan-rgb) / <alpha-value>)",
        "hue-emerald": "rgb(var(--hue-emerald-rgb) / <alpha-value>)",
        "hue-amber": "rgb(var(--hue-amber-rgb) / <alpha-value>)",
        "hue-neutral": "rgb(var(--hue-neutral-rgb) / <alpha-value>)",
        // v25 F1: scrim token. `bg-black/60` is a pure-black overlay that
        // swallows detail on a light page; this tints with the theme instead.
        overlay: "var(--overlay)",
      },
      // v25 F1: the shadow scale reads from the theme, so shadows tint with the
      // surface instead of leaving a pure-black smear on a white page.
      boxShadow: {
        sm: "var(--shadow-sm)",
        md: "var(--shadow-md)",
        lg: "var(--shadow-lg)",
      },
      borderRadius: {
        // v22 Phase 3: radius scale tokens. Use `rounded-sm`, `rounded-md`,
        // `rounded-lg` instead of hardcoded `rounded-[8px]` etc.
        sm: "var(--radius-sm)",
        md: "var(--radius-md)",
        lg: "var(--radius-lg)",
      },
    },
  },
  plugins: [],
};
