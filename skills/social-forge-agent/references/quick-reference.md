# Social Forge — Quick Reference

One-page cheat sheet. Copy-paste ready.

---

## Output Format

All commands output JSON. Use `--pretty` for human-readable or `--json` for machine-parseable.

```bash
social-forge <cmd> --json      # Machine-readable (default)
social-forge <cmd> --pretty    # Human-readable
```

**Exit codes**: `0` = success, `1` = error. Error details are in stderr as JSON.

---

## Finding IDs

Before many commands, you need integration IDs. Here's how to find them:

```bash
social-forge providers --pretty                    # All integration UUIDs + names
social-forge linkedin-page list --pretty           # LinkedIn page IDs
social-forge instagram media <IG_ID> --limit 1     # Verify Instagram account
social-forge reddit browse <sub> --limit 1         # Verify Reddit connection
social-forge facebook pages --pretty               # Facebook page IDs
social-forge youtube channels --pretty             # YouTube channel IDs
```

---

## Discovery

```bash
social-forge --help                    # All commands
social-forge <cmd> --help              # Args for a command
social-forge providers --pretty        # Connected accounts
social-forge doctor                    # Provider health
social-forge connect <provider>        # Connect a new account
```

---

## Post

```bash
social-forge x post "Hello!"                                   # X/Twitter
social-forge linkedin post "Update!"                            # LinkedIn personal
social-forge linkedin-page post <PAGE_ID> "Update!"            # LinkedIn page
social-forge reddit post --title "Title" --text "Body" --target sub  # Reddit
social-forge post "Content" --platforms x,linkedin,bluesky     # Cross-platform
```

**Platform-specific posting via MCP** (no native CLI yet):
```bash
social-forge mcp-call fb_create_post --args '{"page_id":"<ID>","message":"Post"}'       # Facebook
social-forge mcp-call tt_create_post --args '{"text":"Post"}'                           # TikTok
social-forge mcp-call th_create_thread --args '{"text":"Thread post"}'                  # Threads
social-forge mcp-call bs_create_post --args '{"text":"Post"}'                          # Bluesky
social-forge mcp-call ms_create_post --args '{"status":"Post"}'                         # Mastodon
social-forge mcp-call wp_create_post --args '{"title":"T","content":"C"}'              # WordPress
social-forge mcp-call hn_create_post --args '{"title":"T","content_markdown":"C"}'     # Hashnode
social-forge mcp-call dv_create_post --args '{"title":"T","body_markdown":"C"}'        # Dev.to
social-forge mcp-call md_create_post --args '{"title":"T","content":"C","contentFormat":"markdown"}' # Medium
```

**Instagram** (3-step container flow):
```bash
social-forge mcp-call ig_create_container --args '{"ig_id":"<ID>","image_url":"<URL>","caption":"Text"}'
social-forge mcp-call ig_poll_container --args '{"ig_id":"<ID>","container_id":"<CID>"}'
social-forge mcp-call ig_publish_container --args '{"ig_id":"<ID>","container_id":"<CID>"}'
```

---

## Schedule

```bash
social-forge posts create "Content" --integrations <UUID>                  # Queue
social-forge posts create "Content" --integrations <UUID> --schedule <ISO> # Schedule
social-forge posts find-slot --integration <UUID>                          # Find slot
social-forge posts list --limit 10                                         # List
social-forge posts get <POST_ID>                                           # Details
social-forge posts publish <POST_ID>                                       # Publish now
social-forge posts delete <POST_ID>                                        # Cancel
```

---

## Read

```bash
social-forge x timeline --count 5 --pretty           # X timeline
social-forge x search "query" --pretty                # X search
social-forge x user <username> --pretty               # X user profile
social-forge reddit browse <sub> --pretty             # Reddit feed
social-forge reddit search "query" --pretty           # Reddit search
social-forge reddit user <username> --pretty          # Reddit user
social-forge reddit inbox --pretty                    # Reddit inbox
social-forge instagram posts <ACCOUNT_ID> --pretty    # Instagram
social-forge linkedin posts --pretty                  # LinkedIn personal
social-forge linkedin profile --pretty                # LinkedIn profile
social-forge linkedin-page posts <PAGE_ID> --pretty   # LinkedIn page
social-forge facebook posts <PAGE_ID> --pretty        # Facebook page
social-forge feed --pretty                            # Unified feed
```

