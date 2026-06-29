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
            Err(format!(
                "Unknown tool: '{tool_name}'. Run 'social-forge mcp-call --list' to see available tools."
            ))
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
        ("integrations_list_targets", "List posting targets for an integration"),
        // Feed
        ("feed_list", "List imported external posts"),
        // Setup
        ("setup_status", "Check overall setup status"),
        ("setup_config_list", "List all configuration entries"),
        // Tags
        ("tag_create", "Create a new tag"),
        ("tag_list", "List all tags"),
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
        assert_eq!(tools.len(), 34, "Expected 34 tools in bridge, got {}. If you added a tool, update this count.", tools.len());
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
