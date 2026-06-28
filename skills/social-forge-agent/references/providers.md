# Social Forge — Provider Reference

Complete CLI commands and MCP tools for every supported provider.

---

## X / Twitter

**Auth**: Cookie (full access) or OAuth v2
**CLI**: `social-forge x <action> [args]`

| CLI Command | Description |
|-------------|-------------|
| `x timeline --count N` | Home timeline |
| `x search "<query>"` | Search tweets |
| `x post "<text>"` | Post a tweet |
| `x user <username>` | User profile by username |
| `x like <tweet_id>` | Like a tweet |
| `x retweet <tweet_id>` | Retweet |
| `x bookmark <tweet_id>` | Bookmark a tweet |
| `x delete <tweet_id>` | Delete a tweet |

**MCP tools** (20): `x_get_me`, `x_home_timeline`, `x_search_tweets`, `x_tweet_detail`, `x_user_lookup`, `x_user_lookup_by_username`, `x_user_tweets`, `x_like_tweet`, `x_unlike_tweet`, `x_retweet`, `x_unretweet`, `x_bookmark_tweet`, `x_unbookmark_tweet`, `x_bookmarks`, `x_delete_tweet`, `x_follow_user`, `x_unfollow_user`, `x_followers`, `x_following`, `x_list_timeline`

---

## Facebook Pages

**Auth**: OAuth (page-scoped tokens)
**CLI**: `social-forge facebook <action> <PAGE_ID>`

| CLI Command | Description |
|-------------|-------------|
| `facebook posts <PAGE_ID>` | Page feed |
| `facebook insights <PAGE_ID> --metric <metrics>` | Page insights |
| `facebook comment <POST_ID> "<text>"` | Comment on a post |

**MCP tools** (15): `fb_get_feed`, `fb_get_post`, `fb_get_comments`, `fb_create_post`, `fb_create_photo`, `fb_create_video`, `fb_delete_post`, `fb_comment`, `fb_react`, `fb_page_insights`, `fb_conversations`, `fb_conversation_messages`, `fb_send_message`, `fb_search_pages`, `fb_albums`

---

## Instagram Business

**Auth**: OAuth (via Facebook Graph API)
**CLI**: `social-forge instagram <action> <ACCOUNT_ID>`

| CLI Command | Description |
|-------------|-------------|
| `instagram posts <ACCOUNT_ID>` | Recent media |
| `instagram insights <ACCOUNT_ID> --metric <metrics>` | Account insights |
| `instagram comment <MEDIA_ID> "<text>"` | Comment on media |

**Instagram Insights Metrics**:
- Day period: `reach`, `follower_count`, `online_followers`
- Lifetime: `total_interactions`, `comments`, `shares`, `saves`, `likes`, `accounts_engaged`, `profile_views`, `website_clicks`

**MCP tools** (15+): `ig_get_media`, `ig_get_media_detail`, `ig_get_comments`, `ig_get_insights`, `ig_get_insights_audience`, `ig_get_followers`, `ig_get_reels`, `ig_get_stories`, `ig_get_mentions`, `ig_get_tagged`, `ig_search_hashtag`, `ig_get_hashtag_media`, `ig_create_container`, `ig_publish_container`, `ig_poll_container`, `ig_reply_to_comment`, `ig_business_discovery`

---

## LinkedIn Company Pages

**Auth**: OAuth
**CLI**: `social-forge linkedin-page <action> [args]`

| CLI Command | Description |
|-------------|-------------|
| `linkedin-page list` | List managed pages |
| `linkedin-page post <PAGE_ID> "<text>"` | Post as page |
| `linkedin-page analytics <PAGE_ID>` | Page analytics |
| `linkedin-page followers <PAGE_ID>` | Follower count |

**MCP tools** (12): `lip_list_pages`, `lip_get_page`, `lip_get_page_posts`, `lip_create_post`, `lip_create_comment`, `lip_delete_post`, `lip_get_analytics`, `lip_get_post_analytics`, `lip_get_followers`, `lip_get_reactions`, `lip_get_shares`

---

## LinkedIn Personal

**Auth**: OAuth
**CLI**: `social-forge linkedin <action>`

| CLI Command | Description |
|-------------|-------------|
| `linkedin profile` | Your profile |
| `linkedin posts` | Recent posts |
| `linkedin post "<text>"` | Create post |
| `linkedin analytics` | Profile analytics |
| `linkedin reactions <POST_URN>` | Post reactions |
| `linkedin delete <POST_URN>` | Delete post |

**MCP tools** (12): `li_get_profile`, `li_get_posts`, `li_create_post`, `li_delete_post`, `li_get_post_detail`, `li_get_comments`, `li_create_comment`, `li_get_reactions`, `li_get_shares`, `li_get_analytics`, `li_get_post_analytics`

---

## Reddit

**Auth**: Cookie (full access) or OAuth
**CLI**: `social-forge reddit <action> [args]`

| CLI Command | Description |
|-------------|-------------|
| `reddit browse <subreddit> --sort hot` | Browse subreddit |
| `reddit search "<query>"` | Search Reddit |
| `reddit post --title "<title>" --text "<body>" --target <sub>` | Create post |
| `reddit comment <THING_ID> "<text>"` | Comment/reply |
| `reddit vote <THING_ID> up\|down` | Vote |
| `reddit save <THING_ID>` | Save |
| `reddit unsave <THING_ID>` | Unsave |
| `reddit delete <THING_ID>` | Delete |
| `reddit user <username>` | User profile |
| `reddit inbox` | Read inbox |
| `reddit mod remove\|approve\|lock\|unlock <THING_ID>` | Mod actions |

