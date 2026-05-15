// ─── Configuration ─────────────────────────────────────────────
// Loads from environment variables (12-factor app style).
// Uses dotenvy to load .env for local dev.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub app_url: String,
    pub frontend_url: String,

    // Social provider credentials
    pub x_client_id: Option<String>,
    pub x_client_secret: Option<String>,
    pub x_auth_token: Option<String>,
    pub x_ct0: Option<String>,
    pub linkedin_client_id: Option<String>,
    pub linkedin_client_secret: Option<String>,
    pub bluesky_handle: Option<String>,
    pub bluesky_app_password: Option<String>,
    pub facebook_client_id: Option<String>,
    pub facebook_client_secret: Option<String>,
    pub instagram_client_id: Option<String>,
    pub instagram_client_secret: Option<String>,
    pub threads_client_id: Option<String>,
    pub threads_client_secret: Option<String>,
    pub youtube_client_id: Option<String>,
    pub youtube_client_secret: Option<String>,
    pub reddit_client_id: Option<String>,
    pub reddit_client_secret: Option<String>,
    pub reddit_username: Option<String>,
    pub reddit_password: Option<String>,
    pub reddit_access_token: Option<String>,
    pub reddit_refresh_token: Option<String>,
    pub discord_client_id: Option<String>,
    pub discord_client_secret: Option<String>,
    pub discord_bot_token: Option<String>,
    pub telegram_bot_tokens: Option<String>,
    pub telegram_api_id: Option<String>,
    pub telegram_api_hash: Option<String>,
    pub telegram_session_dir: Option<String>,
    pub pinterest_client_id: Option<String>,
    pub pinterest_client_secret: Option<String>,
    pub instagram_app_id: Option<String>,
    pub instagram_app_secret: Option<String>,
    pub whatsapp_store_dir: Option<String>,

    // Slack
    pub slack_client_id: Option<String>,
    pub slack_client_secret: Option<String>,

    // TikTok
    pub tiktok_client_id: Option<String>,
    pub tiktok_client_secret: Option<String>,

    // Mastodon
    pub mastodon_client_id: Option<String>,
    pub mastodon_client_secret: Option<String>,
    pub mastodon_instance_url: Option<String>,

    // Medium (API key-based)
    pub medium_access_token: Option<String>,

    // Dev.to (API key-based)
    pub devto_api_key: Option<String>,

    // Hashnode (API key-based)
    pub hashnode_api_key: Option<String>,

    // Token encryption at rest
    pub token_encryption_key: Option<String>,

    // Media storage
    pub media_dir: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env("DATABASE_URL")?,
            jwt_secret: env("JWT_SECRET")?,
            app_url: env("APP_URL")?,
            frontend_url: env("FRONTEND_URL")?,
            x_client_id: opt("X_CLIENT_ID"),
            x_client_secret: opt("X_CLIENT_SECRET"),
            x_auth_token: opt("X_AUTH_TOKEN"),
            x_ct0: opt("X_CT0"),
            linkedin_client_id: opt("LINKEDIN_CLIENT_ID"),
            linkedin_client_secret: opt("LINKEDIN_CLIENT_SECRET"),
            bluesky_handle: opt("BLUESKY_HANDLE"),
            bluesky_app_password: opt("BLUESKY_APP_PASSWORD"),
            facebook_client_id: opt("FACEBOOK_CLIENT_ID"),
            facebook_client_secret: opt("FACEBOOK_CLIENT_SECRET"),
            instagram_client_id: opt("INSTAGRAM_CLIENT_ID"),
            instagram_client_secret: opt("INSTAGRAM_CLIENT_SECRET"),
            threads_client_id: opt("THREADS_CLIENT_ID"),
            threads_client_secret: opt("THREADS_CLIENT_SECRET"),
            youtube_client_id: opt("YOUTUBE_CLIENT_ID"),
            youtube_client_secret: opt("YOUTUBE_CLIENT_SECRET"),
            reddit_client_id: opt("REDDIT_CLIENT_ID"),
            reddit_client_secret: opt("REDDIT_CLIENT_SECRET"),
            reddit_username: opt("REDDIT_USERNAME"),
            reddit_password: opt("REDDIT_PASSWORD"),
            reddit_access_token: opt("REDDIT_ACCESS_TOKEN"),
            reddit_refresh_token: opt("REDDIT_REFRESH_TOKEN"),
            discord_client_id: opt("DISCORD_CLIENT_ID"),
            discord_client_secret: opt("DISCORD_CLIENT_SECRET"),
            discord_bot_token: opt("DISCORD_BOT_TOKEN"),
            telegram_bot_tokens: opt("TELEGRAM_BOT_TOKENS"),
            telegram_api_id: opt("TELEGRAM_API_ID"),
            telegram_api_hash: opt("TELEGRAM_API_HASH"),
            telegram_session_dir: opt("TELEGRAM_SESSION_DIR"),
            pinterest_client_id: opt("PINTEREST_CLIENT_ID"),
            pinterest_client_secret: opt("PINTEREST_CLIENT_SECRET"),
            instagram_app_id: opt("INSTAGRAM_APP_ID"),
            instagram_app_secret: opt("INSTAGRAM_APP_SECRET"),
            whatsapp_store_dir: opt("WHATSAPP_STORE_DIR"),
            slack_client_id: opt("SLACK_CLIENT_ID"),
            slack_client_secret: opt("SLACK_CLIENT_SECRET"),
            tiktok_client_id: opt("TIKTOK_CLIENT_ID"),
            tiktok_client_secret: opt("TIKTOK_CLIENT_SECRET"),
            mastodon_client_id: opt("MASTODON_CLIENT_ID"),
            mastodon_client_secret: opt("MASTODON_CLIENT_SECRET"),
            mastodon_instance_url: opt("MASTODON_INSTANCE_URL"),
            medium_access_token: opt("MEDIUM_ACCESS_TOKEN"),
            devto_api_key: opt("DEVTO_API_KEY"),
            hashnode_api_key: opt("HASHNODE_API_KEY"),
            token_encryption_key: opt("TOKEN_ENCRYPTION_KEY"),
            media_dir: opt("MEDIA_DIR").unwrap_or_else(|| "./uploads".into()),
        })
    }

    /// Returns (client_id, client_secret) for a given provider
    pub fn provider_credentials(&self, provider: &str) -> Option<(String, String)> {
        match provider {
            "x" => Some((self.x_client_id.clone()?, self.x_client_secret.clone()?)),
            "linkedin" => Some((
                self.linkedin_client_id.clone()?,
                self.linkedin_client_secret.clone()?,
            )),
            "linkedin-page" => Some((
                self.linkedin_client_id.clone()?,
                self.linkedin_client_secret.clone()?,
            )),
            "bluesky" => Some((
                self.bluesky_handle.clone()?,
                self.bluesky_app_password.clone()?,
            )),
            "facebook" => Some((
                self.facebook_client_id.clone()?,
                self.facebook_client_secret.clone()?,
            )),
            "instagram" => Some((
                self.instagram_client_id.clone()?,
                self.instagram_client_secret.clone()?,
            )),
            "instagram-standalone" => Some((
                self.instagram_app_id.clone()?,
                self.instagram_app_secret.clone()?,
            )),
            "threads" => Some((
                self.threads_client_id.clone()?,
                self.threads_client_secret.clone()?,
            )),
            "youtube" => Some((
                self.youtube_client_id.clone()?,
                self.youtube_client_secret.clone()?,
            )),
            "reddit" => {
                let id = self.reddit_client_id.clone()?;
                let secret = self.reddit_client_secret.clone()?;
                Some((id, secret))
            }
            "discord" => Some((
                self.discord_client_id.clone()?,
                self.discord_client_secret.clone()?,
            )),
            "telegram-bot" => Some(("bot".into(), self.telegram_bot_tokens.clone()?)),
            "telegram-user" => Some(("user".into(), "daemon".into())),
            "pinterest" => Some((
                self.pinterest_client_id.clone()?,
                self.pinterest_client_secret.clone()?,
            )),
            "skool" => Some(("skool".into(), "chrome_extension".into())),
            "slack" => Some((self.slack_client_id.clone()?, self.slack_client_secret.clone()?)),
            "mastodon" => Some((self.mastodon_client_id.clone()?, self.mastodon_client_secret.clone()?)),
            "tiktok" => Some((self.tiktok_client_id.clone()?, self.tiktok_client_secret.clone()?)),
            "medium" => Some(("api-key".into(), self.medium_access_token.clone()?)),
            "devto" => Some(("api-key".into(), self.devto_api_key.clone()?)),
            "hashnode" => Some(("api-key".into(), self.hashnode_api_key.clone()?)),
            _ => None,
        }
    }

    /// Returns Reddit username for password grant auth
    pub fn reddit_username(&self) -> Option<String> {
        self.reddit_username.clone()
    }

    /// Returns Reddit password for password grant auth
    pub fn reddit_password(&self) -> Option<String> {
        self.reddit_password.clone()
    }

    /// Returns Reddit access token (pre-authorized via auth code flow)
    pub fn reddit_access_token(&self) -> Option<String> {
        self.reddit_access_token.clone()
    }

    /// Returns Reddit refresh token (obtained via auth code flow)
    pub fn reddit_refresh_token(&self) -> Option<String> {
        self.reddit_refresh_token.clone()
    }
}

fn env(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("Missing required env var: {key}"))
}

fn opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}
