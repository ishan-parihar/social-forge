# Platform matrices

Read and engage command lookup per platform. Loaded on demand from `SKILL.md`.

### Read from any platform

```bash
# TikTok
social-forge tiktok profile --pretty
social-forge tiktok videos --limit 10 --pretty

# Threads
social-forge threads list <ACCOUNT_ID> --limit 10 --pretty
social-forge threads profile <ACCOUNT_ID> --pretty

# Discord
social-forge discord channels <GUILD_ID> --pretty
social-forge discord messages <CHANNEL_ID> --limit 10 --pretty
social-forge discord server <GUILD_ID> --pretty

# Slack
social-forge slack channels --pretty
social-forge slack history <CHANNEL_ID> --limit 10 --pretty
social-forge slack users --pretty

# Telegram Bot
social-forge telegram-bot updates --pretty
social-forge telegram-bot chat <CHAT_ID> --pretty

# Telegram User
social-forge telegram-user dialogs --limit 10 --pretty
social-forge telegram-user contacts --pretty
social-forge telegram-user search "query" --pretty

# WhatsApp
social-forge whatsapp chats --limit 10 --pretty
social-forge whatsapp contacts --pretty
social-forge whatsapp groups --pretty

# Pinterest
social-forge pinterest profile <BOARD_ID> --pretty
social-forge pinterest board <BOARD_ID> --pretty
social-forge pinterest pins <BOARD_ID> --limit 10 --pretty
social-forge pinterest search "query" --pretty

# GitHub
social-forge github me --pretty
social-forge github my-repos --limit 10 --pretty
social-forge github issues <OWNER> <REPO> --pretty
social-forge github prs <OWNER> <REPO> --pretty
social-forge github commits <OWNER> <REPO> --limit 10 --pretty
social-forge github search "query" --pretty

# WordPress
social-forge wordpress list --limit 10 --pretty
social-forge wordpress get 1 --pretty
social-forge wordpress categories --pretty

# Hashnode
social-forge hashnode list <PUB_ID> --pretty
social-forge hashnode get <POST_ID> --pretty

# Medium
social-forge medium-blog list --pretty
social-forge medium-blog get <POST_ID> --pretty

# Dev.to
social-forge devto list --pretty
social-forge devto get <POST_ID> --pretty

# Skool
social-forge skool posts <SLUG> --pretty
social-forge skool info <SLUG> --pretty

# Google Workspace
social-forge google youtube-search <CHANNEL_ID> "query" --pretty
social-forge google video <CHANNEL_ID> <VIDEO_ID> --pretty
social-forge google playlists <CHANNEL_ID> --pretty
social-forge google channel-stats <CHANNEL_ID> --pretty

# Google Drive
social-forge gdrive files --limit 10 --pretty
social-forge gdrive search "query" --pretty
social-forge gdrive folders --pretty

# Google Calendar
social-forge gcal calendars --pretty
social-forge gcal events --limit 10 --pretty
social-forge gcal event <EVENT_ID> --pretty

# Gmail
social-forge gmail-ops messages --limit 10 --pretty
social-forge gmail-ops search "query" --pretty
social-forge gmail-ops labels --pretty
social-forge gmail-ops profile --pretty

# Webhooks
social-forge webhooks list --pretty
social-forge webhooks get <ID> --pretty

# Notifications
social-forge notifications list --limit 10 --pretty

# Tags
social-forge tags list --pretty
social-forge tags get <ID> --pretty

# Analytics
social-forge analytics get <PROVIDER> --days 7 --pretty
social-forge analytics post <POST_ID> --pretty
```

### Engage on any platform

```bash
# X
social-forge x like <TWEET_ID>
social-forge x retweet <TWEET_ID>
social-forge x bookmark <TWEET_ID>
social-forge x reply <TWEET_ID> "reply text"

# Reddit
social-forge reddit vote <THING_ID> up
social-forge reddit save <THING_ID>
social-forge reddit comment <THING_ID> "reply text"

# Facebook
social-forge facebook react <POST_ID> LIKE
social-forge facebook comment <POST_ID> "reply text"

# Instagram
social-forge instagram comment <MEDIA_ID> "reply text"

# LinkedIn
social-forge linkedin reply <COMMENT_ID> "reply text"

# Discord
social-forge discord send <CHANNEL_ID> "reply text"
social-forge discord forum <CHANNEL_ID> "title" "content"

# Slack
social-forge slack send <CHANNEL_ID> "reply text"

# WhatsApp
social-forge whatsapp send <PHONE> "reply text"

# Telegram
social-forge telegram-bot send <CHAT_ID> "reply text"
social-forge telegram-user send <PEER> "reply text"

# GitHub
social-forge github create-issue <OWNER> <REPO> "title" --body "description"
social-forge github close-issue <OWNER> <REPO> <NUMBER>

# WordPress
social-forge wordpress post "title" "content"

# Hashnode
social-forge hashnode post <PUB_ID> "title" "content"

# Medium
social-forge medium-blog post "content"

# Dev.to
social-forge devto post "content"

# Skool
social-forge skool post <GROUP_ID> "title" "content"
social-forge skool comment <POST_ID> <GROUP_ID> "reply text"

# Google Calendar
social-forge gcal create "Meeting" "2026-07-02T09:00:00Z" "2026-07-02T10:00:00Z"
social-forge gcal delete <EVENT_ID>

# Gmail
social-forge gmail-ops send "user@example.com" "Hi" "Hello!"

# Webhooks
social-forge webhooks create "https://example.com/hook" --name "my-hook"
social-forge webhooks test <ID>
social-forge webhooks delete <ID>

# Tags
social-forge tags create "work" --color "#ff0000"
social-forge tags delete <ID>

# Notifications
social-forge notifications read <ID>
social-forge notifications read-all

# Analytics
social-forge analytics get instagram --days 7
social-forge analytics post <POST_ID>
```