**MCP tools** (21): `reddit_browse`, `reddit_search`, `reddit_post_detail`, `reddit_create_post`, `reddit_create_comment`, `reddit_get_comments`, `reddit_vote`, `reddit_save`, `reddit_unsave`, `reddit_delete`, `reddit_edit`, `reddit_hide`, `reddit_subscribe`, `reddit_user_info`, `reddit_send_dm`, `reddit_inbox`, `reddit_get_karma`, `reddit_mod_remove`, `reddit_mod_approve`, `reddit_mod_distinguish`, `reddit_mod_sticky`, `reddit_mod_lock`, `reddit_mod_unlock`

---

## Bluesky

**Auth**: App password
**MCP tools** (5): `bs_timeline`, `bs_search`, `bs_create_post`, `bs_profile`, `bs_feed`

---

## Discord

**Auth**: Bot token
**MCP tools** (10): `di_send_message`, `di_get_messages`, `di_get_channel`, `di_get_guild`, `di_get_guild_channels`, `di_get_server_info`, `di_add_reaction`, `di_delete_message`, `di_create_forum_post`, `di_get_thread_members`

---

## Telegram Bot

**Auth**: Bot token
**MCP tools** (9): `tb_send_message`, `tb_send_photo`, `tb_send_document`, `tb_get_chat`, `tb_get_chat_member_count`, `tb_get_me`, `tb_get_updates`, `tb_pin_message`, `tb_unpin_message`, `tb_forward_message`

---

## Telegram User

**Auth**: MTProto (phone number + code)
**MCP tools** (7): `tu_auth_status`, `tu_list_contacts`, `tu_list_dialogs`, `tu_search`, `tu_send_message`, `tu_request_code`, `tu_sign_in`

---

## WhatsApp

**Auth**: QR code (wa-rs)
**MCP tools** (9): `wa_auth_status`, `wa_chats`, `wa_contacts`, `wa_send_text`, `wa_edit_message`, `wa_revoke_message`, `wa_create_group`, `wa_list_groups`, `wa_group_invite_link`

---

## GitHub

**Auth**: Personal access token
**MCP tools** (18): `gh_list_my_repos`, `gh_list_repos`, `gh_get_repo`, `gh_get_repo_content`, `gh_list_issues`, `gh_get_issue`, `gh_create_issue`, `gh_close_issue`, `gh_list_pull_requests`, `gh_get_pull_request`, `gh_list_branches`, `gh_list_commits`, `gh_list_releases`, `gh_list_contributors`, `gh_get_authenticated_user`, `gh_get_user`, `gh_search_code`, `gh_search_repos`

---

## Google (Calendar, Drive, Gmail, YouTube)

**Auth**: OAuth
**MCP tools** (28): `goog_list_calendars`, `goog_list_events`, `goog_get_event`, `goog_create_event`, `goog_update_event`, `goog_delete_event`, `goog_list_files`, `goog_get_file`, `goog_search_files`, `goog_list_folders`, `goog_export_file`, `goog_get_file_metadata`, `goog_list_messages`, `goog_get_message`, `goog_search_messages`, `goog_get_thread`, `goog_send_message`, `goog_list_labels`, `goog_get_profile`, `goog_get_video`, `goog_search_videos`, `goog_get_analytics`, `goog_get_channel_stats`, `goog_get_comments`, `goog_get_playlist_items`, `goog_list_playlists`, `goog_get_subscriptions`, `goog_find_creators`

---

## WordPress

**Auth**: App password
**MCP tools** (4): `wp_create_post`, `wp_get_post`, `wp_list_posts`, `wp_list_categories`

---

## Other Platforms

| Platform | Auth | MCP Tools |
|----------|------|-----------|
| **Dev.to** | API key | `dv_create_post`, `dv_get_post`, `dv_list_posts` |
| **Hashnode** | PAT | `hn_create_post`, `hn_get_post`, `hn_list_posts` |
| **Medium** | Token | `md_create_post`, `md_get_post`, `md_list_posts` |
| **Mastodon** | Access token | `ms_create_post`, `ms_get_post`, `ms_get_timeline`, `ms_search` |
| **Pinterest** | OAuth | `pi_get_board`, `pi_get_board_pins`, `pi_get_pin`, `pi_search_pins`, `pi_get_user_account`, `pi_get_board_analytics`, `pi_get_pin_analytics` |
| **TikTok** | OAuth | `tt_create_post`, `tt_list_videos`, `tt_profile` |
| **Skool** | API key | `sk_publish`, `sk_list_posts`, `sk_get_post`, `sk_create_comment`, `sk_get_info` |
| **Slack** | OAuth | `sl_send_message`, `sl_list_channels`, `sl_channel_history`, `sl_list_users` |

---

## Cross-Cutting MCP Tools

| Category | Tools |
|----------|-------|
| **Auth** | `auth_register`, `auth_login`, `auth_me` |
| **Integrations** | `integrations_list`, `integrations_list_providers`, `integrations_connect`, `integrations_connect_complete`, `integrations_disconnect`, `integrations_list_targets` |
| **Feed** | `feed_import`, `feed_list` |
| **Posts/Scheduler** | `posts_create`, `posts_list`, `posts_get`, `posts_update`, `posts_delete`, `posts_schedule`, `posts_publish`, `posts_find_slot` |
| **Calendar** | `calendar_get` |
| **Analytics** | `analytics_get`, `analytics_get_post` |
| **Tags** | `tag_create`, `tag_list`, `tag_get`, `tag_update`, `tag_delete` |
| **Notifications** | `notif_create`, `notif_list`, `notif_mark_read`, `notif_mark_all_read` |
| **Webhooks** | `wh_create`, `wh_list`, `wh_get`, `wh_update`, `wh_delete`, `wh_test` |