**Platform-specific reads via MCP**:
```bash
social-forge mcp-call tt_profile --args '{}'                                       # TikTok profile
social-forge mcp-call tt_list_videos --args '{}'                                   # TikTok videos
social-forge mcp-call th_list_threads --args '{}'                                  # Threads
social-forge mcp-call bs_timeline --args '{}'                                      # Bluesky timeline
social-forge mcp-call ms_get_timeline --args '{}'                                  # Mastodon timeline
social-forge mcp-call di_get_messages --args '{"channel_id":"<ID>"}'              # Discord messages
social-forge mcp-call sl_channel_history --args '{"channel_id":"<ID>"}'           # Slack history
social-forge mcp-call tb_get_updates --args '{}'                                  # Telegram Bot updates
social-forge mcp-call tu_list_dialogs --args '{}'                                  # Telegram User dialogs
social-forge mcp-call wa_chats --args '{}'                                         # WhatsApp chats
social-forge mcp-call gh_list_my_repos --args '{}'                                 # GitHub repos
social-forge mcp-call gh_list_issues --args '{"owner":"<O>","repo":"<R>"}'         # GitHub issues
```

---

## Media

```bash
social-forge media upload ./photo.jpg                          # Upload file
social-forge media upload https://example.com/img.png           # Upload URL
social-forge media upload-batch ./a.jpg ./b.jpg ./c.jpg        # Batch upload
social-forge media list --limit 10                              # List media
social-forge media download <url> --output ./local.png          # Download
```

---

## Carousel

```bash
social-forge carousel create "Title" --images a.jpg b.jpg c.jpg --platforms x,linkedin
```

---

## Comments

```bash
social-forge comment get <INTEGRATION_ID> <POST_ID>                 # Get comments
social-forge comment reply <INTEGRATION_ID> <COMMENT_ID> "Reply"    # Reply
social-forge comment delete <INTEGRATION_ID> <COMMENT_ID>           # Delete
social-forge reddit comment <THING_ID> "Reply"                      # Reddit reply
social-forge facebook comment <POST_ID> "Reply"                     # Facebook reply
social-forge instagram comment <MEDIA_ID> "Reply"                   # Instagram reply
```

**Platform-specific comment replies via MCP**:
```bash
social-forge mcp-call ig_reply_to_comment --args '{"ig_id":"<ID>","comment_id":"<CID>","text":"Reply"}'  # Instagram
social-forge mcp-call li_create_comment --args '{"post_urn":"<URN>","message":"Reply","parent_id":"<PID>"}'  # LinkedIn
social-forge mcp-call di_send_message --args '{"channel_id":"<ID>","content":"Reply"}'   # Discord
```

---

## DMs

```bash
social-forge dm send <INTEGRATION_ID> <RECIPIENT_ID> "Hello"    # Send
social-forge dm list <INTEGRATION_ID>                            # Conversations
social-forge dm messages <INTEGRATION_ID> <CONVERSATION_ID>      # Read messages
```

**Platform-specific DMs via MCP**:
```bash
social-forge mcp-call reddit_send_dm --args '{"username":"<USER>","message":"Hello"}'   # Reddit
social-forge mcp-call fb_send_message --args '{"recipient_id":"<ID>","message":"Hello"}'  # Facebook
social-forge mcp-call di_send_message --args '{"channel_id":"<ID>","content":"Hello"}'  # Discord
social-forge mcp-call tb_send_message --args '{"chat_id":"<ID>","text":"Hello"}'       # Telegram Bot
social-forge mcp-call tu_send_message --args '{"dialog_id":"<ID>","message":"Hello"}'  # Telegram User
social-forge mcp-call wa_send_text --args '{"number":"<PHONE>","message":"Hello"}'     # WhatsApp
social-forge mcp-call sl_send_message --args '{"channel":"<ID>","message":"Hello"}'    # Slack
```

---

## Analytics

```bash
social-forge instagram insights <ACCOUNT_ID> --metric reach,follower_count
social-forge linkedin-page analytics <PAGE_ID>
social-forge linkedin-page followers <PAGE_ID>
social-forge linkedin analytics
social-forge facebook insights <PAGE_ID> --metric <metrics>
```

