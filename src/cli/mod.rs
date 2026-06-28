pub mod run;
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
    },

    /// Comment on a post
    Comment {
        /// Post ID to comment on
        post_id: String,

        /// Comment text
        text: String,
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
}
