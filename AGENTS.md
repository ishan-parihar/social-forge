# AGENTS.md — AI Agent Knowledge Base

## Quick Reference

### CLI Usage (preferred for AI agents)
```bash
social-forge --help              # Discover all commands
social-forge providers           # List connected accounts
social-forge x timeline          # Read X timeline
social-forge reddit browse rust  # Browse subreddit
social-forge linkedin profile    # Get LinkedIn profile
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

### Authentication Priority
1. DB-stored cookie tokens (submitted via web form)
2. Browser cookie extraction (auto-detected from Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB
4. Environment variables

### Database
- **Connection**: `DATABASE_URL` env var (PostgreSQL)
- **Migrations**: `migrations/` directory, auto-applied on startup

### Key Patterns
- All CLI output is JSON (machine-readable)
- Errors output JSON to stderr with exit code 1
- Cookie auth enables full platform access (voting, moderation, GraphQL)
- OAuth is the fallback for platforms without cookie support