**Platform-specific analytics via MCP**:
```bash
social-forge mcp-call ig_get_insights --args '{"ig_id":"<ID>","metrics":["reach","follower_count"]}'  # Instagram
social-forge mcp-call ig_get_insights_audience --args '{"ig_id":"<ID>"}'                              # Instagram audience
social-forge mcp-call lip_get_analytics --args '{"page_id":"<ID>"}'                                  # LinkedIn Page
social-forge mcp-call lip_get_post_analytics --args '{"post_urn":"<URN>"}'                           # LinkedIn post
social-forge mcp-call pi_get_board_analytics --args '{"board_id":"<ID>"}'                            # Pinterest board
social-forge mcp-call pi_get_pin_analytics --args '{"pin_id":"<ID>"}'                                # Pinterest pin
social-forge mcp-call yt_get_channel_stats --args '{"channel_id":"<ID>"}'                            # YouTube channel
social-forge mcp-call yt_get_analytics --args '{"channel_id":"<ID>"}'                                # YouTube analytics
social-forge mcp-call th_get_insights --args '{"thread_id":"<ID>"}'                                  # Threads
```

---

## Engage

```bash
social-forge x like <TWEET_ID>
social-forge x retweet <TWEET_ID>
social-forge x bookmark <TWEET_ID>
social-forge reddit vote <THING_ID> up
social-forge reddit save <THING_ID>
```

**Platform-specific engagement via MCP**:
```bash
social-forge mcp-call fb_react --args '{"post_id":"<ID>","reaction":"LIKE"}'      # Facebook reaction
social-forge mcp-call di_add_reaction --args '{"channel_id":"<C>","message_id":"<M>","emoji":"👍"}'  # Discord reaction
social-forge mcp-call x_follow_user --args '{"user_id":"<ID>"}'                   # X follow
```

---

## Staging

```bash
social-forge stage "Content" --integrations <UUID1>,<UUID2>            # Stage
social-forge stage "Content" --platforms x,linkedin --preview          # Preview only
```

---

## Automation

```bash
social-forge automation create <INTEGRATION_ID> "Name" --trigger comment --response "Thanks!"
social-forge automation list
social-forge automation update <RULE_ID> --name "New name" --active true
social-forge automation delete <RULE_ID>
social-forge automation logs <RULE_ID> --limit 50
```

---

## Webhooks

```bash
social-forge mcp-call wh_create --args '{"url":"https://example.com/hook","events":["post.published"]}'
social-forge mcp-call wh_list --args '{}'
social-forge mcp-call wh_get --args '{"webhook_id":"<ID>"}'
social-forge mcp-call wh_update --args '{"webhook_id":"<ID>","events":["post.scheduled"]}'
social-forge mcp-call wh_delete --args '{"webhook_id":"<ID>"}'
social-forge mcp-call wh_test --args '{"webhook_id":"<ID>"}'
```

---

## Notifications

```bash
social-forge mcp-call notif_list --args '{}'
social-forge mcp-call notif_mark_read --args '{"notification_id":"<ID>"}'
social-forge mcp-call notif_mark_all_read --args '{}'
social-forge mcp-call notif_create --args '{"title":"Title","body":"Body"}'
```

---

## Tags

```bash
social-forge mcp-call tag_create --args '{"name":"work"}'
social-forge mcp-call tag_list --args '{}'
social-forge mcp-call tag_get --args '{"tag_id":"<ID>"}'
social-forge mcp-call tag_update --args '{"tag_id":"<ID>","name":"updated"}'
social-forge mcp-call tag_delete --args '{"tag_id":"<ID>"}'
```

---

## Google Workspace

```bash
# YouTube
social-forge mcp-call goog_search_videos --args '{"query":"rust tutorial"}'
social-forge mcp-call goog_get_video --args '{"video_id":"<ID>"}'
social-forge mcp-call yt_get_channel_stats --args '{"channel_id":"<ID>"}'

# Gmail
social-forge mcp-call goog_list_messages --args '{}'
social-forge mcp-call goog_get_message --args '{"message_id":"<ID>"}'
social-forge mcp-call goog_send_message --args '{"to":"user@example.com","subject":"Hi","body":"Hello!"}'
social-forge mcp-call goog_list_labels --args '{}'
social-forge mcp-call goog_get_thread --args '{"thread_id":"<ID>"}'
social-forge mcp-call goog_search_messages --args '{"query":"from:me"}'

# Google Calendar
social-forge mcp-call goog_list_calendars --args '{}'
social-forge mcp-call goog_list_events --args '{"calendar_id":"primary"}'
social-forge mcp-call goog_create_event --args '{"calendar_id":"primary","summary":"Meeting","start":"2026-07-01T09:00:00Z","end":"2026-07-01T10:00:00Z"}'
social-forge mcp-call goog_update_event --args '{"calendar_id":"primary","event_id":"<ID>","summary":"Updated"}'
social-forge mcp-call goog_delete_event --args '{"calendar_id":"primary","event_id":"<ID>"}'

# Google Drive
social-forge mcp-call goog_list_files --args '{}'
social-forge mcp-call goog_get_file --args '{"file_id":"<ID>"}'
social-forge mcp-call goog_search_files --args '{"query":"name contains 'report''}'
social-forge mcp-call goog_export_file --args '{"file_id":"<ID>","mime_type":"application/pdf"}'
```

