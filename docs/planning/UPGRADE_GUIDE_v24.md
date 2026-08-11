# Postgres Migration & Deployment Guide (v22 → v24)

> **Date**: 2026-07-08
> **Scope**: Database schema changes from the v22/v23/v24 upgrade cycle (migrations 031–036) + deployment verification checklist.
> **Audience**: Operators upgrading an existing social-forge deployment, or deploying fresh.

---

## 0. Executive Summary

**For fresh installs**: Nothing to do. The `install.sh` script + `social-forge-start.sh` handle everything — Postgres starts via Docker, the binary connects, and `sqlx::migrate!` applies all 36 migrations automatically on first boot. No manual `psql` scripting needed.

**For existing installs upgrading**: Pull the latest binary, restart the service. The binary detects pending migrations and applies them automatically. The 6 new migrations (031–036) are additive-only (no destructive ops, no column renames, no type changes) and are safe to apply on a running system with minimal downtime (typically < 1 second).

**No manual Postgres scripting is required for any scenario.**

---

## 1. How Migrations Work in Social Forge

Social Forge uses `sqlx::migrate!("./migrations")` which **embeds all migration SQL files into the binary at compile time**. The `./migrations` path is resolved relative to `Cargo.toml` during `cargo build`, not at runtime.

**What this means:**
- The migration files do NOT need to be present on the deployment server.
- The binary does NOT read migration files from disk at runtime.
- On every startup, `create_pool()` in `src/db/mod.rs` calls `sqlx::migrate!().run(&pool)` which:
  1. Checks the `_sqlx_migrations` table for already-applied migrations.
  2. Applies any pending migrations in order (lexicographic by filename).
  3. Records each applied migration in `_sqlx_migrations`.
- Migrations are **idempotent** — running them twice is safe (the `_sqlx_migrations` table prevents re-application).
- Migrations are **additive-only** per AGENTS.md convention — no destructive operations, no column drops, no type changes.

**Startup flow:**
```
social-forge-start.sh
  → docker compose up -d postgres     (starts Postgres in Docker)
  → wait for pg_isready               (up to 60s)
  → exec social-forge serve           (starts the binary)
    → create_pool(DATABASE_URL)        (connects to Postgres)
    → sqlx::migrate!().run(&pool)      (applies pending migrations automatically)
    → ensure_local_user(&pool)         (creates the DEFAULT_USER_ID row if missing)
    → axum server starts               (HTTP + SSE + scheduler + background tasks)
```

---

## 2. New Migrations (031–036)

These are the 6 migrations added during the v22/v23/v24 upgrade cycle. All are additive (new tables, new columns with defaults, new indexes). None are destructive.

### 031_publish_outbox.sql (v22 Phase 2)

**Purpose**: Transactional outbox for publish durability. If `provider.publish()` succeeds but the DB write fails, the outbox drain loop retries the write.

```sql
CREATE TABLE publish_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    idempotency_key UUID NOT NULL,
    platform_post_id  TEXT,
    platform_post_url TEXT,
    published_at      TIMESTAMPTZ,
    error_message     TEXT,
    attempts          INT NOT NULL DEFAULT 0,
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at      TIMESTAMPTZ
);
CREATE INDEX idx_publish_outbox_pending ON publish_outbox(next_attempt_at) WHERE completed_at IS NULL;
CREATE INDEX idx_publish_outbox_post_id ON publish_outbox(post_id);
CREATE INDEX idx_publish_outbox_idempotency ON publish_outbox(idempotency_key) WHERE platform_post_id IS NOT NULL;
```

**Impact**: New table. No existing data affected. Adds ~0 storage overhead until publishes start writing to it.

### 032_posts_workflow_version.sql (v22 Phase 2)

**Purpose**: Workflow versioning for the publish state machine (enables in-flight migration of publish logic).

