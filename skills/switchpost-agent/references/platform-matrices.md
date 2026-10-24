# Platform matrices — read, engage, DM, comment, analytics

Every line verified against the real CLI (`switchpost <cmd> --help`).
Loaded on demand from SKILL.md. Two ID systems: see SKILL.md — `<PAGE_ID>`,
`<ACCOUNT_ID>`, `<CHANNEL_ID>` etc. are PLATFORM ids (`providers` /
platform `list` commands); `--integrations` needs integration UUIDs
(`mcp-call integrations_list`).

## X / Twitter

```bash
switchpost x timeline --count 5           # home timeline (--count, not --limit)
switchpost x search "query"
switchpost x user <username>
switchpost x like <TWEET_ID>              # also: retweet, bookmark <TWEET_ID>
switchpost x delete <TWEET_ID>
switchpost x reply <TWEET_ID> "reply text"
switchpost x dm <RECIPIENT_ID> "text"     # recipient = X user id
switchpost x dm-list
switchpost x dm-messages <CONVERSATION_ID>
```

**Empty-shape trap**: `x user <bad-username>` and `x delete <bad-tweet-id>`
do NOT error — they return exit 0 with an empty-shaped `data:` object
(`id: ""`, `name: ""` / empty `tweet_results`). A missing tweet/user
looks like success. Check the payload's `id` before trusting the result.

## Reddit (cookie auth — richest access)

```bash
switchpost reddit browse <subreddit> --sort hot --limit 25   # sorts: hot,new,top,rising
switchpost reddit search "query"
switchpost reddit user <username>
switchpost reddit inbox
switchpost reddit comment <THING_ID> "reply text"
switchpost reddit vote <THING_ID> up       # or down
switchpost reddit save <THING_ID>          # also: unsave, delete
switchpost reddit mod remove <THING_ID>    # also: approve, lock, unlock
```

## LinkedIn (personal)

```bash
switchpost linkedin profile
switchpost linkedin posts
switchpost linkedin analytics
switchpost linkedin reactions <POST_URN>
switchpost linkedin delete <POST_URN>
switchpost linkedin reply <COMMENT_ID> "reply text"
switchpost linkedin dm <RECIPIENT> "text"
switchpost linkedin dm-list
switchpost linkedin dm-messages <CONVERSATION_ID>
```

## LinkedIn page

```bash
switchpost linkedin-page list             # page ids (platform ids)
switchpost linkedin-page post <PAGE_ID> "text"
switchpost linkedin-page page <PAGE_ID>
switchpost linkedin-page feed <PAGE_ID>
switchpost linkedin-page analytics <PAGE_ID>
switchpost linkedin-page followers <PAGE_ID>
switchpost linkedin-page create-comment <POST_URN> <PAGE_URN> "text"
switchpost linkedin-page delete <POST_URN>
switchpost linkedin-page reactions <POST_URN>
switchpost linkedin-page shares <POST_URN>
switchpost linkedin-page post-analytics <POST_URN>
```

## Facebook (page)

```bash
switchpost facebook posts <PAGE_ID>
switchpost facebook insights <PAGE_ID> --metric page_impressions,page_engaged_users
switchpost facebook page-insights <PAGE_ID>
switchpost facebook albums <PAGE_ID>
switchpost facebook pages <QUERY>          # search public pages
switchpost facebook conversations <PAGE_ID>
switchpost facebook send <PAGE_ID> <CONVERSATION_ID> "text"
switchpost facebook comment <POST_ID> "text"
switchpost facebook react <POST_ID> LIKE   # reaction types: LIKE, LOVE, HAHA, ...
switchpost facebook delete <POST_ID>
switchpost facebook create <PAGE_ID> "message"
switchpost facebook photo <PAGE_ID> <URL>
switchpost facebook video <PAGE_ID> <URL>
```

## Instagram

```bash
switchpost instagram posts <ACCOUNT_ID>
switchpost instagram insights <ACCOUNT_ID> --metric reach,follower_count
# metrics (comma-separated): day: reach,follower_count,online_followers
#   lifetime: total_interactions,comments,shares,saves,likes,accounts_engaged,profile_views,website_clicks
switchpost instagram insights-audience <ACCOUNT_ID>
switchpost instagram media-detail <ACCOUNT_ID> <MEDIA_ID>
switchpost instagram hashtag <ACCOUNT_ID> <QUERY>
switchpost instagram reels <ACCOUNT_ID>
switchpost instagram stories <ACCOUNT_ID>
switchpost instagram followers <ACCOUNT_ID>
switchpost instagram mentions <ACCOUNT_ID>
switchpost instagram comment <MEDIA_ID> "text"
switchpost instagram dm <ACCOUNT_ID> <RECIPIENT> "text"
switchpost instagram dm-list <ACCOUNT_ID>
switchpost instagram dm-messages <ACCOUNT_ID> <CONVERSATION_ID>
switchpost instagram poll-container <ACCOUNT_ID> <CREATION_ID>
```

## YouTube (native family)

```bash
switchpost youtube search <QUERY>
switchpost youtube video <VIDEO_ID>
switchpost youtube playlists <CHANNEL_ID>
switchpost youtube stats <CHANNEL_ID>
switchpost youtube analytics <CHANNEL_ID> --start-date <DATE> --end-date <DATE>
switchpost youtube subscriptions <CHANNEL_ID>
switchpost youtube creators <QUERY>
switchpost youtube reply <COMMENT_ID> "reply text"
```

## Bluesky / Mastodon

