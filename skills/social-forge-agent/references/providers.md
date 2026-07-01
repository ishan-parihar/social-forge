# Social Forge — Provider Reference

Complete CLI commands and MCP tools for every supported provider. Grouped by task.

---

## Post to a platform

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **X/Twitter** | `social-forge x post "<text>"` | Cookie auth for full access |
| **LinkedIn personal** | `social-forge linkedin post "<text>"` | OAuth |
| **LinkedIn page** | `social-forge linkedin-page post <PAGE_ID> "<text>"` | OAuth |
| **Reddit** | `social-forge reddit post --title "<t>" --text "<b>" --target <sub>` | Cookie auth |
| **Facebook** | Use `mcp-call fb_create_post '{"page_id":"...","message":"..."}'` | No native CLI post command |
| **Instagram** | Use `mcp-call ig_create_container` + `ig_poll_container` + `ig_publish_container` | 3-step flow (see below) |
| **Bluesky** | Use `mcp-call bs_create_post '{"text":"..."}'` | App password auth |
| **Mastodon** | Use `mcp-call ms_create_post '{"status":"..."}'` | Access token auth |
| **WordPress** | Use `mcp-call wp_create_post '{"title":"...","content":"..."}'` | App password auth |
| **Dev.to** | Use `mcp-call dv_create_post '{"title":"...","body_markdown":"..."}'` | API key auth |
| **Hashnode** | `social-forge hashnode post <PUB_ID> "title" "content"` | PAT auth |
| **Medium** | `social-forge medium-blog post "content"` | Token auth |
| **TikTok** | `social-forge tiktok post "text"` | OAuth |
| **Skool** | `social-forge skool post <GROUP_ID> "title" "content"` | API key auth |

**Cross-platform**: `social-forge post "<text>" --platforms x,linkedin,bluesky` — auto-splits content.

---

## Read a feed or profile

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **X/Twitter** | `social-forge x timeline --count 5` | Home timeline |
| **X search** | `social-forge x search "<query>"` | Search tweets |
| **X user** | `social-forge x user <username>` | Profile by username |
| **Reddit browse** | `social-forge reddit browse <subreddit> --sort hot` | Subreddit feed |
| **Reddit search** | `social-forge reddit search "<query>"` | Search Reddit |
| **Reddit user** | `social-forge reddit user <username>` | User profile |
| **Reddit inbox** | `social-forge reddit inbox` | Read inbox |
| **Instagram posts** | `social-forge instagram posts <ACCOUNT_ID>` | Recent media |
| **LinkedIn personal** | `social-forge linkedin posts` | Your recent posts |
| **LinkedIn profile** | `social-forge linkedin profile` | Your profile |
| **LinkedIn page** | `social-forge linkedin-page posts <PAGE_ID>` | Page posts |
| **Facebook page** | `social-forge facebook posts <PAGE_ID>` | Page feed |
| **Import + feed** | `social-forge import x --count 20` then `social-forge feed` | Unified local feed |

---

## Schedule a post

| Command | Description |
|---------|-------------|
| `social-forge posts create "<content>" --integrations <UUID>` | Create a queued post |
| `social-forge posts create "<content>" --integrations <UUID> --schedule <ISO>` | Create + schedule |
| `social-forge posts find-slot --integration <UUID>` | Find next available time slot |
| `social-forge posts list --limit 10` | List scheduled/queued posts |
| `social-forge posts get <POST_ID>` | Get post details |
| `social-forge posts publish <POST_ID>` | Publish immediately |
| `social-forge posts delete <POST_ID>` | Cancel/delete a post |

**Staging** (multi-platform review before publish):
| Command | Description |
|---------|-------------|
| `social-forge stage "<content>" --integrations <UUID1>,<UUID2>` | Stage across platforms |
| `social-forge stage "<content>" --platforms x,linkedin --preview` | Preview splits only |

---

## Manage media

| Command | Description |
|---------|-------------|
| `social-forge media upload <file>` | Upload a file |
| `social-forge media upload <url>` | Upload from URL |
| `social-forge media upload-batch <file1> <file2> ...` | Upload multiple files |
| `social-forge media list --limit 10` | List uploaded media |
| `social-forge media download <url> --output <path>` | Download a file |