```sql
ALTER TABLE posts ADD COLUMN publish_workflow_version INT NOT NULL DEFAULT 1;
UPDATE posts SET publish_workflow_version = 2 WHERE state IN ('queued', 'publishing') AND deleted_at IS NULL;
ALTER TABLE posts ALTER COLUMN publish_workflow_version SET DEFAULT 2;
```

**Impact**: New column with default. Existing rows get version 1 (or 2 if queued/publishing). No data loss.

### 033_campaign_expansion.sql (v22 Phase 6)

**Purpose**: Expands the campaigns table with strategic-dashboard fields (status, progress tracking, audience persona, content pillars, budget, KPI targets, soft-delete).

```sql
ALTER TABLE campaigns ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
  CHECK (status IN ('active', 'paused', 'archived', 'completed'));
ALTER TABLE campaigns ADD COLUMN progress_metric TEXT;
ALTER TABLE campaigns ADD COLUMN progress_target INT;
ALTER TABLE campaigns ADD COLUMN audience_persona JSONB;
ALTER TABLE campaigns ADD COLUMN content_pillars JSONB;
ALTER TABLE campaigns ADD COLUMN budget_cents INT;
ALTER TABLE campaigns ADD COLUMN kpi_targets JSONB;
ALTER TABLE campaigns ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE campaigns ADD COLUMN sort_order INT NOT NULL DEFAULT 0;
CREATE INDEX idx_campaigns_status ON campaigns(user_id, status) WHERE deleted_at IS NULL;
CREATE INDEX idx_campaigns_not_deleted ON campaigns(user_id) WHERE deleted_at IS NULL;
```

**Impact**: New columns with defaults/NULLs. Existing campaigns get `status='active'`, `sort_order=0`. No data loss.

### 034_posts_kanban_fields.sql (v22 Phase 6)

**Purpose**: Adds kanban-specific fields to posts (sort order, sub-state, due date, priority).

```sql
ALTER TABLE posts ADD COLUMN kanban_sort_order INT NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN kanban_substate TEXT
  CHECK (kanban_substate IS NULL OR kanban_substate IN ('ready_to_publish', 'in_review', 'blocked'));
ALTER TABLE posts ADD COLUMN due_date TIMESTAMPTZ;
ALTER TABLE posts ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium'
  CHECK (priority IN ('low', 'medium', 'high', 'urgent'));
CREATE INDEX idx_posts_kanban_order ON posts(user_id, state, kanban_sort_order) WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_due_date ON posts(user_id, due_date) WHERE due_date IS NOT NULL AND deleted_at IS NULL;
```

**Impact**: New columns with defaults. Existing posts get `kanban_sort_order=0`, `priority='medium'`. No data loss.

### 035_events_log.sql (v23-1)

**Purpose**: Events log table for the dashboard's "Recent Activity" widget. Persists the last 7 days of events so they're available even when no SSE client was connected.

```sql
CREATE TABLE events_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type  TEXT NOT NULL,
    payload     JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_events_log_user_created ON events_log(user_id, created_at DESC);
CREATE INDEX idx_events_log_created ON events_log(created_at);
```

**Impact**: New table. No existing data affected. A cleanup task (runs every 6h) trims rows older than 7 days (configurable via `EVENTS_LOG_RETENTION_DAYS`).

### 036_brand_profiles.sql (v24-4)

**Purpose**: Brand profile table (brand name, tone of voice, audience, content pillars, keywords, hashtag sets, avoid topics, posting frequency goal). Synced across devices; read by the AiAssistant as context.

```sql
CREATE TABLE brand_profiles (
    user_id             UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    brand_name          TEXT,
    description         TEXT,
    tone_of_voice       TEXT,
    audience            TEXT,
    content_pillars     JSONB,
    keywords            JSONB,
    hashtag_sets        JSONB,
    avoid_topics        JSONB,
    posting_frequency   TEXT,
    posts_per_day_goal  DOUBLE PRECISION,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Impact**: New table. No existing data affected. Single row per user (PK on `user_id`).

---

## 3. Upgrade Procedure (Existing Installations)

### Step 1: Stop the service
```bash
sudo systemctl stop social-forge
```

### Step 2: Pull the latest binary
```bash
# Option A: Re-run the install script (downloads latest release)
curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/master/scripts/install.sh | bash

