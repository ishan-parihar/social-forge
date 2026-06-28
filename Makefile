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

.PHONY: build frontend deploy redeploy restart status logs watch

# ── Build ───────────────────────────────────────────────────────

build: frontend
	cargo build --release

frontend:
	cd frontend && pnpm install && pnpm build

# ── Deploy / Redeploy ───────────────────────────────────────────

# Full deploy (build frontend + Rust, install skill, then restart)
deploy: build install-skill
	sudo install -m 755 target/release/$(APP_NAME) $(BINARY_PATH)
	sudo systemctl daemon-reload
	sudo systemctl restart $(APP_NAME)
	@echo "✓ Deployed $(APP_NAME)"

# One-step redeploy — the daily driver for active development.
# Skips frontend build for speed; use `make deploy` when frontend changes.
redeploy: install-skill
	cargo build --release && sudo install -m 755 target/release/$(APP_NAME) $(BINARY_PATH) && sudo systemctl restart $(APP_NAME)

# ── Service Management ──────────────────────────────────────────

restart:
	sudo systemctl daemon-reload
	sudo systemctl restart $(APP_NAME)

status:
	@echo "=== systemd ==="
	systemctl status $(APP_NAME) --no-pager || true
	@echo ""
	@echo "=== Docker ==="
	docker compose ps

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
	cargo watch -x 'build --release' -s 'sudo install -m 755 target/release/$(APP_NAME) $(BINARY_PATH) && sudo systemctl restart $(APP_NAME)'
