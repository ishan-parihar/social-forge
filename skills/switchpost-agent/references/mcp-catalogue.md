# MCP catalogue — 389 tools by category

Tool names verified against `switchpost mcp-tools` on the shipped binary.
Call any of them via `switchpost mcp-call <tool> --args '<json>'` (not
toolset-filtered). Parameters: run tools_search with a query —
`switchpost mcp-call tools_search --args '{"query":"…"}'` — to print each
tool's description; its `set` field is the toolset name.

## Toolset → tool-prefix map

| Toolset | Tool prefixes |
|---|---|
| `core` | posts_, stage, media_, tag_, notif_, setup_, integrations_, signatures_, calendar_, analytics_, feed_, auth_, inbox_, comment/dm verbs, tools_ (meta) |
| `x` | x_ |
| `reddit` | reddit_ |
| `google` | goog_ (+ yt_ YouTube tools) |
| `github` | gh_ |
| `linkedin` | li_ (personal) + lip_ (page) |
| `facebook` | fb_ |
| `instagram` | ig_ + ias_ (instagram-standalone) |
| `telegram` | tb_ (bot) + tu_ (user) |
| `discord` | di_ |
| `mastodon` | ms_ |
| `bluesky` | bs_ |
| `youtube` | yt_ |
| `skool` | sk_ |
| `whatsapp` | wa_ (Web session) |
| `wc` | wc_ (WhatsApp Cloud API templates) |
| `webhooks` | wh_ |
| `slack` | sl_ |
| `th` | th_ (Threads) |
| `tt` | tt_ (TikTok) |
| `pi` | pi_ (Pinterest) |
| `wp` | wp_ (WordPress) |
| `md` | md_ (Medium) |
| `dv` | dv_ (Dev.to) |
| `hn` | hn_ (Hashnode) |
| `ai` | ai_ |

Live-session budgeting: boot default advertises `core` only (~40 tools).
`SWITCHPOST_TOOLSETS=x,reddit` pre-enables; `tools_enable {"sets":["x"]}`
adds at runtime; `tools_search` always searches all 389.

## Core (cross-platform lifecycle)

- **Auth**: `auth_status` (single-user; there is NO register/login over MCP)
- **Integrations**: `integrations_list`, `integrations_list_providers`,
  `integrations_connect`, `integrations_disconnect`, `integrations_refresh`,
  `integrations_list_targets`, `integrations_connect_complete`
- **Posts**: `posts_create`, `posts_list`, `posts_get`, `posts_update`,
  `posts_delete` (+ `delete_from_platform`), `posts_schedule`,
  `posts_reschedule` (+ `move_group`), `posts_unschedule`, `posts_publish`,
  `posts_find_slot`, `posts_best_times`, `posts_stage` (v2), `posts_thread`,
  `posts_group`, `posts_validate`, `posts_repeat`
- **Media**: `posts_media_upload`, `posts_media_upload_from_path`,
  `posts_media_upload_batch`, `posts_media_upload_from_url`,
  `posts_media_list`
- **Feed/calendar/analytics**: `feed_import`, `feed_list`, `calendar_get`,
  `analytics_get`, `analytics_get_post`
- **Tags**: `tag_create`, `tag_list`, `tag_get`, `tag_update`, `tag_delete`
- **Notifications**: `notif_create`, `notif_list`, `notif_mark_read`,
  `notif_mark_all_read`
- **Comments/DMs (cross-platform)**: `get_comments`, `reply_to_comment`,
  `comment_resolve`, `send_dm`, `get_dm_messages`, `list_dm_conversations`
- **Inbox**: `inbox_list`, `inbox_reply`
- **Sets/templates**: `set_create`, `set_list`, `set_delete`
- **Setup**: `setup_config_get`, `setup_config_list`
- **Signatures**: `signatures_create/list/update/delete`
- **Streak**: `streak_get`
- **RSS**: `rss_feed_create`, `rss_feed_list`, `rss_feed_items`,
  `rss_feed_poll`, `rss_feed_toggle`, `rss_feed_delete`
- **Campaigns**: `campaign_create`, `campaign_list`, `campaign_update`,
  `campaign_delete`
- **Automation**: `create_automation_rule`, `list_automation_rules`,
  `update_automation_rule`, `delete_automation_rule`, `get_automation_logs`
- **AI**: `ai_generate_post`, `ai_generate_bulk`, `ai_improve_writing`,
  `ai_summarize`, `ai_suggest_hashtags`, `ai_change_tone`
- **Webhooks**: `wh_create`, `wh_list`, `wh_get`, `wh_update`, `wh_delete`,
  `wh_test`, `webhook_deliveries`
- **Meta**: `tools_search`, `tools_enable`

## X (x_, 27 tools)

