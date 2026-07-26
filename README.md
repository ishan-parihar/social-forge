# Social-Forge

![Rust](https://img.shields.io/badge/Rust-1.78+-orange?logo=rust)
![License](https://img.shields.io/badge/License-MIT-green)
![MCP](https://img.shields.io/badge/MCP-1.0-orange?logo=modelcontextprotocol)
![Platforms](https://img.shields.io/badge/Platforms-6-blue)
![Interfaces](https://img.shields.io/badge/Interfaces-3-green)


**High-performance social media orchestration engine** — triple-interface architecture: CLI, REST API, and MCP protocol.

![Social-Forge architecture](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/social-forge-arch.png)

---

## What it is

| Interface | Use Case |
|-----------|----------|
| **CLI** | `social-forge post --platform twitter --text "Hello"` |
| **REST API** | `POST /api/v1/posts` — integrate with any backend |
| **MCP Server** | Agents post, schedule, analyze via 12 MCP tools |

**Platforms:** Twitter/X, LinkedIn, Reddit, Telegram, Discord, Bluesky, Mastodon

---

## Quick start

```bash
# Install
pipx install social-forge

# Configure
social-forge config init
# Edit ~/.config/social-forge/config.yaml with API keys

# Post
social-forge post --platform twitter --text "Hello from Social-Forge!"

# Schedule
social-forge schedule --platform linkedin --text "Post" --at "2024-01-15 09:00"
```

---

## MCP Server (for agents)

```bash
social-forge mcp
```

**12 MCP Tools:**
- `post.create`, `post.schedule`, `post.delete`
- `media.upload`, `media.delete`
- `analytics.get`, `engagement.get`
- `account.list`, `account.verify`
- `hashtag.suggest`, `trending.get`

---


## Features

| Feature | Details |
|---------|---------|
| **Multi-platform** | 7 platforms, unified API |
| **Scheduling** | Cron + one-time, timezone-aware |
| **Media** | Images, videos, GIFs, carousels |
| **Analytics** | Engagement, reach, follower growth |
| **Rate limiting** | Per-platform, auto-backoff |
| **MCP** | 12 tools for agent orchestration |

---

## Configuration

```yaml
# ~/.config/social-forge/config.yaml
platforms:
  twitter:
    api_key: "..."
    api_secret: "..."
    access_token: "..."
    access_token_secret: "..."
  linkedin:
    access_token: "..."
  reddit:
    client_id: "..."
    client_secret: "..."
    username: "..."
    password: "..."

scheduler:
  timezone: "UTC"
  max_concurrent: 10

mcp:
  enabled: true
  port: 8001
```

---

## Commands

| Command | Description |
|---------|-------------|
| `social-forge post` | Create post |
| `social-forge schedule` | Schedule post |
| `social-forge analytics` | Get analytics |
| `social-forge mcp` | Start MCP server |
| `social-forge config` | Manage config |

---



## Visual proof

| CLI dashboard | REST API docs | MCP tools |
|:---:|:---:|:---:|
| ![CLI](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/cli.png) | ![API](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/api.png) | ![MCP](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/mcp.png) |

| Content calendar | Analytics | Multi-platform |
|:---:|:---:|:---:|
| ![Calendar](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/calendar.png) | ![Analytics](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/analytics.png) | ![Multi](https://github.com/ishan-parihar/social-forge/raw/main/assets/readme/multi.png) |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Social-Forge Core                          │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  CLI     │  │  REST    │  │  MCP     │  │  Scheduler│     │
│  │  (Typer) │  │  (FastAPI)│  │  (FastMCP)│  │  (APScheduler)│
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────┘
         │            │            │            │
         ▼            ▼            ▼            ▼
┌─────────────────────────────────────────────────────────────┐
│  Platform Adapters: Twitter, LinkedIn, Reddit, Telegram,   │
│  Discord, Bluesky, Mastodon (unified interface)             │
└─────────────────────────────────────────────────────────────┘
```

---

## Requirements

- Python 3.11+
- Platform API credentials

---

## License

MIT — see [LICENSE](LICENSE).
