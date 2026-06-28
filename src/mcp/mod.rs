// ─── MCP Server ───────────────────────────────────────────────
// Model Context Protocol server exposing all Social Forge operations as tools.
// Designed for AI agents to schedule, manage, and monitor posts.
//
// Uses rmcp crate with ServerHandler + #[tool_router] pattern.
// Same business logic as the REST API — shared via AppState.

use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    schemars::JsonSchema,
    tool, tool_router,
    Json,
};
use crate::mcp::schema_optimizer::lean_stdio;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::jwt;
use crate::db::queries;


pub mod schema_optimizer;
mod tools_setup;
mod tools_analytics;
mod tools_bluesky;
mod tools_calendar;
mod tools_devto;
pub mod tools_facebook;
pub mod tools_instagram;
pub mod tools_instagram_standalone;
mod tools_integrations;
pub mod tools_linkedin;
pub mod tools_linkedin_page;
pub mod tools_mastodon;
mod tools_medium;
mod tools_hashnode;
pub mod tools_pinterest;
mod tools_posts;
mod tools_media;
mod tools_comments;
mod tools_dm;
mod tools_automation;
pub mod tools_discord;
pub mod tools_reddit;
mod tools_skool;
pub mod tools_slack;
mod tools_tags;
pub mod tools_telegram_bot;
pub mod tools_telegram_user;
pub mod tools_threads;
pub mod tools_tiktok;
pub mod tools_whatsapp;
mod tools_notifications;
mod tools_webhooks;
mod tools_feed;
pub mod tools_wordpress;
pub mod tools_youtube;
pub mod tools_x;
pub mod tools_github;
pub mod tools_google;

// ══════════════════════════════════════════════════════════════
// AUTH TOOLS
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct LoginOutput {
    pub token: String,
    pub user_id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RegisterInput {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RegisterOutput {
    pub token: String,
    pub user_id: String,
    pub name: String,
}

// ══════════════════════════════════════════════════════════════
// SHARED TYPES
// ══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuccessOutput {
    pub success: bool,
    pub message: String,
}

/// MCP-compatible wrapper around serde_json::Value for tool output.
/// The MCP spec requires outputSchema to have root type \"object\",
/// but schemars generates `true` (any-value schema) for serde_json::Value.
/// This wrapper provides a valid `{\"type\": \"object\"}` schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpJsonValue(pub serde_json::Value);

impl McpJsonValue {
    pub fn value(self) -> serde_json::Value {
        self.0
    }
}

impl JsonSchema for McpJsonValue {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("OutputObject")
    }
    fn json_schema(_gen: &mut rmcp::schemars::SchemaGenerator) -> rmcp::schemars::Schema {
        serde_json::from_value(serde_json::json!({"type": "object"})).unwrap()
    }
}

// ══════════════════════════════════════════════════════════════
// TOOL ROUTER
// ══════════════════════════════════════════════════════════════



#[derive(Clone)]
pub struct SocialForgeMcpServer {
    pub state: AppState,
}

// Helper: get DB pool from state


