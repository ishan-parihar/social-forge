# AGENTS.md — AI Agent Knowledge Base

## Quick Reference

### CLI Usage (preferred for AI agents)
```bash
social-forge --help              # Discover all commands
social-forge providers           # List connected accounts
social-forge post "hello" --platforms x,linkedin  # Multi-platform post
social-forge carousel create "title" --images a.jpg b.jpg --platforms x,linkedin
social-forge posts list           # List scheduled/queued posts
social-forge stage "draft" --integrations <uuid>  # Stage for review
social-forge media list           # List uploaded media
social-forge x timeline           # Read X timeline
social-forge reddit browse rust   # Browse subreddit
social-forge linkedin profile     # Get LinkedIn profile
social-forge comment get <integration_id> <post_id>  # Get comments
social-forge dm send <integration_id> <recipient> "text"  # Send DM
social-forge automation list      # List automation rules
```

### MCP Usage (for Claude Desktop / Cursor)
```json
{
  "mcpServers": {
    "social-forge": {
      "command": "social-forge",
      "args": ["mcp"]
    }
  }
}
```

### MCP Tools
| Category | Tools |
|----------|-------|
| Posts | `posts_create`, `posts_list`, `posts_get`, `posts_update`, `posts_delete` |
| Scheduling | `posts_schedule`, `posts_publish`, `posts_find_slot` |
| Staging | `posts_stage`, `posts_stage_preview`, `posts_stage_publish` |
| Carousels | `posts_create_carousel` |
| Media | `posts_media_upload`, `posts_media_upload_from_path`, `posts_media_upload_batch`, `posts_media_upload_from_url`, `posts_media_list` |
| Comments | `comments_get`, `comments_reply`, `comments_delete` |
| DMs | `dm_send`, `dm_list`, `dm_messages` |
| Automation | `automation_create`, `automation_list`, `automation_update`, `automation_delete`, `automation_logs` |
| Providers | `integrations_list`, `integrations_targets` |

### Authentication Priority
1. DB-stored cookie tokens (submitted via web form)
2. Browser cookie extraction (auto-detected from Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB
4. Environment variables

### Database
- **Connection**: `DATABASE_URL` env var (PostgreSQL)
- **Port**: `5433` (local Docker postgres maps 5433→5432)
- **User/DB**: `social_forge` / `social_forge`
- **Migrations**: `migrations/` directory, auto-applied on startup

### Key Patterns
- All CLI output is JSON (machine-readable)
- Errors output JSON to stderr with exit code 1
- Cookie auth enables full platform access (voting, moderation, GraphQL)
- OAuth is the fallback for platforms without cookie support

---

## Features

### Unified Posting

Post to any platform with auto content-splitting. The `post` command handles platform-specific limits automatically.

```bash
# Post to multiple platforms
social-forge post "Shipping new features 🚀" --platforms x,linkedin,bluesky

# Post with media
social-forge post "Check this out" --media https://example.com/image.jpg

# Schedule a post
social-forge post "Morning update" --schedule 2026-07-01T09:00:00Z

# Post with first comment (Instagram, etc.)
social-forge post "New post" --first-comment "Link in bio!"
```

### Staging (Review Before Publish)

Stage posts across multiple platforms, review splits, then publish.

```bash
# Stage a post (creates drafts, doesn't publish)
social-forge stage "Long content that will be split..." --integrations <uuid1>,<uuid2>

# Preview splits without creating drafts
social-forge stage "Preview mode" --platforms x,linkedin --preview

# Publish a staged post via MCP
# Use posts_stage_publish with the post_id
```

### Media Management

Upload, list, and manage media for post attachments.

```bash
social-forge media upload ./photo.jpg --alt "Description"
social-forge media upload-batch ./img1.jpg ./img2.jpg ./img3.jpg
social-forge media list --limit 10 --search "photo"
social-forge media download https://example.com/image.png --output ./local.png
```

### Carousels

Create multi-image carousel posts (LinkedIn, Instagram, X).

```bash
# Create carousel from local images
social-forge carousel create "5 Tips for Devs" --images tip1.jpg tip2.jpg tip3.jpg --platforms x,linkedin

# Create carousel with captions per slide
social-forge carousel create "Product Walkthrough" --images shot1.png shot2.png \
  --captions "Overview" "Key Features" --platforms linkedin
```

### Media Pipeline

Media uploads flow: **upload → store → resolve → attach**.

- Local files stored in `data/media/`; URLs downloaded and stored locally
- Provider limits: X (4 images/1 video), LinkedIn (20 images), Instagram (10 images), Reddit (1/image)
- Formats: JPEG, PNG, GIF, WEBP (images); MP4 (video)
- Alt text optional but recommended for accessibility

```bash
social-forge media upload https://example.com/photo.jpg --alt "Screenshot"  # URL resolves automatically
social-forge media list  # shows local path + original URL
```

### Comments

Get, reply to, and delete comments across platforms.

```bash
# Get comments for a post
social-forge comment get <integration_id> <post_id> --limit 20

# Reply to a comment
social-forge comment reply <integration_id> <comment_id> "Thanks for feedback!"

# Delete a comment
social-forge comment delete <integration_id> <comment_id>
```

### Direct Messages

Send and read DMs across supported platforms.

```bash
# Send a DM
social-forge dm send <integration_id> <recipient_id> "Hello!"

# List conversations
social-forge dm list <integration_id> --limit 20

# Get messages in a conversation
social-forge dm messages <integration_id> <conversation_id> --limit 20
```

### Automation Rules

Create rules for auto-reply to comments, DMs, mentions, or follows.

```bash
# Create an automation rule
social-forge automation create <integration_id> "Auto-thanks" \
  --trigger comment --response "Thanks for commenting!" --type fixed

# List rules
social-forge automation list

# Update a rule
social-forge automation update <rule_id> --name "New name" --active true

# Delete a rule
social-forge automation delete <rule_id>

# View execution logs
social-forge automation logs <rule_id> --limit 50
```

---

## Development & Deployment

### Architecture
```
System Boot
  ├── docker.service (enabled)
  │    └── postgres container (restart: unless-stopped, port 5433)
  └── social-forge.service (enabled)
       └── /usr/local/bin/social-forge serve  (pre-built binary, NOT built on boot)
```

- **Postgres** runs as a Docker container with `restart: unless-stopped` — auto-starts on boot
- **App binary** runs via systemd — pre-built binary at `/usr/local/bin/social-forge`
- **Frontend** is built separately with `pnpm build`, served by the Rust binary
- **NO cargo build on boot** — the binary must be pre-built before deployment

### Quick redeploy (after code changes) — THE ONE-LINER
```bash
cargo build --release && sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge && sudo systemctl restart social-forge
```

### Or use the Makefile
```bash
make redeploy    # same as the one-liner above
make deploy      # also rebuilds frontend
```

### Auto-watch (auto-rebuild + restart on file changes)
```bash
# Requires cargo-watch:
#   cargo install cargo-watch
make watch
```

### Service management
```bash
make status     # Show systemd + docker status
make logs       # Tail journalctl logs
make restart    # Reload systemd + restart service
```

### Manual steps (first-time setup)
```bash
# 1. Copy the systemd service & startup script
sudo cp scripts/social-forge-start.sh /usr/local/bin/social-forge-start.sh
sudo cp scripts/social-forge.service /etc/systemd/system/social-forge.service
sudo systemctl daemon-reload
sudo systemctl enable social-forge --now

# 2. Copy the pre-built binary
sudo install -m 755 target/release/social-forge /usr/local/bin/social-forge

# 3. Ensure Docker starts on boot
sudo systemctl enable docker --now
```
