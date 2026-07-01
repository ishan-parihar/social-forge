pub mod run;
pub mod mcp_bridge;
pub mod platforms;
pub use run::run_cli;

use clap::{Parser, Subcommand};

/// Social Forge — Dual-Interface Social Media Scheduling Engine
#[derive(Parser, Debug)]
#[command(name = "social-forge", version, about, long_about = None)]
pub struct Cli {
    /// Output as JSON (default: true)
    #[arg(long, global = true, default_value_t = true)]
    pub json: bool,

    /// Pretty-print output for human readability
    #[arg(long, global = true, default_value_t = false)]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Start the HTTP REST server
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = 6543)]
        port: u16,
    },

    /// Start the MCP stdio server
    Mcp,

    /// Initialize ~/.social-forge/ config directory with default .env template
    Init,

    /// List connected social providers
    Providers,

    /// Connect a provider (auto-imports browser cookies for X/Reddit, shows OAuth URL for others)
    Connect {
        /// Provider name (x, reddit, linkedin, facebook, instagram, bluesky, github)
        provider: String,
    },

    /// Check health of all connected providers and report status
    Doctor,

    /// Full guided onboarding: check status, import cookies, connect providers
    Setup,

    /// Auto-import browser cookies for all cookie-based providers (X, Reddit)
    ConnectAll,

    /// Manage configuration values in ~/.social-forge/.env
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// X (Twitter) operations
    X {
        #[command(subcommand)]
        action: XAction,
    },

    /// Reddit operations
    Reddit {
        #[command(subcommand)]
        action: RedditAction,
    },

    /// LinkedIn personal profile operations
    Linkedin {
        #[command(subcommand)]
        action: LinkedinAction,
    },

    /// LinkedIn company page operations
    LinkedinPage {
        #[command(subcommand)]
        action: LinkedinPageAction,
    },

    /// Facebook page operations
    Facebook {
        #[command(subcommand)]
        action: FacebookAction,
    },

    /// Instagram account operations
    Instagram {
        #[command(subcommand)]
        action: InstagramAction,
    },

    /// YouTube operations
    Youtube {
        #[command(subcommand)]
        action: YoutubeAction,
    },

    /// Bluesky operations
    Bluesky {
        #[command(subcommand)]
        action: BlueskyAction,
    },

    /// Mastodon operations
    Mastodon {
        #[command(subcommand)]
        action: MastodonAction,
    },

    /// Import recent posts from a provider into local store
    Import {
        /// Provider name (x, reddit, bluesky)
        provider: String,
        /// Number of recent posts to import
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Show unified feed of imported posts across all providers
    Feed {
        /// Filter by provider name (x, reddit, etc.)
        #[arg(long)]
        provider: Option<String>,
        /// Number of posts to show
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },

    /// Comment operations
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },

    /// Direct message operations
    Dm {
        #[command(subcommand)]
        action: DmAction,
    },

    /// Automation rules management
    Automation {
        #[command(subcommand)]
        action: AutomationAction,
    },

    /// Posts management
    Posts {
        #[command(subcommand)]
        action: PostsAction,
    },

    // ── Unified Commands (mirror MCP tools) ──────────────────

    /// Unified post command — post to any platform with auto content-splitting
    Post {
        /// Post text content
        text: String,
        /// Target platforms (comma-separated: x,bluesky,linkedin,reddit,...). Uses all connected if omitted.
        #[arg(long)]
        platforms: Option<String>,
        /// Media URLs to attach (comma-separated)
        #[arg(long)]
        media: Option<String>,
        /// Schedule for later (ISO8601 datetime)
        #[arg(long)]
        schedule: Option<String>,
        /// First comment (for platforms that support it, e.g. Instagram)
        #[arg(long)]
        first_comment: Option<String>,
    },

    /// Stage a post across multiple platforms with auto content-splitting
    Stage {
        /// Post text content
        text: String,
        /// Target integration IDs (comma-separated UUIDs). Uses all connected if omitted.
        #[arg(long)]
        integrations: Option<String>,
        /// Media URLs to attach (comma-separated)
        #[arg(long)]
        media: Option<String>,
        /// Schedule for later (ISO8601 datetime)
        #[arg(long)]
        schedule: Option<String>,
        /// Preview splits without creating drafts
        #[arg(long, default_value_t = false)]
        preview: bool,
        /// First comment (for platforms that support it, e.g. Instagram)
        #[arg(long)]
        first_comment: Option<String>,
    },

    /// Create a carousel post with multiple images
    Carousel {
        /// Post text content
        text: String,
        /// Target integration ID (UUID)
        #[arg(long)]
        integration: String,
        /// Media URLs (comma-separated, minimum 2)
        #[arg(long)]
        media: String,
        /// Optional post title
        #[arg(long)]
        title: Option<String>,
        /// Schedule for later (ISO8601 datetime)
        #[arg(long)]
        schedule: Option<String>,
    },

    /// Media management — upload, list, download
    Media {
        #[command(subcommand)]
        action: MediaAction,
    },

    /// Call any MCP tool directly by name with JSON arguments
    McpCall {
        /// MCP tool name (e.g. posts_create, x_home_timeline, bs_create_post)
        tool: String,
        /// JSON arguments for the tool
        #[arg(long, default_value = "{}")]
        json: String,
    },

    /// List all available MCP tools that can be called via mcp-call
    McpTools,

    /// Content split preview — see how content will be split across platforms
    SplitPreview {
        /// Content to preview splitting for
        text: String,
        /// Platforms to preview (comma-separated). Shows all if omitted.
        #[arg(long)]
        platforms: Option<String>,
    },

    /// TikTok operations
    Tiktok {
        #[command(subcommand)]
        action: TiktokAction,
    },
    /// Threads operations
    Threads {
        #[command(subcommand)]
        action: ThreadsAction,
    },
    /// Discord operations
    Discord {
        #[command(subcommand)]
        action: DiscordAction,
    },
    /// Slack operations
    Slack {
        #[command(subcommand)]
        action: SlackAction,
    },
    /// Telegram Bot operations
    TelegramBot {
        #[command(subcommand)]
        action: TelegramBotAction,
    },
    /// Telegram User operations
    TelegramUser {
        #[command(subcommand)]
        action: TelegramUserAction,
    },
    /// WhatsApp operations
    Whatsapp {
        #[command(subcommand)]
        action: WhatsappAction,
    },
    /// Pinterest operations
    Pinterest {
        #[command(subcommand)]
        action: PinterestAction,
    },
    /// GitHub operations
    Github {
        #[command(subcommand)]
        action: GithubAction,
    },
    /// WordPress operations
    Wordpress {
        #[command(subcommand)]
        action: WordpressAction,
    },
    /// Hashnode operations
    Hashnode {
        #[command(subcommand)]
        action: HashnodeAction,
    },
    /// Medium blog operations
    MediumBlog {
        #[command(subcommand)]
        action: MediumBlogAction,
    },
    /// Dev.to operations
    Devto {
        #[command(subcommand)]
        action: DevtoAction,
    },
    /// Skool operations
    Skool {
        #[command(subcommand)]
        action: SkoolAction,
    },
    /// Google services (YouTube)
    Google {
        #[command(subcommand)]
        action: GoogleAction,
    },
    /// Google Drive operations
    Gdrive {
        #[command(subcommand)]
        action: DriveAction,
    },
    /// Google Calendar operations
    Gcal {
        #[command(subcommand)]
        action: GcalAction,
    },
    /// Gmail operations
    GmailOps {
        #[command(subcommand)]
        action: GmailAction,
    },
    /// Webhook management
    Webhooks {
        #[command(subcommand)]
        action: WebhooksAction,
    },
    /// Notification management
    Notifications {
        #[command(subcommand)]
        action: NotificationsAction,
    },
    /// Tag management
    Tags {
        #[command(subcommand)]
        action: TagsAction,
    },
    /// Analytics
    Analytics {
        #[command(subcommand)]
        action: AnalyticsAction,
    },
}