---

## GitHub

```bash
social-forge mcp-call gh_get_authenticated_user --args '{}'                # My profile
social-forge mcp-call gh_list_my_repos --args '{}'                         # My repos
social-forge mcp-call gh_list_repos --args '{"owner":"<ORG>"}'             # Org repos
social-forge mcp-call gh_get_repo --args '{"owner":"<O>","repo":"<R>"}'    # Repo details
social-forge mcp-call gh_list_issues --args '{"owner":"<O>","repo":"<R>"}' # Issues
social-forge mcp-call gh_create_issue --args '{"owner":"<O>","repo":"<R>","title":"Bug","body":"Details"}'
social-forge mcp-call gh_close_issue --args '{"owner":"<O>","repo":"<R>","issue_number":42}'
social-forge mcp-call gh_list_pull_requests --args '{"owner":"<O>","repo":"<R>"}'  # PRs
social-forge mcp-call gh_list_branches --args '{"owner":"<O>","repo":"<R>"}'      # Branches
social-forge mcp-call gh_list_commits --args '{"owner":"<O>","repo":"<R>"}'       # Commits
social-forge mcp-call gh_list_releases --args '{"owner":"<O>","repo":"<R>"}'      # Releases
social-forge mcp-call gh_search_repos --args '{"query":"language:rust stars:>100"}'  # Search
social-forge mcp-call gh_search_code --args '{"query":"filename:Cargo.toml org:<O>"}'  # Code search
```

---

## WhatsApp

```bash
social-forge mcp-call wa_auth_status --args '{}'                                  # Check status
social-forge mcp-call wa_chats --args '{}'                                        # List chats
social-forge mcp-call wa_contacts --args '{}'                                     # Contacts
social-forge mcp-call wa_send_text --args '{"number":"+1234567890","message":"Hi"}' # Send text
social-forge mcp-call wa_list_groups --args '{}'                                  # Groups
social-forge mcp-call wa_create_group --args '{"name":"Team","participants":["<PHONE>"]}'
social-forge mcp-call wa_group_invite_link --args '{"group_id":"<ID>"}'           # Invite link
```

---

## Telegram

```bash
# Bot
social-forge mcp-call tb_send_message --args '{"chat_id":"<ID>","text":"Hello"}'
social-forge mcp-call tb_send_photo --args '{"chat_id":"<ID>","photo":"<URL>"}'
social-forge mcp-call tb_get_chat --args '{"chat_id":"<ID>"}'
social-forge mcp-call tb_get_updates --args '{}'

# User
social-forge mcp-call tu_send_message --args '{"dialog_id":"<ID>","message":"Hello"}'
social-forge mcp-call tu_list_dialogs --args '{}'
social-forge mcp-call tu_list_contacts --args '{}'
social-forge mcp-call tu_search --args '{"query":"John"}'
```

---

## Discord

```bash
social-forge mcp-call di_get_server_info --args '{}'
social-forge mcp-call di_get_guild_channels --args '{"guild_id":"<ID>"}'
social-forge mcp-call di_send_message --args '{"channel_id":"<ID>","content":"Hello"}'
social-forge mcp-call di_get_messages --args '{"channel_id":"<ID>","limit":10}'
social-forge mcp-call di_create_forum_post --args '{"channel_id":"<ID>","name":"Topic","content":"Post body"}'
```

---

## Slack

```bash
social-forge mcp-call sl_list_channels --args '{}'
social-forge mcp-call sl_channel_history --args '{"channel_id":"<ID>","limit":10}'
social-forge mcp-call sl_send_message --args '{"channel":"<ID>","message":"Hello"}'
social-forge mcp-call sl_list_users --args '{}'
```


