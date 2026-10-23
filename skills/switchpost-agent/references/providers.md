# Switchpost — Provider Reference

## Contents

- [Post to a platform](#post-to-a-platform)
- [Read a feed or profile](#read-a-feed-or-profile)
- [Schedule a post](#schedule-a-post)
- [Manage media](#manage-media)
- [Create a carousel](#create-a-carousel)
- [Comment and reply](#comment-and-reply)
- [Send a DM](#send-a-dm)
- [Post to Instagram (3-step flow)](#post-to-instagram-3-step-flow)
- [Check analytics](#check-analytics)
- [Engage (like, retweet, vote, bookmark)](#engage-like-retweet-vote-bookmark)
- [Manage automation rules](#manage-automation-rules)
- [New Platform CLI Commands](#new-platform-cli-commands)
- [Cross-cutting MCP tools](#cross-cutting-mcp-tools)
- [Platform-specific MCP tools](#platform-specific-mcp-tools)
  - [X / Twitter (20 tools)](#x-twitter-20-tools)
  - [Facebook (15 tools)](#facebook-15-tools)
  - [Instagram (15+ tools)](#instagram-15-tools)
  - [LinkedIn Page (12 tools)](#linkedin-page-12-tools)
  - [LinkedIn Personal (12 tools)](#linkedin-personal-12-tools)
  - [Reddit (36 tools)](#reddit-36-tools)
  - [Bluesky (5 tools)](#bluesky-5-tools)
  - [Discord (13 tools)](#discord-13-tools)
  - [Telegram Bot (9 tools)](#telegram-bot-9-tools)
  - [Telegram User (13 tools)](#telegram-user-13-tools)
  - [WhatsApp (13 tools)](#whatsapp-13-tools)
  - [GitHub (18 tools)](#github-18-tools)
  - [Google (28 tools)](#google-28-tools)
  - [WordPress (4 tools)](#wordpress-4-tools)
  - [Other platforms](#other-platforms)

Complete CLI commands and MCP tools for every supported provider. Grouped by task.

The MCP surface is 389 MCP tools (389 tools total across the registry), mirrored by the
CLI so an agent never has to fall back to HTTP.

---

## Post to a platform

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **X/Twitter** | `switchpost x post "<text>"` | Cookie auth for full access |
| **LinkedIn personal** | `switchpost linkedin post "<text>"` | OAuth |
| **LinkedIn page** | `switchpost linkedin-page post <PAGE_ID> "<text>"` | OAuth |
| **Reddit** | `switchpost reddit post --title "<t>" --text "<b>" --target <sub>` | Cookie auth |
| **Facebook** | Use `mcp-call fb_create_post '{"page_id":"...","message":"..."}'` | No native CLI post command |
| **Instagram** | Use `mcp-call ig_create_container` + `ig_poll_container` + `ig_publish_container` | 3-step flow (see below) |
| **Bluesky** | Use `mcp-call bs_create_post '{"text":"..."}'` | App password auth |
| **Mastodon** | Use `mcp-call ms_create_post '{"status":"..."}'` | Access token auth |
| **WordPress** | Use `mcp-call wp_create_post '{"title":"...","content":"..."}'` | App password auth |
| **Dev.to** | Use `mcp-call dv_create_post '{"title":"...","body_markdown":"..."}'` | API key auth |
| **Hashnode** | `switchpost hashnode post <PUB_ID> "title" "content"` | PAT auth |
| **Medium** | `switchpost medium-blog post "content"` | Token auth |
| **TikTok** | `switchpost tiktok post "text"` | OAuth |
| **Skool** | `switchpost skool post <GROUP_ID> "title" "content"` | API key auth |

**Cross-platform**: `switchpost post "<text>" --platforms x,linkedin,bluesky` — auto-splits content.

---

## Read a feed or profile

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **X/Twitter** | `switchpost x timeline --count 5` | Home timeline |
| **X search** | `switchpost x search "<query>"` | Search tweets |
| **X user** | `switchpost x user <username>` | Profile by username |
| **Reddit browse** | `switchpost reddit browse <subreddit> --sort hot` | Subreddit feed |
| **Reddit search** | `switchpost reddit search "<query>"` | Search Reddit |
| **Reddit user** | `switchpost reddit user <username>` | User profile |
| **Reddit inbox** | `switchpost reddit inbox` | Read inbox |
| **Instagram posts** | `switchpost instagram posts <ACCOUNT_ID>` | Recent media |
| **LinkedIn personal** | `switchpost linkedin posts` | Your recent posts |
| **LinkedIn profile** | `switchpost linkedin profile` | Your profile |
| **LinkedIn page** | `switchpost linkedin-page feed <PAGE_ID>` | Page posts |
| **Facebook page** | `switchpost facebook posts <PAGE_ID>` | Page feed |
| **Import + feed** | `switchpost import x --count 20` then `switchpost feed` | Unified local feed |

---

## Schedule a post

| Command | Description |
|---------|-------------|
| `switchpost posts create "<content>" --integrations <UUID>` | Create a queued post |
| `switchpost posts create "<content>" --integrations <UUID> --schedule <ISO>` | Create + schedule |
| `switchpost posts find-slot --integration <UUID>` | Find next available time slot |
| `switchpost posts list --limit 10` | List scheduled/queued posts |
| `switchpost posts get <POST_ID>` | Get post details |
| `switchpost posts publish <POST_ID>` | Publish immediately |
| `switchpost posts delete <POST_ID>` | Cancel/delete a post |

**Staging** (multi-platform review before publish):
| Command | Description |
|---------|-------------|
| `switchpost stage "<content>" --integrations <UUID1>,<UUID2>` | Stage across platforms |
| `switchpost stage "<content>" --platforms x,linkedin --preview` | Preview splits only |

---

## Manage media

| Command | Description |
|---------|-------------|
| `switchpost media upload <file>` | Upload a file |
| `switchpost media upload <url>` | Upload from URL |
| `switchpost media upload-batch <file1> <file2> ...` | Upload multiple files |
| `switchpost media list --limit 10` | List uploaded media |
| `switchpost media download <url> --output <path>` | Download a file |

---

## Create a carousel

| Command | Description |
|---------|-------------|
| `switchpost carousel "<text>" --integration <UUID> --media <url1>,<url2>` | Multi-image carousel |

---

## Comment and reply

| Command | Description |
|---------|-------------|
| `switchpost comment get <INTEGRATION_ID> <POST_ID>` | Get comments on a post |
| `switchpost comment reply <INTEGRATION_ID> <COMMENT_ID> "<text>"` | Reply to a comment |

**Platform-specific comments via CLI**:
| Platform | CLI Command |
|----------|-------------|
| **X** | `switchpost x post "<text>"` (replies are posts) |
| **Reddit** | `switchpost reddit comment <THING_ID> "<text>"` |
| **Facebook** | `switchpost facebook comment <POST_ID> "<text>"` |
| **Instagram** | `switchpost instagram comment <MEDIA_ID> "<text>"` |

---

## Send a DM

| Command | Description |
|---------|-------------|
| `switchpost dm send <INTEGRATION_ID> <RECIPIENT_ID> "<text>"` | Send a message |
| `switchpost dm list <INTEGRATION_ID>` | List conversations |
| `switchpost dm messages <INTEGRATION_ID> <CONVERSATION_ID>` | Get messages in a conversation |

**Platform-specific DMs via MCP**:
| Platform | MCP Tool |
|----------|----------|
| **Reddit** | `reddit_send_dm` |
| **Facebook** | `fb_send_message` |
| **Discord** | `di_send_message` |
| **Telegram Bot** | `tb_send_message` |
| **Telegram User** | `tu_send_message` |
| **WhatsApp** | `wa_send_text` |
| **Slack** | `sl_send_message` |

---

## Post to Instagram (3-step flow)

Instagram requires a container → poll → publish flow. No single CLI command handles this.

**Step 1: Create container**
```bash
switchpost mcp-call ig_create_container --args '{"ig_id":"<IG_ACCOUNT_ID>","image_url":"https://example.com/photo.jpg","caption":"Your caption here"}'
# Returns: {"id":"CONTAINER_ID","status_code":"FINISHED"}
```

**Step 2: Poll container** (if status_code is not FINISHED)
```bash
switchpost mcp-call ig_poll_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
# Returns: {"id":"CONTAINER_ID","status_code":"FINISHED"} when ready
```

**Step 3: Publish**
```bash
switchpost mcp-call ig_publish_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
# Returns: {"id":"MEDIA_ID"} on success
```

**Important**:
- `image_url` must be a publicly accessible URL (not a local file path)
- For local files, upload first: `switchpost media upload ./photo.jpg` then use the returned URL
- Poll until `status_code` is `FINISHED` before publishing
- Video containers take longer; poll 2-3 times with 5-second delays

**For Reels**:
```bash
switchpost mcp-call ig_create_container --args '{"ig_id":"<IG_ACCOUNT_ID>","video_url":"https://example.com/video.mp4","caption":"Reel caption","media_type":"REELS"}'
switchpost mcp-call ig_poll_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
switchpost mcp-call ig_publish_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID","media_type":"REELS"}'
```

---

## Check analytics

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **Instagram** | `switchpost instagram insights <ACCOUNT_ID> --metric <metrics>` | Comma-separated metrics |
| **LinkedIn page** | `switchpost linkedin-page analytics <PAGE_ID>` | Page analytics |
| **LinkedIn followers** | `switchpost linkedin-page followers <PAGE_ID>` | Follower count |
| **LinkedIn personal** | `switchpost linkedin analytics` | Profile analytics |
| **Facebook** | `switchpost facebook insights <PAGE_ID> --metric <metrics>` | Page insights |

**Instagram metrics** (comma-separated):
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

---

## Engage (like, retweet, vote, bookmark)

| Platform | CLI Command |
|----------|-------------|
| **X like** | `switchpost x like <TWEET_ID>` |
| **X retweet** | `switchpost x retweet <TWEET_ID>` |
| **X bookmark** | `switchpost x bookmark <TWEET_ID>` |
| **X delete** | `switchpost x delete <TWEET_ID>` |
| **Reddit vote** | `switchpost reddit vote <THING_ID> up` or `down` |
| **Reddit save** | `switchpost reddit save <THING_ID>` |
| **Reddit unsave** | `switchpost reddit unsave <THING_ID>` |
| **Reddit delete** | `switchpost reddit delete <THING_ID>` |

---

## Manage automation rules

| Command | Description |
|---------|-------------|
| `switchpost automation create <INTEGRATION_ID> "<name>" <TRIGGER_TYPE> "<response>" <RESPONSE_TYPE>` | Create rule (trigger: comment, dm, mention, follow; response: fixed, template, ai_generated) |
| `switchpost automation list` | List all rules |
| `switchpost automation update <RULE_ID> --name "<name>" --active true` | Update rule |
| `switchpost automation delete <RULE_ID>` | Delete rule |
| `switchpost automation logs <RULE_ID> --limit 50` | View execution logs |


---

## New Platform CLI Commands

| Platform | CLI Command | Subcommands |
|----------|-------------|-------------|
| **TikTok** | `switchpost tiktok <action>` | `profile`, `post`, `videos` |
| **Threads** | `switchpost threads <action>` | `profile`, `list`, `post`, `reply`, `delete`, `insights` |
| **Discord** | `switchpost discord <action>` | `channels`, `messages`, `send`, `server`, `forum` |
| **Slack** | `switchpost slack <action>` | `channels`, `history`, `send`, `users` |
| **Telegram Bot** | `switchpost telegram-bot <action>` | `send`, `photo`, `document`, `chat`, `updates` |
| **Telegram User** | `switchpost telegram-user <action>` | `send`, `dialogs`, `contacts`, `search` |
| **WhatsApp** | `switchpost whatsapp <action>` | `send`, `chats`, `contacts`, `groups`, `create-group`, `invite-link` |
| **Pinterest** | `switchpost pinterest <action>` | `profile`, `board`, `pins`, `pin`, `search`, `board-analytics`, `pin-analytics` |
| **GitHub** | `switchpost github <action>` | `me`, `my-repos`, `user`, `repos`, `issues`, `prs`, `create-issue`, `close-issue`, `commits`, `branches`, `search`, `releases` |
| **WordPress** | `switchpost wordpress <action>` | `post`, `list`, `get`, `categories` |
| **Hashnode** | `switchpost hashnode <action>` | `post`, `list`, `get` |
| **Medium** | `switchpost medium-blog <action>` | `post`, `list`, `get` |
| **Dev.to** | `switchpost devto <action>` | `post`, `list`, `get` |
| **Skool** | `switchpost skool <action>` | `post`, `info`, `posts`, `comment` |
| **Google (YouTube)** | `switchpost google <action>` | `youtube-search`, `video`, `playlists`, `channel-stats` |
| **Google Drive** | `switchpost gdrive <action>` | `files`, `file`, `search`, `folders`, `metadata`, `export` |
| **Google Calendar** | `switchpost gcal <action>` | `calendars`, `events`, `event`, `create`, `delete` |
| **Gmail** | `switchpost gmail-ops <action>` | `profile`, `messages`, `message`, `send`, `labels`, `thread`, `search` |
| **Webhooks** | `switchpost webhooks <action>` | `list`, `create`, `delete`, `get`, `update`, `test` |
| **Notifications** | `switchpost notifications <action>` | `list`, `read`, `read-all`, `create` |
| **Tags** | `switchpost tags <action>` | `list`, `create`, `delete`, `get`, `update` |
| **Analytics** | `switchpost analytics <action>` | `get`, `post` |

---

## Cross-cutting MCP tools

When CLI doesn't have a command for what you need, use `mcp-call`:

```bash
switchpost mcp-tools --pretty                    # List all 389 tools (bridge-accessible)
switchpost mcp-tools --toolset x --pretty         # Only the X toolset (session-sizing; mirrors SWITCHPOST_TOOLSETS filtering)
switchpost mcp-call <tool_name> --args '{"key":"value"}' # Call any tool
```

| Category | Key MCP Tools |
|----------|---------------|
| **Auth** | `auth_register`, `auth_login`, `auth_me` |
| **Integrations** | `integrations_list`, `integrations_list_providers`, `integrations_connect`, `integrations_disconnect`, `integrations_list_targets` |
| **Posts** | `posts_create`, `posts_list`, `posts_get`, `posts_update`, `posts_delete` (gains `delete_from_platform: true` to also delete the published post on the platform — X/LinkedIn/FB support it; failures keep the local row for retry), `posts_schedule`, `posts_publish`, `posts_find_slot`, `posts_stage` (v2: per-provider `overrides`, `dry_run`, `force`, campaign `group_id`, and `scheduled_at: "next-slot"` for each account's next free slot), `posts_thread`, `posts_unschedule`, `posts_reschedule` (incl. `move_group`), `posts_group`, `posts_validate`, `posts_repeat` (requires `post_id`, `scheduled_at`, `end_date`; creates the copies and returns `group_id` + `post_ids` + `scheduled_dates`) |
| **Media** | `posts_media_upload`, `posts_media_upload_from_path`, `posts_media_upload_batch`, `posts_media_upload_from_url`, `posts_media_list` |
| **Feed** | `feed_import`, `feed_list` |
| **Calendar** | `calendar_get` |
| **Analytics** | `analytics_get`, `analytics_get_post` |
| **Tags** | `tag_create`, `tag_list`, `tag_get`, `tag_update`, `tag_delete` |
| **Notifications** | `notif_create`, `notif_list`, `notif_mark_read`, `notif_mark_all_read` |
| **Webhooks** | `wh_create`, `wh_list`, `wh_get`, `wh_update`, `wh_delete`, `wh_test` |

---

## Platform-specific MCP tools

### X / Twitter (27 tools)

**Not listed above (7 more, 20 shown):** `x_create_tweet`, `x_get_analytics`, `x_get_dm_conversation`, `x_get_post_analytics`, `x_list_dms`, `x_reply_tweet`, `x_send_dm`
`x_get_me`, `x_home_timeline`, `x_search_tweets`, `x_tweet_detail`, `x_user_lookup`, `x_user_lookup_by_username`, `x_user_tweets`, `x_like_tweet`, `x_unlike_tweet`, `x_retweet`, `x_unretweet`, `x_bookmark_tweet`, `x_unbookmark_tweet`, `x_bookmarks`, `x_delete_tweet`, `x_follow_user`, `x_unfollow_user`, `x_followers`, `x_following`, `x_list_timeline`

### Facebook (15 tools)
`fb_get_feed`, `fb_get_post`, `fb_get_comments`, `fb_create_post`, `fb_create_photo`, `fb_create_video`, `fb_delete_post`, `fb_comment`, `fb_react`, `fb_page_insights`, `fb_conversations`, `fb_conversation_messages`, `fb_send_message`, `fb_search_pages`, `fb_albums`

### Instagram (21 tools)

**Not listed above (4 more, 17 shown):** `ig_get_messages`, `ig_list_conversations`, `ig_post`, `ig_send_dm`
`ig_get_media`, `ig_get_media_detail`, `ig_get_comments`, `ig_get_insights`, `ig_get_insights_audience`, `ig_get_followers`, `ig_get_reels`, `ig_get_stories`, `ig_get_mentions`, `ig_get_tagged`, `ig_search_hashtag`, `ig_get_hashtag_media`, `ig_create_container`, `ig_publish_container`, `ig_poll_container`, `ig_reply_to_comment`, `ig_business_discovery`

### LinkedIn Page (11 tools)
`lip_list_pages`, `lip_get_page`, `lip_get_page_posts`, `lip_create_post`, `lip_create_comment`, `lip_delete_post`, `lip_get_analytics`, `lip_get_post_analytics`, `lip_get_followers`, `lip_get_reactions`, `lip_get_shares`

### LinkedIn Personal (15 tools)

**Not listed above (4 more, 11 shown):** `li_get_messages`, `li_list_conversations`, `li_reply_comment`, `li_send_dm`
`li_get_profile`, `li_get_posts`, `li_create_post`, `li_delete_post`, `li_get_post_detail`, `li_get_comments`, `li_create_comment`, `li_get_reactions`, `li_get_shares`, `li_get_analytics`, `li_get_post_analytics`

### Reddit (37 tools)
`reddit_browse`, `reddit_search`, `reddit_post_detail`, `reddit_create_post`, `reddit_create_comment`, `reddit_get_comments`, `reddit_vote`, `reddit_save`, `reddit_unsave`, `reddit_delete`, `reddit_edit`, `reddit_hide`, `reddit_subscribe`, `reddit_user_info`, `reddit_send_dm`, `reddit_inbox`, `reddit_get_karma`, `reddit_mod_remove`, `reddit_mod_approve`, `reddit_mod_distinguish`, `reddit_mod_sticky`, `reddit_mod_lock`, `reddit_mod_unlock`, `reddit_report`, `reddit_block_user`, `reddit_list_friends`, `reddit_add_friend`, `reddit_remove_friend`, `reddit_mark_nsfw`, `reddit_unmark_nsfw`, `reddit_mark_spoiler`, `reddit_unmark_spoiler`, `reddit_get_rules`, `reddit_list_flairs` (link flairs → flair_id for create_post), `reddit_get_saved`, `reddit_read_all_messages`, `reddit_crosspost`

### Bluesky (7 tools)

**Not listed above (2 more, 5 shown):** `bs_get_analytics`, `bs_reply`
`bs_timeline`, `bs_search`, `bs_create_post`, `bs_profile`, `bs_feed`

### Discord (15 tools)

**Not listed above (2 more, 13 shown):** `di_webhook_info`, `di_webhook_post`
`di_send_message`, `di_get_messages`, `di_get_channel`, `di_get_guild`, `di_get_guild_channels`, `di_get_server_info`, `di_add_reaction`, `di_delete_message`, `di_create_forum_post`, `di_get_thread_members`, `di_history` (paginated, chronological), `di_search` (guild-wide), `di_tail` (newest-id cursor for watch loops)

### Telegram Bot (11 tools)

**Not listed above (1 more, 10 shown):** `tb_list_chats`
`tb_send_message`, `tb_send_photo`, `tb_send_document`, `tb_get_chat`, `tb_get_chat_member_count`, `tb_get_me`, `tb_get_updates`, `tb_pin_message`, `tb_unpin_message`, `tb_forward_message`

### Telegram User (13 tools)
`tu_auth_status`, `tu_list_contacts`, `tu_list_dialogs`, `tu_search`, `tu_send_message`, `tu_request_code`, `tu_sign_in`, `tu_history` (chronological, offset_id paging), `tu_tail` (after_id cursor), `tu_edit`, `tu_delete`, `tu_queue_send` (rate-limited write queue; in-memory — a restart drops pending jobs), `tu_queue_status`

### WhatsApp (13 tools: 9 `wa_` Web session + 4 `wc_` Cloud API)
`wa_auth_status`, `wa_chats`, `wa_contacts`, `wa_send_text`, `wa_edit_message`, `wa_revoke_message`, `wa_create_group`, `wa_list_groups`, `wa_group_invite_link`, `wc_list_templates`, `wc_create_template`, `wc_delete_template`, `wc_send_template` (WhatsApp Cloud API for business numbers; scheduled posts with settings.auth_method=cloud publish via approved templates)

### GitHub (18 tools)
`gh_list_my_repos`, `gh_list_repos`, `gh_get_repo`, `gh_get_repo_content`, `gh_list_issues`, `gh_get_issue`, `gh_create_issue`, `gh_close_issue`, `gh_list_pull_requests`, `gh_get_pull_request`, `gh_list_branches`, `gh_list_commits`, `gh_list_releases`, `gh_list_contributors`, `gh_get_authenticated_user`, `gh_get_user`, `gh_search_code`, `gh_search_repos`

### Google (28 tools)
`goog_list_calendars`, `goog_list_events`, `goog_get_event`, `goog_create_event`, `goog_update_event`, `goog_delete_event`, `goog_list_files`, `goog_get_file`, `goog_search_files`, `goog_list_folders`, `goog_export_file`, `goog_get_file_metadata`, `goog_list_messages`, `goog_get_message`, `goog_search_messages`, `goog_get_thread`, `goog_send_message`, `goog_list_labels`, `goog_get_profile`, `goog_get_video`, `goog_search_videos`, `goog_get_analytics`, `goog_get_channel_stats`, `goog_get_comments`, `goog_get_playlist_items`, `goog_list_playlists`, `goog_get_subscriptions`, `goog_find_creators`

### WordPress (4 tools)
`wp_create_post`, `wp_get_post`, `wp_list_posts`, `wp_list_categories`

### Other platforms
| Platform | MCP Tools |
|----------|-----------|
| **Dev.to** | `dv_create_post`, `dv_get_post`, `dv_list_posts` |
| **Hashnode** | `hn_create_post`, `hn_get_post`, `hn_list_posts` |
| **Medium** | `md_create_post`, `md_get_post`, `md_list_posts` |
| **Mastodon** | `ms_create_post`, `ms_get_post`, `ms_get_timeline`, `ms_search` |
| **Pinterest** | `pi_get_board`, `pi_get_board_pins`, `pi_get_pin`, `pi_search_pins`, `pi_get_user_account`, `pi_get_board_analytics`, `pi_get_pin_analytics` |
| **TikTok** | `tt_create_post`, `tt_list_videos`, `tt_profile` |
| **Skool** | `sk_publish`, `sk_list_posts`, `sk_get_post`, `sk_create_comment`, `sk_get_info` |
| **Slack** | `sl_send_message`, `sl_list_channels`, `sl_channel_history`, `sl_list_users` |
| **Threads** | `th_get_profile`, `th_get_threads`, `th_get_thread_detail`, `th_get_replies`, `th_create_thread`, `th_reply_to_thread`, `th_delete_thread`, `th_get_insights`, `th_poll_publish_status` |
| **YouTube** | `yt_upload_video`, `yt_reply_comment` |
