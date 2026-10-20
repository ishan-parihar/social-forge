#!/usr/bin/env python3
"""Give the hand-rolled dialogs the shared dialog contract.

The dialogs that use `Modal.svelte` or `ModalManager.svelte` already
have: a focus trap, `aria-modal`, an accessible name, and a themed
scrim.

Seven hand-rolled dialogs did not:

  automation          Create/Edit Automation Rule, Execution Logs
  campaigns           Create Campaign
  comments            Reply
  search              Save Search
  settings/webhooks   Create/Edit Webhook, Delivery log

Each one was:
  - a `role="dialog"` with NO focus trap, so Tab walked out of the
    dialog and into the page behind it
  - missing `aria-modal="true"`, so assistive technology did not know
    the rest of the page was inert
  - missing an accessible NAME, so a screen reader announced "dialog"
  - painted on a hardcoded `bg-black/60`, which does not theme - the
    light theme got the same scrim as the dark one, and AGENTS.md 9.7
    forbids a raw colour in a component

The fix is the shape the working dialogs use: the SCRIM is a separate,
aria-hidden button and the PANEL is its sibling, so the focus trap can
find the panel's focusable children.

Usage: python3 scripts/fix_dialogs.py
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# file -> [(the panel's opening class, the heading class, the id, the close expression)]
DIALOGS = [
    (
        "frontend/src/routes/automation/+page.svelte",
        [
            ("bg-background-input border border-line rounded-xl p-6 w-full max-w-md", "automation-rule",
             "showModal = false"),
            ("bg-background-input border border-line rounded-xl p-6 w-full max-w-lg", "automation-logs",
             "showLogs = null"),
        ],
    ),
    (
        "frontend/src/routes/campaigns/+page.svelte",
        [("bg-background-input border border-line rounded-xl p-6 w-full max-w-md", "create-campaign",
          "showCreateModal = false")],
    ),
    (
        "frontend/src/routes/comments/+page.svelte",
        [("bg-background-input border border-line rounded-xl p-6 w-full max-w-md", "comment-reply",
          "replyModal = null")],
    ),
    (
        "frontend/src/routes/search/+page.svelte",
        [("bg-background-input border border-line rounded-xl p-6 w-full max-w-sm", "save-search",
          "showSaveDialog = false")],
    ),
    (
        "frontend/src/routes/settings/webhooks/+page.svelte",
        [
            ("bg-background-input border border-line rounded-xl p-6 w-full max-w-md", "webhook",
             "showModal = false"),
            ("bg-background-input border border-line rounded-xl p-6 w-full max-w-2xl", "webhook-deliveries",
             "showDeliveries = null"),
        ],
    ),
]

NOTE = """<!--
  The scrim is a separate, aria-hidden BUTTON and the panel is its
  SIBLING.

  The wrapper used to carry `aria-hidden="true"`, which made the whole
  dialog invisible to a screen reader, and the shared focus trap skips
  anything in an aria-hidden subtree - so it found no focusable child
  and the focus never entered the dialog. See AGENTS.md 9.39.

  The scrim carries the aria-hidden and the tabindex="-1": a screen
  reader should not offer a nameless "close" button, and Escape
  already closes the dialog.
-->"""


def main() -> int:
    changed = []
    for rel, dialogs in DIALOGS:
        p = ROOT / rel
        s = p.read_text()
        for panel_class, ident, close_expr in dialogs:
            # The panel's opening tag.
            marker = f'<div class="{panel_class}"'
            at = s.find(marker)
            if at < 0:
                print(f"SKIP {rel}: no panel {panel_class[:40]!r}", file=sys.stderr)
                continue
            panel_tag_end = s.find('>', at) + 1
            panel_open = s[at:panel_tag_end]
            if "focusTrap" in panel_open:
                print(f"SKIP {rel}/{ident}: already trapped", file=sys.stderr)
                continue

            # The wrapper is the tag before the panel.
            wrapper_start = s.rfind('<div class="fixed inset-0', 0, at)
            if wrapper_start < 0:
                print(f"SKIP {rel}/{ident}: no wrapper", file=sys.stderr)
                continue
            wrapper_tag_end = s.find('>', wrapper_start) + 1
            wrapper = s[wrapper_start:wrapper_tag_end]
            if "role=\"dialog\"" not in wrapper:
                print(f"SKIP {rel}/{ident}: the wrapper is not the dialog", file=sys.stderr)
                continue

            new_wrapper = (
                '  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">\n'
                f'    {NOTE}\n'
                '    <button\n'
                '      type="button"\n'
                '      class="absolute inset-0 bg-overlay cursor-default"\n'
                '      aria-hidden="true"\n'
                '      tabindex="-1"\n'
                f'      onclick={{() => {{ {close_expr} }}}}\n'
                '    ></button>\n'
            )
            panel_new = (
                panel_open[:-1].rstrip()
                + '\n      role="dialog"'
                + '\n      aria-modal="true"'
                + f'\n      aria-labelledby="{ident}-title"'
                + '\n      tabindex="-1"'
                + '\n      use:focusTrap'
                + '\n    >'
            )
            s = s[:wrapper_start] + new_wrapper + panel_new + s[panel_tag_end:]

            # The heading carries the id the name points at. It is the
            # first <h2> after the panel.
            h2 = s.find('<h2', panel_tag_end)
            if h2 >= 0 and 'id="' not in s[h2:h2 + 120]:
                h2_tag_end = s.find('>', h2) + 1
                h2_tag = s[h2:h2_tag_end]
                s = s[:h2] + h2_tag.replace('<h2', f'<h2 id="{ident}-title"', 1) + s[h2_tag_end:]

            changed.append(f"{rel}  ({ident})")

        # The import, once per file.
        if "use:focusTrap" in s and "focus-trap" not in s:
            s = re.sub(
                r"(\n(\s*)import [^\n]*\n)",
                r"\1\2import { focusTrap } from '$lib/ui/focus-trap';\n",
                s,
                count=1,
            )
        p.write_text(s)

    print(f"fixed {len(changed)} dialog(s):")
    for c in changed:
        print("  " + c)
    return 0


if __name__ == "__main__":
    sys.exit(main())