---

## New Platform CLI Commands

### TikTok
```bash
social-forge tiktok profile --json                          # Get profile
social-forge tiktok post 'Caption text' --json              # Post video
social-forge tiktok videos --limit 10 --json                # List videos
```

### Threads
```bash
social-forge threads profile <ACCOUNT_ID> --json            # Get profile
social-forge threads list <ACCOUNT_ID> --limit 10 --json    # List posts
social-forge threads post <ACCOUNT_ID> 'Text' --json        # Create post
social-forge threads reply <ACCOUNT_ID> <MEDIA_ID> 'Reply' --json  # Reply
social-forge threads delete <ACCOUNT_ID> <MEDIA_ID> --json  # Delete post
social-forge threads insights <ACCOUNT_ID> 'impression_count' --json  # Insights
```

### Discord
```bash
social-forge discord channels <GUILD_ID> --json             # List channels
social-forge discord messages <CHANNEL_ID> --limit 10 --json # Get messages
social-forge discord send <CHANNEL_ID> 'Hello' --json       # Send message
social-forge discord server <GUILD_ID> --json               # Server info
social-forge discord forum <CHANNEL_ID> 'Title' 'Content' --json  # Forum post
```

### Slack
```bash
social-forge slack channels --json                          # List channels
social-forge slack history <CHANNEL_ID> --limit 10 --json   # Channel history
social-forge slack send <CHANNEL_ID> 'Hello' --json         # Send message
social-forge slack users --json                             # List users
```

### Telegram Bot
```bash
social-forge telegram-bot send --chat-id <ID> 'Hello' --json  # Send message
social-forge telegram-bot photo --chat-id <ID> <URL> --json  # Send photo
social-forge telegram-bot document --chat-id <ID> <PATH> --json  # Send document
social-forge telegram-bot chat --chat-id <ID> --json         # Chat info
social-forge telegram-bot updates --json                     # Get updates
```

### Telegram User
```bash
social-forge telegram-user send <PEER> 'Hello' --json       # Send message
social-forge telegram-user dialogs --limit 10 --json        # List dialogs
social-forge telegram-user contacts --json                  # List contacts
social-forge telegram-user search 'query' --json            # Search messages
```

### WhatsApp
```bash
social-forge whatsapp send +1234567890 'Hello' --json       # Send text
social-forge whatsapp chats --limit 10 --json               # List chats
social-forge whatsapp contacts --limit 20 --json            # List contacts
social-forge whatsapp groups --json                         # List groups
social-forge whatsapp create-group 'Name' <PHONE1> <PHONE2> --json  # Create group
social-forge whatsapp invite-link <GROUP_JID> --json        # Get invite link
```

### Pinterest
```bash
social-forge pinterest profile <BOARD_ID> --json            # Get profile
social-forge pinterest board <BOARD_ID> --json              # Get board
social-forge pinterest pins <BOARD_ID> --limit 25 --json    # Get pins
social-forge pinterest pin <BOARD_ID> <PIN_ID> --json       # Get pin
social-forge pinterest search 'query' --limit 20 --json     # Search pins
social-forge pinterest board-analytics <BOARD_ID> --start-date <D> --end-date <D> --json  # Board analytics
social-forge pinterest pin-analytics <BOARD_ID> <PIN_ID> --start-date <D> --end-date <D> --json  # Pin analytics
```

### GitHub
```bash
social-forge github me --json                               # My profile
social-forge github my-repos --limit 30 --json              # My repos
social-forge github user <LOGIN> --json                     # User profile
social-forge github repos <USERNAME> --limit 30 --json      # User repos
social-forge github issues <OWNER> <REPO> --json            # List issues
social-forge github prs <OWNER> <REPO> --json               # List PRs
social-forge github create-issue <OWNER> <REPO> 'Title' --body 'Body' --json  # Create issue
social-forge github close-issue <OWNER> <REPO> <NUMBER> --json  # Close issue
social-forge github commits <OWNER> <REPO> --limit 30 --json   # List commits
social-forge github branches <OWNER> <REPO> --limit 30 --json  # List branches
social-forge github search 'query' --limit 10 --json        # Search repos
social-forge github releases <OWNER> <REPO> --limit 30 --json  # List releases
```