`x_get_me`, `x_home_timeline`, `x_list_timeline`, `x_search_tweets`,
`x_tweet_detail`, `x_user_lookup`, `x_user_lookup_by_username`,
`x_user_tweets`, `x_followers`, `x_following`, `x_create_tweet`,
`x_reply_tweet`, `x_delete_tweet`, `x_like_tweet`, `x_unlike_tweet`,
`x_retweet`, `x_unretweet`, `x_bookmark_tweet`, `x_unbookmark_tweet`,
`x_bookmarks`, `x_follow_user`, `x_unfollow_user`, `x_get_analytics`,
`x_get_post_analytics`, `x_list_dms`, `x_get_dm_conversation`, `x_send_dm`

## Reddit (reddit_, 37 tools)

Read: `reddit_browse`, `reddit_search`, `reddit_post_detail`,
`reddit_get_comments`, `reddit_user_info`, `reddit_inbox`,
`reddit_get_karma`, `reddit_get_saved`, `reddit_get_rules`,
`reddit_list_flairs`, `reddit_read_all_messages`, `reddit_list_friends`
Write: `reddit_create_post`, `reddit_create_comment`, `reddit_vote`,
`reddit_save`, `reddit_unsave`, `reddit_delete`, `reddit_edit`,
`reddit_hide`, `reddit_subscribe`, `reddit_send_dm`, `reddit_report`,
`reddit_block_user`, `reddit_add_friend`, `reddit_remove_friend`,
`reddit_mark_nsfw`, `reddit_unmark_nsfw`, `reddit_mark_spoiler`,
`reddit_unmark_spoiler`, `reddit_crosspost`
Mod: `reddit_mod_remove`, `reddit_mod_approve`, `reddit_mod_distinguish`,
`reddit_mod_sticky`, `reddit_mod_lock`, `reddit_mod_unlock`

## Google (goog_, 28 tools)

Calendar: `goog_list_calendars`, `goog_list_events`, `goog_get_event`,
`goog_create_event`, `goog_update_event`, `goog_delete_event`
Drive: `goog_list_files`, `goog_get_file`, `goog_search_files`,
`goog_list_folders`, `goog_export_file`, `goog_get_file_metadata`
Gmail: `goog_list_messages`, `goog_get_message`, `goog_search_messages`,
`goog_get_thread`, `goog_send_message`, `goog_list_labels`, `goog_get_profile`
YouTube: `goog_get_video`, `goog_search_videos`, `goog_get_analytics`,
`goog_get_channel_stats`, `goog_get_comments`, `goog_get_playlist_items`,
`goog_list_playlists`, `goog_get_subscriptions`, `goog_find_creators`

## GitHub (gh_, 18 tools)

`gh_get_authenticated_user`, `gh_get_user`, `gh_list_my_repos`,
`gh_list_repos`, `gh_get_repo`, `gh_get_repo_content`, `gh_list_issues`,
`gh_get_issue`, `gh_create_issue`, `gh_close_issue`,
`gh_list_pull_requests`, `gh_get_pull_request`, `gh_list_branches`,
`gh_list_commits`, `gh_list_releases`, `gh_list_contributors`,
`gh_search_code`, `gh_search_repos`

## Facebook (fb_, 15 tools)

`fb_get_feed`, `fb_get_post`, `fb_get_comments`, `fb_create_post`,
`fb_create_photo`, `fb_create_video`, `fb_delete_post`, `fb_comment`,
`fb_react`, `fb_page_insights`, `fb_conversations`,
`fb_conversation_messages`, `fb_send_message`, `fb_search_pages`, `fb_albums`

## Instagram (ig_ + ias_, 21+7 tools)

ig_: `ig_get_media`, `ig_get_media_detail`, `ig_get_comments`,
`ig_get_insights`, `ig_get_insights_audience`, `ig_get_followers`,
`ig_get_reels`, `ig_get_stories`, `ig_get_mentions`, `ig_get_tagged`,
`ig_search_hashtag`, `ig_get_hashtag_media`, `ig_create_container`,
`ig_poll_container`, `ig_publish_container`, `ig_reply_to_comment`,
`ig_business_discovery`, `ig_get_messages`, `ig_list_conversations`,
`ig_send_dm`, `ig_post`
ias_ (instagram-standalone): `ias_create_container`, `ias_poll_container`,
`ias_publish_container`, `ias_get_media`, `ias_get_media_detail`,
`ias_get_comments`, `ias_reply_to_comment`

## LinkedIn (li_ personal 15 + lip_ page 11 tools)

li_: `li_get_profile`, `li_get_posts`, `li_create_post`, `li_delete_post`,
`li_get_post_detail`, `li_get_comments`, `li_create_comment`,
`li_get_reactions`, `li_get_shares`, `li_get_analytics`,
`li_get_post_analytics`, `li_get_messages`, `li_list_conversations`,
`li_reply_comment`, `li_send_dm`
lip_: `lip_list_pages`, `lip_get_page`, `lip_get_page_posts`,
`lip_create_post`, `lip_create_comment`, `lip_delete_post`,
`lip_get_analytics`, `lip_get_post_analytics`, `lip_get_followers`,
`lip_get_reactions`, `lip_get_shares`

