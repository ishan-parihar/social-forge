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

.PHONY: build frontend vendor-pins check deploy redeploy restart status logs watch

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
check: vendor-pins
	SQLX_OFFLINE=true cargo check --lib --bin social-forge

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
	@echo "✓ Installed skill to $(SKILL_DEST)"

# ── Auto-watch (requires cargo-watch) ──────────────────────────
#   cargo install cargo-watch
watch:
	cargo watch -x 'zigbuild --release --target $(TARGET)' -s 'sudo install -m 755 $(RELEASE_DIR)/$(APP_NAME) $(BINARY_PATH) && sudo systemctl restart $(APP_NAME)'
