# Platform matrices

Read and engage command lookup per platform. Loaded on demand from `SKILL.md`.

### Read from any platform

```bash
# TikTok
switchpost tiktok profile --pretty
switchpost tiktok videos --limit 10 --pretty

# Threads
switchpost threads list <ACCOUNT_ID> --limit 10 --pretty
switchpost threads profile <ACCOUNT_ID> --pretty

# Discord
switchpost discord channels <GUILD_ID> --pretty
switchpost discord messages <CHANNEL_ID> --limit 10 --pretty
switchpost discord server <GUILD_ID> --pretty

# Slack
switchpost slack channels --pretty
switchpost slack history <CHANNEL_ID> --limit 10 --pretty
switchpost slack users --pretty

# Telegram Bot
switchpost telegram-bot updates --pretty
switchpost telegram-bot chat <CHAT_ID> --pretty

# Telegram User
switchpost telegram-user dialogs --limit 10 --pretty
switchpost telegram-user contacts --pretty
switchpost telegram-user search "query" --pretty

# WhatsApp
switchpost whatsapp chats --limit 10 --pretty
switchpost whatsapp contacts --pretty
switchpost whatsapp groups --pretty

# Pinterest
switchpost pinterest profile <BOARD_ID> --pretty
switchpost pinterest board <BOARD_ID> --pretty
switchpost pinterest pins <BOARD_ID> --limit 10 --pretty
switchpost pinterest search "query" --pretty

# GitHub
switchpost github me --pretty
switchpost github my-repos --limit 10 --pretty
switchpost github issues <OWNER> <REPO> --pretty
switchpost github prs <OWNER> <REPO> --pretty
switchpost github commits <OWNER> <REPO> --limit 10 --pretty
switchpost github search "query" --pretty

# WordPress
switchpost wordpress list --limit 10 --pretty
switchpost wordpress get 1 --pretty
switchpost wordpress categories --pretty

# Hashnode
switchpost hashnode list <PUB_ID> --pretty
switchpost hashnode get <POST_ID> --pretty

# Medium
switchpost medium-blog list --pretty
switchpost medium-blog get <POST_ID> --pretty

# Dev.to
switchpost devto list --pretty
switchpost devto get <POST_ID> --pretty

# Skool
switchpost skool posts <SLUG> --pretty
switchpost skool info <SLUG> --pretty

# Google Workspace
switchpost google youtube-search <CHANNEL_ID> "query" --pretty
switchpost google video <CHANNEL_ID> <VIDEO_ID> --pretty
switchpost google playlists <CHANNEL_ID> --pretty
switchpost google channel-stats <CHANNEL_ID> --pretty

# Google Drive
switchpost gdrive files --limit 10 --pretty
switchpost gdrive search "query" --pretty
switchpost gdrive folders --pretty

# Google Calendar
switchpost gcal calendars --pretty
switchpost gcal events --limit 10 --pretty
switchpost gcal event <EVENT_ID> --pretty

# Gmail
switchpost gmail-ops messages --limit 10 --pretty
switchpost gmail-ops search "query" --pretty
switchpost gmail-ops labels --pretty
switchpost gmail-ops profile --pretty

# Webhooks
switchpost webhooks list --pretty
switchpost webhooks get <ID> --pretty

# Notifications
switchpost notifications list --limit 10 --pretty

# Tags
switchpost tags list --pretty
switchpost tags get <ID> --pretty

# Analytics
switchpost analytics get <PROVIDER> --days 7 --pretty
switchpost analytics post <POST_ID> --pretty
```

### Engage on any platform

```bash
# X
switchpost x like <TWEET_ID>
switchpost x retweet <TWEET_ID>
switchpost x bookmark <TWEET_ID>
switchpost x reply <TWEET_ID> "reply text"

# Reddit
switchpost reddit vote <THING_ID> up
switchpost reddit save <THING_ID>
switchpost reddit comment <THING_ID> "reply text"

# Facebook
switchpost facebook react <POST_ID> LIKE
switchpost facebook comment <POST_ID> "reply text"

# Instagram
switchpost instagram comment <MEDIA_ID> "reply text"

# LinkedIn
switchpost linkedin reply <COMMENT_ID> "reply text"

# Discord
switchpost discord send <CHANNEL_ID> "reply text"
switchpost discord forum <CHANNEL_ID> "title" "content"

# Slack
switchpost slack send <CHANNEL_ID> "reply text"

# WhatsApp
switchpost whatsapp send <PHONE> "reply text"

# Telegram
switchpost telegram-bot send <CHAT_ID> "reply text"
switchpost telegram-user send <PEER> "reply text"

# GitHub
switchpost github create-issue <OWNER> <REPO> "title" --body "description"
switchpost github close-issue <OWNER> <REPO> <NUMBER>

# WordPress
switchpost wordpress post "title" "content"

# Hashnode
switchpost hashnode post <PUB_ID> "title" "content"

# Medium
switchpost medium-blog post "content"

# Dev.to
switchpost devto post "content"

# Skool
switchpost skool post <GROUP_ID> "title" "content"
switchpost skool comment <POST_ID> <GROUP_ID> "reply text"

# Google Calendar
switchpost gcal create "Meeting" "2026-07-02T09:00:00Z" "2026-07-02T10:00:00Z"
switchpost gcal delete <EVENT_ID>

# Gmail
switchpost gmail-ops send "user@example.com" "Hi" "Hello!"

# Webhooks
switchpost webhooks create "https://example.com/hook" --name "my-hook"
switchpost webhooks test <ID>
switchpost webhooks delete <ID>

# Tags
switchpost tags create "work" --color "#ff0000"
switchpost tags delete <ID>

# Notifications
switchpost notifications read <ID>
switchpost notifications read-all

# Analytics
switchpost analytics get instagram --days 7
switchpost analytics post <POST_ID>
```