# Option B: Download a specific version
VERSION=v0.3.0 curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/master/scripts/install.sh | bash

# Option C: Build from source
cd social-forge
git pull origin master
cd frontend && pnpm install && pnpm build && cd ..
cargo build --release
sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge
```

### Step 3: Start the service
```bash
sudo systemctl start social-forge
```

### Step 4: Verify migrations applied
```bash
# Check the logs for the migration message
sudo journalctl -u social-forge --since "1 min ago" | grep -i "migration"

# Expected output:
# ... Database connected — pool: max=20, ... Migrations applied.

# Verify the new tables exist
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge -c "\dt"
# Expected: publish_outbox, events_log, brand_profiles should be listed

# Verify the new columns exist
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge -c "\d posts" | grep -E "publish_workflow_version|kanban_sort_order|kanban_substate|due_date|priority"
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge -c "\d campaigns" | grep -E "status|progress_metric|progress_target|audience_persona|content_pillars|budget_cents|kpi_targets|deleted_at|sort_order"
```

### Step 5: Verify the app is working
```bash
# Health check
curl -sk https://localhost:6543/health
# Expected: {"status":"ok"}

# Ready check
curl -sk https://localhost:6543/ready
# Expected: {"status":"ready","database":"connected"}

# Open the dashboard
open https://localhost:6543
```

---

## 4. Fresh Install Procedure

```bash
# One command — the installer does everything:
curl -fsSL https://raw.githubusercontent.com/ishan-parihar/social-forge/master/scripts/install.sh | bash

# Then:
cd ~/social-forge
docker compose up -d postgres    # Start Postgres
sudo systemctl enable social-forge --now   # Start the app (auto-runs migrations)
```

**What the installer does:**
1. Downloads the pre-built binary from GitHub Releases.
2. Creates `~/social-forge/` with `.env`, `docker-compose.yml`, `data/` dirs.
3. Installs the binary to `/usr/local/bin/social-forge`.
4. Installs `postgresql-client` (for `pg_isready`).
5. Installs the systemd service + start script.
6. Installs the AI agent skill.

**What the start script does:**
1. Sources `.env`.
2. Starts Postgres via `docker compose up -d postgres`.
3. Waits for Postgres to be ready (up to 60s).
4. Launches `social-forge serve`.

**What the binary does on first boot:**
1. Connects to Postgres.
2. Runs `sqlx::migrate!()` — applies all 36 migrations automatically.
3. Creates the local user row (`DEFAULT_USER_ID`).
4. Starts the HTTP server, scheduler, SSE broadcaster, and background tasks.

**No manual `psql` commands needed. No migration scripts to run. No database configuration beyond what `docker-compose.yml` provides.**

---

## 5. Post-Upgrade Verification Checklist

After upgrading, verify these critical flows:

### Backend
- [ ] `curl -sk https://localhost:6543/health` returns `{"status":"ok"}`
- [ ] `curl -sk https://localhost:6543/ready` returns `{"status":"ready","database":"connected"}`
- [ ] `sudo journalctl -u social-forge --since "5 min ago" | grep -i error` shows no errors
- [ ] `sudo journalctl -u social-forge --since "5 min ago" | grep "Migrations applied"` confirms migrations ran

### Dashboard
- [ ] Dashboard loads at `https://localhost:6543`
- [ ] Stat cards (Drafts/Queued/Published/Errors) show correct counts
- [ ] "Scheduled vs Actual" widget shows adherence data (or empty if no posts)
- [ ] "Posting Cadence" widget shows streak + posts/day
- [ ] "Recent Events" widget shows recent activity (or empty if no events yet)

