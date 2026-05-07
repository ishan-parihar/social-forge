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
    pub linkedin_client_id: Option<String>,
    pub linkedin_client_secret: Option<String>,
    pub bluesky_handle: Option<String>,
    pub bluesky_app_password: Option<String>,
    pub facebook_client_id: Option<String>,
    pub facebook_client_secret: Option<String>,
    pub instagram_client_id: Option<String>,
    pub instagram_client_secret: Option<String>,

    // Media storage
    pub media_storage: String,
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
            linkedin_client_id: opt("LINKEDIN_CLIENT_ID"),
            linkedin_client_secret: opt("LINKEDIN_CLIENT_SECRET"),
            bluesky_handle: opt("BLUESKY_HANDLE"),
            bluesky_app_password: opt("BLUESKY_APP_PASSWORD"),
            facebook_client_id: opt("FACEBOOK_CLIENT_ID"),
            facebook_client_secret: opt("FACEBOOK_CLIENT_SECRET"),
            instagram_client_id: opt("INSTAGRAM_CLIENT_ID"),
            instagram_client_secret: opt("INSTAGRAM_CLIENT_SECRET"),
            media_storage: opt("MEDIA_STORAGE").unwrap_or_else(|| "local".into()),
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
            _ => None,
        }
    }
}

fn env(key: &str) -> anyhow::Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("Missing required env var: {key}"))
}

fn opt(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}