```bash
switchpost bluesky timeline
switchpost bluesky feed
switchpost bluesky search "query"
switchpost bluesky profile <HANDLE>
switchpost bluesky reply <POST_URI> "reply text"

switchpost mastodon timeline
switchpost mastodon whoami
switchpost mastodon search "query"
switchpost mastodon get <POST_ID>
switchpost mastodon reply <STATUS_ID> "reply text"
```

## TikTok / Threads

```bash
switchpost tiktok profile
switchpost tiktok videos

switchpost threads profile <THREADS_ID>
switchpost threads list <THREADS_ID>
switchpost threads reply <THREADS_ID> <MEDIA_ID> "text"
switchpost threads delete <THREADS_ID> <MEDIA_ID>
switchpost threads insights <THREADS_ID> <METRIC>
```

## Discord / Slack

```bash
switchpost discord channels <GUILD_ID>
switchpost discord messages <CHANNEL_ID> --limit 10
switchpost discord send <CHANNEL_ID> "text"
switchpost discord server <GUILD_ID>
switchpost discord forum <CHANNEL_ID> "title" "content"

switchpost slack channels
switchpost slack history <CHANNEL_ID>
switchpost slack send <CHANNEL_ID> "text"
switchpost slack users
```

## Telegram

```bash
switchpost telegram-bot send <CHAT_ID> "text"
switchpost telegram-bot photo <CHAT_ID> <URL>
switchpost telegram-bot document <CHAT_ID> <PATH>
switchpost telegram-bot chat <CHAT_ID>
switchpost telegram-bot updates

switchpost telegram-user send <PEER> "text"
switchpost telegram-user dialogs
switchpost telegram-user contacts
switchpost telegram-user search "query"
```

## WhatsApp

```bash
switchpost whatsapp send <PHONE> "text"
switchpost whatsapp chats
switchpost whatsapp contacts
switchpost whatsapp groups
switchpost whatsapp create-group <NAME> [PARTICIPANTS]...
switchpost whatsapp invite-link <GROUP_JID>
```

## Pinterest

```bash
switchpost pinterest profile <BOARD_ID>    # board id used for token resolution
switchpost pinterest board <BOARD_ID>
switchpost pinterest pins <BOARD_ID>
switchpost pinterest pin <BOARD_ID> <PIN_ID>
switchpost pinterest search "query"
switchpost pinterest board-analytics <BOARD_ID> --start-date <D> --end-date <D>
switchpost pinterest pin-analytics <BOARD_ID> <PIN_ID> --start-date <D> --end-date <D>
```

## GitHub

```bash
switchpost github me
switchpost github my-repos
switchpost github user <LOGIN>
switchpost github repos <USERNAME>
switchpost github issues <OWNER> <REPO>
switchpost github prs <OWNER> <REPO>
switchpost github commits <OWNER> <REPO>
switchpost github branches <OWNER> <REPO>
switchpost github releases <OWNER> <REPO>
switchpost github search "query"
switchpost github create-issue <OWNER> <REPO> "title" --body "description"
switchpost github close-issue <OWNER> <REPO> <NUMBER>
```

## Blogs (WordPress / Hashnode / Medium / Dev.to)

```bash
switchpost wordpress list
switchpost wordpress get <NUMBER_ID>              # numeric post id
switchpost wordpress categories
switchpost hashnode list <PUBLICATION_ID>
switchpost hashnode get <POST_ID>
switchpost medium-blog list
switchpost medium-blog get <POST_ID>
switchpost devto list
switchpost devto get <POST_ID>
```

## Skool

```bash
switchpost skool info <COMMUNITY_SLUG>
switchpost skool posts <COMMUNITY_SLUG>
switchpost skool comment <POST_ID> <GROUP_ID> "text"
```

## Google (older `google` family + Drive/Calendar/Gmail)

```bash
switchpost google youtube-search <CHANNEL_ID> "query"
switchpost google video <CHANNEL_ID> <VIDEO_ID>
switchpost google playlists <CHANNEL_ID>
switchpost google channel-stats <CHANNEL_ID>

switchpost gdrive files
switchpost gdrive file <FILE_ID>
switchpost gdrive search "query"
switchpost gdrive folders
switchpost gdrive metadata <FILE_ID>
switchpost gdrive export <FILE_ID> <MIME_TYPE>

switchpost gcal calendars
switchpost gcal events --calendar-id <ID> --limit 20
switchpost gcal event <EVENT_ID>
switchpost gcal create "Title" <START_ISO> <END_ISO>
switchpost gcal update <EVENT_ID>
switchpost gcal delete <EVENT_ID>

switchpost gmail-ops profile
switchpost gmail-ops messages --limit 20
switchpost gmail-ops message <ID>
switchpost gmail-ops search "query"
switchpost gmail-ops thread <ID>
switchpost gmail-ops labels
switchpost gmail-ops send <TO> "Subject" "Body"
```

## Cross-platform engagement (any provider)

```bash
switchpost comment get <INTEGRATION_UUID> <POST_ID> --limit 20
switchpost comment reply <INTEGRATION_UUID> <COMMENT_ID> "text"
switchpost dm send <INTEGRATION_UUID> <RECIPIENT_ID> "text"
switchpost dm list <INTEGRATION_UUID>
switchpost dm messages <INTEGRATION_UUID> <CONVERSATION_ID>
switchpost import x --count 20      # pull a provider's posts into the local store
switchpost feed --provider x        # unified local feed
switchpost analytics get <PROVIDER> --days 7
switchpost analytics post <POST_ID>
```
