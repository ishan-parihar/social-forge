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

        // ── tools_facebook (manual mappings) ──────────────────────────────────────
        "fb_conversation_messages" => {
            let input: crate::mcp::tools_facebook::FbConversationMsgsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_conversation_msgs(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_authenticated_user" => {
            let out = crate::mcp::tools_github::handle_gh_get_authenticated_user(state, &()).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_profile" => {
            let out = crate::mcp::tools_google::handle_goog_get_profile(state, &()).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_labels" => {
            let out = crate::mcp::tools_google::handle_goog_list_labels(state, &()).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "integrations_list_providers" => {
            let input: crate::mcp::tools_integrations::ListProvidersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_integrations::list_providers(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "setup_config_get" => {
            let input: crate::mcp::tools_setup::ConfigGetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_setup::config_get(state, &input)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "setup_config_set" => {
            let input: crate::mcp::tools_setup::ConfigSetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_setup::config_set(state, &input)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "setup_import_cookies" => {
            let input: crate::mcp::tools_setup::ImportCookiesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_setup::import_cookies(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_analytics ──────────────────────────────────────
        "analytics_get" => {
            let input: crate::mcp::tools_analytics::AnalyticsGetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_analytics::handle_analytics_get(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "analytics_get_post" => {
            let input: crate::mcp::tools_analytics::AnalyticsPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_analytics::handle_analytics_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_bluesky ──────────────────────────────────────
        "bs_create_post" => {
            let input: crate::mcp::tools_bluesky::BsCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "bs_feed" => {
            let input: crate::mcp::tools_bluesky::BsFeedInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_feed(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "bs_profile" => {
            let input: crate::mcp::tools_bluesky::BsProfileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_profile(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "bs_reply" => {
            let input: crate::mcp::tools_bluesky::BsReplyInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_reply(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "bs_search" => {
            let input: crate::mcp::tools_bluesky::BsSearchInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_search(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "bs_timeline" => {
            let input: crate::mcp::tools_bluesky::BsTimelineInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_bluesky::handle_bs_timeline(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_devto ──────────────────────────────────────
        "dv_create_post" => {
            let input: crate::mcp::tools_devto::DvCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_devto::handle_dv_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "dv_get_post" => {
            let input: crate::mcp::tools_devto::DvGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_devto::handle_dv_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "dv_list_posts" => {
            let input: crate::mcp::tools_devto::DvListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_devto::handle_dv_list_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_discord ──────────────────────────────────────
        "di_add_reaction" => {
            let input: crate::mcp::tools_discord::DiAddReactionInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_add_reaction(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_create_forum_post" => {
            let input: crate::mcp::tools_discord::DiCreateForumPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_create_forum_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_delete_message" => {
            let input: crate::mcp::tools_discord::DiDeleteMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_delete_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_channel" => {
            let input: crate::mcp::tools_discord::DiGetChannelInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_channel(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_guild" => {
            let input: crate::mcp::tools_discord::DiGetGuildInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_guild(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_guild_channels" => {
            let input: crate::mcp::tools_discord::DiGetGuildChannelsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_guild_channels(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_messages" => {
            let input: crate::mcp::tools_discord::DiGetMessagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_messages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_server_info" => {
            let input: crate::mcp::tools_discord::DiGetServerInfoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_server_info(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_get_thread_members" => {
            let input: crate::mcp::tools_discord::DiGetThreadMembersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_get_thread_members(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "di_send_message" => {
            let input: crate::mcp::tools_discord::DiSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_discord::handle_di_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_facebook ──────────────────────────────────────
        "fb_albums" => {
            let input: crate::mcp::tools_facebook::FbAlbumsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_albums(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_comment" => {
            let input: crate::mcp::tools_facebook::FbCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_conversations" => {
            let input: crate::mcp::tools_facebook::FbConversationsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_conversations(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_create_photo" => {
            let input: crate::mcp::tools_facebook::FbCreatePhotoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_create_photo(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_create_post" => {
            let input: crate::mcp::tools_facebook::FbCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_create_video" => {
            let input: crate::mcp::tools_facebook::FbCreateVideoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_create_video(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_delete_post" => {
            let input: crate::mcp::tools_facebook::FbDeletePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_delete_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_get_comments" => {
            let input: crate::mcp::tools_facebook::FbGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_get_feed" => {
            let input: crate::mcp::tools_facebook::FbGetFeedInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_get_feed(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_get_post" => {
            let input: crate::mcp::tools_facebook::FbGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_page_insights" => {
            let input: crate::mcp::tools_facebook::FbPageInsightsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_page_insights(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_react" => {
            let input: crate::mcp::tools_facebook::FbReactInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_react(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_search_pages" => {
            let input: crate::mcp::tools_facebook::FbSearchPagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_search_pages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "fb_send_message" => {
            let input: crate::mcp::tools_facebook::FbSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_facebook::handle_fb_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_feed ──────────────────────────────────────
        "feed_import" => {
            let out = crate::mcp::tools_feed::handle_feed_import(state).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_github ──────────────────────────────────────
        "gh_close_issue" => {
            let input: crate::mcp::tools_github::GhCloseIssueInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_close_issue(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_create_issue" => {
            let input: crate::mcp::tools_github::GhCreateIssueInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_create_issue(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_issue" => {
            let input: crate::mcp::tools_github::GhGetIssueInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_get_issue(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_pull_request" => {
            let input: crate::mcp::tools_github::GhGetPullRequestInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_get_pull_request(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_repo" => {
            let input: crate::mcp::tools_github::GhGetRepoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_get_repo(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_repo_content" => {
            let input: crate::mcp::tools_github::GhGetRepoContentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_get_repo_content(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_get_user" => {
            let input: crate::mcp::tools_github::GhGetUserInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_get_user(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_branches" => {
            let input: crate::mcp::tools_github::GhListBranchesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_branches(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_commits" => {
            let input: crate::mcp::tools_github::GhListCommitsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_commits(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_contributors" => {
            let input: crate::mcp::tools_github::GhListContributorsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_contributors(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_issues" => {
            let input: crate::mcp::tools_github::GhListIssuesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_issues(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_my_repos" => {
            let input: crate::mcp::tools_github::GhListMyReposInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_my_repos(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_pull_requests" => {
            let input: crate::mcp::tools_github::GhListPullRequestsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_pull_requests(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_releases" => {
            let input: crate::mcp::tools_github::GhListReleasesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_releases(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_list_repos" => {
            let input: crate::mcp::tools_github::GhListReposInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_list_repos(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_search_code" => {
            let input: crate::mcp::tools_github::GhSearchCodeInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_search_code(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "gh_search_repos" => {
            let input: crate::mcp::tools_github::GhSearchReposInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_github::handle_gh_search_repos(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_google ──────────────────────────────────────
        "goog_create_event" => {
            let input: crate::mcp::tools_google::GcalCreateEventInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_create_event(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_delete_event" => {
            let input: crate::mcp::tools_google::GcalDeleteEventInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_delete_event(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_export_file" => {
            let input: crate::mcp::tools_google::DrExportFileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_export_file(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_find_creators" => {
            let input: crate::mcp::tools_google::YtFindCreatorsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_find_creators(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_analytics" => {
            let input: crate::mcp::tools_google::YtGetAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_channel_stats" => {
            let input: crate::mcp::tools_google::YtGetChannelStatsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_channel_stats(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_comments" => {
            let input: crate::mcp::tools_google::YtGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_event" => {
            let input: crate::mcp::tools_google::GcalGetEventInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_event(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_file" => {
            let input: crate::mcp::tools_google::DrGetFileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_file(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_file_metadata" => {
            let input: crate::mcp::tools_google::DrGetFileMetadataInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_file_metadata(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_message" => {
            let input: crate::mcp::tools_google::GmGetMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_playlist_items" => {
            let input: crate::mcp::tools_google::YtGetPlaylistItemsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_playlist_items(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_playlists" => {
            let input: crate::mcp::tools_google::YtListPlaylistsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_playlists(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_subscriptions" => {
            let input: crate::mcp::tools_google::YtGetSubscriptionsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_subscriptions(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_thread" => {
            let input: crate::mcp::tools_google::GmGetThreadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_thread(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_get_video" => {
            let input: crate::mcp::tools_google::YtGetVideoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_get_video(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_calendars" => {
            let input: crate::mcp::tools_google::GcalListCalendarsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_list_calendars(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_events" => {
            let input: crate::mcp::tools_google::GcalListEventsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_list_events(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_files" => {
            let input: crate::mcp::tools_google::DrListFilesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_list_files(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_folders" => {
            let input: crate::mcp::tools_google::DrListFoldersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_list_folders(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_list_messages" => {
            let input: crate::mcp::tools_google::GmListMessagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_list_messages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_search_files" => {
            let input: crate::mcp::tools_google::DrSearchFilesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_search_files(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_search_messages" => {
            let input: crate::mcp::tools_google::GmSearchMessagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_search_messages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_search_videos" => {
            let input: crate::mcp::tools_google::YtSearchVideosInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_search_videos(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_send_message" => {
            let input: crate::mcp::tools_google::GmSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "goog_update_event" => {
            let input: crate::mcp::tools_google::GcalUpdateEventInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_google::handle_goog_update_event(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_hashnode ──────────────────────────────────────
        "hn_create_post" => {
            let input: crate::mcp::tools_hashnode::HnCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_hashnode::handle_hn_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "hn_get_post" => {
            let input: crate::mcp::tools_hashnode::HnGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_hashnode::handle_hn_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "hn_list_posts" => {
            let input: crate::mcp::tools_hashnode::HnListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_hashnode::handle_hn_list_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_instagram ──────────────────────────────────────
        "ig_business_discovery" => {
            let input: crate::mcp::tools_instagram::IgBusinessDiscoveryInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_business_discovery(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_create_container" => {
            let input: crate::mcp::tools_instagram::IgCreateContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_create_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_comments" => {
            let input: crate::mcp::tools_instagram::IgGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_followers" => {
            let input: crate::mcp::tools_instagram::IgGetFollowersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_followers(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_hashtag_media" => {
            let input: crate::mcp::tools_instagram::IgGetHashtagMediaInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_hashtag_media(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_insights" => {
            let input: crate::mcp::tools_instagram::IgGetInsightsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_insights(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_insights_audience" => {
            let input: crate::mcp::tools_instagram::IgGetInsightsAudienceInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_insights_audience(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_media" => {
            let input: crate::mcp::tools_instagram::IgGetMediaInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_media(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_media_detail" => {
            let input: crate::mcp::tools_instagram::IgGetMediaDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_media_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_mentions" => {
            let input: crate::mcp::tools_instagram::IgGetMentionsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_mentions(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_messages" => {
            let input: crate::mcp::tools_instagram::IgGetMessagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_messages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_reels" => {
            let input: crate::mcp::tools_instagram::IgGetReelsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_reels(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_stories" => {
            let input: crate::mcp::tools_instagram::IgGetStoriesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_stories(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_get_tagged" => {
            let input: crate::mcp::tools_instagram::IgGetTaggedInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_get_tagged(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_list_conversations" => {
            let input: crate::mcp::tools_instagram::IgListConversationsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_list_conversations(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_poll_container" => {
            let input: crate::mcp::tools_instagram::IgPollContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_poll_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_publish_container" => {
            let input: crate::mcp::tools_instagram::IgPublishContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_publish_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_reply_to_comment" => {
            let input: crate::mcp::tools_instagram::IgReplyToCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_reply_to_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_search_hashtag" => {
            let input: crate::mcp::tools_instagram::IgSearchHashtagInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_search_hashtag(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ig_send_dm" => {
            let input: crate::mcp::tools_instagram::IgSendDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram::handle_ig_send_dm(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_instagram_standalone ──────────────────────────────────────
        "ias_create_container" => {
            let input: crate::mcp::tools_instagram_standalone::IasCreateContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_create_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_get_comments" => {
            let input: crate::mcp::tools_instagram_standalone::IasGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_get_media" => {
            let input: crate::mcp::tools_instagram_standalone::IasGetMediaInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_get_media(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_get_media_detail" => {
            let input: crate::mcp::tools_instagram_standalone::IasGetMediaDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_get_media_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_poll_container" => {
            let input: crate::mcp::tools_instagram_standalone::IasPollContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_poll_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_publish_container" => {
            let input: crate::mcp::tools_instagram_standalone::IasPublishContainerInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_publish_container(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ias_reply_to_comment" => {
            let input: crate::mcp::tools_instagram_standalone::IasReplyToCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_instagram_standalone::handle_ias_reply_to_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_linkedin ──────────────────────────────────────
        "li_create_comment" => {
            let input: crate::mcp::tools_linkedin::LiCreateCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_create_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_create_post" => {
            let input: crate::mcp::tools_linkedin::LiCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_delete_post" => {
            let input: crate::mcp::tools_linkedin::LiDeletePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_delete_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_analytics" => {
            let input: crate::mcp::tools_linkedin::LiGetAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_comments" => {
            let input: crate::mcp::tools_linkedin::LiGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_messages" => {
            let input: crate::mcp::tools_linkedin::LiGetMessagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_messages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_post_analytics" => {
            let input: crate::mcp::tools_linkedin::LiGetPostAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_post_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_post_detail" => {
            let input: crate::mcp::tools_linkedin::LiGetPostDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_post_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_posts" => {
            let input: crate::mcp::tools_linkedin::LiGetPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_profile" => {
            let input: crate::mcp::tools_linkedin::LiGetProfileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_profile(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_reactions" => {
            let input: crate::mcp::tools_linkedin::LiGetReactionsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_reactions(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_get_shares" => {
            let input: crate::mcp::tools_linkedin::LiGetSharesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_get_shares(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_list_conversations" => {
            let input: crate::mcp::tools_linkedin::LiListConversationsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_list_conversations(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_reply_comment" => {
            let input: crate::mcp::tools_linkedin::LiReplyCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_reply_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "li_send_dm" => {
            let input: crate::mcp::tools_linkedin::LiSendDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin::handle_li_send_dm(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_linkedin_page ──────────────────────────────────────
        "lip_create_comment" => {
            let input: crate::mcp::tools_linkedin_page::LipCreateCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_create_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_create_post" => {
            let input: crate::mcp::tools_linkedin_page::LipCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_delete_post" => {
            let input: crate::mcp::tools_linkedin_page::LipDeletePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_delete_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_analytics" => {
            let input: crate::mcp::tools_linkedin_page::LipGetAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_followers" => {
            let input: crate::mcp::tools_linkedin_page::LipGetFollowersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_followers(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_page" => {
            let input: crate::mcp::tools_linkedin_page::LipGetPageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_page(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_page_posts" => {
            let input: crate::mcp::tools_linkedin_page::LipGetPagePostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_page_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_post_analytics" => {
            let input: crate::mcp::tools_linkedin_page::LipGetPostAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_post_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_reactions" => {
            let input: crate::mcp::tools_linkedin_page::LipGetReactionsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_reactions(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_get_shares" => {
            let input: crate::mcp::tools_linkedin_page::LipGetSharesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_get_shares(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "lip_list_pages" => {
            let input: crate::mcp::tools_linkedin_page::LipListPagesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_linkedin_page::handle_lip_list_pages(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_mastodon ──────────────────────────────────────
        "ms_create_post" => {
            let input: crate::mcp::tools_mastodon::MsCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_mastodon::handle_ms_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ms_get_post" => {
            let input: crate::mcp::tools_mastodon::MsGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_mastodon::handle_ms_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ms_get_timeline" => {
            let input: crate::mcp::tools_mastodon::MsGetTimelineInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_mastodon::handle_ms_get_timeline(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ms_reply" => {
            let input: crate::mcp::tools_mastodon::MsReplyInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_mastodon::handle_ms_reply(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "ms_search" => {
            let input: crate::mcp::tools_mastodon::MsSearchInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_mastodon::handle_ms_search(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_medium ──────────────────────────────────────
        "md_create_post" => {
            let input: crate::mcp::tools_medium::MdCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_medium::handle_md_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "md_get_post" => {
            let input: crate::mcp::tools_medium::MdGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_medium::handle_md_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "md_list_posts" => {
            let input: crate::mcp::tools_medium::MdListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_medium::handle_md_list_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_notifications ──────────────────────────────────────
        "notif_create" => {
            let input: crate::mcp::tools_notifications::NotifCreateInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_notifications::handle_notif_create(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "notif_list" => {
            let input: crate::mcp::tools_notifications::NotifListInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_notifications::handle_notif_list(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "notif_mark_all_read" => {
            let input: crate::mcp::tools_notifications::NotifMarkAllReadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_notifications::handle_notif_mark_all_read(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "notif_mark_read" => {
            let input: crate::mcp::tools_notifications::NotifMarkReadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_notifications::handle_notif_mark_read(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_pinterest ──────────────────────────────────────
        "pi_get_board" => {
            let input: crate::mcp::tools_pinterest::PiGetBoardInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_board(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_get_board_analytics" => {
            let input: crate::mcp::tools_pinterest::PiGetBoardAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_board_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_get_board_pins" => {
            let input: crate::mcp::tools_pinterest::PiGetBoardPinsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_board_pins(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_get_pin" => {
            let input: crate::mcp::tools_pinterest::PiGetPinInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_pin(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_get_pin_analytics" => {
            let input: crate::mcp::tools_pinterest::PiGetPinAnalyticsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_pin_analytics(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_get_user_account" => {
            let input: crate::mcp::tools_pinterest::PiGetUserAccountInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_get_user_account(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "pi_search_pins" => {
            let input: crate::mcp::tools_pinterest::PiSearchPinsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_pinterest::handle_pi_search_pins(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_reddit ──────────────────────────────────────
        "reddit_browse" => {
            let input: crate::mcp::tools_reddit::RedditBrowseInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_browse(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_create_comment" => {
            let input: crate::mcp::tools_reddit::RedditCreateCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_create_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_create_post" => {
            let input: crate::mcp::tools_reddit::RedditCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_delete" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_delete(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_edit" => {
            let input: crate::mcp::tools_reddit::RedditEditInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_edit(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_get_comments" => {
            let input: crate::mcp::tools_reddit::RedditGetCommentsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_get_comments(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_get_karma" => {
            let out = crate::mcp::tools_reddit::handle_reddit_get_karma(state).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_hide" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_hide(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_inbox" => {
            let input: crate::mcp::tools_reddit::RedditInboxInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_inbox(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_approve" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_approve(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_distinguish" => {
            let input: crate::mcp::tools_reddit::RedditModDistinguishInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_distinguish(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_lock" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_lock(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_remove" => {
            let input: crate::mcp::tools_reddit::RedditModRemoveInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_remove(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_sticky" => {
            let input: crate::mcp::tools_reddit::RedditModStickyInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_sticky(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_mod_unlock" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_mod_unlock(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_post_detail" => {
            let input: crate::mcp::tools_reddit::RedditPostDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_post_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_save" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_save(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_search" => {
            let input: crate::mcp::tools_reddit::RedditSearchInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_search(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_send_dm" => {
            let input: crate::mcp::tools_reddit::RedditSendDmInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_send_dm(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_subscribe" => {
            let input: crate::mcp::tools_reddit::RedditSubscribeInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_subscribe(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_unsave" => {
            let input: crate::mcp::tools_reddit::RedditThingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_unsave(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_user_info" => {
            let input: crate::mcp::tools_reddit::RedditUserInfoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::reddit_user_info(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "reddit_vote" => {
            let input: crate::mcp::tools_reddit::RedditVoteInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_reddit::handle_reddit_vote(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_setup ──────────────────────────────────────
        "setup_guide" => {
            let input: crate::mcp::tools_setup::SetupGuideInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_setup::setup_guide(state, &input)
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_skool ──────────────────────────────────────
        "sk_create_comment" => {
            let input: crate::mcp::tools_skool::SkCreateCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_skool::handle_sk_create_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sk_get_info" => {
            let input: crate::mcp::tools_skool::SkGetInfoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_skool::handle_sk_get_info(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sk_get_post" => {
            let input: crate::mcp::tools_skool::SkGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_skool::handle_sk_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sk_list_posts" => {
            let input: crate::mcp::tools_skool::SkListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_skool::handle_sk_list_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sk_publish" => {
            let input: crate::mcp::tools_skool::SkPublishInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_skool::handle_sk_publish(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_slack ──────────────────────────────────────
        "sl_channel_history" => {
            let input: crate::mcp::tools_slack::SlChannelHistoryInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_slack::handle_sl_channel_history(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sl_list_channels" => {
            let input: crate::mcp::tools_slack::SlListChannelsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_slack::handle_sl_list_channels(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sl_list_users" => {
            let input: crate::mcp::tools_slack::SlListUsersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_slack::handle_sl_list_users(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "sl_send_message" => {
            let input: crate::mcp::tools_slack::SlSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_slack::handle_sl_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_tags ──────────────────────────────────────
        "tag_delete" => {
            let input: crate::mcp::tools_tags::TagDeleteInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tags::handle_tag_delete(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tag_get" => {
            let input: crate::mcp::tools_tags::TagGetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tags::handle_tag_get(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tag_update" => {
            let input: crate::mcp::tools_tags::TagUpdateInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tags::handle_tag_update(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_telegram_bot ──────────────────────────────────────
        "tb_forward_message" => {
            let input: crate::mcp::tools_telegram_bot::TbForwardInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_forward_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_get_chat" => {
            let input: crate::mcp::tools_telegram_bot::TbChatInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_get_chat(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_get_chat_member_count" => {
            let input: crate::mcp::tools_telegram_bot::TbChatInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_get_chat_member_count(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_get_me" => {
            let input: crate::mcp::tools_telegram_bot::TbTokenInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_get_me(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_get_updates" => {
            let input: crate::mcp::tools_telegram_bot::TbGetUpdatesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_get_updates(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_pin_message" => {
            let input: crate::mcp::tools_telegram_bot::TbPinInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_pin_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_send_document" => {
            let input: crate::mcp::tools_telegram_bot::TbSendDocumentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_send_document(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_send_message" => {
            let input: crate::mcp::tools_telegram_bot::TbSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_send_photo" => {
            let input: crate::mcp::tools_telegram_bot::TbSendPhotoInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_send_photo(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tb_unpin_message" => {
            let input: crate::mcp::tools_telegram_bot::TbPinInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_bot::handle_tb_unpin_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_telegram_user ──────────────────────────────────────
        "tu_auth_status" => {
            let out = crate::mcp::tools_telegram_user::handle_tu_auth_status(state).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_list_contacts" => {
            let input: crate::mcp::tools_telegram_user::TuListContactsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_list_contacts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_list_dialogs" => {
            let input: crate::mcp::tools_telegram_user::TuListDialogsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_list_dialogs(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_request_code" => {
            let input: crate::mcp::tools_telegram_user::TuRequestCodeInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_request_code(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_search" => {
            let input: crate::mcp::tools_telegram_user::TuSearchInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_search(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_send_message" => {
            let input: crate::mcp::tools_telegram_user::TuSendMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_send_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tu_sign_in" => {
            let input: crate::mcp::tools_telegram_user::TuSignInInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_telegram_user::handle_tu_sign_in(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_threads ──────────────────────────────────────
        "th_create_thread" => {
            let input: crate::mcp::tools_threads::ThreadsCreateThreadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_create_thread(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_delete_thread" => {
            let input: crate::mcp::tools_threads::ThreadsDeleteThreadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_delete_thread(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_get_insights" => {
            let input: crate::mcp::tools_threads::ThreadsGetInsightsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_get_insights(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_get_profile" => {
            let input: crate::mcp::tools_threads::ThreadsGetProfileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_get_profile(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_get_replies" => {
            let input: crate::mcp::tools_threads::ThreadsGetRepliesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_get_replies(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_get_thread_detail" => {
            let input: crate::mcp::tools_threads::ThreadsGetThreadDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_get_thread_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_get_threads" => {
            let input: crate::mcp::tools_threads::ThreadsGetThreadsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_get_threads(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_poll_publish_status" => {
            let input: crate::mcp::tools_threads::ThreadsPollStatusInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_poll_publish_status(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "th_reply_to_thread" => {
            let input: crate::mcp::tools_threads::ThreadsReplyToThreadInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_threads::handle_th_reply_to_thread(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_tiktok ──────────────────────────────────────
        "tt_create_post" => {
            let input: crate::mcp::tools_tiktok::TtCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tiktok::handle_tt_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tt_list_videos" => {
            let input: crate::mcp::tools_tiktok::TtListVideosInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tiktok::handle_tt_list_videos(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "tt_profile" => {
            let input: crate::mcp::tools_tiktok::TtProfileInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_tiktok::handle_tt_profile(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_webhooks ──────────────────────────────────────
        "wh_create" => {
            let input: crate::mcp::tools_webhooks::WhCreateInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_create(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wh_delete" => {
            let input: crate::mcp::tools_webhooks::WhDeleteInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_delete(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wh_get" => {
            let input: crate::mcp::tools_webhooks::WhGetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_get(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wh_list" => {
            let input: crate::mcp::tools_webhooks::WhListInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_list(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wh_test" => {
            let input: crate::mcp::tools_webhooks::WhTestInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_test(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wh_update" => {
            let input: crate::mcp::tools_webhooks::WhUpdateInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_webhooks::handle_wh_update(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_whatsapp ──────────────────────────────────────
        "wa_auth_status" => {
            let out = crate::mcp::tools_whatsapp::handle_wa_auth_status(state).await?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_chats" => {
            let input: crate::mcp::tools_whatsapp::WaChatsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_chats(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_contacts" => {
            let input: crate::mcp::tools_whatsapp::WaContactsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_contacts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_create_group" => {
            let input: crate::mcp::tools_whatsapp::WaCreateGroupInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_create_group(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_edit_message" => {
            let input: crate::mcp::tools_whatsapp::WaEditMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_edit_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_group_invite_link" => {
            let input: crate::mcp::tools_whatsapp::WaGroupInviteLinkInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_group_invite_link(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_list_groups" => {
            let out = crate::mcp::tools_whatsapp::handle_wa_list_groups(state).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_revoke_message" => {
            let input: crate::mcp::tools_whatsapp::WaRevokeMessageInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_revoke_message(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wa_send_text" => {
            let input: crate::mcp::tools_whatsapp::WaSendTextInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_whatsapp::handle_wa_send_text(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_wordpress ──────────────────────────────────────
        "wp_create_post" => {
            let input: crate::mcp::tools_wordpress::WpCreatePostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_wordpress::handle_wp_create_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wp_get_post" => {
            let input: crate::mcp::tools_wordpress::WpGetPostInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_wordpress::handle_wp_get_post(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wp_list_categories" => {
            let input: crate::mcp::tools_wordpress::WpListCategoriesInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_wordpress::handle_wp_list_categories(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "wp_list_posts" => {
            let input: crate::mcp::tools_wordpress::WpListPostsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_wordpress::handle_wp_list_posts(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_x ──────────────────────────────────────
        "x_bookmark_tweet" => {
            let input: crate::mcp::tools_x::XBookmarkTweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_bookmark_tweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_bookmarks" => {
            let input: crate::mcp::tools_x::XBookmarksInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_bookmarks(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_delete_tweet" => {
            let input: crate::mcp::tools_x::XDeleteTweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_delete_tweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_follow_user" => {
            let input: crate::mcp::tools_x::XFollowUserInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_follow_user(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_followers" => {
            let input: crate::mcp::tools_x::XFollowersInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_followers(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_following" => {
            let input: crate::mcp::tools_x::XFollowingInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_following(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_get_me" => {
            let out = crate::mcp::tools_x::x_get_me(state).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_home_timeline" => {
            let input: crate::mcp::tools_x::XHomeTimelineInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_home_timeline(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_like_tweet" => {
            let input: crate::mcp::tools_x::XLikeTweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_like_tweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_list_timeline" => {
            let input: crate::mcp::tools_x::XListTimelineInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_list_timeline(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_retweet" => {
            let input: crate::mcp::tools_x::XRetweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_retweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_search_tweets" => {
            let input: crate::mcp::tools_x::XSearchTweetsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_search_tweets(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_tweet_detail" => {
            let input: crate::mcp::tools_x::XTweetDetailInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_tweet_detail(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_unbookmark_tweet" => {
            let input: crate::mcp::tools_x::XUnbookmarkTweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_unbookmark_tweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_unfollow_user" => {
            let input: crate::mcp::tools_x::XUnfollowUserInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_unfollow_user(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_unlike_tweet" => {
            let input: crate::mcp::tools_x::XUnlikeTweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_unlike_tweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_unretweet" => {
            let input: crate::mcp::tools_x::XUnretweetInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_unretweet(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_user_lookup" => {
            let input: crate::mcp::tools_x::XUserLookupInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_user_lookup(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_user_lookup_by_username" => {
            let input: crate::mcp::tools_x::XUserLookupByUsernameInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_user_lookup_by_username(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }
        "x_user_tweets" => {
            let input: crate::mcp::tools_x::XUserTweetsInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_x::x_user_tweets(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

        // ── tools_youtube ──────────────────────────────────────
        "yt_reply_comment" => {
            let input: crate::mcp::tools_youtube::YtReplyCommentInput = serde_json::from_value(args)
                .map_err(|e| format!("Invalid args: {e}"))?;
            let out = crate::mcp::tools_youtube::handle_yt_reply_comment(state, &input).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(out.0).unwrap_or_default())
        }

_ => {
            // Tool is listed in list_tools() but not registered in call_tool().
            // Use the MCP server directly for full tool access.
            if list_tools().iter().any(|(name, _)| *name == tool_name) {
                Err(format!(
                    "Tool '{tool_name}' is available via MCP but not registered in the CLI bridge. Use 'social-forge mcp' for full access."
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