### WordPress
```bash
social-forge wordpress post 'Title' 'Content' --json        # Create post
social-forge wordpress list --limit 10 --json               # List posts
social-forge wordpress get <POST_ID> --json                 # Get post
social-forge wordpress categories --json                    # List categories
```

### Hashnode
```bash
social-forge hashnode post <PUB_ID> 'Title' 'Content' --json  # Create post
social-forge hashnode list <PUB_ID> --json                  # List posts
social-forge hashnode get <POST_ID> --json                  # Get post
```

### Medium
```bash
social-forge medium-blog post 'Content' --json              # Create post
social-forge medium-blog list --json                        # List posts
social-forge medium-blog get <POST_ID> --json               # Get post
```

### Dev.to
```bash
social-forge devto post 'Content' --json                    # Create article
social-forge devto list --json                              # List articles
social-forge devto get <POST_ID> --json                     # Get article
```

### Skool
```bash
social-forge skool post <GROUP_ID> 'Title' 'Content' --json  # Publish post
social-forge skool info <SLUG> --json                       # Community info
social-forge skool posts <SLUG> --json                      # List posts
social-forge skool comment <POST_ID> <GROUP_ID> 'Content' --json  # Create comment
```

### Google Workspace
```bash
# YouTube
social-forge google youtube-search <CHANNEL_ID> 'query' --limit 10 --json  # Search videos
social-forge google video <CHANNEL_ID> <VIDEO_ID> --json   # Get video details
social-forge google playlists <CHANNEL_ID> --limit 10 --json  # List playlists
social-forge google channel-stats <CHANNEL_ID> --json      # Channel stats

# Google Drive
social-forge gdrive files --limit 20 --json                 # List files
social-forge gdrive file <FILE_ID> --json                   # Get file
social-forge gdrive search 'query' --limit 20 --json        # Search files
social-forge gdrive folders --limit 50 --json               # List folders
social-forge gdrive metadata <FILE_ID> --json               # File metadata
social-forge gdrive export <FILE_ID> 'application/pdf' --json  # Export file

# Google Calendar
social-forge gcal calendars --json                          # List calendars
social-forge gcal events --limit 20 --json                  # List events
social-forge gcal event --event-id <ID> --json              # Get event
social-forge gcal create --title 'Meeting' --start '2026-07-02T09:00:00Z' --end '2026-07-02T10:00:00Z' --json  # Create event
social-forge gcal delete --event-id <ID> --json             # Delete event

# Gmail
social-forge gmail-ops profile --json                       # Gmail profile
social-forge gmail-ops messages --limit 20 --json           # List messages
social-forge gmail-ops message <ID> --json                  # Get message
social-forge gmail-ops send --to 'user@example.com' --subject 'Hi' --body 'Hello!' --json  # Send email
social-forge gmail-ops labels --json                        # List labels
social-forge gmail-ops thread <ID> --json                   # Get thread
social-forge gmail-ops search 'from:me' --limit 20 --json   # Search messages
```

### Webhooks
```bash
social-forge webhooks list --json                           # List webhooks
social-forge webhooks create 'https://example.com/hook' --name 'my-hook' --json  # Create
social-forge webhooks get <ID> --json                       # Get webhook
social-forge webhooks update <ID> --name 'new-name' --url 'https://new.url' --json  # Update
social-forge webhooks delete <ID> --json                    # Delete webhook
social-forge webhooks test <ID> --json                      # Test webhook
```

### Notifications
```bash
social-forge notifications list --limit 50 --json           # List notifications
social-forge notifications read <ID> --json                 # Mark as read
social-forge notifications read-all --json                  # Mark all as read
social-forge notifications create 'Title' 'Body' --json     # Create notification
```

### Tags
```bash
social-forge tags list --json                               # List tags
social-forge tags create 'work' --color '#ff0000' --json   # Create tag
social-forge tags get <ID> --json                           # Get tag
social-forge tags update <ID> --name 'updated' --color '#00ff00' --json  # Update tag
social-forge tags delete <ID> --json                        # Delete tag
```

### Analytics
```bash
social-forge analytics get <PROVIDER> --days 7 --json      # Provider analytics
social-forge analytics post <POST_ID> --json               # Post analytics
```

---

## Errors

```bash
social-forge doctor                    # Check all providers
social-forge providers --pretty        # See what's connected
social-forge connect <provider>        # Fix missing provider
social-forge <cmd> --help              # Fix wrong flags
```
