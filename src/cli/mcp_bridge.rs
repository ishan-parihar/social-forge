// ─── MCP Bridge ───────────────────────────────────────────────
// Generic bridge that routes CLI calls through MCP tool handlers.
// This means any new MCP tool automatically becomes CLI-accessible.
// Usage: social-forge mcp-call <tool-name> --json '<args>'

use crate::api::AppState;

/// Call an MCP tool by name with a JSON argument string.
/// Returns the tool's JSON output.
pub async fn call_tool(
    state: &AppState,
    tool_name: &str,
    args_json: &str,
) -> Result<serde_json::Value, String> {
    let args: serde_json::Value = if args_json.is_empty() || args_json == "{}" {
        serde_json::json!({})
    } else {
        serde_json::from_str(args_json)
            .map_err(|e| format!("Invalid JSON arguments: {e}"))?
    };

    // Route to the appropriate tool handler
    // This is the single dispatch point — add new tools here
    match tool_name {
        // ── Auth ──────────────────────────────────────────────
        "auth_register" | "auth_login" => {
            Err("Auth tools are only available via MCP protocol (social-forge mcp). Use the web UI for registration.".to_string())
        }

        // ── Posts ─────────────────────────────────────────────
        "posts_create" => {
            let input: crate::mcp::tools_posts::CreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::create_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_list" => {
            let input: crate::mcp::tools_posts::ListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::list_posts(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_get" => {
            let input: crate::mcp::tools_posts::GetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::get_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_update" => {
            let input: crate::mcp::tools_posts::UpdatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::update_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_delete" => {
            let input: crate::mcp::tools_posts::DeletePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::delete_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_schedule" => {
            let input: crate::mcp::tools_posts::SchedulePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::schedule_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_publish" => {
            let input: crate::mcp::tools_posts::GetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::publish_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_stage" => {
            let input: crate::mcp::tools_posts::StagePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::stage_post(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_create_carousel" => {
            let input: crate::mcp::tools_posts::CreateCarouselInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::create_carousel(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_find_slot" => {
            let input: crate::mcp::tools_posts::FindSlotInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_posts::find_slot(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Media ─────────────────────────────────────────────
        "posts_media_upload" => {
            let input: crate::mcp::tools_media::MediaUploadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_media::upload_media(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_media_upload_from_path" => {
            let input: crate::mcp::tools_media::MediaUploadFromPathInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_media::upload_from_path_mcp(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_media_upload_batch" => {
            let input: crate::mcp::tools_media::MediaUploadBatchInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_media::upload_batch(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_media_upload_from_url" => {
            let input: crate::mcp::tools_media::MediaUploadFromUrlInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_media::upload_from_url(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "posts_media_list" => {
            let input: crate::mcp::tools_media::MediaListInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_media::list_media(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Comments ──────────────────────────────────────────
        "get_comments" => {
            let input: crate::mcp::tools_comments::GetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_comments::get_comments(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reply_to_comment" => {
            let input: crate::mcp::tools_comments::ReplyToCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_comments::reply_to_comment(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "delete_comment" => {
            let input: crate::mcp::tools_comments::DeleteCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_comments::delete_comment(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── DMs ──────────────────────────────────────────────
        "send_dm" => {
            let input: crate::mcp::tools_dm::SendDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_dm::send_dm(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "list_dm_conversations" => {
            let input: crate::mcp::tools_dm::ListDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_dm::list_dm_conversations(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "get_dm_messages" => {
            let input: crate::mcp::tools_dm::GetDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_dm::get_dm_messages(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Automation ────────────────────────────────────────
        "create_automation_rule" => {
            let input: crate::mcp::tools_automation::CreateRuleInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_automation::create_rule(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "list_automation_rules" => {
            let input: crate::mcp::tools_automation::ListRulesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_automation::list_rules(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "update_automation_rule" => {
            let input: crate::mcp::tools_automation::UpdateRuleInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_automation::update_rule(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "delete_automation_rule" => {
            let input: crate::mcp::tools_automation::DeleteRuleInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_automation::delete_rule(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "get_automation_logs" => {
            let input: crate::mcp::tools_automation::GetLogsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_automation::get_logs(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Integrations ──────────────────────────────────────
        "integrations_list" => {
            let input: crate::mcp::tools_integrations::ListIntegrationsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_integrations::list_integrations(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "integrations_list_targets" => {
            let input: crate::mcp::tools_integrations::ListTargetsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_integrations::list_targets(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Feed ──────────────────────────────────────────────
        "feed_list" => {
            let input: crate::mcp::tools_feed::FeedListInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_feed::handle_feed_list(state, &input).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Setup ─────────────────────────────────────────────
        "setup_status" => {
            let out = crate::mcp::tools_setup::setup_status(state, &crate::mcp::tools_setup::SetupStatusInput {}).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "setup_config_list" => {
            let out = crate::mcp::tools_setup::config_list(state)?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Tags ──────────────────────────────────────────────
        "tag_create" => {
            let input: crate::mcp::tools_tags::TagCreateInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tags::handle_tag_create(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tag_list" => {
            let input: crate::mcp::tools_tags::TagListInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tags::handle_tag_list(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── Unknown tool ──────────────────────────────────────
        _ => {
            // Tool is listed in list_tools() but not registered in call_tool().
            // Use the MCP server directly for full tool access.
            if list_tools().iter().any(|(name, _)| *name == tool_name) {
                Err(format!(
                    "Tool '{tool_name}' is available via MCP but not yet registered in the CLI bridge.                      Use 'social-forge mcp' to start the MCP server, or request this tool be added to the bridge."
                ))
            } else {
                Err(format!(
                    "Unknown tool: '{tool_name}'. Run 'social-forge mcp-tools' to see available tools."
                ))
            }
        }
    }
}

/// List all available MCP tools that can be called from CLI.
pub fn list_tools() -> Vec<(&'static str, &'static str)> {
    vec![
        // Auth
        ("auth_register", "Register a new account"),
        ("auth_login", "Login with email and password"),
        // Posts
        ("posts_create", "Create a new post"),
        ("posts_list", "List posts with optional state filter"),
        ("posts_get", "Get a single post by ID"),
        ("posts_update", "Update a post's content"),
        ("posts_delete", "Delete a post by ID"),
        ("posts_schedule", "Schedule a post for publishing"),
        ("posts_publish", "Publish a post immediately"),
        ("posts_stage", "Stage a post across multiple platforms with auto-splitting"),
        ("posts_create_carousel", "Create a carousel post with multiple images"),
        ("posts_find_slot", "Find the next available free time slot"),
        // Media
        ("posts_media_upload", "Upload media (image/video) for post attachments"),
        ("posts_media_upload_from_path", "Upload media from a local file path"),
        ("posts_media_upload_batch", "Batch upload multiple media files from local paths"),
        ("posts_media_upload_from_url", "Download and store media from an external URL"),
        ("posts_media_list", "List uploaded media files"),
        // Comments
        ("get_comments", "Get comments for a post on any platform"),
        ("reply_to_comment", "Reply to a comment on any platform"),
        ("delete_comment", "Delete a comment on any platform"),
        // DMs
        ("send_dm", "Send a direct message on any platform"),
        ("list_dm_conversations", "List DM conversations on any platform"),
        ("get_dm_messages", "Get messages in a DM conversation"),
        // Automation
        ("create_automation_rule", "Create an automation rule"),
        ("list_automation_rules", "List automation rules"),
        ("update_automation_rule", "Update an automation rule"),
        ("delete_automation_rule", "Delete an automation rule"),
        ("get_automation_logs", "Get execution logs for an automation rule"),
        // Integrations
        ("integrations_list", "List connected integrations"),
        ("integrations_list_providers", "List all available social media providers"),
        ("integrations_list_targets", "List posting targets for an integration"),
        // Feed
        ("feed_list", "List imported external posts"),
        ("feed_import", "Trigger immediate import of recent posts"),
        // Setup
        ("setup_status", "Check overall setup status"),
        ("setup_import_cookies", "Import browser cookies for X/Reddit"),
        ("setup_guide", "Get setup guidance for connecting providers"),
        ("setup_config_set", "Set a configuration value"),
        ("setup_config_get", "Get a configuration value"),
        ("setup_config_list", "List all configuration entries"),
        // Calendar
        ("calendar_get", "Get posts for a date range"),
        // Tags
        ("tag_create", "Create a new tag"),
        ("tag_list", "List all tags"),
        ("tag_get", "Get a tag by ID"),
        ("tag_update", "Update a tag"),
        ("tag_delete", "Delete a tag"),
        // Webhooks
        ("wh_create", "Create a webhook"),
        ("wh_list", "List webhooks"),
        ("wh_get", "Get a webhook by ID"),
        ("wh_update", "Update a webhook"),
        ("wh_delete", "Delete a webhook"),
        ("wh_test", "Test a webhook"),
        // Notifications
        ("notif_list", "List notifications"),
        ("notif_mark_read", "Mark notification as read"),
        ("notif_mark_all_read", "Mark all notifications as read"),
        ("notif_create", "Create a notification"),
        // Analytics
        ("analytics_get", "Get analytics for a provider"),
        ("analytics_get_post", "Get analytics for a specific post"),
        // Reddit
        ("reddit_browse", "Browse a subreddit"),
        ("reddit_search", "Search Reddit"),
        ("reddit_post_detail", "Get a Reddit post's full content"),
        ("reddit_user_info", "Get Reddit user info"),
        ("reddit_send_dm", "Send a DM to a Reddit user"),
        ("reddit_inbox", "Read Reddit inbox messages"),
        ("reddit_get_comments", "Get comments for a Reddit post"),
        ("reddit_create_post", "Create a new Reddit post"),
        ("reddit_create_comment", "Create a comment on Reddit"),
        ("reddit_get_karma", "Get karma breakdown by subreddit"),
        ("reddit_vote", "Vote on a Reddit post or comment"),
        ("reddit_save", "Save a Reddit post or comment"),
        ("reddit_unsave", "Unsave a Reddit post or comment"),
        ("reddit_hide", "Hide a Reddit post from feed"),
        ("reddit_subscribe", "Subscribe or unsubscribe to a subreddit"),
        ("reddit_edit", "Edit a Reddit post or comment"),
        ("reddit_delete", "Delete a Reddit post or comment"),
        ("reddit_mod_remove", "Remove a post or comment as moderator"),
        ("reddit_mod_approve", "Approve a post or comment as moderator"),
        ("reddit_mod_distinguish", "Distinguish a post or comment as moderator"),
        ("reddit_mod_sticky", "Sticky a post or comment as moderator"),
        ("reddit_mod_lock", "Lock a post or comment"),
        ("reddit_mod_unlock", "Unlock a post or comment"),
        // X/Twitter
        ("x_tweet", "Create a new tweet"),
        ("x_get_me", "Get authenticated user info"),
        ("x_home_timeline", "Get home timeline"),
        ("x_search_tweets", "Search tweets"),
        ("x_tweet_detail", "Get tweet details"),
        ("x_user_lookup", "Look up a user by ID"),
        ("x_user_lookup_by_username", "Look up a user by username"),
        ("x_user_tweets", "Get a user's tweets"),
        ("x_like_tweet", "Like a tweet"),
        ("x_unlike_tweet", "Unlike a tweet"),
        ("x_retweet", "Retweet a tweet"),
        ("x_unretweet", "Unretweet a tweet"),
        ("x_bookmark_tweet", "Bookmark a tweet"),
        ("x_unbookmark_tweet", "Remove a bookmark"),
        ("x_bookmarks", "List bookmarks"),
        ("x_delete_tweet", "Delete a tweet"),
        ("x_follow_user", "Follow a user"),
        ("x_unfollow_user", "Unfollow a user"),
        ("x_followers", "List followers"),
        ("x_following", "List following"),
        ("x_list_timeline", "Get list timeline"),
        // Facebook
        ("fb_get_feed", "Get Facebook page feed"),
        ("fb_get_post", "Get a Facebook post"),
        ("fb_get_comments", "Get comments on a post"),
        ("fb_create_post", "Create a Facebook page post"),
        ("fb_create_photo", "Create a photo post"),
        ("fb_create_video", "Create a video post"),
        ("fb_delete_post", "Delete a post"),
        ("fb_comment", "Comment on a post"),
        ("fb_react", "React to a post"),
        ("fb_page_insights", "Get page insights"),
        ("fb_conversations", "List conversations"),
        ("fb_conversation_messages", "Get conversation messages"),
        ("fb_send_message", "Send a message"),
        ("fb_search_pages", "Search for pages"),
        ("fb_albums", "List page albums"),
        // Instagram
        ("ig_get_media", "Get Instagram posts"),
        ("ig_get_media_detail", "Get media details"),
        ("ig_get_comments", "Get comments on a post"),
        ("ig_get_insights", "Get account insights"),
        ("ig_get_insights_audience", "Get audience insights"),
        ("ig_get_followers", "Get followers list"),
        ("ig_get_reels", "Get reels"),
        ("ig_get_stories", "Get stories"),
        ("ig_get_mentions", "Get mentions"),
        ("ig_get_tagged", "Get tagged posts"),
        ("ig_search_hashtag", "Search hashtags"),
        ("ig_get_hashtag_media", "Get media by hashtag"),
        ("ig_create_container", "Create a container for posting"),
        ("ig_publish_container", "Publish a container"),
        ("ig_poll_container", "Poll container status"),
        ("ig_reply_to_comment", "Reply to a comment"),
        ("ig_business_discovery", "Get business discovery"),
        ("ig_send_dm", "Send a DM"),
        ("ig_list_conversations", "List DM conversations"),
        ("ig_get_messages", "Get DM messages"),
        // LinkedIn
        ("li_get_profile", "Get LinkedIn profile"),
        ("li_get_posts", "Get LinkedIn posts"),
        ("li_create_post", "Create a LinkedIn post"),
        ("li_delete_post", "Delete a post"),
        ("li_get_post_detail", "Get post details"),
        ("li_get_comments", "Get post comments"),
        ("li_create_comment", "Create a comment"),
        ("li_get_reactions", "Get post reactions"),
        ("li_get_shares", "Get post shares"),
        ("li_get_analytics", "Get analytics"),
        ("li_get_post_analytics", "Get post analytics"),
        ("li_reply_comment", "Reply to a comment"),
        ("li_send_dm", "Send a DM"),
        ("li_list_conversations", "List DM conversations"),
        ("li_get_messages", "Get DM messages"),
        // LinkedIn Page
        ("lip_list_pages", "List LinkedIn pages"),
        ("lip_get_page", "Get page details"),
        ("lip_get_page_posts", "Get page posts"),
        ("lip_create_post", "Create a page post"),
        ("lip_create_comment", "Create a comment"),
        ("lip_delete_post", "Delete a post"),
        ("lip_get_analytics", "Get page analytics"),
        ("lip_get_post_analytics", "Get post analytics"),
        ("lip_get_followers", "Get page followers"),
        ("lip_get_reactions", "Get post reactions"),
        ("lip_get_shares", "Get post shares"),
        // YouTube
        ("yt_reply_comment", "Reply to a YouTube comment"),
        // Bluesky
        ("bs_profile", "Get a Bluesky profile"),
        ("bs_timeline", "Get Bluesky timeline"),
        ("bs_create_post", "Create a Bluesky post"),
        ("bs_search", "Search Bluesky"),
        ("bs_feed", "Get Bluesky feed"),
        ("bs_reply", "Reply to a Bluesky post"),
        // Mastodon
        ("ms_create_post", "Create a Mastodon post"),
        ("ms_get_timeline", "Get Mastodon timeline"),
        ("ms_get_post", "Get a Mastodon post"),
        ("ms_search", "Search Mastodon"),
        ("ms_reply", "Reply to a Mastodon post"),
        // TikTok
        ("tt_profile", "Get TikTok profile"),
        ("tt_create_post", "Create a TikTok video"),
        ("tt_list_videos", "List TikTok videos"),
        // Threads
        ("th_get_profile", "Get Threads profile"),
        ("th_get_threads", "List threads"),
        ("th_create_thread", "Create a thread"),
        ("th_get_thread_detail", "Get thread details"),
        ("th_get_replies", "Get thread replies"),
        ("th_reply_to_thread", "Reply to a thread"),
        ("th_delete_thread", "Delete a thread"),
        ("th_get_insights", "Get thread insights"),
        ("th_poll_publish_status", "Poll publish status"),
        // Discord
        ("di_get_channel", "Get channel info"),
        ("di_get_messages", "Get channel messages"),
        ("di_send_message", "Send a message"),
        ("di_add_reaction", "Add a reaction"),
        ("di_delete_message", "Delete a message"),
        ("di_get_guild", "Get guild info"),
        ("di_get_guild_channels", "Get guild channels"),
        ("di_get_server_info", "Get server info"),
        ("di_create_forum_post", "Create a forum post"),
        ("di_get_thread_members", "Get thread members"),
        // Slack
        ("sl_send_message", "Send a Slack message"),
        ("sl_list_channels", "List Slack channels"),
        ("sl_channel_history", "Get channel history"),
        ("sl_list_users", "List Slack users"),
        // Telegram Bot
        ("tb_send_message", "Send a Telegram message"),
        ("tb_send_photo", "Send a photo"),
        ("tb_send_document", "Send a document"),
        ("tb_get_chat", "Get chat info"),
        ("tb_get_chat_member_count", "Get member count"),
        ("tb_get_me", "Get bot info"),
        ("tb_get_updates", "Get updates"),
        ("tb_pin_message", "Pin a message"),
        ("tb_unpin_message", "Unpin a message"),
        ("tb_forward_message", "Forward a message"),
        // Telegram User
        ("tu_send_message", "Send a message"),
        ("tu_list_dialogs", "List dialogs"),
        ("tu_list_contacts", "List contacts"),
        ("tu_search", "Search messages"),
        ("tu_auth_status", "Check auth status"),
        ("tu_request_code", "Request auth code"),
        ("tu_sign_in", "Sign in"),
        // WhatsApp
        ("wa_send_text", "Send a text message"),
        ("wa_chats", "List chats"),
        ("wa_contacts", "List contacts"),
        ("wa_create_group", "Create a group"),
        ("wa_edit_message", "Edit a message"),
        ("wa_revoke_message", "Revoke a message"),
        ("wa_auth_status", "Check auth status"),
        ("wa_list_groups", "List groups"),
        ("wa_group_invite_link", "Get group invite link"),
        // GitHub
        ("gh_get_authenticated_user", "Get authenticated user"),
        ("gh_get_user", "Get a user"),
        ("gh_list_repos", "List repositories"),
        ("gh_get_repo", "Get repository details"),
        ("gh_list_issues", "List issues"),
        ("gh_get_issue", "Get issue details"),
        ("gh_create_issue", "Create an issue"),
        ("gh_list_pull_requests", "List pull requests"),
        ("gh_get_pull_request", "Get pull request details"),
        ("gh_list_commits", "List commits"),
        ("gh_list_branches", "List branches"),
        ("gh_list_releases", "List releases"),
        ("gh_search_repos", "Search repositories"),
        ("gh_search_code", "Search code"),
        ("gh_list_contributors", "List contributors"),
        ("gh_get_repo_content", "Get repo file contents"),
        ("gh_close_issue", "Close an issue"),
        ("gh_list_my_repos", "List my repositories"),
        // Google/YouTube
        ("goog_search_videos", "Search YouTube videos"),
        ("goog_get_video", "Get video details"),
        ("goog_get_playlists", "List playlists"),
        ("goog_get_playlist_items", "List playlist items"),
        ("goog_get_channel_stats", "Get channel stats"),
        ("goog_get_analytics", "Get YouTube analytics"),
        ("goog_get_comments", "Get video comments"),
        ("goog_get_subscriptions", "Get subscriptions"),
        ("goog_find_creators", "Find creators by topic"),
        ("goog_list_calendars", "List Google Calendar calendars"),
        ("goog_list_events", "List calendar events"),
        ("goog_get_event", "Get a calendar event"),
        ("goog_create_event", "Create a calendar event"),
        ("goog_update_event", "Update a calendar event"),
        ("goog_delete_event", "Delete a calendar event"),
        ("goog_list_files", "List Google Drive files"),
        ("goog_get_file", "Get a file"),
        ("goog_search_files", "Search Drive files"),
        ("goog_list_folders", "List folders"),
        ("goog_get_file_metadata", "Get file metadata"),
        ("goog_export_file", "Export a file"),
        ("goog_list_messages", "List Gmail messages"),
        ("goog_get_message", "Get a Gmail message"),
        ("goog_search_messages", "Search Gmail messages"),
        ("goog_send_message", "Send an email"),
        ("goog_list_labels", "List Gmail labels"),
        ("goog_get_thread", "Get a Gmail thread"),
        ("goog_get_profile", "Get Gmail profile"),
        // WordPress
        ("wp_create_post", "Create a WordPress post"),
        ("wp_get_post", "Get a WordPress post"),
        ("wp_list_posts", "List WordPress posts"),
        ("wp_list_categories", "List categories"),
        // Pinterest
        ("pi_get_board", "Get a board"),
        ("pi_get_board_pins", "Get board pins"),
        ("pi_get_pin", "Get a pin"),
        ("pi_search_pins", "Search pins"),
        ("pi_get_user_account", "Get user account"),
        ("pi_get_board_analytics", "Get board analytics"),
        ("pi_get_pin_analytics", "Get pin analytics"),
        // Medium
        ("md_create_post", "Create a Medium post"),
        ("md_get_post", "Get a Medium post"),
        ("md_list_posts", "List Medium posts"),
        // Hashnode
        ("hn_create_post", "Create a Hashnode post"),
        ("hn_get_post", "Get a Hashnode post"),
        ("hn_list_posts", "List Hashnode posts"),
        // Dev.to
        ("dv_create_post", "Create a Dev.to article"),
        ("dv_get_post", "Get a Dev.to article"),
        ("dv_list_posts", "List Dev.to articles"),
        // Skool
        ("sk_publish", "Publish a Skool post"),
        ("sk_get_info", "Get community info"),
        ("sk_list_posts", "List Skool posts"),
        ("sk_get_post", "Get a Skool post"),
        ("sk_create_comment", "Create a comment"),
        // Instagram Standalone
        ("ias_get_media", "Get media"),
        ("ias_get_media_detail", "Get media details"),
        ("ias_get_comments", "Get comments"),
        ("ias_reply_to_comment", "Reply to a comment"),
        ("ias_create_container", "Create a container"),
        ("ias_publish_container", "Publish a container"),
        ("ias_poll_container", "Poll container status"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensures list_tools() returns a reasonable number of tools.
    /// If you add a new MCP tool, increment this count.
    #[test]
    fn list_tools_count() {
        let tools = list_tools();
        // Update this number when adding new tools to the bridge.
        // This catches tools added to list_tools but not the match (or vice versa)
        // as long as you remember to bump the count.
        assert!(tools.len() >= 300, "Expected 300+ tools in bridge list, got {}. If you removed tools, update this count.", tools.len());
    }

    /// Ensures all listed tools are well-formed.
    #[test]
    fn tool_names_are_well_formed() {
        for (name, desc) in list_tools() {
            assert!(!name.is_empty(), "Tool name cannot be empty");
            assert!(!desc.is_empty(), "Tool '{name}' has no description");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "Tool name '{name}' is not lowercase_snake_case"
            );
        }
    }
}
