# ─── Social Forge Makefile ─────────────────────────────────────
# Development targets for fast iteration.
#
# Quick redeploy after code changes:
#   make redeploy
#
# That's it. One command builds + copies + restarts.

APP_NAME    = social-forge
APP_DIR     = $(shell pwd)
BINARY_PATH = /usr/local/bin/$(APP_NAME)
SKILL_SRC   = $(APP_DIR)/skills/social-forge-agent
SKILL_DEST  = $(HOME)/.agents/skills/social-forge-agent

# zigbuild target — glibc 2.36 matches Debian 12 (bookworm) VPS
TARGET      = x86_64-unknown-linux-gnu.2.36
RELEASE_DIR = target/x86_64-unknown-linux-gnu/release

.PHONY: build frontend vendor-pins check audit deploy redeploy restart status logs watch

# ── Build ───────────────────────────────────────────────────────

build: vendor-pins frontend
	cargo zigbuild --release --target $(TARGET)

frontend:
	cd frontend && pnpm install && pnpm build

# ── Vendor pins (plan v25 §9, phase B0) ─────────────────────────
# Verifies every vendored/pinned upstream source against the contract in
# docs/planning/VENDOR_PINS.md. Local drift is fatal (non-zero + diff).
# Upstream advisories (newer release, yanked pin) are reported loudly but do not
# block a local build; VENDOR_PULL_STRICT=1 makes them fatal too. CI uses the
# same gate — see VENDOR_PINS.md §1 for the ratchet to turn strict on for good.
# Run `scripts/vendor-pull.sh --update` after an intentional pin change.
vendor-pins:
	@./scripts/vendor-pull.sh --check

# Fast compile gate (AGENTS.md §3.2). SQLX_OFFLINE is required: the .sqlx/
# cache is committed, so the sqlx macros must not reach for a live DATABASE_URL.
#
# The two SOURCE gates run here, because both need the built frontend:
#
#   check-hex-tokens.mjs       a literal hex outside the token layer does
#                               not retheme, and cannot
#   check-tailwind-classes.mjs a Tailwind class the stylesheet does not
#                               define is a SILENT failure - the element
#                               simply does not get the style, and nothing
#                               reports it. A truncated class name
#                               (`hover:text-accent-stro`) is not a build
#                               error, not a type error and not a
#                               svelte-check warning.
check: vendor-pins
	SQLX_OFFLINE=true cargo check --lib --bin social-forge
	cd frontend && node scripts/check-hex-tokens.mjs
	cd frontend && node scripts/check-tailwind-classes.mjs

# ── Visual + a11y audits (AGENTS.md §12) ───────────────────────
#
# Five gates over the RENDERED app, not the source:
#
#   probe.mjs         the static page probe - 27 routes x 3 widths x
#                     2 themes, with the overflow, contrast, tap-target,
#                     heading, font and type-scale checks
#   audit-targets.mjs the rendered TARGETS, driving the real app so the
#                     composer, its panels and the popovers are reached
#   audit-contrast.mjs the rendered CONTRAST, same driving, measuring
#                     every text element against its EFFECTIVE
#                     background
#   audit-analytics.mjs  the ANALYTICS dashboard with a seeded account:
#                        non-zero engagement, a DENSE posting-volume
#                        series, and a success rate that RECONCILES with
#                        the numbers printed beside it.
#   audit-dialogs.mjs   the DIALOG contract: a name, aria-modal, the
#                     focus inside it, and Escape that closes it (or
#                     asks first)
#   e2e-calendar-drag.mjs
#                     the calendar's DRAG-to-reschedule, end to end: a
#                     real drag, the wall clock converted in the app's
#                     zone, and the move sticking after a reload
#
# They need a RUNNING instance and the Chromium under
# ~/.cache/ms-playwright. Each exits 3 (BLOCKED) when the browser is
# missing - never "passed".
#
#   make audit URL=https://localhost:6544 PASSWORD=...
#
# The two browser-side audits are the ones that find what the static
# probe cannot: the probe only ever renders a page's DEFAULT state.
audit:
	@test -n "$(URL)" || (echo "URL is required" && exit 2)
	@test -n "$(PASSWORD)" || (echo "PASSWORD is required" && exit 2)
	cd frontend && node scripts/probe.mjs --url $(URL) --password '$(PASSWORD)' --out /tmp/sf-probe
	cd frontend && node scripts/audit-targets.mjs --url $(URL) --password '$(PASSWORD)' --out /tmp/sf-targets.json
	cd frontend && node scripts/audit-contrast.mjs --url $(URL) --password '$(PASSWORD)' --theme dark --out /tmp/sf-contrast-dark.json
	cd frontend && node scripts/audit-contrast.mjs --url $(URL) --password '$(PASSWORD)' --theme light --out /tmp/sf-contrast-light.json
	cd frontend && node scripts/audit-dialogs.mjs --url $(URL) --password '$(PASSWORD)'
	cd frontend && node scripts/audit-analytics.mjs --url $(URL) --password '$(PASSWORD)'
	cd frontend && node scripts/e2e-calendar-drag.mjs --url $(URL) --password '$(PASSWORD)'

# ── Deploy / Redeploy ───────────────────────────────────────────

# Full deploy (build frontend + Rust, install skill, then restart)
deploy: build install-skill
	sudo install -m 755 $(RELEASE_DIR)/$(APP_NAME) $(BINARY_PATH)
	sudo systemctl daemon-reload
	sudo systemctl restart $(APP_NAME)
	@echo "✓ Deployed $(APP_NAME) (zigbuild, glibc 2.36)"

# One-step redeploy — the daily driver for active development.
# Skips frontend build for speed; use `make deploy` when frontend changes.
redeploy: install-skill
	cargo zigbuild --release --target $(TARGET) && sudo install -m 755 $(RELEASE_DIR)/$(APP_NAME) $(BINARY_PATH) && sudo systemctl restart $(APP_NAME)
	@echo "✓ Redeployed $(APP_NAME) (zigbuild, glibc 2.36)"

# ── Service Management ──────────────────────────────────────────

restart:
	sudo systemctl daemon-reload
	sudo systemctl restart $(APP_NAME)

status:
	@echo "=== systemd ==="
	systemctl status $(APP_NAME) --no-pager || true
	@echo ""
	@echo "=== database ==="
	@echo "$(APP_DIR)/data/social-forge.db"

logs:
	journalctl -u $(APP_NAME) -n 50 --no-pager -f

# ── Install AI Agent Skill ──────────────────────────────────────
install-skill:
	@mkdir -p $(SKILL_DEST)/references
	@cp $(SKILL_SRC)/SKILL.md $(SKILL_DEST)/SKILL.md
	@cp $(SKILL_SRC)/references/providers.md $(SKILL_DEST)/references/providers.md
	@cp $(SKILL_SRC)/references/platform-matrices.md $(SKILL_DEST)/references/platform-matrices.md
	@cp $(SKILL_SRC)/references/recipes-extended.md $(SKILL_DEST)/references/recipes-extended.md
	@echo "✓ Installed skill to $(SKILL_DEST)"

# ── Auto-watch (requires cargo-watch) ──────────────────────────
#   cargo install cargo-watch
watch:
	cargo watch -x 'zigbuild --release --target $(TARGET)' -s 'sudo install -m 755 $(RELEASE_DIR)/$(APP_NAME) $(BINARY_PATH) && sudo systemctl restart $(APP_NAME)'