// ─── Comment Actions ────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum CommentAction {
    /// Get comments for a post
    Get {
        /// Integration ID
        integration_id: String,
        /// Post ID
        post_id: String,
        /// Number of comments to fetch
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Reply to a comment
    Reply {
        /// Integration ID
        integration_id: String,
        /// Comment ID
        comment_id: String,
        /// Reply content
        content: String,
    },
    /// Delete a comment
    Delete {
        /// Integration ID
        integration_id: String,
        /// Comment ID
        comment_id: String,
    },
}

// ─── DM Actions ─────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DmAction {
    /// Send a direct message
    Send {
        /// Integration ID
        integration_id: String,
        /// Recipient ID
        recipient: String,
        /// Message content
        content: String,
    },
    /// List DM conversations
    List {
        /// Integration ID
        integration_id: String,
        /// Number of conversations to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get messages in a conversation
    Messages {
        /// Integration ID
        integration_id: String,
        /// Conversation ID
        conversation_id: String,
        /// Number of messages to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

// ─── Automation Actions ─────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AutomationAction {
    /// Create an automation rule
    Create {
        /// Integration ID
        integration_id: String,
        /// Rule name
        name: String,
        /// Trigger type (comment, dm, mention, follow)
        trigger_type: String,
        /// Response template
        response_template: String,
        /// Response type (fixed, template, ai_generated)
        response_type: String,
    },
    /// List automation rules
    List {
        /// Integration ID (optional)
        integration_id: Option<String>,
    },
    /// Update an automation rule
    Update {
        /// Rule ID
        rule_id: String,
        /// Rule name (optional)
        name: Option<String>,
        /// Response template (optional)
        response_template: Option<String>,
        /// Is active (optional)
        is_active: Option<bool>,
    },
    /// Delete an automation rule
    Delete {
        /// Rule ID
        rule_id: String,
    },
    /// Get execution logs
    Logs {
        /// Rule ID
        rule_id: String,
        /// Number of logs to fetch
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
}

// ─── Posts Actions ────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum PostsAction {
    /// List posts (draft, queued, published, errored)
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Get a single post by ID
    Get {
        id: String,
    },
    /// Create a new post
    Create {
        content: String,
        #[arg(long)]
        integrations: String,
        #[arg(long)]
        schedule: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        media: Option<String>,
        #[arg(long)]
        first_comment: Option<String>,
        #[arg(long)]
        settings: Option<String>,
        #[arg(long)]
        state: Option<String>,
    },
    /// Re-schedule an existing post
    Schedule {
        id: String,
        scheduled_at: String,
    },
    /// Publish a queued or errored post immediately via the provider
    Publish {
        id: String,
    },
    /// Delete a post
    Delete {
        id: String,
    },
    /// Find the next available posting slot (optionally for one integration)
    FindSlot {
        #[arg(long)]
        integration: Option<String>,
    },
}

// ─── Media Actions ──────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum MediaAction {
    /// Upload a media file (image/video) for post attachments
    Upload {
        /// File path to upload
        path: String,
        /// Alt text for accessibility
        #[arg(long)]
        alt: Option<String>,
    },
    /// Batch upload multiple media files from local paths
    UploadBatch {
        /// File paths to upload (space-separated)
        paths: Vec<String>,
    },
    /// List uploaded media files
    List {
        /// Number of items to show
        #[arg(long, default_value_t = 20)]
        limit: u32,
        /// Search filter
        #[arg(long)]
        search: Option<String>,
    },
    /// Download media from a URL to local file
    Download {
        /// URL to download
        url: String,
        /// Output file path
        #[arg(long, default_value = "./download")]
        output: String,
    },
}

