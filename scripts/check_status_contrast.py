#!/usr/bin/env python3
"""Check every semantic status token in both themes for WCAG AA.

A status hue is used in TWO roles in this app:

  1. as TEXT on a wash of itself  (`text-error bg-error/10`)  -> needs 4.5:1
     against the COMPOSITED wash
  2. as a SOLID fill under an ink (`bg-error` + `text-error-fg`) -> needs 4.5:1
     between the fill and its ink

The 2026-10-20 red-team found the light theme's hues cleared role 1 against
the CARD but not against their own wash, so every tinted message box failed.
This script is the regression gate for that.

Usage: python3 scripts/check_status_contrast.py
Exit 1 on any failing pair.
"""

import re
import sys

CSS = "frontend/src/app.css"

# The wash opacities a status HUE is used as TEXT on in the app.
#
# The app's rule (the Badge component's) is: a tinted fill + a ring, with
# `text-content` for the text. The hue-as-text role survives only in the
# message boxes, which are `bg-*/10`. The /20 and /25 washes are used with
# `text-content`, so they are checked against the CONTENT ink, not the
# hue, and this gate does not have a job on them.
WASHES = [0.10]

# The washes a chip FILL is painted at, checked against the content ink
# that sits on it.
CHIP_WASHES = [0.10, 0.15, 0.20, 0.25]

SURFACES = {
    # The card a chip or a message box sits on.
    "card": {"dark": (19, 23, 32), "light": (255, 255, 255)},
    # The page behind it.
    "page": {"dark": (11, 14, 20), "light": (248, 250, 252)},
}

HUES = ["success", "warning", "error", "info"]


def lum(rgb):
    v = []
    for x in rgb:
        x = x / 255
        v.append(x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4)
    return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]


def ratio(a, b):
    la, lb = lum(a), lum(b)
    hi, lo = max(la, lb), min(la, lb)
    return (hi + 0.05) / (lo + 0.05)


def blend(fg, alpha, bg):
    return tuple(round(fg[i] * alpha + bg[i] * (1 - alpha)) for i in range(3))


def parse_themes(css: str) -> dict:
    # The dark block is `:root,\n:root.dark {` and the light one is
    # `:root.light {`. Both are found by their opening selector.
    themes = {}
    for name, opener in (("dark", ":root.dark {"), ("light", ":root.light {")):
        start = css.index(opener)
        depth = 0
        end = None
        for i in range(start, len(css)):
            if css[i] == "{":
                depth += 1
            elif css[i] == "}":
                depth -= 1
                if depth == 0:
                    end = i
                    break
        body = css[start:end]
        themes[name] = {
            k: v.strip() for k, v in re.findall(r"--([a-z0-9-]+):\s*([^;]+);", body)
        }
    return themes


def rgb(tok: str):
    parts = [int(x) for x in re.findall(r"\d+", tok)[:3]]
    return tuple(parts) if len(parts) == 3 else None


def main() -> int:
    css = open(CSS).read()
    themes = parse_themes(css)
    fails = []
    rows = []

    for theme in ("dark", "light"):
        toks = themes[theme]
        for hue in HUES:
            hue_tok = toks.get(f"{hue}-rgb")
            fg_tok = toks.get(f"{hue}-fg-rgb")
            if not hue_tok or not fg_tok:
                fails.append(f"{theme}/{hue}: token missing")
                continue
            c = rgb(hue_tok)
            ink = rgb(fg_tok)
            if not c or not ink:
                fails.append(f"{theme}/{hue}: token not a colour: {hue_tok!r} / {fg_tok!r}")
                continue

            # Role 1: the hue as TEXT on a wash of itself.
            for surf_name, surf in SURFACES.items():
                surface = surf[theme]
                for a in WASHES:
                    wash = blend(c, a, surface)
                    r = ratio(c, wash)
                    ok = r >= 4.5
                    rows.append((theme, hue, f"hue text on /{int(a * 100)} wash over {surf_name}",
                                 r, ok))
                    if not ok:
                        fails.append(
                            f"{theme}/{hue}: text {hue_tok} on its /{int(a * 100)} wash "
                            f"over {surf_name} = {r:.2f}:1 < 4.5:1"
                        )

            # Role 1b: a chip FILL at every wash the app uses, with the
            # content ink on it (the Badge pattern).
            content = {"dark": (228, 232, 240), "light": (30, 41, 59)}[theme]
            for surf_name, surf in SURFACES.items():
                surface = surf[theme]
                for a in CHIP_WASHES:
                    wash = blend(c, a, surface)
                    r = ratio(content, wash)
                    ok = r >= 4.5
                    rows.append((theme, hue,
                                 f"content ink on /{int(a * 100)} chip over {surf_name}",
                                 r, ok))
                    if not ok:
                        fails.append(
                            f"{theme}/{hue}: content ink on the /{int(a * 100)} chip "
                            f"over {surf_name} = {r:.2f}:1 < 4.5:1"
                        )

            # Role 2: the solid fill under its own ink.
            r = ratio(ink, c)
            ok = r >= 4.5
            rows.append((theme, hue, "ink on solid fill", r, ok))
            if not ok:
                fails.append(
                    f"{theme}/{hue}: ink {fg_tok} on the solid fill {hue_tok} "
                    f"= {r:.2f}:1 < 4.5:1"
                )

    for theme, hue, what, r, ok in rows:
        print(f"{'ok  ' if ok else 'FAIL'} {theme:5} {hue:8} {what:36} {r:5.2f}:1")
    print()
    if fails:
        print(f"{len(fails)} FAILING PAIR(S):")
        for f in fails:
            print("  " + f)
        return 1
    print(f"all {len(rows)} status-token pairs clear WCAG AA 4.5:1")
    return 0


if __name__ == "__main__":
    sys.exit(main())