#[tool_router(server_handler)]
impl SocialForgeMcpServer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // ── Auth Tools ──────────────────────────────────────────

    #[tool(description = "Register a new account. Returns JWT")]
    async fn auth_register(
        &self,
        params: Parameters<RegisterInput>,
    ) -> Result<Json<RegisterOutput>, String> {
        let input = params.0;
        // Rate limit by email
        self.state.rate_limiter.check(&input.email).await.map_err(|e| format!("Rate limited: {e}"))?;

        if input.email.is_empty() || !input.email.contains('@') {
            return Err("Invalid email".into());
        }
        if input.password.len() < 6 {
            return Err("Password must be at least 6 characters".into());
        }

        if queries::get_user_by_email(&self.state.db, &input.email)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err("Email already registered".into());
        }

        let hash = jwt::hash_password(&input.password).map_err(|e| e.to_string())?;
        let user = queries::create_user(&self.state.db, &input.email, &hash, &input.name)
            .await
            .map_err(|e| e.to_string())?;

        let token = jwt::create_token(user.id, &self.state.config.jwt_secret)
            .map_err(|e| e.to_string())?;

        Ok(Json(RegisterOutput {
            token,
            user_id: user.id.to_string(),
            name: user.name,
        }))
    }

    #[tool(description = "Login with email and password.")]
    async fn auth_login(
        &self,
        params: Parameters<LoginInput>,
    ) -> Result<Json<LoginOutput>, String> {
        let input = params.0;
        // Rate limit by email
        self.state.rate_limiter.check(&input.email).await.map_err(|e| format!("Rate limited: {e}"))?;

        let user = queries::get_user_by_email(&self.state.db, &input.email)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Invalid email or password".to_string())?;

        let valid = jwt::verify_password(&input.password, &user.password)
            .map_err(|e| e.to_string())?;
        if !valid {
            return Err("Invalid email or password".into());
        }

        let token = jwt::create_token(user.id, &self.state.config.jwt_secret)
            .map_err(|e| e.to_string())?;

        Ok(Json(LoginOutput {
            token,
            user_id: user.id.to_string(),
            name: user.name,
        }))
    }

    #[tool(description = "Get user info from a JWT token")]
    async fn auth_me(
        &self,
        params: Parameters<MeInput>,
    ) -> Result<Json<MeOutput>, String> {
        let claims = jwt::validate_token(&params.0.token, &self.state.config.jwt_secret)
            .map_err(|e| format!("Invalid token: {e}"))?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| "Invalid user ID in token".to_string())?;

        let user = queries::get_user_by_id(&self.state.db, user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "User not found".to_string())?;

        Ok(Json(MeOutput {
            user_id: user.id.to_string(),
            email: user.email,
            name: user.name,
        }))
    }

    // ── Calendar Tools ──────────────────────────────────────

    #[tool(description = "Get posts for a date range (for")]
    async fn calendar_get(
        &self,
        params: Parameters<tools_calendar::CalendarInput>,
    ) -> Result<Json<tools_calendar::CalendarOutput>, String> {
        tools_calendar::get_calendar(&self.state, &params.0).await
    }

    // ── Integration Tools ────────────────────────────────────

    #[tool(description = "List all available social media")]
    async fn integrations_list_providers(
        &self,
        params: Parameters<tools_integrations::ListProvidersInput>,
    ) -> Result<Json<tools_integrations::ListProvidersOutput>, String> {
        tools_integrations::list_providers(&self.state, &params.0).await
    }

    #[tool(description = "List all connected social media")]
    async fn integrations_list(
        &self,
        params: Parameters<tools_integrations::ListIntegrationsInput>,
    ) -> Result<Json<tools_integrations::ListIntegrationsOutput>, String> {
        tools_integrations::list_integrations(&self.state, &params.0).await
    }

    #[tool(description = "Get OAuth URL to connect a social")]
    async fn integrations_connect(
        &self,
        params: Parameters<tools_integrations::ConnectInput>,
    ) -> Result<Json<tools_integrations::ConnectOutput>, String> {
        tools_integrations::connect_integration(&self.state, &params.0).await
    }

    #[tool(description = "Complete OAuth connection after")]
    async fn integrations_connect_complete(
        &self,
        params: Parameters<tools_integrations::ConnectCompleteInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_integrations::complete_connect_integration(&self.state, &params.0).await
    }

    #[tool(description = "Disconnect/remove a social media")]
    async fn integrations_disconnect(
        &self,
        params: Parameters<tools_integrations::DisconnectInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_integrations::disconnect_integration(&self.state, &params.0).await
    }

    #[tool(description = "List discoverable posting targets")]
    async fn integrations_list_targets(
        &self,
        params: Parameters<tools_integrations::ListTargetsInput>,
    ) -> Result<Json<tools_integrations::ListTargetsOutput>, String> {
        tools_integrations::list_targets(&self.state, &params.0).await
    }

    // ── Setup/Onboarding Tools ─────────────────────────────────

    #[tool(description = "Check overall setup status: database, user, connected providers, and next actions for AI agent guided setup")]
    async fn setup_status(
        &self,
        _params: Parameters<tools_setup::SetupStatusInput>,
    ) -> Result<Json<tools_setup::SetupStatusOutput>, String> {
        tools_setup::setup_status(&self.state, &tools_setup::SetupStatusInput {}).await
    }

    #[tool(description = "Import browser cookies for X/Twitter and/or Reddit. Auto-detects Chrome, Brave, Firefox, Zen. Use provider='all' to import both.")]
    async fn setup_import_cookies(
        &self,
        params: Parameters<tools_setup::ImportCookiesInput>,
    ) -> Result<Json<tools_setup::ImportCookiesOutput>, String> {
        tools_setup::import_cookies(&self.state, &params.0).await
    }

    #[tool(description = "Get setup guidance for connecting social media providers. Returns what credentials are needed, where to get them, and how to configure them. Call without provider to see all providers.")]
    async fn setup_guide(
        &self,
        params: Parameters<tools_setup::SetupGuideInput>,
    ) -> Result<Json<tools_setup::SetupGuideOutput>, String> {
        tools_setup::setup_guide(&self.state, &params.0)
    }

    #[tool(description = "Set a configuration value in ~/.social-forge/.env. Creates file if needed. Restart social-forge after setting.")]
    async fn setup_config_set(
        &self,
        params: Parameters<tools_setup::ConfigSetInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_setup::config_set(&self.state, &params.0)
    }

    #[tool(description = "Get a configuration value (redacts secrets)")]
    async fn setup_config_get(
        &self,
        params: Parameters<tools_setup::ConfigGetInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_setup::config_get(&self.state, &params.0)
    }

    #[tool(description = "List all configuration entries in ~/.social-forge/.env")]
    async fn setup_config_list(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_setup::ConfigListOutput>, String> {
        tools_setup::config_list(&self.state)
    }

    // ── Post Tools ───────────────────────────────────────────

    #[tool(description = "Create a new post. Set scheduled_at")]
    async fn posts_create(
        &self,
        params: Parameters<tools_posts::CreatePostInput>,
    ) -> Result<Json<tools_posts::CreatePostOutput>, String> {
        tools_posts::create_post(&self.state, &params.0).await
    }

    #[tool(description = "List posts with optional state filter")]
    async fn posts_list(
        &self,
        params: Parameters<tools_posts::ListPostsInput>,
    ) -> Result<Json<tools_posts::ListPostsOutput>, String> {
        tools_posts::list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single post by ID")]
    async fn posts_get(
        &self,
        params: Parameters<tools_posts::GetPostInput>,
    ) -> Result<Json<tools_posts::GetPostOutput>, String> {
        tools_posts::get_post(&self.state, &params.0).await
    }

    #[tool(description = "Schedule a post for publishing at a")]
    async fn posts_schedule(
        &self,
        params: Parameters<tools_posts::SchedulePostInput>,
    ) -> Result<Json<tools_posts::SchedulePostOutput>, String> {
        tools_posts::schedule_post(&self.state, &params.0).await
    }

    #[tool(description = "Publish a post immediately. Accepts")]
    async fn posts_publish(
        &self,
        params: Parameters<tools_posts::GetPostInput>,
    ) -> Result<Json<tools_posts::SchedulePostOutput>, String> {
        tools_posts::publish_post(&self.state, &params.0).await
    }

    #[tool(description = "Delete a post by ID")]
    async fn posts_delete(
        &self,
        params: Parameters<tools_posts::DeletePostInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_posts::delete_post(&self.state, &params.0).await
    }

    #[tool(description = "Find the next available free time")]
    async fn posts_find_slot(
        &self,
        params: Parameters<tools_posts::FindSlotInput>,
    ) -> Result<Json<tools_posts::FindSlotOutput>, String> {
        tools_posts::find_slot(&self.state, &params.0).await
    }

    #[tool(description = "Update a post's content or title")]
    async fn posts_update(
        &self,
        params: Parameters<tools_posts::UpdatePostInput>,
    ) -> Result<Json<tools_posts::UpdatePostOutput>, String> {
        tools_posts::update_post(&self.state, &params.0).await
    }

    #[tool(description = "Upload media (image/video) for post attachments. Pass base64-encoded file data.")]
    async fn posts_media_upload(
        &self,
        params: Parameters<tools_media::MediaUploadInput>,
    ) -> Result<Json<tools_media::MediaUploadOutput>, String> {
        tools_media::upload_media(&self.state, &params.0).await
    }

    #[tool(description = "List uploaded media files")]
    async fn posts_media_list(
        &self,
        params: Parameters<tools_media::MediaListInput>,
    ) -> Result<Json<tools_media::MediaListOutput>, String> {
        tools_media::list_media(&self.state, &params.0).await
    }

    #[tool(description = "Stage a post across multiple platforms. Auto-splits content per platform character limits. Creates draft posts for each integration.")]
    async fn posts_stage(
        &self,
        params: Parameters<tools_posts::StagePostInput>,
    ) -> Result<Json<tools_posts::StagePostOutput>, String> {
        tools_posts::stage_post(&self.state, &params.0).await
    }

    // ── Comment Tools ───────────────────────────────────────

    #[tool(description = "Get comments for a post on any platform")]
    async fn get_comments(
        &self,
        params: Parameters<tools_comments::GetCommentsInput>,
    ) -> Result<Json<tools_comments::GetCommentsOutput>, String> {
        tools_comments::get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Reply to a comment on any platform")]
    async fn reply_to_comment(
        &self,
        params: Parameters<tools_comments::ReplyToCommentInput>,
    ) -> Result<Json<tools_comments::ReplyToCommentOutput>, String> {
        tools_comments::reply_to_comment(&self.state, &params.0).await
    }

    #[tool(description = "Delete a comment on any platform")]
    async fn delete_comment(
        &self,
        params: Parameters<tools_comments::DeleteCommentInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_comments::delete_comment(&self.state, &params.0).await
    }

    // ── DM Tools ────────────────────────────────────────────

    #[tool(description = "Send a direct message on any platform")]
    async fn send_dm(
        &self,
        params: Parameters<tools_dm::SendDmInput>,
    ) -> Result<Json<tools_dm::SendDmOutput>, String> {
        tools_dm::send_dm(&self.state, &params.0).await
    }

    #[tool(description = "List DM conversations on any platform")]
    async fn list_dm_conversations(
        &self,
        params: Parameters<tools_dm::ListDmInput>,
    ) -> Result<Json<tools_dm::ListDmOutput>, String> {
        tools_dm::list_dm_conversations(&self.state, &params.0).await
    }

    #[tool(description = "Get messages in a DM conversation")]
    async fn get_dm_messages(
        &self,
        params: Parameters<tools_dm::GetDmInput>,
    ) -> Result<Json<tools_dm::GetDmOutput>, String> {
        tools_dm::get_dm_messages(&self.state, &params.0).await
    }

    // ── Automation Tools ─────────────────────────────────────

    #[tool(description = "Create an automation rule for auto-reply to comments/DMs")]
    async fn create_automation_rule(
        &self,
        params: Parameters<tools_automation::CreateRuleInput>,
    ) -> Result<Json<tools_automation::CreateRuleOutput>, String> {
        tools_automation::create_rule(&self.state, &params.0).await
    }

    #[tool(description = "List automation rules")]
    async fn list_automation_rules(
        &self,
        params: Parameters<tools_automation::ListRulesInput>,
    ) -> Result<Json<tools_automation::ListRulesOutput>, String> {
        tools_automation::list_rules(&self.state, &params.0).await
    }

    #[tool(description = "Update an automation rule")]
    async fn update_automation_rule(
        &self,
        params: Parameters<tools_automation::UpdateRuleInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_automation::update_rule(&self.state, &params.0).await
    }

    #[tool(description = "Delete an automation rule")]
    async fn delete_automation_rule(
        &self,
        params: Parameters<tools_automation::DeleteRuleInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_automation::delete_rule(&self.state, &params.0).await
    }

    #[tool(description = "Get execution logs for an automation rule")]
    async fn get_automation_logs(
        &self,
        params: Parameters<tools_automation::GetLogsInput>,
    ) -> Result<Json<tools_automation::GetLogsOutput>, String> {
        tools_automation::get_logs(&self.state, &params.0).await
    }

    // ── Reddit Read/Query Tools ──────────────────────────────────

    #[tool(description = "Browse a subreddit and list posts.")]
    async fn reddit_browse(
        &self,
        params: Parameters<tools_reddit::RedditBrowseInput>,
    ) -> Result<Json<tools_reddit::RedditBrowseOutput>, String> {
        tools_reddit::reddit_browse(&self.state, &params.0).await
    }

    #[tool(description = "Search Reddit. Optionally restrict to")]
    async fn reddit_search(
        &self,
        params: Parameters<tools_reddit::RedditSearchInput>,
    ) -> Result<Json<tools_reddit::RedditSearchOutput>, String> {
        tools_reddit::reddit_search(&self.state, &params.0).await
    }

    #[tool(description = "Get a Reddit post's full content with")]
    async fn reddit_post_detail(
        &self,
        params: Parameters<tools_reddit::RedditPostDetailInput>,
    ) -> Result<Json<tools_reddit::RedditPostDetailOutput>, String> {
        tools_reddit::reddit_post_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get Reddit user info with optional")]
    async fn reddit_user_info(
        &self,
        params: Parameters<tools_reddit::RedditUserInfoInput>,
    ) -> Result<Json<tools_reddit::RedditUserInfoOutput>, String> {
        tools_reddit::reddit_user_info(&self.state, &params.0).await
    }

    #[tool(description = "Send a direct message to a Reddit")]
    async fn reddit_send_dm(
        &self,
        params: Parameters<tools_reddit::RedditSendDmInput>,
    ) -> Result<Json<tools_reddit::RedditSendDmOutput>, String> {
        tools_reddit::reddit_send_dm(&self.state, &params.0).await
    }

    #[tool(description = "Read Reddit inbox messages")]
    async fn reddit_inbox(
        &self,
        params: Parameters<tools_reddit::RedditInboxInput>,
    ) -> Result<Json<tools_reddit::RedditInboxOutput>, String> {
        tools_reddit::reddit_inbox(&self.state, &params.0).await
    }

    #[tool(description = "Get comments for a Reddit post with")]
    async fn reddit_get_comments(
        &self,
        params: Parameters<tools_reddit::RedditGetCommentsInput>,
    ) -> Result<Json<tools_reddit::RedditGetCommentsOutput>, String> {
        tools_reddit::reddit_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Create a new Reddit post (text or")]
    pub async fn reddit_create_post(
        &self,
        params: Parameters<tools_reddit::RedditCreatePostInput>,
    ) -> Result<Json<tools_reddit::RedditCreatePostOutput>, String> {
        tools_reddit::handle_reddit_create_post(&self.state, &params.0).await
    }

    #[tool(description = "Create a comment or reply on Reddit.")]
    pub async fn reddit_create_comment(
        &self,
        params: Parameters<tools_reddit::RedditCreateCommentInput>,
    ) -> Result<Json<tools_reddit::RedditCreateCommentOutput>, String> {
        tools_reddit::handle_reddit_create_comment(&self.state, &params.0).await
    }

    #[tool(description = "Get karma breakdown by subreddit for")]
    pub async fn reddit_get_karma(
        &self,
    ) -> Result<Json<tools_reddit::RedditGetKarmaOutput>, String> {
        tools_reddit::handle_reddit_get_karma(&self.state).await
    }

    #[tool(description = "Vote on a Reddit post or comment.")]
    pub async fn reddit_vote(
        &self,
        params: Parameters<tools_reddit::RedditVoteInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_vote(&self.state, &params.0).await
    }

    #[tool(description = "Save a Reddit post or comment.")]
    pub async fn reddit_save(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_save(&self.state, &params.0).await
    }

    #[tool(description = "Unsave a Reddit post or comment.")]
    pub async fn reddit_unsave(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_unsave(&self.state, &params.0).await
    }

    #[tool(description = "Hide a Reddit post from your feed.")]
    pub async fn reddit_hide(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_hide(&self.state, &params.0).await
    }

    #[tool(description = "Subscribe or unsubscribe to a")]
    pub async fn reddit_subscribe(
        &self,
        params: Parameters<tools_reddit::RedditSubscribeInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_subscribe(&self.state, &params.0).await
    }

    #[tool(description = "Edit a Reddit post or comment text.")]
    pub async fn reddit_edit(
        &self,
        params: Parameters<tools_reddit::RedditEditInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_edit(&self.state, &params.0).await
    }

    #[tool(description = "Delete a Reddit post or comment.")]
    pub async fn reddit_delete(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_delete(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: remove a post/comment.")]
    pub async fn reddit_mod_remove(
        &self,
        params: Parameters<tools_reddit::RedditModRemoveInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_remove(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: approve a post/comment.")]
    pub async fn reddit_mod_approve(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_approve(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: distinguish a comment")]
    pub async fn reddit_mod_distinguish(
        &self,
        params: Parameters<tools_reddit::RedditModDistinguishInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_distinguish(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: sticky or unsticky a")]
    pub async fn reddit_mod_sticky(
        &self,
        params: Parameters<tools_reddit::RedditModStickyInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_sticky(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: lock a post/comment")]
    pub async fn reddit_mod_lock(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_lock(&self.state, &params.0).await
    }

    #[tool(description = "Moderation: unlock a post/comment")]
    pub async fn reddit_mod_unlock(
        &self,
        params: Parameters<tools_reddit::RedditThingInput>,
    ) -> Result<Json<tools_reddit::RedditActionOutput>, String> {
        tools_reddit::handle_reddit_mod_unlock(&self.state, &params.0).await
    }

    // ── X/Twitter Read Tools ─────────────────────────────────────

    #[tool(description = "Get the authenticated X/Twitter")]
    async fn x_get_me(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_x::XGetMeOutput>, String> {
        tools_x::x_get_me(&self.state).await
    }

    #[tool(description = "Get X/Twitter home timeline (reverse")]
    async fn x_home_timeline(
        &self,
        params: Parameters<tools_x::XHomeTimelineInput>,
    ) -> Result<Json<tools_x::XHomeTimelineOutput>, String> {
        tools_x::x_home_timeline(&self.state, &params.0).await
    }

    #[tool(description = "Lookup an X/Twitter user by their")]
    async fn x_user_lookup(
        &self,
        params: Parameters<tools_x::XUserLookupInput>,
    ) -> Result<Json<tools_x::XUserLookupOutput>, String> {
        tools_x::x_user_lookup(&self.state, &params.0).await
    }

    #[tool(description = "Lookup an X/Twitter user by their")]
    async fn x_user_lookup_by_username(
        &self,
        params: Parameters<tools_x::XUserLookupByUsernameInput>,
    ) -> Result<Json<tools_x::XUserLookupByUsernameOutput>, String> {
        tools_x::x_user_lookup_by_username(&self.state, &params.0).await
    }

    #[tool(description = "Get tweets from a specific X/Twitter")]
    async fn x_user_tweets(
        &self,
        params: Parameters<tools_x::XUserTweetsInput>,
    ) -> Result<Json<tools_x::XUserTweetsOutput>, String> {
        tools_x::x_user_tweets(&self.state, &params.0).await
    }

    #[tool(description = "Get a single X/Twitter tweet with")]
    async fn x_tweet_detail(
        &self,
        params: Parameters<tools_x::XTweetDetailInput>,
    ) -> Result<Json<tools_x::XTweetDetailOutput>, String> {
        tools_x::x_tweet_detail(&self.state, &params.0).await
    }

    #[tool(description = "Search recent X/Twitter tweets. Query")]
    async fn x_search_tweets(
        &self,
        params: Parameters<tools_x::XSearchTweetsInput>,
    ) -> Result<Json<tools_x::XSearchTweetsOutput>, String> {
        tools_x::x_search_tweets(&self.state, &params.0).await
    }

    #[tool(description = "Get followers of an X/Twitter user")]
    async fn x_followers(
        &self,
        params: Parameters<tools_x::XFollowersInput>,
    ) -> Result<Json<tools_x::XFollowersOutput>, String> {
        tools_x::x_followers(&self.state, &params.0).await
    }

    #[tool(description = "Get who an X/Twitter user is following")]
    async fn x_following(
        &self,
        params: Parameters<tools_x::XFollowingInput>,
    ) -> Result<Json<tools_x::XFollowingOutput>, String> {
        tools_x::x_following(&self.state, &params.0).await
    }

    #[tool(description = "Get bookmarked X/Twitter tweets")]
    async fn x_bookmarks(
        &self,
        params: Parameters<tools_x::XBookmarksInput>,
    ) -> Result<Json<tools_x::XBookmarksOutput>, String> {
        tools_x::x_bookmarks(&self.state, &params.0).await
    }

    // ── X/Twitter Write Tools ────────────────────────────────────

    #[tool(description = "Delete an X/Twitter tweet by its ID")]
    async fn x_delete_tweet(
        &self,
        params: Parameters<tools_x::XDeleteTweetInput>,
    ) -> Result<Json<tools_x::XDeleteTweetOutput>, String> {
        tools_x::x_delete_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Like a tweet on X/Twitter")]
    async fn x_like_tweet(
        &self,
        params: Parameters<tools_x::XLikeTweetInput>,
    ) -> Result<Json<tools_x::XLikeTweetOutput>, String> {
        tools_x::x_like_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Unlike a tweet on X/Twitter")]
    async fn x_unlike_tweet(
        &self,
        params: Parameters<tools_x::XUnlikeTweetInput>,
    ) -> Result<Json<tools_x::XUnlikeTweetOutput>, String> {
        tools_x::x_unlike_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Retweet a tweet on X/Twitter")]
    async fn x_retweet(
        &self,
        params: Parameters<tools_x::XRetweetInput>,
    ) -> Result<Json<tools_x::XRetweetOutput>, String> {
        tools_x::x_retweet(&self.state, &params.0).await
    }

    #[tool(description = "Unretweet (remove retweet of) a tweet")]
    async fn x_unretweet(
        &self,
        params: Parameters<tools_x::XUnretweetInput>,
    ) -> Result<Json<tools_x::XUnretweetOutput>, String> {
        tools_x::x_unretweet(&self.state, &params.0).await
    }

    #[tool(description = "Bookmark a tweet on X/Twitter")]
    async fn x_bookmark_tweet(
        &self,
        params: Parameters<tools_x::XBookmarkTweetInput>,
    ) -> Result<Json<tools_x::XBookmarkTweetOutput>, String> {
        tools_x::x_bookmark_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Remove a bookmark from a tweet on")]
    async fn x_unbookmark_tweet(
        &self,
        params: Parameters<tools_x::XUnbookmarkTweetInput>,
    ) -> Result<Json<tools_x::XUnbookmarkTweetOutput>, String> {
        tools_x::x_unbookmark_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Follow a user on X/Twitter by their")]
    async fn x_follow_user(
        &self,
        params: Parameters<tools_x::XFollowUserInput>,
    ) -> Result<Json<tools_x::XFollowUserOutput>, String> {
        tools_x::x_follow_user(&self.state, &params.0).await
    }

    #[tool(description = "Unfollow a user on X/Twitter by their")]
    async fn x_unfollow_user(
        &self,
        params: Parameters<tools_x::XUnfollowUserInput>,
    ) -> Result<Json<tools_x::XUnfollowUserOutput>, String> {
        tools_x::x_unfollow_user(&self.state, &params.0).await
    }

    #[tool(description = "Get tweets from an X/Twitter List by")]
    async fn x_list_timeline(
        &self,
        params: Parameters<tools_x::XListTimelineInput>,
    ) -> Result<Json<tools_x::XListTimelineOutput>, String> {
        tools_x::x_list_timeline(&self.state, &params.0).await
    }

    #[tool(description = "Reply to an X/Twitter tweet")]
    async fn x_reply_tweet(
        &self,
        params: Parameters<tools_x::XReplyTweetInput>,
    ) -> Result<Json<tools_x::XReplyTweetOutput>, String> {
        tools_x::x_reply_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Send a direct message on X/Twitter")]
    async fn x_send_dm(
        &self,
        params: Parameters<tools_x::XSendDmInput>,
    ) -> Result<Json<tools_x::XSendDmOutput>, String> {
        tools_x::x_send_dm(&self.state, &params.0).await
    }

    #[tool(description = "List X/Twitter DM conversations")]
    async fn x_list_dms(
        &self,
        params: Parameters<tools_x::XListDmsInput>,
    ) -> Result<Json<tools_x::XListDmsOutput>, String> {
        tools_x::x_list_dms(&self.state, &params.0).await
    }

    #[tool(description = "Get messages in an X/Twitter DM conversation")]
    async fn x_get_dm_conversation(
        &self,
        params: Parameters<tools_x::XGetDmConversationInput>,
    ) -> Result<Json<tools_x::XGetDmConversationOutput>, String> {
        tools_x::x_get_dm_conversation(&self.state, &params.0).await
    }

    // ── Facebook Tools ──────────────────────────────────────────────

    #[tool(description = "Get a Facebook page's feed (recent")]
    pub async fn fb_get_feed(
        &self,
        params: Parameters<tools_facebook::FbGetFeedInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_get_feed(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Facebook post by ID")]
    pub async fn fb_get_post(
        &self,
        params: Parameters<tools_facebook::FbGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get comments on a Facebook post")]
    pub async fn fb_get_comments(
        &self,
        params: Parameters<tools_facebook::FbGetCommentsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_get_comments(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a text/link post on a Facebook")]
    pub async fn fb_create_post(
        &self,
        params: Parameters<tools_facebook::FbCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a photo post on a Facebook page")]
    pub async fn fb_create_photo(
        &self,
        params: Parameters<tools_facebook::FbCreatePhotoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_create_photo(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a video post on a Facebook page")]
    pub async fn fb_create_video(
        &self,
        params: Parameters<tools_facebook::FbCreateVideoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_create_video(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a Facebook post by ID")]
    pub async fn fb_delete_post(
        &self,
        params: Parameters<tools_facebook::FbDeletePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_delete_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Comment on a Facebook post")]
    pub async fn fb_comment(
        &self,
        params: Parameters<tools_facebook::FbCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "React to a Facebook post")]
    pub async fn fb_react(
        &self,
        params: Parameters<tools_facebook::FbReactInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_react(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get insights/analytics for a Facebook")]
    pub async fn fb_page_insights(
        &self,
        params: Parameters<tools_facebook::FbPageInsightsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_page_insights(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get conversations (inbox) for a")]
    pub async fn fb_conversations(
        &self,
        params: Parameters<tools_facebook::FbConversationsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_conversations(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get messages in a Facebook")]
    pub async fn fb_conversation_messages(
        &self,
        params: Parameters<tools_facebook::FbConversationMsgsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_conversation_msgs(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Send a message to a Facebook")]
    pub async fn fb_send_message(
        &self,
        params: Parameters<tools_facebook::FbSendMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_send_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search for public Facebook pages by")]
    pub async fn fb_search_pages(
        &self,
        params: Parameters<tools_facebook::FbSearchPagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_search_pages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get albums from a Facebook page")]
    pub async fn fb_albums(
        &self,
        params: Parameters<tools_facebook::FbAlbumsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_facebook::handle_fb_albums(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Instagram Tools ─────────────────────────────────────────────

    #[tool(description = "Get Instagram media (posts/reels) for")]
    pub async fn ig_get_media(
        &self,
        params: Parameters<tools_instagram::IgGetMediaInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_media(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific Instagram")]
    pub async fn ig_get_media_detail(
        &self,
        params: Parameters<tools_instagram::IgGetMediaDetailInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_media_detail(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get comments on an Instagram media")]
    pub async fn ig_get_comments(
        &self,
        params: Parameters<tools_instagram::IgGetCommentsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_comments(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search for Instagram hashtags by name")]
    pub async fn ig_search_hashtag(
        &self,
        params: Parameters<tools_instagram::IgSearchHashtagInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_search_hashtag(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get recent media for an Instagram")]
    pub async fn ig_get_hashtag_media(
        &self,
        params: Parameters<tools_instagram::IgGetHashtagMediaInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_hashtag_media(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get insights for an Instagram")]
    pub async fn ig_get_insights(
        &self,
        params: Parameters<tools_instagram::IgGetInsightsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_insights(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get media where the Instagram account")]
    pub async fn ig_get_tagged(
        &self,
        params: Parameters<tools_instagram::IgGetTaggedInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_tagged(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create an Instagram media container")]
    pub async fn ig_create_container(
        &self,
        params: Parameters<tools_instagram::IgCreateContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_create_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Publish an Instagram media container")]
    pub async fn ig_publish_container(
        &self,
        params: Parameters<tools_instagram::IgPublishContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_publish_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Reply to an Instagram comment")]
    pub async fn ig_reply_to_comment(
        &self,
        params: Parameters<tools_instagram::IgReplyToCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_reply_to_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Instagram reels for a business")]
    pub async fn ig_get_reels(
        &self,
        params: Parameters<tools_instagram::IgGetReelsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_reels(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Instagram stories for a business")]
    pub async fn ig_get_stories(
        &self,
        params: Parameters<tools_instagram::IgGetStoriesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_stories(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get followers of an Instagram")]
    pub async fn ig_get_followers(
        &self,
        params: Parameters<tools_instagram::IgGetFollowersInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_followers(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Discover an Instagram business")]
    pub async fn ig_business_discovery(
        &self,
        params: Parameters<tools_instagram::IgBusinessDiscoveryInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_business_discovery(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }


    #[tool(description = "Get recent mentions of an Instagram")]
    pub async fn ig_get_mentions(
        &self,
        params: Parameters<tools_instagram::IgGetMentionsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_mentions(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Poll the status of an Instagram")]
    pub async fn ig_poll_container(
        &self,
        params: Parameters<tools_instagram::IgPollContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_poll_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get audience insights for an")]
    pub async fn ig_get_insights_audience(
        &self,
        params: Parameters<tools_instagram::IgGetInsightsAudienceInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram::handle_ig_get_insights_audience(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Instagram Standalone (Basic Display API) Tools ─────────────────

    #[tool(description = "Get Instagram media feed for a")]
    pub async fn ias_get_media(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetMediaInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_get_media(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific Instagram")]
    pub async fn ias_get_media_detail(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetMediaDetailInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_get_media_detail(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get comments on an Instagram media")]
    pub async fn ias_get_comments(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetCommentsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_get_comments(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Reply to an Instagram comment (Basic")]
    pub async fn ias_reply_to_comment(
        &self,
        params: Parameters<tools_instagram_standalone::IasReplyToCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_reply_to_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create an Instagram media container")]
    pub async fn ias_create_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasCreateContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_create_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Publish an Instagram media container")]
    pub async fn ias_publish_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasPublishContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_publish_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Poll Instagram container publish")]
    pub async fn ias_poll_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasPollContainerInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_instagram_standalone::handle_ias_poll_container(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Google Suite Tools ─────────────────────────────────────────
    // YouTube, Gmail, Calendar, Drive — all use Google OAuth single sign-on

    #[tool(description = "Search YouTube videos by query")]
    pub async fn goog_search_videos(
        &self,
        params: Parameters<tools_google::YtSearchVideosInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_search_videos(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific YouTube")]
    pub async fn goog_get_video(
        &self,
        params: Parameters<tools_google::YtGetVideoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_video(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List playlists for a YouTube channel")]
    pub async fn goog_list_playlists(
        &self,
        params: Parameters<tools_google::YtListPlaylistsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_playlists(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get items in a YouTube playlist")]
    pub async fn goog_get_playlist_items(
        &self,
        params: Parameters<tools_google::YtGetPlaylistItemsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_playlist_items(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get comments on a YouTube video")]
    pub async fn goog_get_comments(
        &self,
        params: Parameters<tools_google::YtGetCommentsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_comments(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get statistics for a YouTube channel")]
    pub async fn goog_get_channel_stats(
        &self,
        params: Parameters<tools_google::YtGetChannelStatsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_channel_stats(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get YouTube Analytics reports for a")]
    pub async fn goog_get_analytics(
        &self,
        params: Parameters<tools_google::YtGetAnalyticsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_analytics(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get subscriptions for a YouTube")]
    pub async fn goog_get_subscriptions(
        &self,
        params: Parameters<tools_google::YtGetSubscriptionsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_subscriptions(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Find YouTube creators by topic")]
    pub async fn goog_find_creators(
        &self,
        params: Parameters<tools_google::YtFindCreatorsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_find_creators(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Gmail profile info")]
    pub async fn goog_get_profile(
        &self,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_profile(&self.state, &()).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Gmail messages with optional")]
    pub async fn goog_list_messages(
        &self,
        params: Parameters<tools_google::GmListMessagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_messages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Gmail message by ID")]
    pub async fn goog_get_message(
        &self,
        params: Parameters<tools_google::GmGetMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Send an email via Gmail")]
    pub async fn goog_send_message(
        &self,
        params: Parameters<tools_google::GmSendMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_send_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Gmail labels")]
    pub async fn goog_list_labels(
        &self,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_labels(&self.state, &()).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a Gmail thread by ID")]
    pub async fn goog_get_thread(
        &self,
        params: Parameters<tools_google::GmGetThreadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_thread(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search Gmail messages with a query")]
    pub async fn goog_search_messages(
        &self,
        params: Parameters<tools_google::GmSearchMessagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_search_messages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Google calendars")]
    pub async fn goog_list_calendars(
        &self,
        params: Parameters<tools_google::GcalListCalendarsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_calendars(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List calendar events")]
    pub async fn goog_list_events(
        &self,
        params: Parameters<tools_google::GcalListEventsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_events(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single calendar event")]
    pub async fn goog_get_event(
        &self,
        params: Parameters<tools_google::GcalGetEventInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_event(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a new calendar event")]
    pub async fn goog_create_event(
        &self,
        params: Parameters<tools_google::GcalCreateEventInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_create_event(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Update a calendar event")]
    pub async fn goog_update_event(
        &self,
        params: Parameters<tools_google::GcalUpdateEventInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_update_event(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a calendar event")]
    pub async fn goog_delete_event(
        &self,
        params: Parameters<tools_google::GcalDeleteEventInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_delete_event(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List files in Google Drive")]
    pub async fn goog_list_files(
        &self,
        params: Parameters<tools_google::DrListFilesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_files(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a file from Google Drive by ID")]
    pub async fn goog_get_file(
        &self,
        params: Parameters<tools_google::DrGetFileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_file(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search files in Google Drive")]
    pub async fn goog_search_files(
        &self,
        params: Parameters<tools_google::DrSearchFilesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_search_files(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List folders in Google Drive")]
    pub async fn goog_list_folders(
        &self,
        params: Parameters<tools_google::DrListFoldersInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_list_folders(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get file metadata from Google Drive")]
    pub async fn goog_get_file_metadata(
        &self,
        params: Parameters<tools_google::DrGetFileMetadataInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_get_file_metadata(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Export a Google Drive file to a")]
    pub async fn goog_export_file(
        &self,
        params: Parameters<tools_google::DrExportFileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_google::handle_goog_export_file(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Pinterest Tools ──────────────────────────────────────────

    #[tool(description = "Get Pinterest user account info")]
    pub async fn pi_get_user_account(
        &self,
        params: Parameters<tools_pinterest::PiGetUserAccountInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_user_account(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Pinterest board details")]
    pub async fn pi_get_board(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_board(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get pins on a Pinterest board")]
    pub async fn pi_get_board_pins(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardPinsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_board_pins(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Pinterest pin")]
    pub async fn pi_get_pin(
        &self,
        params: Parameters<tools_pinterest::PiGetPinInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_pin(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get analytics for a Pinterest board")]
    pub async fn pi_get_board_analytics(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardAnalyticsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_board_analytics(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get analytics for a Pinterest pin")]
    pub async fn pi_get_pin_analytics(
        &self,
        params: Parameters<tools_pinterest::PiGetPinAnalyticsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_get_pin_analytics(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search Pinterest pins by keyword")]
    pub async fn pi_search_pins(
        &self,
        params: Parameters<tools_pinterest::PiSearchPinsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_pinterest::handle_pi_search_pins(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Discord Tools ───────────────────────────────────────────

    #[tool(description = "Get Discord channel info")]
    pub async fn di_get_channel(
        &self,
        params: Parameters<tools_discord::DiGetChannelInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_channel(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get messages from a Discord channel")]
    pub async fn di_get_messages(
        &self,
        params: Parameters<tools_discord::DiGetMessagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_messages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Discord guild/server info")]
    pub async fn di_get_guild(
        &self,
        params: Parameters<tools_discord::DiGetGuildInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_guild(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get thread members in a Discord")]
    pub async fn di_get_thread_members(
        &self,
        params: Parameters<tools_discord::DiGetThreadMembersInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_thread_members(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Send a message to a Discord channel")]
    pub async fn di_send_message(
        &self,
        params: Parameters<tools_discord::DiSendMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_send_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a message from a Discord")]
    pub async fn di_delete_message(
        &self,
        params: Parameters<tools_discord::DiDeleteMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_delete_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Add a reaction (emoji) to a Discord")]
    pub async fn di_add_reaction(
        &self,
        params: Parameters<tools_discord::DiAddReactionInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_add_reaction(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List all channels in a Discord")]
    pub async fn di_get_guild_channels(
        &self,
        params: Parameters<tools_discord::DiGetGuildChannelsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_guild_channels(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get detailed info about a Discord")]
    pub async fn di_get_server_info(
        &self,
        params: Parameters<tools_discord::DiGetServerInfoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_get_server_info(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a forum post in a Discord")]
    pub async fn di_create_forum_post(
        &self,
        params: Parameters<tools_discord::DiCreateForumPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_discord::handle_di_create_forum_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── WhatsApp Tools ───────────────────────────────────────────

    #[tool(description = "Check WhatsApp authentication status")]
    pub async fn wa_auth_status(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_whatsapp::WaAuthStatusOutput>, String> {
        tools_whatsapp::handle_wa_auth_status(&self.state).await
    }

    #[tool(description = "Send a text message via WhatsApp")]
    pub async fn wa_send_text(
        &self,
        params: Parameters<tools_whatsapp::WaSendTextInput>,
    ) -> Result<Json<tools_whatsapp::WaSendTextOutput>, String> {
        tools_whatsapp::handle_wa_send_text(&self.state, &params.0).await
    }

    #[tool(description = "List WhatsApp chats")]
    pub async fn wa_chats(
        &self,
        params: Parameters<tools_whatsapp::WaChatsInput>,
    ) -> Result<Json<tools_whatsapp::WaChatsOutput>, String> {
        tools_whatsapp::handle_wa_chats(&self.state, &params.0).await
    }

    #[tool(description = "List WhatsApp contacts")]
    pub async fn wa_contacts(
        &self,
        params: Parameters<tools_whatsapp::WaContactsInput>,
    ) -> Result<Json<tools_whatsapp::WaContactsOutput>, String> {
        tools_whatsapp::handle_wa_contacts(&self.state, &params.0).await
    }

    #[tool(description = "Edit a previously sent WhatsApp")]
    pub async fn wa_edit_message(
        &self,
        params: Parameters<tools_whatsapp::WaEditMessageInput>,
    ) -> Result<Json<tools_whatsapp::WaEditMessageOutput>, String> {
        tools_whatsapp::handle_wa_edit_message(&self.state, &params.0).await
    }

    #[tool(description = "Delete/revoke a WhatsApp message for")]
    pub async fn wa_revoke_message(
        &self,
        params: Parameters<tools_whatsapp::WaRevokeMessageInput>,
    ) -> Result<Json<tools_whatsapp::WaRevokeMessageOutput>, String> {
        tools_whatsapp::handle_wa_revoke_message(&self.state, &params.0).await
    }

    #[tool(description = "List all WhatsApp groups the user")]
    pub async fn wa_list_groups(
        &self,
    ) -> Result<Json<tools_whatsapp::WaListGroupsOutput>, String> {
        tools_whatsapp::handle_wa_list_groups(&self.state).await
    }

    #[tool(description = "Create a new WhatsApp group with")]
    pub async fn wa_create_group(
        &self,
        params: Parameters<tools_whatsapp::WaCreateGroupInput>,
    ) -> Result<Json<tools_whatsapp::WaCreateGroupOutput>, String> {
        tools_whatsapp::handle_wa_create_group(&self.state, &params.0).await
    }

    #[tool(description = "Get the invite link for a WhatsApp")]
    pub async fn wa_group_invite_link(
        &self,
        params: Parameters<tools_whatsapp::WaGroupInviteLinkInput>,
    ) -> Result<Json<tools_whatsapp::WaGroupInviteLinkOutput>, String> {
        tools_whatsapp::handle_wa_group_invite_link(&self.state, &params.0).await
    }

    // ── WordPress Tools ───────────────────────────────────────────────

    #[tool(description = "Create a new WordPress post. Requires")]
    pub async fn wp_create_post(
        &self,
        params: Parameters<tools_wordpress::WpCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_wordpress::handle_wp_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List WordPress posts with optional")]
    pub async fn wp_list_posts(
        &self,
        params: Parameters<tools_wordpress::WpListPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_wordpress::handle_wp_list_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single WordPress post by ID")]
    pub async fn wp_get_post(
        &self,
        params: Parameters<tools_wordpress::WpGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_wordpress::handle_wp_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List categories for a WordPress site")]
    pub async fn wp_list_categories(
        &self,
        params: Parameters<tools_wordpress::WpListCategoriesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_wordpress::handle_wp_list_categories(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Threads Tools ─────────────────────────────────────────────────

    #[tool(description = "Get your Threads profile")]
    pub async fn th_get_profile(
        &self,
        params: Parameters<tools_threads::ThreadsGetProfileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_get_profile(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Threads threads (posts) for your")]
    pub async fn th_get_threads(
        &self,
        params: Parameters<tools_threads::ThreadsGetThreadsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_get_threads(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific Threads")]
    pub async fn th_get_thread_detail(
        &self,
        params: Parameters<tools_threads::ThreadsGetThreadDetailInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_get_thread_detail(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get replies on a Threads thread")]
    pub async fn th_get_replies(
        &self,
        params: Parameters<tools_threads::ThreadsGetRepliesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_get_replies(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Reply to a Threads thread")]
    pub async fn th_reply_to_thread(
        &self,
        params: Parameters<tools_threads::ThreadsReplyToThreadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_reply_to_thread(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create and publish a new Threads post")]
    pub async fn th_create_thread(
        &self,
        params: Parameters<tools_threads::ThreadsCreateThreadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_create_thread(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a Threads thread (post)")]
    pub async fn th_delete_thread(
        &self,
        params: Parameters<tools_threads::ThreadsDeleteThreadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_delete_thread(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get insights/analytics for your")]
    pub async fn th_get_insights(
        &self,
        params: Parameters<tools_threads::ThreadsGetInsightsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_get_insights(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Poll publish status of a Threads")]
    pub async fn th_poll_publish_status(
        &self,
        params: Parameters<tools_threads::ThreadsPollStatusInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_threads::handle_th_poll_publish_status(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── LinkedIn Personal Tools ─────────────────────────────────

    #[tool(description = "Get your LinkedIn profile information")]
    pub async fn li_get_profile(
        &self,
        params: Parameters<tools_linkedin::LiGetProfileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_profile(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List LinkedIn posts for an author URN")]
    pub async fn li_get_posts(
        &self,
        params: Parameters<tools_linkedin::LiGetPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific LinkedIn")]
    pub async fn li_get_post_detail(
        &self,
        params: Parameters<tools_linkedin::LiGetPostDetailInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_post_detail(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get comments on a LinkedIn post by")]
    pub async fn li_get_comments(
        &self,
        params: Parameters<tools_linkedin::LiGetCommentsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_comments(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a comment on a LinkedIn post.")]
    pub async fn li_create_comment(
        &self,
        params: Parameters<tools_linkedin::LiCreateCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_create_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a new LinkedIn post")]
    pub async fn li_create_post(
        &self,
        params: Parameters<tools_linkedin::LiCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a LinkedIn post by its URN")]
    pub async fn li_delete_post(
        &self,
        params: Parameters<tools_linkedin::LiDeletePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_delete_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get reactions (likes) on a LinkedIn")]
    pub async fn li_get_reactions(
        &self,
        params: Parameters<tools_linkedin::LiGetReactionsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_reactions(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get shares (reposts) of a LinkedIn")]
    pub async fn li_get_shares(
        &self,
        params: Parameters<tools_linkedin::LiGetSharesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_shares(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get analytics for your LinkedIn")]
    pub async fn li_get_analytics(
        &self,
        params: Parameters<tools_linkedin::LiGetAnalyticsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_analytics(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get analytics for a specific LinkedIn")]
    pub async fn li_get_post_analytics(
        &self,
        params: Parameters<tools_linkedin::LiGetPostAnalyticsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_post_analytics(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Reply to a LinkedIn comment")]
    pub async fn li_reply_comment(
        &self,
        params: Parameters<tools_linkedin::LiReplyCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_reply_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Send a direct message on LinkedIn")]
    pub async fn li_send_dm(
        &self,
        params: Parameters<tools_linkedin::LiSendDmInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_send_dm(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List LinkedIn message conversations")]
    pub async fn li_list_conversations(
        &self,
        params: Parameters<tools_linkedin::LiListConversationsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_list_conversations(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get messages in a LinkedIn conversation")]
    pub async fn li_get_messages(
        &self,
        params: Parameters<tools_linkedin::LiGetMessagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin::handle_li_get_messages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── LinkedIn Page Tools ────────────────────────────────────

    #[tool(description = "List LinkedIn company pages you")]
    pub async fn lip_list_pages(
        &self,
        params: Parameters<tools_linkedin_page::LipListPagesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_list_pages(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a LinkedIn company")]
    pub async fn lip_get_page(
        &self,
        params: Parameters<tools_linkedin_page::LipGetPageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_get_page(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get posts from a LinkedIn company")]
    pub async fn lip_get_page_posts(
        &self,
        params: Parameters<tools_linkedin_page::LipGetPagePostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_get_page_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a comment on a LinkedIn post")]
    pub async fn lip_create_comment(
        &self,
        params: Parameters<tools_linkedin_page::LipCreateCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_create_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a post as a LinkedIn Page")]
    pub async fn lip_create_post(
        &self,
        params: Parameters<tools_linkedin_page::LipCreatePostInput>,
    ) -> Result<Json<tools_linkedin_page::LipCreatePostOutput>, String> {
        tools_linkedin_page::handle_lip_create_post(&self.state, &params.0).await
    }

    #[tool(description = "Get LinkedIn Page analytics")]
    pub async fn lip_get_analytics(
        &self,
        params: Parameters<tools_linkedin_page::LipGetAnalyticsInput>,
    ) -> Result<Json<tools_linkedin_page::LipGetAnalyticsOutput>, String> {
        tools_linkedin_page::handle_lip_get_analytics(&self.state, &params.0).await
    }

    #[tool(description = "Get analytics for a specific LinkedIn")]
    pub async fn lip_get_post_analytics(
        &self,
        params: Parameters<tools_linkedin_page::LipGetPostAnalyticsInput>,
    ) -> Result<Json<tools_linkedin_page::LipGetAnalyticsOutput>, String> {
        tools_linkedin_page::handle_lip_get_post_analytics(&self.state, &params.0).await
    }

    #[tool(description = "Get follower count for a LinkedIn")]
    pub async fn lip_get_followers(
        &self,
        params: Parameters<tools_linkedin_page::LipGetFollowersInput>,
    ) -> Result<Json<tools_linkedin_page::LipGetFollowersOutput>, String> {
        tools_linkedin_page::handle_lip_get_followers(&self.state, &params.0).await
    }

    #[tool(description = "Delete a LinkedIn Page post by its")]
    pub async fn lip_delete_post(
        &self,
        params: Parameters<tools_linkedin_page::LipDeletePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_delete_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get reactions (likes) on a LinkedIn")]
    pub async fn lip_get_reactions(
        &self,
        params: Parameters<tools_linkedin_page::LipGetReactionsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_get_reactions(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get shares (reposts) of a LinkedIn")]
    pub async fn lip_get_shares(
        &self,
        params: Parameters<tools_linkedin_page::LipGetSharesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_linkedin_page::handle_lip_get_shares(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Telegram Bot Tools ───────────────────────────────────────
    #[tool(description = "Send a message via Telegram Bot API")]
    pub async fn tb_send_message(
        &self,
        params: Parameters<tools_telegram_bot::TbSendMessageInput>,
    ) -> Result<Json<tools_telegram_bot::TbSendMessageOutput>, String> {
        tools_telegram_bot::handle_tb_send_message(&self.state, &params.0).await
    }

    #[tool(description = "Get updates from Telegram Bot API for")]
    pub async fn tb_get_updates(
        &self,
        params: Parameters<tools_telegram_bot::TbGetUpdatesInput>,
    ) -> Result<Json<tools_telegram_bot::TbGetUpdatesOutput>, String> {
        tools_telegram_bot::handle_tb_get_updates(&self.state, &params.0).await
    }

    #[tool(description = "Get Telegram bot info and username")]
    pub async fn tb_get_me(
        &self,
        params: Parameters<tools_telegram_bot::TbTokenInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_get_me(&self.state, &params.0).await
    }

    #[tool(description = "Get chat details (title, type, member")]
    pub async fn tb_get_chat(
        &self,
        params: Parameters<tools_telegram_bot::TbChatInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_get_chat(&self.state, &params.0).await
    }

    #[tool(description = "Get member count of a Telegram")]
    pub async fn tb_get_chat_member_count(
        &self,
        params: Parameters<tools_telegram_bot::TbChatInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_get_chat_member_count(&self.state, &params.0).await
    }

    #[tool(description = "Send a photo via Telegram Bot to a")]
    pub async fn tb_send_photo(
        &self,
        params: Parameters<tools_telegram_bot::TbSendPhotoInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_send_photo(&self.state, &params.0).await
    }

    #[tool(description = "Send a document/file via Telegram Bot")]
    pub async fn tb_send_document(
        &self,
        params: Parameters<tools_telegram_bot::TbSendDocumentInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_send_document(&self.state, &params.0).await
    }

    #[tool(description = "Forward a message from one Telegram")]
    pub async fn tb_forward_message(
        &self,
        params: Parameters<tools_telegram_bot::TbForwardInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_forward_message(&self.state, &params.0).await
    }

    #[tool(description = "Pin a message in a Telegram chat.")]
    pub async fn tb_pin_message(
        &self,
        params: Parameters<tools_telegram_bot::TbPinInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_pin_message(&self.state, &params.0).await
    }

    #[tool(description = "Unpin a message in a Telegram chat.")]
    pub async fn tb_unpin_message(
        &self,
        params: Parameters<tools_telegram_bot::TbPinInput>,
    ) -> Result<Json<tools_telegram_bot::TbApiOutput>, String> {
        tools_telegram_bot::handle_tb_unpin_message(&self.state, &params.0).await
    }

    // ── Telegram User Tools ──────────────────────────────────────

    #[tool(description = "Check Telegram user client")]
    pub async fn tu_auth_status(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_telegram_user::TuAuthStatusOutput>, String> {
        tools_telegram_user::handle_tu_auth_status(&self.state).await
    }

    #[tool(description = "Send a message via Telegram user")]
    pub async fn tu_send_message(
        &self,
        params: Parameters<tools_telegram_user::TuSendMessageInput>,
    ) -> Result<Json<tools_telegram_user::TuSendMessageOutput>, String> {
        tools_telegram_user::handle_tu_send_message(&self.state, &params.0).await
    }

    #[tool(description = "List dialogs/conversations via")]
    pub async fn tu_list_dialogs(
        &self,
        params: Parameters<tools_telegram_user::TuListDialogsInput>,
    ) -> Result<Json<tools_telegram_user::TuListDialogsOutput>, String> {
        tools_telegram_user::handle_tu_list_dialogs(&self.state, &params.0).await
    }

    #[tool(description = "List contacts via Telegram user client")]
    pub async fn tu_list_contacts(
        &self,
        params: Parameters<tools_telegram_user::TuListContactsInput>,
    ) -> Result<Json<tools_telegram_user::TuListContactsOutput>, String> {
        tools_telegram_user::handle_tu_list_contacts(&self.state, &params.0).await
    }

    #[tool(description = "Search Telegram user client")]
    pub async fn tu_search(
        &self,
        params: Parameters<tools_telegram_user::TuSearchInput>,
    ) -> Result<Json<tools_telegram_user::TuSearchOutput>, String> {
        tools_telegram_user::handle_tu_search(&self.state, &params.0).await
    }

    #[tool(description = "Request a login code for Telegram")]
    pub async fn tu_request_code(
        &self,
        params: Parameters<tools_telegram_user::TuRequestCodeInput>,
    ) -> Result<Json<tools_telegram_user::TuRequestCodeOutput>, String> {
        tools_telegram_user::handle_tu_request_code(&self.state, &params.0).await
    }

    #[tool(description = "Sign in to Telegram user account with")]
    pub async fn tu_sign_in(
        &self,
        params: Parameters<tools_telegram_user::TuSignInInput>,
    ) -> Result<Json<tools_telegram_user::TuSignInOutput>, String> {
        tools_telegram_user::handle_tu_sign_in(&self.state, &params.0).await
    }

    // ── Skool Tools ─────────────────────────────────────────────────

    #[tool(description = "Publish a post to a Skool group.")]
    pub async fn sk_publish(
        &self,
        params: Parameters<tools_skool::SkPublishInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_skool::handle_sk_publish(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Skool community details")]
    pub async fn sk_get_info(
        &self,
        params: Parameters<tools_skool::SkGetInfoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_skool::handle_sk_get_info(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List posts in a Skool community with")]
    pub async fn sk_list_posts(
        &self,
        params: Parameters<tools_skool::SkListPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_skool::handle_sk_list_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Skool post by community")]
    pub async fn sk_get_post(
        &self,
        params: Parameters<tools_skool::SkGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_skool::handle_sk_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a comment on a Skool post")]
    pub async fn sk_create_comment(
        &self,
        params: Parameters<tools_skool::SkCreateCommentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_skool::handle_sk_create_comment(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Slack Tools ─────────────────────────────────────────────────

    #[tool(description = "Send a message to a Slack channel.")]
    pub async fn sl_send_message(
        &self,
        params: Parameters<tools_slack::SlSendMessageInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_slack::handle_sl_send_message(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List all public and private channels")]
    pub async fn sl_list_channels(
        &self,
        params: Parameters<tools_slack::SlListChannelsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_slack::handle_sl_list_channels(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get message history for a Slack")]
    pub async fn sl_channel_history(
        &self,
        params: Parameters<tools_slack::SlChannelHistoryInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_slack::handle_sl_channel_history(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List all users in the Slack workspace")]
    pub async fn sl_list_users(
        &self,
        params: Parameters<tools_slack::SlListUsersInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_slack::handle_sl_list_users(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Bluesky Tools ─────────────────────────────────────────────────

    #[tool(description = "Get a Bluesky user's profile by")]
    pub async fn bs_profile(
        &self,
        params: Parameters<tools_bluesky::BsProfileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_bluesky::handle_bs_profile(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get the Bluesky home timeline")]
    pub async fn bs_timeline(
        &self,
        params: Parameters<tools_bluesky::BsTimelineInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_bluesky::handle_bs_timeline(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a new Bluesky post (text with")]
    pub async fn bs_create_post(
        &self,
        params: Parameters<tools_bluesky::BsCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_bluesky::handle_bs_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search Bluesky for posts and users")]
    pub async fn bs_search(
        &self,
        params: Parameters<tools_bluesky::BsSearchInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_bluesky::handle_bs_search(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get the Bluesky feed (popular/recent")]
    pub async fn bs_feed(
        &self,
        params: Parameters<tools_bluesky::BsFeedInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_bluesky::handle_bs_feed(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── TikTok Tools ─────────────────────────────────────────────────

    #[tool(description = "Get TikTok user profile info")]
    pub async fn tt_profile(
        &self,
        params: Parameters<tools_tiktok::TtProfileInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tiktok::handle_tt_profile(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create and publish a TikTok video")]
    pub async fn tt_create_post(
        &self,
        params: Parameters<tools_tiktok::TtCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tiktok::handle_tt_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List TikTok videos for the")]
    pub async fn tt_list_videos(
        &self,
        params: Parameters<tools_tiktok::TtListVideosInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tiktok::handle_tt_list_videos(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Mastodon Tools ───────────────────────────────────────────────

    #[tool(description = "Create a new Mastodon post (toot)")]
    pub async fn ms_create_post(
        &self,
        params: Parameters<tools_mastodon::MsCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_mastodon::handle_ms_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get Mastodon timeline feed")]
    pub async fn ms_get_timeline(
        &self,
        params: Parameters<tools_mastodon::MsGetTimelineInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_mastodon::handle_ms_get_timeline(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Mastodon post/status by")]
    pub async fn ms_get_post(
        &self,
        params: Parameters<tools_mastodon::MsGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_mastodon::handle_ms_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search Mastodon for posts/users")]
    pub async fn ms_search(
        &self,
        params: Parameters<tools_mastodon::MsSearchInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_mastodon::handle_ms_search(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Medium Tools ─────────────────────────────────────────────────

    #[tool(description = "Create a new Medium story (post)")]
    pub async fn md_create_post(
        &self,
        params: Parameters<tools_medium::MdCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_medium::handle_md_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Medium posts for the")]
    pub async fn md_list_posts(
        &self,
        params: Parameters<tools_medium::MdListPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_medium::handle_md_list_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Medium post by post ID")]
    pub async fn md_get_post(
        &self,
        params: Parameters<tools_medium::MdGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_medium::handle_md_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Hashnode Tools ────────────────────────────────────────────────

    #[tool(description = "Create a new Hashnode story (post)")]
    pub async fn hn_create_post(
        &self,
        params: Parameters<tools_hashnode::HnCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_hashnode::handle_hn_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Hashnode posts for a publication")]
    pub async fn hn_list_posts(
        &self,
        params: Parameters<tools_hashnode::HnListPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_hashnode::handle_hn_list_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Hashnode post by post ID")]
    pub async fn hn_get_post(
        &self,
        params: Parameters<tools_hashnode::HnGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_hashnode::handle_hn_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Dev.to Tools ─────────────────────────────────────────────────

    #[tool(description = "Create a new Dev.to article (post)")]
    pub async fn dv_create_post(
        &self,
        params: Parameters<tools_devto::DvCreatePostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_devto::handle_dv_create_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List Dev.to articles for the")]
    pub async fn dv_list_posts(
        &self,
        params: Parameters<tools_devto::DvListPostsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_devto::handle_dv_list_posts(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single Dev.to article by")]
    pub async fn dv_get_post(
        &self,
        params: Parameters<tools_devto::DvGetPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_devto::handle_dv_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Analytics Tools ──────────────────────────────────────────────

    #[tool(description = "Get analytics data for a connected")]
    pub async fn analytics_get(
        &self,
        params: Parameters<tools_analytics::AnalyticsGetInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_analytics::handle_analytics_get(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get analytics data for a specific post")]
    pub async fn analytics_get_post(
        &self,
        params: Parameters<tools_analytics::AnalyticsPostInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_analytics::handle_analytics_get_post(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Feed Tools ──────────────────────────────────────

    #[tool(description = "List imported external posts (unified")]
    async fn feed_list(
        &self,
        params: Parameters<tools_feed::FeedListInput>,
    ) -> Result<Json<tools_feed::FeedListOutput>, String> {
        tools_feed::handle_feed_list(&self.state, &params.0).await
    }

    #[tool(description = "Trigger immediate import of recent")]
    async fn feed_import(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_feed::FeedImportOutput>, String> {
        tools_feed::handle_feed_import(&self.state).await
    }

    // ── Tags Tools ──────────────────────────────────────────────────

    #[tool(description = "Create a new tag")]
    pub async fn tag_create(
        &self,
        params: Parameters<tools_tags::TagCreateInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tags::handle_tag_create(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List all tags for the authenticated")]
    pub async fn tag_list(
        &self,
        params: Parameters<tools_tags::TagListInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tags::handle_tag_list(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single tag by ID")]
    pub async fn tag_get(
        &self,
        params: Parameters<tools_tags::TagGetInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tags::handle_tag_get(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Update an existing tag")]
    pub async fn tag_update(
        &self,
        params: Parameters<tools_tags::TagUpdateInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tags::handle_tag_update(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a tag")]
    pub async fn tag_delete(
        &self,
        params: Parameters<tools_tags::TagDeleteInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_tags::handle_tag_delete(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Webhooks Tools ──────────────────────────────────────────────

    #[tool(description = "Create a new outgoing webhook")]
    pub async fn wh_create(
        &self,
        params: Parameters<tools_webhooks::WhCreateInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_create(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List all outgoing webhooks")]
    pub async fn wh_list(
        &self,
        params: Parameters<tools_webhooks::WhListInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_list(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a single webhook by ID")]
    pub async fn wh_get(
        &self,
        params: Parameters<tools_webhooks::WhGetInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_get(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Update an existing webhook")]
    pub async fn wh_update(
        &self,
        params: Parameters<tools_webhooks::WhUpdateInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_update(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Delete a webhook")]
    pub async fn wh_delete(
        &self,
        params: Parameters<tools_webhooks::WhDeleteInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_delete(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Test a webhook by sending a sample")]
    pub async fn wh_test(
        &self,
        params: Parameters<tools_webhooks::WhTestInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_webhooks::handle_wh_test(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── Notification Tools ─────────────────────────────────────

    #[tool(description = "List notifications with optional")]
    async fn notif_list(
        &self,
        params: Parameters<tools_notifications::NotifListInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_notifications::handle_notif_list(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Mark a notification as read by its ID")]
    async fn notif_mark_read(
        &self,
        params: Parameters<tools_notifications::NotifMarkReadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_notifications::handle_notif_mark_read(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Mark all notifications as read")]
    async fn notif_mark_all_read(
        &self,
        params: Parameters<tools_notifications::NotifMarkAllReadInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_notifications::handle_notif_mark_all_read(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a notification (for testing or")]
    async fn notif_create(
        &self,
        params: Parameters<tools_notifications::NotifCreateInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_notifications::handle_notif_create(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    // ── GitHub Tools ─────────────────────────────────────────────

    #[tool(description = "Get the currently authenticated")]
    pub async fn gh_get_authenticated_user(
        &self,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_authenticated_user(&self.state, &()).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a GitHub user by login username")]
    pub async fn gh_get_user(
        &self,
        params: Parameters<tools_github::GhGetUserInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_user(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List repositories for a GitHub user")]
    pub async fn gh_list_repos(
        &self,
        params: Parameters<tools_github::GhListReposInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_repos(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get details of a specific GitHub")]
    pub async fn gh_get_repo(
        &self,
        params: Parameters<tools_github::GhGetRepoInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_repo(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List issues for a GitHub repository")]
    pub async fn gh_list_issues(
        &self,
        params: Parameters<tools_github::GhListIssuesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_issues(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a specific GitHub issue")]
    pub async fn gh_get_issue(
        &self,
        params: Parameters<tools_github::GhGetIssueInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_issue(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Create a new issue in a GitHub")]
    pub async fn gh_create_issue(
        &self,
        params: Parameters<tools_github::GhCreateIssueInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_create_issue(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List pull requests for a GitHub")]
    pub async fn gh_list_pull_requests(
        &self,
        params: Parameters<tools_github::GhListPullRequestsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_pull_requests(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get a specific GitHub pull request")]
    pub async fn gh_get_pull_request(
        &self,
        params: Parameters<tools_github::GhGetPullRequestInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_pull_request(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List commits for a GitHub repository")]
    pub async fn gh_list_commits(
        &self,
        params: Parameters<tools_github::GhListCommitsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_commits(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List branches for a GitHub repository")]
    pub async fn gh_list_branches(
        &self,
        params: Parameters<tools_github::GhListBranchesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_branches(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List releases for a GitHub repository")]
    pub async fn gh_list_releases(
        &self,
        params: Parameters<tools_github::GhListReleasesInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_releases(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search GitHub repositories by query")]
    pub async fn gh_search_repos(
        &self,
        params: Parameters<tools_github::GhSearchReposInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_search_repos(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Search code on GitHub by query")]
    pub async fn gh_search_code(
        &self,
        params: Parameters<tools_github::GhSearchCodeInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_search_code(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List contributors for a GitHub")]
    pub async fn gh_list_contributors(
        &self,
        params: Parameters<tools_github::GhListContributorsInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_contributors(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Get file or directory contents from a")]
    pub async fn gh_get_repo_content(
        &self,
        params: Parameters<tools_github::GhGetRepoContentInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_get_repo_content(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "Close a GitHub issue")]
    pub async fn gh_close_issue(
        &self,
        params: Parameters<tools_github::GhCloseIssueInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_close_issue(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }

    #[tool(description = "List repositories for the")]
    pub async fn gh_list_my_repos(
        &self,
        params: Parameters<tools_github::GhListMyReposInput>,
    ) -> Result<Json<McpJsonValue>, String> {
        tools_github::handle_gh_list_my_repos(&self.state, &params.0).await.map(|Json(v)| Json(McpJsonValue(v)))
    }


}

// ══════════════════════════════════════════════════════════════
// RUNNER
// ══════════════════════════════════════════════════════════════

/// Start the MCP server on stdio (for AI clients that spawn the binary)
pub async fn run_mcp_stdio(state: AppState) -> anyhow::Result<()> {
    let server = SocialForgeMcpServer::new(state);
    let service = server.serve(lean_stdio()).await?;
    tracing::info!("MCP server started on stdio (schema-optimized)");
    service.waiting().await?;
    Ok(())
}

// ── Helper Input Types ──────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MeInput {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MeOutput {
    pub user_id: String,
    pub email: String,
    pub name: String,
}