// ─── Config Management ─────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Set a configuration value in ~/.social-forge/.env
    Set {
        /// Environment variable name (e.g. BLUESKY_HANDLE, GITHUB_TOKEN)
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Environment variable name
        key: String,
    },
    /// List all configured values (redacts secrets)
    List,
}

// ─── X (Twitter) ────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum XAction {
    /// Post a tweet
    Post {
        /// Tweet text content
        text: String,
    },

    /// View home timeline
    Timeline {
        /// Number of tweets to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Search tweets
    Search {
        /// Search query
        query: String,
    },

    /// Like a tweet
    Like {
        /// Tweet ID to like
        tweet_id: String,
    },

    /// Retweet a tweet
    Retweet {
        /// Tweet ID to retweet
        tweet_id: String,
    },

    /// Delete a tweet
    Delete {
        /// Tweet ID to delete
        tweet_id: String,
    },

    /// Bookmark a tweet
    Bookmark {
        /// Tweet ID to bookmark
        tweet_id: String,
    },

    /// Get a user's profile
    User {
        /// Username to look up
        username: String,
    },

    /// Reply to a tweet
    Reply {
        /// Tweet ID to reply to
        tweet_id: String,
        /// Reply text
        text: String,
    },

    /// Send a direct message
    Dm {
        /// Recipient user ID
        recipient: String,
        /// Message text
        text: String,
    },

    /// List DM conversations
    DmList {
        /// Number of conversations to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Get messages in a DM conversation
    DmMessages {
        /// Conversation ID
        conversation_id: String,
        /// Number of messages to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },
}

// ─── Reddit ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum RedditAction {
    /// Browse a subreddit
    Browse {
        /// Subreddit name (without r/)
        subreddit: String,

        /// Sort order (hot, new, top, rising)
        #[arg(long, default_value = "hot")]
        sort: String,

        /// Number of posts to fetch
        #[arg(long, default_value_t = 25)]
        limit: u32,
    },

    /// Search Reddit
    Search {
        /// Search query
        query: String,

        /// Limit search to a subreddit
        #[arg(long)]
        subreddit: Option<String>,

        /// Sort order (relevance, hot, top, new, comments)
        #[arg(long, default_value = "relevance")]
        sort: String,
    },

    /// Submit a post to a subreddit
    Post {
        /// Post title
        #[arg(long)]
        title: String,

        /// Self-text body
        #[arg(long)]
        text: Option<String>,

        /// Link URL (for link posts)
        #[arg(long)]
        url: Option<String>,

        /// Target subreddit (omit to pick interactively)
        #[arg(long)]
        target: Option<String>,

        /// Multiple posting targets (comma-separated). Creates one post per target.
        #[arg(long)]
        targets: Option<String>,
    },

    /// Comment on a post or reply to a comment
    Comment {
        /// Parent thing ID (t3_ for post, t1_ for comment)
        thing_id: String,

        /// Comment text
        text: String,
    },

    /// Vote on a post or comment
    Vote {
        /// Thing ID to vote on
        thing_id: String,

        /// Vote direction (up, down, none)
        direction: String,
    },

    /// Save a post or comment
    Save {
        /// Thing ID to save
        thing_id: String,
    },

    /// Unsave a post or comment
    Unsave {
        /// Thing ID to unsave
        thing_id: String,
    },

    /// Delete a post or comment
    Delete {
        /// Thing ID to delete
        thing_id: String,
    },

    /// Get a user's profile
    User {
        /// Username to look up
        username: String,
    },

    /// View inbox messages
    Inbox {
        /// Inbox folder (inbox, unread, sent)
        #[arg(long, default_value = "inbox")]
        folder: String,
    },

    /// Moderator actions
    Mod {
        #[command(subcommand)]
        action: RedditModAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum RedditModAction {
    /// Remove a post or comment
    Remove {
        /// Thing ID to remove
        thing_id: String,

        /// Mark as spam
        #[arg(long)]
        spam: bool,
    },

    /// Approve a post or comment
    Approve {
        /// Thing ID to approve
        thing_id: String,
    },

    /// Lock a post or comment
    Lock {
        /// Thing ID to lock
        thing_id: String,
    },

    /// Unlock a post or comment
    Unlock {
        /// Thing ID to unlock
        thing_id: String,
    },
}

// ─── LinkedIn ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum LinkedinAction {
    /// View your LinkedIn profile
    Profile,

    /// List your recent posts
    Posts {
        /// Number of posts to fetch
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },

    /// Create a new post
    Post {
        /// Post text content
        text: String,
    },

    /// Delete a post
    Delete {
        /// Post URN to delete
        post_urn: String,
    },

    /// View reactions on a post
    Reactions {
        /// Post URN
        post_urn: String,
    },

    /// View profile analytics
    Analytics,

    /// Reply to a LinkedIn comment
    Reply {
        /// Comment URN
        comment_id: String,
        /// Reply content
        content: String,
    },

    /// Send a direct message on LinkedIn
    Dm {
        /// Recipient URN
        recipient: String,
        /// Message content
        content: String,
    },

    /// List LinkedIn message conversations
    DmList {
        /// Number of conversations to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Get messages in a LinkedIn conversation
    DmMessages {
        /// Conversation ID
        conversation_id: String,
        /// Number of messages to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },
}

// ─── LinkedIn Page ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum LinkedinPageAction {
    /// List managed company pages
    List,

    /// Post on behalf of a company page
    Post {
        /// Page ID
        page_id: String,

        /// Post text content
        text: String,
    },

    /// View page analytics
    Analytics {
        /// Page ID
        page_id: String,
    },

    /// View page followers
    Followers {
        /// Page ID
        page_id: String,
    },
    /// Get page details
    Page {
        /// Page ID
        page_id: String,
    },
    /// Get page feed
    Feed {
        /// Page ID
        page_id: String,
        /// Number of posts
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Create a comment on a post
    CreateComment {
        /// Post URN
        post_urn: String,
        /// Page URN
        page_urn: String,
        /// Comment text
        text: String,
    },
    /// Delete a post
    Delete {
        /// Post URN
        post_urn: String,
    },
    /// Get reactions on a post
    Reactions {
        /// Post URN
        post_urn: String,
    },
    /// Get shares on a post
    Shares {
        /// Post URN
        post_urn: String,
    },
    /// Get analytics for a specific post
    PostAnalytics {
        /// Post URN
        post_urn: String,
    },
}

// ─── Facebook ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum FacebookAction {
    /// List posts on a page
    Posts {
        /// Facebook Page ID
        page_id: String,
    },

    /// View page insights
    Insights {
        /// Facebook Page ID
        page_id: String,

        /// Comma-separated metrics (e.g. page_impressions,page_engaged_users,page_fans).
        /// Default: page_impressions,page_engaged_users,page_fans
        #[arg(long, default_value = "page_impressions,page_engaged_users,page_fans")]
        metric: String,
    },    /// Comment on a post
    Comment {
        /// Post ID to comment on
        post_id: String,
        /// Comment text
        text: String,
    },
    /// Create a post on a page
    Create {
        /// Facebook Page ID
        page_id: String,
        /// Post message text
        message: String,
    },
    /// Create a photo post
    Photo {
        /// Facebook Page ID
        page_id: String,
        /// Photo URL
        url: String,
        /// Optional caption
        #[arg(long)]
        caption: Option<String>,
    },
    /// Create a video post
    Video {
        /// Facebook Page ID
        page_id: String,
        /// Video file URL
        url: String,
        /// Optional title
        #[arg(long)]
        title: Option<String>,
    },
    /// Delete a post
    Delete {
        /// Post ID to delete
        post_id: String,
    },
    /// React to a post
    React {
        /// Post ID
        post_id: String,
        /// Reaction type (LIKE, LOVE, HAHA, WOW, SAD, ANGRY)
        reaction_type: String,
    },
    /// List conversations (inbox)
    Conversations {
        /// Facebook Page ID
        page_id: String,
    },
    /// Send a message to a conversation
    Send {
        /// Facebook Page ID
        page_id: String,
        /// Conversation ID
        conversation_id: String,
        /// Message text
        text: String,
    },
    /// Search for public pages
    Pages {
        /// Search query
        query: String,
    },
    /// Get page insights
    PageInsights {
        /// Facebook Page ID
        page_id: String,
        /// Comma-separated metrics
        #[arg(long, default_value = "page_impressions,page_engaged_users")]
        metric: String,
    },
    /// Get page albums
    Albums {
        /// Facebook Page ID
        page_id: String,
    },
}

// ─── Instagram ──────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum InstagramAction {
    /// List recent posts
    Posts {
        /// Instagram account ID
        account_id: String,
    },

    /// View account insights
    Insights {
        /// Instagram account ID
        account_id: String,

        /// Comma-separated metrics (e.g. reach,follower_count,profile_views).
        /// Default: reach,follower_count
        #[arg(long, default_value = "reach,follower_count")]
        metric: String,
    },

    /// Comment on a media post
    Comment {
        /// Media ID to comment on
        media_id: String,

        /// Comment text
        text: String,
    },

    /// Send a direct message on Instagram
    Dm {
        /// Account ID
        account_id: String,
        /// Recipient ID
        recipient: String,
        /// Message content
        content: String,
    },

    /// List Instagram message conversations
    DmList {
        /// Account ID
        account_id: String,
        /// Number of conversations to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },

    /// Get messages in an Instagram conversation
    DmMessages {
        /// Account ID
        account_id: String,
        /// Conversation ID
        conversation_id: String,
        /// Number of messages to fetch
        #[arg(long, default_value_t = 20)]
        count: u32,
    },
    /// Get media details
    MediaDetail {
        /// Instagram account ID
        account_id: String,
        /// Media ID
        media_id: String,
    },
    /// Search hashtags
    Hashtag {
        /// Instagram account ID
        account_id: String,
        /// Hashtag query
        query: String,
    },
    /// Get reels
    Reels {
        /// Instagram account ID
        account_id: String,
    },
    /// Get stories
    Stories {
        /// Instagram account ID
        account_id: String,
    },
    /// Get followers
    Followers {
        /// Instagram account ID
        account_id: String,
    },
    /// Get audience insights
    InsightsAudience {
        /// Instagram account ID
        account_id: String,
    },
    /// Get mentions
    Mentions {
        /// Instagram account ID
        account_id: String,
    },
    /// Poll container status
    PollContainer {
        /// Instagram account ID
        account_id: String,
        /// Creation ID
        creation_id: String,
    },
}

// ─── YouTube ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum YoutubeAction {
    /// Reply to a YouTube comment
    Reply {
        /// Comment ID
        comment_id: String,
        /// Reply content
        content: String,
    },

    /// Search YouTube videos
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },

    /// Get video details
    Video {
        /// Video ID
        video_id: String,
    },
    /// List playlists
    Playlists {
        /// Channel ID
        channel_id: String,
        /// Max results
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Get channel stats
    Stats {
        /// Channel ID
        channel_id: String,
    },
    /// Get analytics
    Analytics {
        /// Channel ID
        channel_id: String,
        /// Metrics (comma-separated)
        #[arg(long, default_value = "views,likes,comments")]
        metrics: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start_date: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end_date: String,
    },
    /// Get subscriptions
    Subscriptions {
        /// Channel ID
        channel_id: String,
        /// Max results
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Find creators by topic
    Creators {
        /// Search query
        query: String,
        /// Max results
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
}

// ─── Bluesky ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum BlueskyAction {
    /// Reply to a Bluesky post
    Reply {
        /// Post URI
        post_uri: String,
        /// Reply content
        content: String,
    },

    /// Get user profile
    Profile {
        /// Handle or DID
        handle: String,
    },

    /// Get timeline
    Timeline {
        /// Number of posts to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },

    /// Search posts
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Create a new post
    Post {
        /// Post text content
        text: String,
        /// Image URLs (comma-separated)
        #[arg(long)]
        images: Option<String>,
    },
    /// Get the feed
    Feed {
        /// Feed type (popular, etc.)
        #[arg(long)]
        feed_type: Option<String>,
        /// Number of posts
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

// ─── Mastodon ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum MastodonAction {
    /// Reply to a Mastodon status
    Reply {
        /// Status ID
        status_id: String,
        /// Reply content
        content: String,
    },

    /// Get user info
    Whoami,

    /// Get timeline
    Timeline {
        /// Timeline type (home, local, public)
        #[arg(long, default_value = "home")]
        kind: String,
        /// Number of posts to fetch
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },

    /// Search
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Create a new post (toot)
    Post {
        /// Post content (max 500 chars)
        text: String,
        /// Visibility (public, unlisted, private, direct)
        #[arg(long, default_value = "public")]
        visibility: String,
    },
    /// Get a specific post by ID
    Get {
        /// Status ID
        status_id: String,
    },
}

// ─── TikTok

#[derive(Subcommand, Debug)]
pub enum TiktokAction {
    /// Get TikTok profile
    Profile,
    /// Post a video
    Post {
        /// Video caption text
        text: String,
        /// Video URL to upload
        #[arg(long)]
        video_url: Option<String>,
    },
    /// List recent videos
    Videos {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

// ─── Threads

#[derive(Subcommand, Debug)]
pub enum ThreadsAction {
    /// Get your Threads profile
    Profile {
        /// Threads account ID
        threads_id: String,
    },
    /// List your threads (posts)
    List {
        /// Threads account ID
        threads_id: String,
        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Create a new thread
    Post {
        /// Threads account ID
        threads_id: String,
        /// Post text
        text: String,
        /// Optional image/video URL
        #[arg(long)]
        media_url: Option<String>,
    },
    /// Reply to a thread
    Reply {
        /// Threads account ID
        threads_id: String,
        /// Media ID to reply to
        media_id: String,
        /// Reply text
        text: String,
    },
    /// Delete a thread
    Delete {
        /// Threads account ID
        threads_id: String,
        /// Media ID to delete
        media_id: String,
    },
    /// Get insights/analytics
    Insights {
        /// Threads account ID
        threads_id: String,
        /// Metric (e.g. impression_count)
        metric: String,
        /// Period (day, etc.)
        #[arg(long)]
        period: Option<String>,
    },
}

// ─── Discord

#[derive(Subcommand, Debug)]
pub enum DiscordAction {
    /// List channels in a guild
    Channels {
        /// Guild ID
        guild_id: String,
    },
    /// Get messages from a channel
    Messages {
        /// Channel ID
        channel_id: String,
        /// Max messages
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Send a message to a channel
    Send {
        /// Channel ID
        channel_id: String,
        /// Message text
        text: String,
    },
    /// Get server/guild info
    Server {
        /// Guild ID
        guild_id: String,
    },
    /// Create a forum post
    Forum {
        /// Channel ID (forum channel)
        channel_id: String,
        /// Post title
        title: String,
        /// Post content
        content: String,
    },
}

// ─── Slack

#[derive(Subcommand, Debug)]
pub enum SlackAction {
    /// List channels
    Channels,
    /// Get channel history
    History {
        /// Channel ID
        channel_id: String,
        /// Max messages
        #[arg(long, default_value_t = 50)]
        limit: i32,
    },
    /// Send a message
    Send {
        /// Channel ID
        channel_id: String,
        /// Message text
        text: String,
    },
    /// List users
    Users,
}

// ─── Telegram Bot

#[derive(Subcommand, Debug)]
pub enum TelegramBotAction {
    /// Send a message
    Send {
        /// Bot token index (0-based)
        #[arg(long, default_value_t = 0)]
        bot_index: usize,
        /// Chat ID
        chat_id: String,
        /// Message text
        text: String,
    },
    /// Send a photo
    Photo {
        #[arg(long, default_value_t = 0)]
        bot_index: usize,
        chat_id: String,
        /// Photo URL
        url: String,
        #[arg(long)]
        caption: Option<String>,
    },
    /// Send a document
    Document {
        #[arg(long, default_value_t = 0)]
        bot_index: usize,
        chat_id: String,
        /// Document URL or path
        path: String,
        #[arg(long)]
        caption: Option<String>,
    },
    /// Get chat info
    Chat {
        #[arg(long, default_value_t = 0)]
        bot_index: usize,
        chat_id: String,
    },
    /// Get updates
    Updates {
        #[arg(long, default_value_t = 0)]
        bot_index: usize,
    },
}

// ─── Telegram User

#[derive(Subcommand, Debug)]
pub enum TelegramUserAction {
    /// Send a message
    Send {
        /// Peer (username, phone, or chat ID)
        peer: String,
        /// Message text
        text: String,
    },
    /// List dialogs/conversations
    Dialogs {
        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List contacts
    Contacts,
    /// Search messages
    Search {
        /// Search query
        query: String,
    },
}

// ─── WhatsApp

#[derive(Subcommand, Debug)]
pub enum WhatsappAction {
    /// Send a text message
    Send {
        /// Recipient phone number
        to: String,
        /// Message text
        text: String,
    },
    /// List chats
    Chats {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List contacts
    Contacts {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List groups
    Groups,
    /// Create a group
    CreateGroup {
        /// Group name
        name: String,
        /// Participant phone numbers
        participants: Vec<String>,
    },
    /// Get invite link for a group
    InviteLink {
        /// Group JID
        group_jid: String,
    },
}

// ─── Pinterest

#[derive(Subcommand, Debug)]
pub enum PinterestAction {
    /// Get user account info
    Profile {
        /// Board ID (used for token resolution)
        board_id: String,
    },
    /// Get board details
    Board {
        board_id: String,
    },
    /// Get pins on a board
    Pins {
        board_id: String,
        #[arg(long, default_value_t = 25)]
        limit: u32,
    },
    /// Get a single pin
    Pin {
        board_id: String,
        pin_id: String,
    },
    /// Search pins
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get board analytics
    BoardAnalytics {
        board_id: String,
        #[arg(long)]
        start_date: String,
        #[arg(long)]
        end_date: String,
    },
    /// Get pin analytics
    PinAnalytics {
        board_id: String,
        pin_id: String,
        #[arg(long)]
        start_date: String,
        #[arg(long)]
        end_date: String,
    },
}

// ─── GitHub

#[derive(Subcommand, Debug)]
pub enum GithubAction {
    /// Get authenticated user info
    Me,
    /// Get my repos
    MyRepos {
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// Get a user's profile
    User {
        login: String,
    },
    /// List repos for a user
    Repos {
        username: String,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// List issues
    Issues {
        owner: String,
        repo: String,
        #[arg(long)]
        state_filter: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// List pull requests
    Prs {
        owner: String,
        repo: String,
        #[arg(long)]
        state_filter: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// Create an issue
    CreateIssue {
        owner: String,
        repo: String,
        title: String,
        #[arg(long)]
        body: Option<String>,
    },
    /// Close an issue
    CloseIssue {
        owner: String,
        repo: String,
        number: u32,
    },
    /// List commits
    Commits {
        owner: String,
        repo: String,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// List branches
    Branches {
        owner: String,
        repo: String,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
    /// Search repos
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// List releases
    Releases {
        owner: String,
        repo: String,
        #[arg(long, default_value_t = 30)]
        limit: u32,
    },
}

// ─── WordPress

#[derive(Subcommand, Debug)]
pub enum WordpressAction {
    /// Create a post
    Post {
        title: String,
        content: String,
        #[arg(long)]
        status: Option<String>,
    },
    /// List posts
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Get a post by ID
    Get {
        id: i32,
    },
    /// List categories
    Categories,
}

// ─── Hashnode

#[derive(Subcommand, Debug)]
pub enum HashnodeAction {
    /// Create a post
    Post {
        publication_id: String,
        title: String,
        content: String,
    },
    /// List posts
    List {
        publication_id: String,
        #[arg(long)]
        page: Option<i32>,
    },
    /// Get a post
    Get {
        post_id: String,
    },
}

// ─── Medium

#[derive(Subcommand, Debug)]
pub enum MediumBlogAction {
    /// Create a post
    Post {
        #[arg(long)]
        title: Option<String>,
        content: String,
        #[arg(long)]
        tags: Option<Vec<String>>,
    },
    /// List posts
    List,
    /// Get a post
    Get {
        id: String,
    },
}

// ─── Dev.to

#[derive(Subcommand, Debug)]
pub enum DevtoAction {
    /// Create an article
    Post {
        #[arg(long)]
        title: Option<String>,
        content: String,
        #[arg(long)]
        tags: Option<Vec<String>>,
        /// Publish immediately (default: save as draft)
        #[arg(long, default_value_t = false)]
        publish: bool,
    },
    /// List articles
    List,
    /// Get an article
    Get {
        id: String,
    },
}

// ─── Skool

#[derive(Subcommand, Debug)]
pub enum SkoolAction {
    /// Publish a post
    Post {
        group_id: String,
        title: String,
        content: String,
        #[arg(long)]
        label: Option<String>,
    },
    /// Get community info
    Info {
        community_slug: String,
    },
    /// List posts
    Posts {
        community_slug: String,
        #[arg(long)]
        page: Option<u32>,
        #[arg(long)]
        sort: Option<String>,
    },
    /// Create a comment
    Comment {
        post_id: String,
        group_id: String,
        content: String,
    },
}

// ─── Google (YouTube)

#[derive(Subcommand, Debug)]
pub enum GoogleAction {
    /// Search YouTube videos
    YoutubeSearch {
        channel_id: String,
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Get video details
    Video {
        channel_id: String,
        video_id: String,
    },
    /// List playlists
    Playlists {
        channel_id: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Get channel stats
    ChannelStats {
        channel_id: String,
    },
}

// ─── Google Drive

#[derive(Subcommand, Debug)]
pub enum DriveAction {
    /// List files
    Files {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get a file
    File {
        file_id: String,
    },
    /// Search files
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List folders
    Folders {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Get file metadata
    Metadata {
        file_id: String,
    },
    /// Export a file
    Export {
        file_id: String,
        mime_type: String,
    },
}

// ─── Google Calendar

#[derive(Subcommand, Debug)]
pub enum GcalAction {
    /// List calendars
    Calendars,
    /// List events
    Events {
        #[arg(long)]
        calendar_id: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get a specific event
    Event {
        #[arg(long)]
        calendar_id: Option<String>,
        event_id: String,
    },
    /// Create an event
    Create {
        #[arg(long)]
        calendar_id: Option<String>,
        title: String,
        start: String,
        end: String,
        #[arg(long)]
        description: Option<String>,
    },
    /// Update an event
    Update {
        #[arg(long)]
        calendar_id: Option<String>,
        event_id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        start: Option<String>,
        #[arg(long)]
        end: Option<String>,
    },
    /// Delete an event
    Delete {
        #[arg(long)]
        calendar_id: Option<String>,
        event_id: String,
    },
}

// ─── Gmail

#[derive(Subcommand, Debug)]
pub enum GmailAction {
    /// Get Gmail profile
    Profile,
    /// List messages
    Messages {
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        query: Option<String>,
    },
    /// Get a message
    Message {
        id: String,
    },
    /// Send an email
    Send {
        to: String,
        subject: String,
        body: String,
    },
    /// List labels
    Labels,
    /// Get a thread
    Thread {
        id: String,
    },
    /// Search messages
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

// ─── Webhooks

#[derive(Subcommand, Debug)]
pub enum WebhooksAction {
    /// List webhooks
    List,
    /// Create a webhook
    Create {
        /// Webhook URL
        url: String,
        /// Webhook name
        #[arg(long, default_value = "webhook")]
        name: String,
    },
    /// Delete a webhook
    Delete {
        /// Webhook ID
        id: String,
    },
    /// Get a webhook by ID
    Get {
        /// Webhook ID
        id: String,
    },
    /// Update a webhook
    Update {
        /// Webhook ID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New URL
        #[arg(long)]
        url: Option<String>,
        /// Enable/disable
        #[arg(long)]
        active: Option<bool>,
    },
    /// Test a webhook
    Test {
        /// Webhook ID
        id: String,
    },
}

// ─── Notifications

#[derive(Subcommand, Debug)]
pub enum NotificationsAction {
    /// List notifications
    List {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Mark a notification as read
    Read {
        id: String,
    },
    /// Mark all notifications as read
    ReadAll,
    /// Create a notification
    Create {
        /// Title
        title: String,
        /// Body
        body: String,
    },
}

// ─── Tags

#[derive(Subcommand, Debug)]
pub enum TagsAction {
    /// List tags
    List,
    /// Create a tag
    Create {
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    /// Delete a tag
    Delete {
        id: String,
    },

    /// Get a tag by ID
    Get {
        /// Tag ID
        id: String,
    },
    /// Update a tag
    Update {
        /// Tag ID
        id: String,
        /// New name
        #[arg(long)]
        name: Option<String>,
        /// New color
        #[arg(long)]
        color: Option<String>,
    },
}

// ─── Analytics

#[derive(Subcommand, Debug)]
pub enum AnalyticsAction {
    /// Get analytics for a provider
    Get {
        /// Provider name (instagram, facebook, etc.)
        provider: String,
        /// Number of days
        #[arg(long, default_value_t = 7)]
        days: i32,
    },
    /// Get analytics for a specific post
    Post {
        /// Post ID (UUID)
        post_id: String,
    },
}
