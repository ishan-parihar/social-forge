// ─── Configuration ─────────────────────────────────────────────
// Loads from environment variables (12-factor app style).
// Priority: env vars > .env (cwd) > ~/.social-forge/.env (user config)

use serde::Deserialize;

/// Returns the user config directory: ~/.social-forge/
pub fn config_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/tmp".into());
    std::path::PathBuf::from(home).join(".social-forge")
}

/// Load .env files in priority order (later values don't override earlier ones)
pub fn load_dotenv() {
    // 1. CWD .env (highest priority for local dev)
    dotenvy::dotenv().ok();
    // 2. ~/.social-forge/.env (user-level config)
    let user_env = config_dir().join(".env");
    if user_env.exists() {
        dotenvy::from_path(&user_env).ok();
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    /// HMAC secret for signing session cookies AND OAuth state tokens.
    /// If unset, derived from `app_password` at startup (see `Config::from_env`).
    pub jwt_secret: String,
    /// Single password gate for the WebUI. If unset, a random one is
    /// generated and persisted to `~/.social-forge/.env` on first run.
    pub app_password: String,
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
    pub threads_app_id: Option<String>,
    pub threads_app_secret: Option<String>,
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

    // GitHub (PAT-based)
    pub github_token: Option<String>,

    // Twitch (provider deleted — field kept for Config-struct compat)
    pub twitch_client_id: Option<String>,
    pub twitch_client_secret: Option<String>,

    // VK
    pub vk_client_id: Option<String>,
    pub vk_client_secret: Option<String>,

    // Whop
    pub whop_client_id: Option<String>,
    pub whop_client_secret: Option<String>,

    // MeWe (provider deleted — field kept for Config-struct compat)
    pub mewe_client_id: Option<String>,
    pub mewe_client_secret: Option<String>,

    // Moltbook (provider deleted — field kept for Config-struct compat)
    pub moltbook_client_id: Option<String>,
    pub moltbook_client_secret: Option<String>,

    // Kick
    pub kick_client_id: Option<String>,
    pub kick_client_secret: Option<String>,

    // Neynar (Farcaster) API key
    pub neynar_api_key: Option<String>,

    // Nostr (provider deleted — field kept for Config-struct compat)
    pub nostr_private_key: Option<String>,

    // Token encryption at rest
    pub token_encryption_key: Option<String>,

    // Media storage
    pub media_dir: String,

    // Stripe billing
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub stripe_price_free: Option<String>,
    pub stripe_price_pro_monthly: Option<String>,
    pub stripe_price_pro_annual: Option<String>,
    pub stripe_price_business_monthly: Option<String>,
    pub stripe_price_business_annual: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let app_url = opt("APP_URL").unwrap_or_else(|| "https://localhost:6543".into());
        let frontend_url = opt("FRONTEND_URL").unwrap_or_else(|| app_url.clone());

        // ── Single-user password gate ────────────────────────────
        // Priority: APP_PASSWORD env var > persisted ~/.social-forge/.env
        // > generate-and-persist a fresh random one (first run).
        let app_password = match opt("APP_PASSWORD") {
            Some(p) if !p.is_empty() => p,
            _ => {
                // Try to load a previously-persisted password.
                let user_env = config_dir().join(".env");
                let persisted = if user_env.exists() {
                    std::fs::read_to_string(&user_env)
                        .ok()
                        .and_then(|s| {
                            s.lines().find_map(|l| {
                                l.strip_prefix("APP_PASSWORD=").map(|v| v.trim().to_string())
                            })
                        })
                        .filter(|s| !s.is_empty())
                } else {
                    None
                };

                if let Some(p) = persisted {
                    p
                } else {
                    let generated = generate_random_password(32);
                    persist_app_password(&generated);
                    // Do NOT log the password value — it would leak into
                    // container logs, journald, shell history, etc. Point
                    // the user at the persisted file instead.
                    let env_path = config_dir().join(".env");
                    tracing::warn!(
                        "┌──────────────────────────────────────────────────────────┐"
                    );
                    tracing::warn!("│ No APP_PASSWORD set. Generated a random one and persisted │");
                    tracing::warn!("│ it to {}|",
                        format!("{:<48}", env_path.display().to_string()).trim_end()
                    );
                    tracing::warn!("│                                                          │");
                    tracing::warn!("│ To view it:  cat ~/.social-forge/.env                    │");
                    tracing::warn!("│ To change it: edit that file or set APP_PASSWORD in env  │");
                    tracing::warn!(
                        "└──────────────────────────────────────────────────────────┘"
                    );
                    generated
                }
            }
        };

        // Derive the session/JWT secret from the password if not explicitly set.
        // This way one env var (`APP_PASSWORD`) is sufficient to secure both
        // the cookie signing and the OAuth state tokens.
        let jwt_secret = match opt("JWT_SECRET") {
            Some(s) if !s.is_empty() => s,
            _ => format!("sf-session-{}", app_password),
        };

        Ok(Self {
            database_url: env("DATABASE_URL")?,
            jwt_secret,
            app_password,
            app_url,
            frontend_url,
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
            threads_app_id: opt("THREADS_APP_ID"),
            threads_app_secret: opt("THREADS_APP_SECRET"),
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
            github_token: opt("GITHUB_TOKEN"),
            twitch_client_id: opt("TWITCH_CLIENT_ID"),
            twitch_client_secret: opt("TWITCH_CLIENT_SECRET"),
            vk_client_id: opt("VK_CLIENT_ID"),
            vk_client_secret: opt("VK_CLIENT_SECRET"),
            whop_client_id: opt("WHOP_CLIENT_ID"),
            whop_client_secret: opt("WHOP_CLIENT_SECRET"),
            mewe_client_id: opt("MEWE_CLIENT_ID"),
            mewe_client_secret: opt("MEWE_CLIENT_SECRET"),
            moltbook_client_id: opt("MOLTBOOK_CLIENT_ID"),
            moltbook_client_secret: opt("MOLTBOOK_CLIENT_SECRET"),
            kick_client_id: opt("KICK_CLIENT_ID"),
            kick_client_secret: opt("KICK_CLIENT_SECRET"),
            neynar_api_key: opt("NEYNAR_API_KEY"),
            nostr_private_key: opt("NOSTR_PRIVATE_KEY"),
            token_encryption_key: opt("TOKEN_ENCRYPTION_KEY"),
            media_dir: opt("MEDIA_DIR").unwrap_or_else(|| "./uploads".into()),

            stripe_secret_key: opt("STRIPE_SECRET_KEY"),
            stripe_webhook_secret: opt("STRIPE_WEBHOOK_SECRET"),
            stripe_price_free: opt("STRIPE_PRICE_FREE"),
            stripe_price_pro_monthly: opt("STRIPE_PRICE_PRO_MONTHLY"),
            stripe_price_pro_annual: opt("STRIPE_PRICE_PRO_ANNUAL"),
            stripe_price_business_monthly: opt("STRIPE_PRICE_BUSINESS_MONTHLY"),
            stripe_price_business_annual: opt("STRIPE_PRICE_BUSINESS_ANNUAL"),
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
                self.threads_app_id.clone()?,
                self.threads_app_secret.clone()?,
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
            "whatsapp" => self.whatsapp_store_dir.as_ref().map(|d| ("whatsapp".into(), d.clone())),
            "pinterest" => Some((
                self.pinterest_client_id.clone()?,
                self.pinterest_client_secret.clone()?,
            )),
            "skool" => Some(("skool".into(), "chrome_extension".into())),
            "slack" => Some((self.slack_client_id.clone()?, self.slack_client_secret.clone()?)),
            "mastodon" => Some((self.mastodon_client_id.clone()?, self.mastodon_client_secret.clone()?)),
            "tiktok" => Some((self.tiktok_client_id.clone()?, self.tiktok_client_secret.clone()?)),
            "twitch" => Some((self.twitch_client_id.clone()?, self.twitch_client_secret.clone()?)),
            "vk" => Some((self.vk_client_id.clone()?, self.vk_client_secret.clone()?)),
            "whop" => Some((self.whop_client_id.clone()?, self.whop_client_secret.clone()?)),
            "mewe" => Some((self.mewe_client_id.clone()?, self.mewe_client_secret.clone()?)),
            "moltbook" => Some((self.moltbook_client_id.clone()?, self.moltbook_client_secret.clone()?)),
            "kick" => Some((self.kick_client_id.clone()?, self.kick_client_secret.clone()?)),
            "medium" => Some(("api-key".into(), self.medium_access_token.clone()?)),
            "devto" => Some(("api-key".into(), self.devto_api_key.clone()?)),
            "hashnode" => Some(("api-key".into(), self.hashnode_api_key.clone()?)),
            "github" => Some((self.github_token.clone()?, self.github_token.clone()?)),
            "google" | "google_my_business" | "gmail" | "calendar" | "drive" => Some((self.youtube_client_id.clone()?, self.youtube_client_secret.clone()?)),
            "lemmy" => Some(("lemmy".into(), "api_key".into())),
            "farcaster" => Some(("neynar".into(), self.neynar_api_key.clone()?)),
            "nostr" => Some(("nostr".into(), self.nostr_private_key.clone()?)),
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

/// Generate a random URL-safe password of the given length.
fn generate_random_password(len: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Persist `APP_PASSWORD=<pw>` to `~/.social-forge/.env`, creating
/// the directory if needed. Idempotent — replaces any existing line.
fn persist_app_password(password: &str) {
    let dir = config_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("Failed to create {}: {e}", dir.display());
        return;
    }
    let env_path = dir.join(".env");
    let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| !l.starts_with("APP_PASSWORD="))
        .map(|l| l.to_string())
        .collect();
    lines.push(format!("APP_PASSWORD={password}"));
    let body = lines.join("\n") + "\n";
    if let Err(e) = std::fs::write(&env_path, body) {
        tracing::warn!("Failed to write {}: {e}", env_path.display());
    }
}