---

## Create a carousel

| Command | Description |
|---------|-------------|
| `social-forge carousel create "<title>" --images <img1> <img2> ... --platforms x,linkedin` | Multi-image carousel |

---

## Comment and reply

| Command | Description |
|---------|-------------|
| `social-forge comment get <INTEGRATION_ID> <POST_ID>` | Get comments on a post |
| `social-forge comment reply <INTEGRATION_ID> <COMMENT_ID> "<text>"` | Reply to a comment |
| `social-forge comment delete <INTEGRATION_ID> <COMMENT_ID>` | Delete a comment |

**Platform-specific comments via CLI**:
| Platform | CLI Command |
|----------|-------------|
| **X** | `social-forge x post "<text>"` (replies are posts) |
| **Reddit** | `social-forge reddit comment <THING_ID> "<text>"` |
| **Facebook** | `social-forge facebook comment <POST_ID> "<text>"` |
| **Instagram** | `social-forge instagram comment <MEDIA_ID> "<text>"` |

---

## Send a DM

| Command | Description |
|---------|-------------|
| `social-forge dm send <INTEGRATION_ID> <RECIPIENT_ID> "<text>"` | Send a message |
| `social-forge dm list <INTEGRATION_ID>` | List conversations |
| `social-forge dm messages <INTEGRATION_ID> <CONVERSATION_ID>` | Get messages in a conversation |

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
social-forge mcp-call ig_create_container --args '{"ig_id":"<IG_ACCOUNT_ID>","image_url":"https://example.com/photo.jpg","caption":"Your caption here"}'
# Returns: {"id":"CONTAINER_ID","status_code":"FINISHED"}
```

**Step 2: Poll container** (if status_code is not FINISHED)
```bash
social-forge mcp-call ig_poll_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
# Returns: {"id":"CONTAINER_ID","status_code":"FINISHED"} when ready
```

**Step 3: Publish**
```bash
social-forge mcp-call ig_publish_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
# Returns: {"id":"MEDIA_ID"} on success
```

**Important**:
- `image_url` must be a publicly accessible URL (not a local file path)
- For local files, upload first: `social-forge media upload ./photo.jpg` then use the returned URL
- Poll until `status_code` is `FINISHED` before publishing
- Video containers take longer; poll 2-3 times with 5-second delays

**For Reels**:
```bash
social-forge mcp-call ig_create_container --args '{"ig_id":"<IG_ACCOUNT_ID>","video_url":"https://example.com/video.mp4","caption":"Reel caption","media_type":"REELS"}'
social-forge mcp-call ig_poll_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID"}'
social-forge mcp-call ig_publish_container --args '{"ig_id":"<IG_ACCOUNT_ID>","container_id":"CONTAINER_ID","media_type":"REELS"}'
```

---

## Check analytics

| Platform | CLI Command | Notes |
|----------|-------------|-------|
| **Instagram** | `social-forge instagram insights <ACCOUNT_ID> --metric <metrics>` | Comma-separated metrics |
| **LinkedIn page** | `social-forge linkedin-page analytics <PAGE_ID>` | Page analytics |
| **LinkedIn followers** | `social-forge linkedin-page followers <PAGE_ID>` | Follower count |
| **LinkedIn personal** | `social-forge linkedin analytics` | Profile analytics |
| **Facebook** | `social-forge facebook insights <PAGE_ID> --metric <metrics>` | Page insights |

**Instagram metrics** (comma-separated):
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

---

## Engage (like, retweet, vote, bookmark)

| Platform | CLI Command |
|----------|-------------|
| **X like** | `social-forge x like <TWEET_ID>` |
| **X retweet** | `social-forge x retweet <TWEET_ID>` |
| **X bookmark** | `social-forge x bookmark <TWEET_ID>` |
| **X delete** | `social-forge x delete <TWEET_ID>` |
| **Reddit vote** | `social-forge reddit vote <THING_ID> up` or `down` |
| **Reddit save** | `social-forge reddit save <THING_ID>` |
| **Reddit unsave** | `social-forge reddit unsave <THING_ID>` |
| **Reddit delete** | `social-forge reddit delete <THING_ID>` |

---

## Manage automation rules

| Command | Description |
|---------|-------------|
| `social-forge automation create <INTEGRATION_ID> "<name>" --trigger <type> --response "<text>"` | Create rule |
| `social-forge automation list` | List all rules |
| `social-forge automation update <RULE_ID> --name "<name>" --active true` | Update rule |
| `social-forge automation delete <RULE_ID>` | Delete rule |
| `social-forge automation logs <RULE_ID> --limit 50` | View execution logs |


---

## New Platform CLI Commands

| Platform | CLI Command | Subcommands |
|----------|-------------|-------------|
| **TikTok** | `social-forge tiktok <action>` | `profile`, `post`, `videos` |
| **Threads** | `social-forge threads <action>` | `profile`, `list`, `post`, `reply`, `delete`, `insights` |
| **Discord** | `social-forge discord <action>` | `channels`, `messages`, `send`, `server`, `forum` |
| **Slack** | `social-forge slack <action>` | `channels`, `history`, `send`, `users` |
| **Telegram Bot** | `social-forge telegram-bot <action>` | `send`, `photo`, `document`, `chat`, `updates` |
| **Telegram User** | `social-forge telegram-user <action>` | `send`, `dialogs`, `contacts`, `search` |
| **WhatsApp** | `social-forge whatsapp <action>` | `send`, `chats`, `contacts`, `groups`, `create-group`, `invite-link` |
| **Pinterest** | `social-forge pinterest <action>` | `profile`, `board`, `pins`, `pin`, `search`, `board-analytics`, `pin-analytics` |
| **GitHub** | `social-forge github <action>` | `me`, `my-repos`, `user`, `repos`, `issues`, `prs`, `create-issue`, `close-issue`, `commits`, `branches`, `search`, `releases` |
| **WordPress** | `social-forge wordpress <action>` | `post`, `list`, `get`, `categories` |
| **Hashnode** | `social-forge hashnode <action>` | `post`, `list`, `get` |
| **Medium** | `social-forge medium-blog <action>` | `post`, `list`, `get` |
| **Dev.to** | `social-forge devto <action>` | `post`, `list`, `get` |
| **Skool** | `social-forge skool <action>` | `post`, `info`, `posts`, `comment` |
| **Google (YouTube)** | `social-forge google <action>` | `youtube-search`, `video`, `playlists`, `channel-stats` |
| **Google Drive** | `social-forge gdrive <action>` | `files`, `file`, `search`, `folders`, `metadata`, `export` |
| **Google Calendar** | `social-forge gcal <action>` | `calendars`, `events`, `event`, `create`, `delete` |
| **Gmail** | `social-forge gmail-ops <action>` | `profile`, `messages`, `message`, `send`, `labels`, `thread`, `search` |
| **Webhooks** | `social-forge webhooks <action>` | `list`, `create`, `delete`, `get`, `update`, `test` |
| **Notifications** | `social-forge notifications <action>` | `list`, `read`, `read-all`, `create` |
| **Tags** | `social-forge tags <action>` | `list`, `create`, `delete`, `get`, `update` |
| **Analytics** | `social-forge analytics <action>` | `get`, `post` |

---

## Cross-cutting MCP tools

When CLI doesn't have a command for what you need, use `mcp-call`:

```bash
social-forge mcp-tools --pretty                    # List all 311 tools
social-forge mcp-call <tool_name> --args '{"key":"value"}' # Call any tool
```

| Category | Key MCP Tools |
|----------|---------------|
| **Auth** | `auth_register`, `auth_login`, `auth_me` |
| **Integrations** | `integrations_list`, `integrations_list_providers`, `integrations_connect`, `integrations_disconnect`, `integrations_list_targets` |
| **Posts** | `posts_create`, `posts_list`, `posts_get`, `posts_update`, `posts_delete`, `posts_schedule`, `posts_publish`, `posts_find_slot` |
| **Media** | `posts_media_upload`, `posts_media_upload_from_path`, `posts_media_upload_batch`, `posts_media_upload_from_url`, `posts_media_list` |
| **Feed** | `feed_import`, `feed_list` |
| **Calendar** | `calendar_get` |
| **Analytics** | `analytics_get`, `analytics_get_post` |
| **Tags** | `tag_create`, `tag_list`, `tag_get`, `tag_update`, `tag_delete` |
| **Notifications** | `notif_create`, `notif_list`, `notif_mark_read`, `notif_mark_all_read` |
| **Webhooks** | `wh_create`, `wh_list`, `wh_get`, `wh_update`, `wh_delete`, `wh_test` |

---

## Platform-specific MCP tools

### X / Twitter (20 tools)
`x_get_me`, `x_home_timeline`, `x_search_tweets`, `x_tweet_detail`, `x_user_lookup`, `x_user_lookup_by_username`, `x_user_tweets`, `x_like_tweet`, `x_unlike_tweet`, `x_retweet`, `x_unretweet`, `x_bookmark_tweet`, `x_unbookmark_tweet`, `x_bookmarks`, `x_delete_tweet`, `x_follow_user`, `x_unfollow_user`, `x_followers`, `x_following`, `x_list_timeline`

### Facebook (15 tools)
`fb_get_feed`, `fb_get_post`, `fb_get_comments`, `fb_create_post`, `fb_create_photo`, `fb_create_video`, `fb_delete_post`, `fb_comment`, `fb_react`, `fb_page_insights`, `fb_conversations`, `fb_conversation_messages`, `fb_send_message`, `fb_search_pages`, `fb_albums`

### Instagram (15+ tools)
`ig_get_media`, `ig_get_media_detail`, `ig_get_comments`, `ig_get_insights`, `ig_get_insights_audience`, `ig_get_followers`, `ig_get_reels`, `ig_get_stories`, `ig_get_mentions`, `ig_get_tagged`, `ig_search_hashtag`, `ig_get_hashtag_media`, `ig_create_container`, `ig_publish_container`, `ig_poll_container`, `ig_reply_to_comment`, `ig_business_discovery`

### LinkedIn Page (12 tools)
`lip_list_pages`, `lip_get_page`, `lip_get_page_posts`, `lip_create_post`, `lip_create_comment`, `lip_delete_post`, `lip_get_analytics`, `lip_get_post_analytics`, `lip_get_followers`, `lip_get_reactions`, `lip_get_shares`

### LinkedIn Personal (12 tools)
`li_get_profile`, `li_get_posts`, `li_create_post`, `li_delete_post`, `li_get_post_detail`, `li_get_comments`, `li_create_comment`, `li_get_reactions`, `li_get_shares`, `li_get_analytics`, `li_get_post_analytics`

### Reddit (21 tools)
`reddit_browse`, `reddit_search`, `reddit_post_detail`, `reddit_create_post`, `reddit_create_comment`, `reddit_get_comments`, `reddit_vote`, `reddit_save`, `reddit_unsave`, `reddit_delete`, `reddit_edit`, `reddit_hide`, `reddit_subscribe`, `reddit_user_info`, `reddit_send_dm`, `reddit_inbox`, `reddit_get_karma`, `reddit_mod_remove`, `reddit_mod_approve`, `reddit_mod_distinguish`, `reddit_mod_sticky`, `reddit_mod_lock`, `reddit_mod_unlock`

### Bluesky (5 tools)
`bs_timeline`, `bs_search`, `bs_create_post`, `bs_profile`, `bs_feed`

### Discord (10 tools)
`di_send_message`, `di_get_messages`, `di_get_channel`, `di_get_guild`, `di_get_guild_channels`, `di_get_server_info`, `di_add_reaction`, `di_delete_message`, `di_create_forum_post`, `di_get_thread_members`

### Telegram Bot (9 tools)
`tb_send_message`, `tb_send_photo`, `tb_send_document`, `tb_get_chat`, `tb_get_chat_member_count`, `tb_get_me`, `tb_get_updates`, `tb_pin_message`, `tb_unpin_message`, `tb_forward_message`

### Telegram User (7 tools)
`tu_auth_status`, `tu_list_contacts`, `tu_list_dialogs`, `tu_search`, `tu_send_message`, `tu_request_code`, `tu_sign_in`

### WhatsApp (9 tools)
`wa_auth_status`, `wa_chats`, `wa_contacts`, `wa_send_text`, `wa_edit_message`, `wa_revoke_message`, `wa_create_group`, `wa_list_groups`, `wa_group_invite_link`

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
