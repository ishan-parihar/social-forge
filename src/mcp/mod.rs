// ─── MCP Server ───────────────────────────────────────────────
// Model Context Protocol server exposing all Postiz operations as tools.
// Designed for AI agents to schedule, manage, and monitor posts.
//
// Uses rmcp crate with ServerHandler + #[tool_router] pattern.
// Same business logic as the REST API — shared via AppState.

use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    schemars::JsonSchema,
    tool, tool_router,
    transport::stdio,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::auth::jwt;
use crate::db::queries;


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
mod tools_medium;
mod tools_hashnode;
pub mod tools_pinterest;
mod tools_posts;
pub mod tools_discord;
mod tools_reddit;
mod tools_skool;
mod tools_tags;
pub mod tools_telegram_bot;
pub mod tools_telegram_user;
pub mod tools_threads;
pub mod tools_tiktok;
pub mod tools_whatsapp;
mod tools_webhooks;
pub mod tools_youtube;
mod tools_x;

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

// ══════════════════════════════════════════════════════════════
// TOOL ROUTER
// ══════════════════════════════════════════════════════════════



#[derive(Clone)]
pub struct PostizMcpServer {
    pub state: AppState,
}

// Helper: get DB pool from state


#[tool_router(server_handler)]
impl PostizMcpServer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    // ── Auth Tools ──────────────────────────────────────────

    #[tool(description = "Register a new account. Returns JWT token for authentication.")]
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

    #[tool(description = "Login with email and password. Returns JWT token.")]
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

    #[tool(description = "Get posts for a date range (for content calendar)")]
    async fn calendar_get(
        &self,
        params: Parameters<tools_calendar::CalendarInput>,
    ) -> Result<Json<tools_calendar::CalendarOutput>, String> {
        tools_calendar::get_calendar(&self.state, &params.0).await
    }

    // ── Integration Tools ────────────────────────────────────

    #[tool(description = "List all available social media providers with their configuration status")]
    async fn integrations_list_providers(
        &self,
        params: Parameters<tools_integrations::ListProvidersInput>,
    ) -> Result<Json<tools_integrations::ListProvidersOutput>, String> {
        tools_integrations::list_providers(&self.state, &params.0).await
    }

    #[tool(description = "List all connected social media channels")]
    async fn integrations_list(
        &self,
        params: Parameters<tools_integrations::ListIntegrationsInput>,
    ) -> Result<Json<tools_integrations::ListIntegrationsOutput>, String> {
        tools_integrations::list_integrations(&self.state, &params.0).await
    }

    #[tool(description = "Get OAuth URL to connect a social media channel")]
    async fn integrations_connect(
        &self,
        params: Parameters<tools_integrations::ConnectInput>,
    ) -> Result<Json<tools_integrations::ConnectOutput>, String> {
        tools_integrations::connect_integration(&self.state, &params.0).await
    }

    #[tool(description = "Complete OAuth connection after authorizing in browser. Pass code and state from the callback URL.")]
    async fn integrations_connect_complete(
        &self,
        params: Parameters<tools_integrations::ConnectCompleteInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_integrations::complete_connect_integration(&self.state, &params.0).await
    }

    #[tool(description = "Disconnect/remove a social media channel")]
    async fn integrations_disconnect(
        &self,
        params: Parameters<tools_integrations::DisconnectInput>,
    ) -> Result<Json<SuccessOutput>, String> {
        tools_integrations::disconnect_integration(&self.state, &params.0).await
    }

    // ── Post Tools ───────────────────────────────────────────

    #[tool(description = "Create a new post. Set scheduled_at to auto-schedule. Returns post ID and state.")]
    async fn posts_create(
        &self,
        params: Parameters<tools_posts::CreatePostInput>,
    ) -> Result<Json<tools_posts::CreatePostOutput>, String> {
        tools_posts::create_post(&self.state, &params.0).await
    }

    #[tool(description = "List posts with optional state filter (draft|queued|published|error)")]
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

    #[tool(description = "Schedule a post for publishing at a specific time")]
    async fn posts_schedule(
        &self,
        params: Parameters<tools_posts::SchedulePostInput>,
    ) -> Result<Json<tools_posts::SchedulePostOutput>, String> {
        tools_posts::schedule_post(&self.state, &params.0).await
    }

    #[tool(description = "Publish a post immediately. Accepts queued or errored posts.")]
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

    #[tool(description = "Find the next available free time slot for scheduling")]
    async fn posts_find_slot(
        &self,
        params: Parameters<tools_posts::FindSlotInput>,
    ) -> Result<Json<tools_posts::FindSlotOutput>, String> {
        tools_posts::find_slot(&self.state, &params.0).await
    }

    #[tool(description = "Update a post's content, title, media, or settings by ID")]
    async fn posts_update(
        &self,
        params: Parameters<tools_posts::UpdatePostInput>,
    ) -> Result<Json<tools_posts::UpdatePostOutput>, String> {
        tools_posts::update_post(&self.state, &params.0).await
    }

    // ── Reddit Read/Query Tools ──────────────────────────────────

    #[tool(description = "Browse a subreddit and list posts. Sort options: hot, new, top, rising, controversial. For top/controversial, set time to: hour, day, week, month, year, all.")]
    async fn reddit_browse(
        &self,
        params: Parameters<tools_reddit::RedditBrowseInput>,
    ) -> Result<Json<tools_reddit::RedditBrowseOutput>, String> {
        tools_reddit::reddit_browse(&self.state, &params.0).await
    }

    #[tool(description = "Search Reddit. Optionally restrict to a subreddit. Sort: relevance, hot, top, new, comments. Time: hour, day, week, month, year, all.")]
    async fn reddit_search(
        &self,
        params: Parameters<tools_reddit::RedditSearchInput>,
    ) -> Result<Json<tools_reddit::RedditSearchOutput>, String> {
        tools_reddit::reddit_search(&self.state, &params.0).await
    }

    #[tool(description = "Get a Reddit post's full content with nested comments. Pass post_id as the base36 ID or full URL.")]
    async fn reddit_post_detail(
        &self,
        params: Parameters<tools_reddit::RedditPostDetailInput>,
    ) -> Result<Json<tools_reddit::RedditPostDetailOutput>, String> {
        tools_reddit::reddit_post_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get Reddit user info with optional posts and comments.")]
    async fn reddit_user_info(
        &self,
        params: Parameters<tools_reddit::RedditUserInfoInput>,
    ) -> Result<Json<tools_reddit::RedditUserInfoOutput>, String> {
        tools_reddit::reddit_user_info(&self.state, &params.0).await
    }

    #[tool(description = "Send a direct message to a Reddit user.")]
    async fn reddit_send_dm(
        &self,
        params: Parameters<tools_reddit::RedditSendDmInput>,
    ) -> Result<Json<tools_reddit::RedditSendDmOutput>, String> {
        tools_reddit::reddit_send_dm(&self.state, &params.0).await
    }

    #[tool(description = "Read Reddit inbox. Folders: inbox, unread, sent, messages, mentions, comments, selfreply.")]
    async fn reddit_inbox(
        &self,
        params: Parameters<tools_reddit::RedditInboxInput>,
    ) -> Result<Json<tools_reddit::RedditInboxOutput>, String> {
        tools_reddit::reddit_inbox(&self.state, &params.0).await
    }

    #[tool(description = "Get comments for a Reddit post with sort option. Sort: confidence (default), top, new, controversial, old, qa.")]
    async fn reddit_get_comments(
        &self,
        params: Parameters<tools_reddit::RedditGetCommentsInput>,
    ) -> Result<Json<tools_reddit::RedditGetCommentsOutput>, String> {
        tools_reddit::reddit_get_comments(&self.state, &params.0).await
    }

    // ── X/Twitter Read Tools ─────────────────────────────────────

    #[tool(description = "Get the authenticated X/Twitter user's profile")]
    async fn x_get_me(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_x::XGetMeOutput>, String> {
        tools_x::x_get_me(&self.state).await
    }

    #[tool(description = "Get X/Twitter home timeline (reverse chronological). Shows recent tweets from people you follow.")]
    async fn x_home_timeline(
        &self,
        params: Parameters<tools_x::XHomeTimelineInput>,
    ) -> Result<Json<tools_x::XHomeTimelineOutput>, String> {
        tools_x::x_home_timeline(&self.state, &params.0).await
    }

    #[tool(description = "Lookup an X/Twitter user by their numeric user ID")]
    async fn x_user_lookup(
        &self,
        params: Parameters<tools_x::XUserLookupInput>,
    ) -> Result<Json<tools_x::XUserLookupOutput>, String> {
        tools_x::x_user_lookup(&self.state, &params.0).await
    }

    #[tool(description = "Lookup an X/Twitter user by their @username")]
    async fn x_user_lookup_by_username(
        &self,
        params: Parameters<tools_x::XUserLookupByUsernameInput>,
    ) -> Result<Json<tools_x::XUserLookupByUsernameOutput>, String> {
        tools_x::x_user_lookup_by_username(&self.state, &params.0).await
    }

    #[tool(description = "Get tweets from a specific X/Twitter user by their user ID")]
    async fn x_user_tweets(
        &self,
        params: Parameters<tools_x::XUserTweetsInput>,
    ) -> Result<Json<tools_x::XUserTweetsOutput>, String> {
        tools_x::x_user_tweets(&self.state, &params.0).await
    }

    #[tool(description = "Get a single X/Twitter tweet with full details, author info, and media")]
    async fn x_tweet_detail(
        &self,
        params: Parameters<tools_x::XTweetDetailInput>,
    ) -> Result<Json<tools_x::XTweetDetailOutput>, String> {
        tools_x::x_tweet_detail(&self.state, &params.0).await
    }

    #[tool(description = "Search recent X/Twitter tweets. Query supports standard Twitter search syntax (from:user, #hashtag, etc.).")]
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

    #[tool(description = "Unretweet (remove retweet of) a tweet on X/Twitter")]
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

    #[tool(description = "Remove a bookmark from a tweet on X/Twitter")]
    async fn x_unbookmark_tweet(
        &self,
        params: Parameters<tools_x::XUnbookmarkTweetInput>,
    ) -> Result<Json<tools_x::XUnbookmarkTweetOutput>, String> {
        tools_x::x_unbookmark_tweet(&self.state, &params.0).await
    }

    #[tool(description = "Follow a user on X/Twitter by their user ID")]
    async fn x_follow_user(
        &self,
        params: Parameters<tools_x::XFollowUserInput>,
    ) -> Result<Json<tools_x::XFollowUserOutput>, String> {
        tools_x::x_follow_user(&self.state, &params.0).await
    }

    #[tool(description = "Unfollow a user on X/Twitter by their user ID")]
    async fn x_unfollow_user(
        &self,
        params: Parameters<tools_x::XUnfollowUserInput>,
    ) -> Result<Json<tools_x::XUnfollowUserOutput>, String> {
        tools_x::x_unfollow_user(&self.state, &params.0).await
    }

    #[tool(description = "Get tweets from an X/Twitter List by list ID")]
    async fn x_list_timeline(
        &self,
        params: Parameters<tools_x::XListTimelineInput>,
    ) -> Result<Json<tools_x::XListTimelineOutput>, String> {
        tools_x::x_list_timeline(&self.state, &params.0).await
    }

    // ── Facebook Tools ──────────────────────────────────────────────

    #[tool(description = "Get a Facebook page's feed (recent posts)")]
    pub async fn fb_get_feed(
        &self,
        params: Parameters<tools_facebook::FbGetFeedInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_get_feed(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Facebook post by ID")]
    pub async fn fb_get_post(
        &self,
        params: Parameters<tools_facebook::FbGetPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_get_post(&self.state, &params.0).await
    }

    #[tool(description = "Get comments on a Facebook post")]
    pub async fn fb_get_comments(
        &self,
        params: Parameters<tools_facebook::FbGetCommentsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Create a text/link post on a Facebook page")]
    pub async fn fb_create_post(
        &self,
        params: Parameters<tools_facebook::FbCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_create_post(&self.state, &params.0).await
    }

    #[tool(description = "Create a photo post on a Facebook page")]
    pub async fn fb_create_photo(
        &self,
        params: Parameters<tools_facebook::FbCreatePhotoInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_create_photo(&self.state, &params.0).await
    }

    #[tool(description = "Create a video post on a Facebook page")]
    pub async fn fb_create_video(
        &self,
        params: Parameters<tools_facebook::FbCreateVideoInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_create_video(&self.state, &params.0).await
    }

    #[tool(description = "Delete a Facebook post by ID")]
    pub async fn fb_delete_post(
        &self,
        params: Parameters<tools_facebook::FbDeletePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_delete_post(&self.state, &params.0).await
    }

    #[tool(description = "Comment on a Facebook post")]
    pub async fn fb_comment(
        &self,
        params: Parameters<tools_facebook::FbCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_comment(&self.state, &params.0).await
    }

    #[tool(description = "React to a Facebook post (LIKE, LOVE, WOW, HAHA, SAD, ANGRY)")]
    pub async fn fb_react(
        &self,
        params: Parameters<tools_facebook::FbReactInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_react(&self.state, &params.0).await
    }

    #[tool(description = "Get insights/analytics for a Facebook page")]
    pub async fn fb_page_insights(
        &self,
        params: Parameters<tools_facebook::FbPageInsightsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_page_insights(&self.state, &params.0).await
    }

    #[tool(description = "Get conversations (inbox) for a Facebook page")]
    pub async fn fb_conversations(
        &self,
        params: Parameters<tools_facebook::FbConversationsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_conversations(&self.state, &params.0).await
    }

    #[tool(description = "Get messages in a Facebook conversation")]
    pub async fn fb_conversation_messages(
        &self,
        params: Parameters<tools_facebook::FbConversationMsgsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_conversation_msgs(&self.state, &params.0).await
    }

    #[tool(description = "Send a message to a Facebook conversation")]
    pub async fn fb_send_message(
        &self,
        params: Parameters<tools_facebook::FbSendMessageInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_send_message(&self.state, &params.0).await
    }

    #[tool(description = "Search for public Facebook pages by name")]
    pub async fn fb_search_pages(
        &self,
        params: Parameters<tools_facebook::FbSearchPagesInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_search_pages(&self.state, &params.0).await
    }

    #[tool(description = "Get albums from a Facebook page")]
    pub async fn fb_albums(
        &self,
        params: Parameters<tools_facebook::FbAlbumsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_facebook::handle_fb_albums(&self.state, &params.0).await
    }

    // ── Instagram Tools ─────────────────────────────────────────────

    #[tool(description = "Get Instagram media (posts/reels) for a business account")]
    pub async fn ig_get_media(
        &self,
        params: Parameters<tools_instagram::IgGetMediaInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_media(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a specific Instagram media item")]
    pub async fn ig_get_media_detail(
        &self,
        params: Parameters<tools_instagram::IgGetMediaDetailInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_media_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get comments on an Instagram media item")]
    pub async fn ig_get_comments(
        &self,
        params: Parameters<tools_instagram::IgGetCommentsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Search for Instagram hashtags by name")]
    pub async fn ig_search_hashtag(
        &self,
        params: Parameters<tools_instagram::IgSearchHashtagInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_search_hashtag(&self.state, &params.0).await
    }

    #[tool(description = "Get recent media for an Instagram hashtag")]
    pub async fn ig_get_hashtag_media(
        &self,
        params: Parameters<tools_instagram::IgGetHashtagMediaInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_hashtag_media(&self.state, &params.0).await
    }

    #[tool(description = "Get insights for an Instagram business account")]
    pub async fn ig_get_insights(
        &self,
        params: Parameters<tools_instagram::IgGetInsightsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_insights(&self.state, &params.0).await
    }

    #[tool(description = "Get media where the Instagram account is tagged")]
    pub async fn ig_get_tagged(
        &self,
        params: Parameters<tools_instagram::IgGetTaggedInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_tagged(&self.state, &params.0).await
    }

    #[tool(description = "Create an Instagram media container (step 1 of publish)")]
    pub async fn ig_create_container(
        &self,
        params: Parameters<tools_instagram::IgCreateContainerInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_create_container(&self.state, &params.0).await
    }

    #[tool(description = "Publish an Instagram media container (step 2 of publish)")]
    pub async fn ig_publish_container(
        &self,
        params: Parameters<tools_instagram::IgPublishContainerInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_publish_container(&self.state, &params.0).await
    }

    #[tool(description = "Reply to an Instagram comment")]
    pub async fn ig_reply_to_comment(
        &self,
        params: Parameters<tools_instagram::IgReplyToCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_reply_to_comment(&self.state, &params.0).await
    }

    #[tool(description = "Get Instagram reels for a business account")]
    pub async fn ig_get_reels(
        &self,
        params: Parameters<tools_instagram::IgGetReelsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_reels(&self.state, &params.0).await
    }

    #[tool(description = "Get Instagram stories for a business account")]
    pub async fn ig_get_stories(
        &self,
        params: Parameters<tools_instagram::IgGetStoriesInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_stories(&self.state, &params.0).await
    }

    #[tool(description = "Get followers of an Instagram business account")]
    pub async fn ig_get_followers(
        &self,
        params: Parameters<tools_instagram::IgGetFollowersInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_followers(&self.state, &params.0).await
    }

    #[tool(description = "Discover an Instagram business account by username")]
    pub async fn ig_business_discovery(
        &self,
        params: Parameters<tools_instagram::IgBusinessDiscoveryInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_business_discovery(&self.state, &params.0).await
    }


    #[tool(description = "Get audience insights for an Instagram business account")]
    pub async fn ig_get_insights_audience(
        &self,
        params: Parameters<tools_instagram::IgGetInsightsAudienceInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram::handle_ig_get_insights_audience(&self.state, &params.0).await
    }

    // ── Instagram Standalone (Basic Display API) Tools ─────────────────

    #[tool(description = "Get Instagram media feed for a personal (Basic Display API) account")]
    pub async fn ias_get_media(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetMediaInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_get_media(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a specific Instagram media item (Basic Display API)")]
    pub async fn ias_get_media_detail(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetMediaDetailInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_get_media_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get comments on an Instagram media item (Basic Display API)")]
    pub async fn ias_get_comments(
        &self,
        params: Parameters<tools_instagram_standalone::IasGetCommentsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Reply to an Instagram comment (Basic Display API)")]
    pub async fn ias_reply_to_comment(
        &self,
        params: Parameters<tools_instagram_standalone::IasReplyToCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_reply_to_comment(&self.state, &params.0).await
    }

    #[tool(description = "Create an Instagram media container (step 1 of publish, Basic Display API)")]
    pub async fn ias_create_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasCreateContainerInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_create_container(&self.state, &params.0).await
    }

    #[tool(description = "Publish an Instagram media container (step 2 of publish, Basic Display API)")]
    pub async fn ias_publish_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasPublishContainerInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_publish_container(&self.state, &params.0).await
    }

    #[tool(description = "Poll Instagram container publish status (Basic Display API)")]
    pub async fn ias_poll_container(
        &self,
        params: Parameters<tools_instagram_standalone::IasPollContainerInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_instagram_standalone::handle_ias_poll_container(&self.state, &params.0).await
    }

    // ── YouTube Tools ──────────────────────────────────────────────

    #[tool(description = "Search YouTube videos by query")]
    pub async fn yt_search_videos(
        &self,
        params: Parameters<tools_youtube::YtSearchVideosInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_search_videos(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a specific YouTube video by video ID")]
    pub async fn yt_get_video(
        &self,
        params: Parameters<tools_youtube::YtGetVideoInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_video(&self.state, &params.0).await
    }

    #[tool(description = "List playlists for a YouTube channel")]
    pub async fn yt_list_playlists(
        &self,
        params: Parameters<tools_youtube::YtListPlaylistsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_list_playlists(&self.state, &params.0).await
    }

    #[tool(description = "Get items in a YouTube playlist")]
    pub async fn yt_get_playlist_items(
        &self,
        params: Parameters<tools_youtube::YtGetPlaylistItemsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_playlist_items(&self.state, &params.0).await
    }

    #[tool(description = "Get comments on a YouTube video")]
    pub async fn yt_get_comments(
        &self,
        params: Parameters<tools_youtube::YtGetCommentsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Get statistics for a YouTube channel (subscribers, views, videos)")]
    pub async fn yt_get_channel_stats(
        &self,
        params: Parameters<tools_youtube::YtGetChannelStatsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_channel_stats(&self.state, &params.0).await
    }

    #[tool(description = "Get YouTube Analytics reports for a channel (views, watch time, etc.)")]
    pub async fn yt_get_analytics(
        &self,
        params: Parameters<tools_youtube::YtGetAnalyticsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_analytics(&self.state, &params.0).await
    }

    #[tool(description = "Get subscriptions for a YouTube channel")]
    pub async fn yt_get_subscriptions(
        &self,
        params: Parameters<tools_youtube::YtGetSubscriptionsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_get_subscriptions(&self.state, &params.0).await
    }

    #[tool(description = "Find YouTube creators by topic. Searches videos, groups by channel, enriches with subscriber counts and email detection.")]
    pub async fn yt_find_creators(
        &self,
        params: Parameters<tools_youtube::YtFindCreatorsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_youtube::handle_yt_find_creators(&self.state, &params.0).await
    }

    // ── Pinterest Tools ──────────────────────────────────────────

    #[tool(description = "Get Pinterest user account info")]
    pub async fn pi_get_user_account(
        &self,
        params: Parameters<tools_pinterest::PiGetUserAccountInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_user_account(&self.state, &params.0).await
    }

    #[tool(description = "Get Pinterest board details")]
    pub async fn pi_get_board(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_board(&self.state, &params.0).await
    }

    #[tool(description = "Get pins on a Pinterest board")]
    pub async fn pi_get_board_pins(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardPinsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_board_pins(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Pinterest pin")]
    pub async fn pi_get_pin(
        &self,
        params: Parameters<tools_pinterest::PiGetPinInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_pin(&self.state, &params.0).await
    }

    #[tool(description = "Get analytics for a Pinterest board")]
    pub async fn pi_get_board_analytics(
        &self,
        params: Parameters<tools_pinterest::PiGetBoardAnalyticsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_board_analytics(&self.state, &params.0).await
    }

    #[tool(description = "Get analytics for a Pinterest pin within a board")]
    pub async fn pi_get_pin_analytics(
        &self,
        params: Parameters<tools_pinterest::PiGetPinAnalyticsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_get_pin_analytics(&self.state, &params.0).await
    }

    #[tool(description = "Search Pinterest pins by keyword using Pinterest API v5")]
    pub async fn pi_search_pins(
        &self,
        params: Parameters<tools_pinterest::PiSearchPinsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_pinterest::handle_pi_search_pins(&self.state, &params.0).await
    }

    // ── Discord Tools ───────────────────────────────────────────

    #[tool(description = "Get Discord channel info")]
    pub async fn di_get_channel(
        &self,
        params: Parameters<tools_discord::DiGetChannelInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_channel(&self.state, &params.0).await
    }

    #[tool(description = "Get messages from a Discord channel")]
    pub async fn di_get_messages(
        &self,
        params: Parameters<tools_discord::DiGetMessagesInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_messages(&self.state, &params.0).await
    }

    #[tool(description = "Get Discord guild/server info")]
    pub async fn di_get_guild(
        &self,
        params: Parameters<tools_discord::DiGetGuildInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_guild(&self.state, &params.0).await
    }

    #[tool(description = "Get thread members in a Discord channel")]
    pub async fn di_get_thread_members(
        &self,
        params: Parameters<tools_discord::DiGetThreadMembersInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_thread_members(&self.state, &params.0).await
    }

    #[tool(description = "Send a message to a Discord channel")]
    pub async fn di_send_message(
        &self,
        params: Parameters<tools_discord::DiSendMessageInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_send_message(&self.state, &params.0).await
    }

    #[tool(description = "Delete a message from a Discord channel")]
    pub async fn di_delete_message(
        &self,
        params: Parameters<tools_discord::DiDeleteMessageInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_delete_message(&self.state, &params.0).await
    }

    #[tool(description = "Add a reaction (emoji) to a Discord message")]
    pub async fn di_add_reaction(
        &self,
        params: Parameters<tools_discord::DiAddReactionInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_add_reaction(&self.state, &params.0).await
    }

    #[tool(description = "List all channels in a Discord guild/server")]
    pub async fn di_get_guild_channels(
        &self,
        params: Parameters<tools_discord::DiGetGuildChannelsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_guild_channels(&self.state, &params.0).await
    }

    #[tool(description = "Get detailed info about a Discord guild/server including member counts")]
    pub async fn di_get_server_info(
        &self,
        params: Parameters<tools_discord::DiGetServerInfoInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_get_server_info(&self.state, &params.0).await
    }

    #[tool(description = "Create a forum post in a Discord forum channel")]
    pub async fn di_create_forum_post(
        &self,
        params: Parameters<tools_discord::DiCreateForumPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_discord::handle_di_create_forum_post(&self.state, &params.0).await
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

    // ── Threads Tools ─────────────────────────────────────────────────

    #[tool(description = "Get your Threads profile")]
    pub async fn th_get_profile(
        &self,
        params: Parameters<tools_threads::ThreadsGetProfileInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_get_profile(&self.state, &params.0).await
    }

    #[tool(description = "List Threads threads (posts) for your account")]
    pub async fn th_get_threads(
        &self,
        params: Parameters<tools_threads::ThreadsGetThreadsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_get_threads(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a specific Threads thread (post)")]
    pub async fn th_get_thread_detail(
        &self,
        params: Parameters<tools_threads::ThreadsGetThreadDetailInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_get_thread_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get replies on a Threads thread")]
    pub async fn th_get_replies(
        &self,
        params: Parameters<tools_threads::ThreadsGetRepliesInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_get_replies(&self.state, &params.0).await
    }

    #[tool(description = "Reply to a Threads thread")]
    pub async fn th_reply_to_thread(
        &self,
        params: Parameters<tools_threads::ThreadsReplyToThreadInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_reply_to_thread(&self.state, &params.0).await
    }

    #[tool(description = "Create and publish a new Threads post (text, image, or video)")]
    pub async fn th_create_thread(
        &self,
        params: Parameters<tools_threads::ThreadsCreateThreadInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_create_thread(&self.state, &params.0).await
    }

    #[tool(description = "Delete a Threads thread (post)")]
    pub async fn th_delete_thread(
        &self,
        params: Parameters<tools_threads::ThreadsDeleteThreadInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_delete_thread(&self.state, &params.0).await
    }

    #[tool(description = "Get insights/analytics for your Threads account")]
    pub async fn th_get_insights(
        &self,
        params: Parameters<tools_threads::ThreadsGetInsightsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_get_insights(&self.state, &params.0).await
    }

    #[tool(description = "Poll publish status of a Threads creation ID")]
    pub async fn th_poll_publish_status(
        &self,
        params: Parameters<tools_threads::ThreadsPollStatusInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_threads::handle_th_poll_publish_status(&self.state, &params.0).await
    }

    // ── LinkedIn Personal Tools ─────────────────────────────────

    #[tool(description = "Get your LinkedIn profile information (name, headline, industry, etc.)")]
    pub async fn li_get_profile(
        &self,
        params: Parameters<tools_linkedin::LiGetProfileInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_get_profile(&self.state, &params.0).await
    }

    #[tool(description = "List LinkedIn posts for an author URN (e.g. urn:li:person:abc). Use this to fetch your recent posts.")]
    pub async fn li_get_posts(
        &self,
        params: Parameters<tools_linkedin::LiGetPostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_get_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a specific LinkedIn post by its URN (e.g. urn:li:activity:xyz)")]
    pub async fn li_get_post_detail(
        &self,
        params: Parameters<tools_linkedin::LiGetPostDetailInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_get_post_detail(&self.state, &params.0).await
    }

    #[tool(description = "Get comments on a LinkedIn post by its URN")]
    pub async fn li_get_comments(
        &self,
        params: Parameters<tools_linkedin::LiGetCommentsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_get_comments(&self.state, &params.0).await
    }

    #[tool(description = "Create a comment on a LinkedIn post. Provide your actor URN (e.g. urn:li:person:abc) and the message text.")]
    pub async fn li_create_comment(
        &self,
        params: Parameters<tools_linkedin::LiCreateCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_create_comment(&self.state, &params.0).await
    }

    #[tool(description = "Create a new LinkedIn post (text-only). Content will be published immediately to your LinkedIn profile.")]
    pub async fn li_create_post(
        &self,
        params: Parameters<tools_linkedin::LiCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin::handle_li_create_post(&self.state, &params.0).await
    }

    // ── LinkedIn Page Tools ────────────────────────────────────

    #[tool(description = "List LinkedIn company pages you administer. Returns page IDs and names needed for page operations.")]
    pub async fn lip_list_pages(
        &self,
        params: Parameters<tools_linkedin_page::LipListPagesInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin_page::handle_lip_list_pages(&self.state, &params.0).await
    }

    #[tool(description = "Get details of a LinkedIn company page by its page ID")]
    pub async fn lip_get_page(
        &self,
        params: Parameters<tools_linkedin_page::LipGetPageInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin_page::handle_lip_get_page(&self.state, &params.0).await
    }

    #[tool(description = "Get posts from a LinkedIn company page by page ID")]
    pub async fn lip_get_page_posts(
        &self,
        params: Parameters<tools_linkedin_page::LipGetPagePostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin_page::handle_lip_get_page_posts(&self.state, &params.0).await
    }

    #[tool(description = "Create a comment on a LinkedIn post as a company page. Provide the page URN, post URN, and message.")]
    pub async fn lip_create_comment(
        &self,
        params: Parameters<tools_linkedin_page::LipCreateCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_linkedin_page::handle_lip_create_comment(&self.state, &params.0).await
    }

    // ── Telegram Bot Tools ───────────────────────────────────────
    pub async fn tb_send_message(
        &self,
        params: Parameters<tools_telegram_bot::TbSendMessageInput>,
    ) -> Result<Json<tools_telegram_bot::TbSendMessageOutput>, String> {
        tools_telegram_bot::handle_tb_send_message(&self.state, &params.0).await
    }

    #[tool(description = "Get updates from Telegram Bot API for a specific bot account by token_index.")]
    pub async fn tb_get_updates(
        &self,
        params: Parameters<tools_telegram_bot::TbGetUpdatesInput>,
    ) -> Result<Json<tools_telegram_bot::TbGetUpdatesOutput>, String> {
        tools_telegram_bot::handle_tb_get_updates(&self.state, &params.0).await
    }

    // ── Telegram User Tools ──────────────────────────────────────

    #[tool(description = "Check Telegram user client authentication status (via telegram-cli daemon)")]
    pub async fn tu_auth_status(
        &self,
        _params: Parameters<()>,
    ) -> Result<Json<tools_telegram_user::TuAuthStatusOutput>, String> {
        tools_telegram_user::handle_tu_auth_status(&self.state).await
    }

    #[tool(description = "Send a message via Telegram user client (telegram-cli daemon)")]
    pub async fn tu_send_message(
        &self,
        params: Parameters<tools_telegram_user::TuSendMessageInput>,
    ) -> Result<Json<tools_telegram_user::TuSendMessageOutput>, String> {
        tools_telegram_user::handle_tu_send_message(&self.state, &params.0).await
    }

    #[tool(description = "List dialogs/conversations via Telegram user client")]
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

    #[tool(description = "Search Telegram user client (contacts, dialogs, messages)")]
    pub async fn tu_search(
        &self,
        params: Parameters<tools_telegram_user::TuSearchInput>,
    ) -> Result<Json<tools_telegram_user::TuSearchOutput>, String> {
        tools_telegram_user::handle_tu_search(&self.state, &params.0).await
    }

    #[tool(description = "Request a login code for Telegram user account (Grammers MTProto)")]
    pub async fn tu_request_code(
        &self,
        params: Parameters<tools_telegram_user::TuRequestCodeInput>,
    ) -> Result<Json<tools_telegram_user::TuRequestCodeOutput>, String> {
        tools_telegram_user::handle_tu_request_code(&self.state, &params.0).await
    }

    #[tool(description = "Sign in to Telegram user account with code from tu_request_code")]
    pub async fn tu_sign_in(
        &self,
        params: Parameters<tools_telegram_user::TuSignInInput>,
    ) -> Result<Json<tools_telegram_user::TuSignInOutput>, String> {
        tools_telegram_user::handle_tu_sign_in(&self.state, &params.0).await
    }

    // ── Skool Tools ─────────────────────────────────────────────────

    #[tool(description = "Publish a post to a Skool group. Requires group_id, title, content. Optionally set a label.")]
    pub async fn sk_publish(
        &self,
        params: Parameters<tools_skool::SkPublishInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_skool::handle_sk_publish(&self.state, &params.0).await
    }

    #[tool(description = "Get Skool community info (name, description, member count)")]
    pub async fn sk_get_info(
        &self,
        params: Parameters<tools_skool::SkGetInfoInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_skool::handle_sk_get_info(&self.state, &params.0).await
    }

    #[tool(description = "List posts in a Skool community with optional pagination/sort/category")]
    pub async fn sk_list_posts(
        &self,
        params: Parameters<tools_skool::SkListPostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_skool::handle_sk_list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Skool post by community slug and post slug")]
    pub async fn sk_get_post(
        &self,
        params: Parameters<tools_skool::SkGetPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_skool::handle_sk_get_post(&self.state, &params.0).await
    }

    #[tool(description = "Create a comment on a Skool post")]
    pub async fn sk_create_comment(
        &self,
        params: Parameters<tools_skool::SkCreateCommentInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_skool::handle_sk_create_comment(&self.state, &params.0).await
    }

    // ── Bluesky Tools ─────────────────────────────────────────────────

    #[tool(description = "Get a Bluesky user's profile by handle or DID")]
    pub async fn bs_profile(
        &self,
        params: Parameters<tools_bluesky::BsProfileInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_bluesky::handle_bs_profile(&self.state, &params.0).await
    }

    #[tool(description = "Get the Bluesky home timeline")]
    pub async fn bs_timeline(
        &self,
        params: Parameters<tools_bluesky::BsTimelineInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_bluesky::handle_bs_timeline(&self.state, &params.0).await
    }

    #[tool(description = "Create a new Bluesky post (text with optional images)")]
    pub async fn bs_create_post(
        &self,
        params: Parameters<tools_bluesky::BsCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_bluesky::handle_bs_create_post(&self.state, &params.0).await
    }

    #[tool(description = "Search Bluesky for posts and users")]
    pub async fn bs_search(
        &self,
        params: Parameters<tools_bluesky::BsSearchInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_bluesky::handle_bs_search(&self.state, &params.0).await
    }

    #[tool(description = "Get the Bluesky feed (popular/recent posts)")]
    pub async fn bs_feed(
        &self,
        params: Parameters<tools_bluesky::BsFeedInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_bluesky::handle_bs_feed(&self.state, &params.0).await
    }

    // ── TikTok Tools ─────────────────────────────────────────────────

    #[tool(description = "Get TikTok user profile info")]
    pub async fn tt_profile(
        &self,
        params: Parameters<tools_tiktok::TtProfileInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tiktok::handle_tt_profile(&self.state, &params.0).await
    }

    #[tool(description = "Create and publish a TikTok video")]
    pub async fn tt_create_post(
        &self,
        params: Parameters<tools_tiktok::TtCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tiktok::handle_tt_create_post(&self.state, &params.0).await
    }

    #[tool(description = "List TikTok videos for the authenticated user")]
    pub async fn tt_list_videos(
        &self,
        params: Parameters<tools_tiktok::TtListVideosInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tiktok::handle_tt_list_videos(&self.state, &params.0).await
    }

    // ── Medium Tools ─────────────────────────────────────────────────

    #[tool(description = "Create a new Medium story (post)")]
    pub async fn md_create_post(
        &self,
        params: Parameters<tools_medium::MdCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_medium::handle_md_create_post(&self.state, &params.0).await
    }

    #[tool(description = "List Medium posts for the authenticated user")]
    pub async fn md_list_posts(
        &self,
        params: Parameters<tools_medium::MdListPostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_medium::handle_md_list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Medium post by post ID")]
    pub async fn md_get_post(
        &self,
        params: Parameters<tools_medium::MdGetPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_medium::handle_md_get_post(&self.state, &params.0).await
    }

    // ── Hashnode Tools ────────────────────────────────────────────────

    #[tool(description = "Create a new Hashnode story (post)")]
    pub async fn hn_create_post(
        &self,
        params: Parameters<tools_hashnode::HnCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_hashnode::handle_hn_create_post(&self.state, &params.0).await
    }

    #[tool(description = "List Hashnode posts for a publication")]
    pub async fn hn_list_posts(
        &self,
        params: Parameters<tools_hashnode::HnListPostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_hashnode::handle_hn_list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Hashnode post by post ID")]
    pub async fn hn_get_post(
        &self,
        params: Parameters<tools_hashnode::HnGetPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_hashnode::handle_hn_get_post(&self.state, &params.0).await
    }

    // ── Dev.to Tools ─────────────────────────────────────────────────

    #[tool(description = "Create a new Dev.to article (post)")]
    pub async fn dv_create_post(
        &self,
        params: Parameters<tools_devto::DvCreatePostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_devto::handle_dv_create_post(&self.state, &params.0).await
    }

    #[tool(description = "List Dev.to articles for the authenticated user")]
    pub async fn dv_list_posts(
        &self,
        params: Parameters<tools_devto::DvListPostsInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_devto::handle_dv_list_posts(&self.state, &params.0).await
    }

    #[tool(description = "Get a single Dev.to article by article ID")]
    pub async fn dv_get_post(
        &self,
        params: Parameters<tools_devto::DvGetPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_devto::handle_dv_get_post(&self.state, &params.0).await
    }

    // ── Analytics Tools ──────────────────────────────────────────────

    #[tool(description = "Get analytics data for a connected social provider")]
    pub async fn analytics_get(
        &self,
        params: Parameters<tools_analytics::AnalyticsGetInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_analytics::handle_analytics_get(&self.state, &params.0).await
    }

    #[tool(description = "Get analytics data for a specific post")]
    pub async fn analytics_get_post(
        &self,
        params: Parameters<tools_analytics::AnalyticsPostInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_analytics::handle_analytics_get_post(&self.state, &params.0).await
    }

    // ── Tags Tools ──────────────────────────────────────────────────

    #[tool(description = "Create a new tag")]
    pub async fn tag_create(
        &self,
        params: Parameters<tools_tags::TagCreateInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tags::handle_tag_create(&self.state, &params.0).await
    }

    #[tool(description = "List all tags for the authenticated user")]
    pub async fn tag_list(
        &self,
        params: Parameters<tools_tags::TagListInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tags::handle_tag_list(&self.state, &params.0).await
    }

    #[tool(description = "Get a single tag by ID")]
    pub async fn tag_get(
        &self,
        params: Parameters<tools_tags::TagGetInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tags::handle_tag_get(&self.state, &params.0).await
    }

    #[tool(description = "Update an existing tag")]
    pub async fn tag_update(
        &self,
        params: Parameters<tools_tags::TagUpdateInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tags::handle_tag_update(&self.state, &params.0).await
    }

    #[tool(description = "Delete a tag")]
    pub async fn tag_delete(
        &self,
        params: Parameters<tools_tags::TagDeleteInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_tags::handle_tag_delete(&self.state, &params.0).await
    }

    // ── Webhooks Tools ──────────────────────────────────────────────

    #[tool(description = "Create a new outgoing webhook")]
    pub async fn wh_create(
        &self,
        params: Parameters<tools_webhooks::WhCreateInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_create(&self.state, &params.0).await
    }

    #[tool(description = "List all outgoing webhooks")]
    pub async fn wh_list(
        &self,
        params: Parameters<tools_webhooks::WhListInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_list(&self.state, &params.0).await
    }

    #[tool(description = "Get a single webhook by ID")]
    pub async fn wh_get(
        &self,
        params: Parameters<tools_webhooks::WhGetInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_get(&self.state, &params.0).await
    }

    #[tool(description = "Update an existing webhook")]
    pub async fn wh_update(
        &self,
        params: Parameters<tools_webhooks::WhUpdateInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_update(&self.state, &params.0).await
    }

    #[tool(description = "Delete a webhook")]
    pub async fn wh_delete(
        &self,
        params: Parameters<tools_webhooks::WhDeleteInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_delete(&self.state, &params.0).await
    }

    #[tool(description = "Test a webhook by sending a sample event")]
    pub async fn wh_test(
        &self,
        params: Parameters<tools_webhooks::WhTestInput>,
    ) -> Result<Json<serde_json::Value>, String> {
        tools_webhooks::handle_wh_test(&self.state, &params.0).await
    }
}

// ══════════════════════════════════════════════════════════════
// RUNNER
// ══════════════════════════════════════════════════════════════

/// Start the MCP server on stdio (for AI clients that spawn the binary)
pub async fn run_mcp_stdio(state: AppState) -> anyhow::Result<()> {
    let server = PostizMcpServer::new(state);
    let service = server.serve(stdio()).await?;
    tracing::info!("MCP server started on stdio");
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