## Telegram (tb_ bot 11 + tu_ user 13 tools)

tb_: `tb_send_message`, `tb_send_photo`, `tb_send_document`, `tb_get_chat`,
`tb_get_chat_member_count`, `tb_get_me`, `tb_get_updates`, `tb_pin_message`,
`tb_unpin_message`, `tb_forward_message`, `tb_list_chats`
tu_: `tu_auth_status`, `tu_list_contacts`, `tu_list_dialogs`, `tu_search`,
`tu_send_message`, `tu_request_code`, `tu_sign_in`, `tu_history`,
`tu_tail`, `tu_edit`, `tu_delete`, `tu_queue_send`, `tu_queue_status`

## Discord (di_, 15 tools)

`di_send_message`, `di_get_messages`, `di_get_channel`, `di_get_guild`,
`di_get_guild_channels`, `di_get_server_info`, `di_add_reaction`,
`di_delete_message`, `di_create_forum_post`, `di_get_thread_members`,
`di_history`, `di_search`, `di_tail`, `di_webhook_info`, `di_webhook_post`

## WhatsApp (wa_ 9 + wc_ 4 tools)

wa_ (Web session): `wa_auth_status`, `wa_chats`, `wa_contacts`,
`wa_send_text`, `wa_edit_message`, `wa_revoke_message`, `wa_create_group`,
`wa_list_groups`, `wa_group_invite_link`
wc_ (Cloud API): `wc_list_templates`, `wc_create_template`,
`wc_delete_template`, `wc_send_template`

## Bluesky (bs_, 7 tools) / Mastodon (ms_, 6 tools)

bs_: `bs_timeline`, `bs_search`, `bs_create_post`, `bs_profile`, `bs_feed`,
`bs_reply`, `bs_get_analytics`
ms_: `ms_get_timeline`, `ms_search`, `ms_create_post`, `ms_get_post`,
`ms_reply`, `ms_get_analytics`

## YouTube (yt_, 2 tools)

`yt_upload_video`, `yt_reply_comment` (the other YouTube reads are `goog_*`)

## Threads (th_, 9 tools) / TikTok (tt_, 4 tools)

th_: `th_get_profile`, `th_get_threads`, `th_get_thread_detail`,
`th_get_replies`, `th_create_thread`, `th_reply_to_thread`,
`th_delete_thread`, `th_get_insights`, `th_poll_publish_status`
tt_: `tt_create_post`, `tt_list_videos`, `tt_profile`, `tt_get_analytics`

## Pinterest (pi_, 8 tools)

`pi_get_user_account`, `pi_get_board`, `pi_get_board_pins`, `pi_get_pin`,
`pi_search_pins`, `pi_get_board_analytics`, `pi_get_pin_analytics`,
`pi_create_pin`

## Blogs: WordPress (wp_, 4) / Medium (md_, 3) / Dev.to (dv_, 3) / Hashnode (hn_, 3)

wp_: `wp_create_post`, `wp_get_post`, `wp_list_posts`, `wp_list_categories`
md_: `md_create_post`, `md_get_post`, `md_list_posts`
dv_: `dv_create_post`, `dv_get_post`, `dv_list_posts`
hn_: `hn_create_post`, `hn_get_post`, `hn_list_posts`

## Skool (sk_, 5 tools) / Slack (sl_, 4 tools)

sk_: `sk_publish`, `sk_list_posts`, `sk_get_post`, `sk_create_comment`,
`sk_get_info`
sl_: `sl_send_message`, `sl_list_channels`, `sl_channel_history`,
`sl_list_users`

## Staging v2 contracts (posts_stage)

`posts_stage` input: `content`, `integration_ids` (UUID list), optional
`media`, `settings`, `first_comment`, `overrides` (per-provider
content/settings), `dry_run` (readiness only, writes nothing), `force`
(stage despite readiness errors), `scheduled_at` (ISO8601 or
`"next-slot"` = each account's next free slot), `group_id` (campaign).
Readiness errors REJECT that integration unless `force` is set.

## Remote/HTTP MCP

With the `serve` daemon running, the same tool surface is served at
`POST {APP_URL}/mcp` (MCP streamable-HTTP). Connect by URL with
`Authorization: Bearer sf_<api-key>` (create one in the WebUI developer
page) or the `sf_session` cookie. Toolsets behave as in stdio mode.

## MCP client config (stdio)

```json
{
  "mcpServers": {
    "switchpost": {
      "command": "switchpost",
      "args": ["mcp"]
    }
  }
}
```