### Calendar
- [ ] Calendar loads with the correct default view (Week)
- [ ] Channel filter dropdown works (selects a channel → calendar filters)
- [ ] Campaign filter dropdown works (selects a campaign → calendar filters)
- [ ] Drag-and-drop reschedule works
- [ ] "Just update vs Reschedule" modal appears when dragging a published post

### Composer
- [ ] "New Post" button opens the composer modal
- [ ] Channel selector shows connected integrations
- [ ] Per-platform char count badges show (X uses weighted length)
- [ ] Platform preview pane shows the correct preview per channel (X, Reddit, Threads, Bluesky, Instagram, LinkedIn, Facebook)
- [ ] Underline (U) button works in the rich text editor
- [ ] Alt text on media persists after save
- [ ] Save as Draft works (creates a draft)
- [ ] Schedule works (creates a queued post)
- [ ] Edit mode: editing a post and clicking "Save as Draft" properly unschedules it

### Kanban
- [ ] Kanban board loads with 4 columns (Ideas/Drafts/Scheduled/Published)
- [ ] Quick-add to Ideas requires a channel selection (no more empty integration_ids)
- [ ] Drag between columns works (state-transition validation rejects illegal moves)
- [ ] Campaign filter works (selects a campaign → only that campaign's posts show)
- [ ] Multi-tab sync: dragging in tab A updates tab B within 1s

### Campaigns
- [ ] `/campaigns` list page loads with campaign cards
- [ ] Clicking a campaign card navigates to `/campaigns/[id]`
- [ ] Campaign detail page shows Overview/Posts/Settings tabs
- [ ] Settings tab: editing fields + Save works
- [ ] Archive button soft-deletes the campaign

### Feed
- [ ] Feed loads with imported posts
- [ ] "View original" link opens the post on the platform
- [ ] "Manage on {platform}" link opens the post's management URL
- [ ] Repurpose button creates a draft + opens the composer
- [ ] Hide (delete) button works

### Settings
- [ ] Settings page shows a single sidebar (not duplicated)
- [ ] Brand Profile page saves to backend (not just localStorage)
- [ ] Sidebar collapse toggle works (w-56 ↔ w-14)
- [ ] Cmd+K opens the command palette

### Sidebar
- [ ] Only ONE "Settings" entry in the main sidebar (not 8)
- [ ] Active-link highlighting works on sub-routes (e.g. `/settings/profile` highlights "Settings")
- [ ] Collapse-to-icon-rail toggle persists across reloads
- [ ] Command palette (Cmd+K) opens, search works, Enter navigates

### Theme
- [ ] Dark mode renders correctly (all components use semantic tokens)
- [ ] Light mode renders correctly (toggle in sidebar footer)
- [ ] No hardcoded hex colors visible (all use CSS variables)
- [ ] Button colors retheme in light mode (primary CTA, secondary, ghost, danger)

### Realtime
- [ ] SSE connection established (check Network tab for `/api/events`)
- [ ] Publishing a post in one tab updates the calendar in another tab within 1s
- [ ] Dragging a kanban card in one tab updates the kanban in another tab within 1s

### Security
- [ ] `curl -sk https://localhost:6543/api/events` without a cookie returns 401 (not 200)
- [ ] `PUT /api/integrations/{id}/timeslots` with `minutes: 99999` returns 400 (not 500)
- [ ] `PATCH /api/posts/{id}/stage` with `state: "published"` on a non-published post returns 400

---

## 6. Rollback Procedure (If Needed)

If the upgrade causes issues, rollback is straightforward because all migrations are additive:

### Option A: Binary rollback (recommended)
```bash
# Stop the service
sudo systemctl stop social-forge

# Install the previous version's binary
# (download from GitHub Releases or build from a previous git tag)
sudo install -m 755 /path/to/old/social-forge /usr/local/bin/social-forge

# Start the service
sudo systemctl start social-forge
```

The old binary will ignore the new columns/tables (they're all optional/nullable/defaulted). The `_sqlx_migrations` table will show migrations 031–036 as applied, but the old binary doesn't reference them, so there's no conflict.

### Option B: Full rollback (including schema)
```bash
# Stop the service
sudo systemctl stop social-forge

# Drop the new tables (safe — they're all new, no existing data)
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge << 'SQL'
DROP TABLE IF EXISTS publish_outbox CASCADE;
DROP TABLE IF EXISTS events_log CASCADE;
DROP TABLE IF EXISTS brand_profiles CASCADE;
ALTER TABLE posts DROP COLUMN IF EXISTS publish_workflow_version;
ALTER TABLE posts DROP COLUMN IF EXISTS kanban_sort_order;
ALTER TABLE posts DROP COLUMN IF EXISTS kanban_substate;
ALTER TABLE posts DROP COLUMN IF EXISTS due_date;
ALTER TABLE posts DROP COLUMN IF EXISTS priority;
ALTER TABLE campaigns DROP COLUMN IF EXISTS status;
ALTER TABLE campaigns DROP COLUMN IF EXISTS progress_metric;
ALTER TABLE campaigns DROP COLUMN IF EXISTS progress_target;
ALTER TABLE campaigns DROP COLUMN IF EXISTS audience_persona;
ALTER TABLE campaigns DROP COLUMN IF EXISTS content_pillars;
ALTER TABLE campaigns DROP COLUMN IF EXISTS budget_cents;
ALTER TABLE campaigns DROP COLUMN IF EXISTS kpi_targets;
ALTER TABLE campaigns DROP COLUMN IF EXISTS deleted_at;
ALTER TABLE campaigns DROP COLUMN IF EXISTS sort_order;
DELETE FROM _sqlx_migrations WHERE version >= 31;
SQL

# Install the old binary
sudo install -m 755 /path/to/old/social-forge /usr/local/bin/social-forge

# Start the service
sudo systemctl start social-forge
```

---

## 7. Environment Variables (New in v22–v24)

| Var | Default | Purpose |
|---|---|---|
| `EVENTS_LOG_RETENTION_DAYS` | `7` | Days to keep events_log rows before cleanup (v23-1) |
| `FRONTEND_URL` | = `APP_URL` | CSRF allowed origin(s). Now accepts comma-separated list for multi-origin deployments (v23-9). Example: `https://sf.example.com,http://localhost:6543` |

All other env vars are unchanged from previous versions. See `.env.example` (generated by `install.sh`) for the full list.

---

## 8. Troubleshooting

### "Migration failed: checksum mismatch"
This means a migration file was modified after it was applied. sqlx records the checksum of each migration file. To fix:
```bash
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge \
  -c "DELETE FROM _sqlx_migrations WHERE version = <N>;"
```
Then restart the service. **Only do this if you're sure the migration hasn't been applied.**

### "Migration failed: column already exists"
This shouldn't happen because all migrations use `ADD COLUMN IF NOT EXISTS`. If it does, the migration may have been partially applied. Check the `_sqlx_migrations` table:
```bash
docker exec -it $(docker ps -q -f name=postgres) psql -U social_forge -d social_forge \
  -c "SELECT version, description, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 10;"
```

### "Database connected" but app crashes immediately after
Check the logs for the specific error:
```bash
sudo journalctl -u social-forge --since "1 min ago" --no-pager
```
Common causes:
- Missing `APP_PASSWORD` (auto-generated on first run — check `~/.social-forge/.env`)
- Port 6543 already in use
- Postgres not ready (the start script waits up to 60s — increase if needed)

### Calendar shows "error:500"
This was fixed in v22 Phase 1 (the calendar SQL `NULL::bigint` fix). If you still see it, ensure you're running the latest binary:
```bash
social-forge --version
# Should show v0.3.0 or later
```
