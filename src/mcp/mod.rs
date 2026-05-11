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


mod tools_calendar;
pub mod tools_facebook;
pub mod tools_instagram;
pub mod tools_instagram_standalone;
mod tools_integrations;
mod tools_posts;
mod tools_reddit;
pub mod tools_telegram_bot;
pub mod tools_telegram_user;
pub mod tools_threads;
pub mod tools_whatsapp;
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

    // ── Telegram Bot Tools ───────────────────────────────────────

    #[tool(description = "Send a message via Telegram Bot API. Use token_index to select which bot account (0 = first).")]
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
