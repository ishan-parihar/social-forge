// ─── CLI Handler Implementations ─────────────────────────────────
// Dispatches CLI subcommands to provider methods, resolving tokens
// the same way MCP tools do (DB > browser extraction > env vars).

use uuid::Uuid;

use crate::api::AppState;
use crate::config::Config;
use crate::crypto;
use crate::db;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;

use super::{Cli, Command, ConfigAction, CommentAction, DmAction, AutomationAction, MediaAction, PostsAction};
use crate::social::TargetInfo;
use toon_helper;
use crate::db::models::Integration;

// ── Output Helpers ───────────────────────────────────────────

pub(crate) fn output_json(value: &serde_json::Value) {
    println!("{}", toon_helper::format_text(value, "toon"));
}

pub(crate) fn output_error(msg: &str) -> anyhow::Result<()> {
    let err = serde_json::json!({"error": msg});
    println!("{}", toon_helper::format_text(&err, "toon"));
    std::process::exit(2);
}

pub(crate) fn output_error_with_hint(msg: &str, hint: &str) -> ! {
    let err = serde_json::json!({"error": msg, "help": hint});
    println!("{}", toon_helper::format_text(&err, "toon"));
    std::process::exit(2);
}



// ── Target Discovery Helpers ─────────────────────────────────

pub(crate) async fn fetch_targets(state: &AppState, integration: &Integration) -> anyhow::Result<Vec<TargetInfo>> {
    let provider_obj = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| anyhow::anyhow!("Provider not found in registry"))?;

    let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let targets = provider_obj.targets(&token).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch targets: {}", e))?;

    Ok(targets)
}

pub(crate) async fn find_integration(state: &AppState, user_id: Uuid, provider: &str) -> anyhow::Result<Integration> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    integrations.into_iter()
        .find(|i| i.provider_identifier == provider)
        .ok_or_else(|| anyhow::anyhow!("No {} integration found", provider))
}

pub(crate) fn pick_target_interactive(targets: &[TargetInfo], provider: &str) -> anyhow::Result<String> {
    if targets.is_empty() {
        return Err(anyhow::anyhow!("No posting targets found for this {} account", provider));
    }

    eprintln!("\nAvailable {} targets:", provider);
    for (i, t) in targets.iter().enumerate() {
        let type_label = if !t.target_type.is_empty() {
            format!(" [{}]", t.target_type)
        } else {
            String::new()
        };
        eprintln!("  {}. {}{}", i + 1, t.name, type_label);
    }
    eprint!("\nSelect target (1-{}): ", targets.len());

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)
        .map_err(|e| anyhow::anyhow!("Failed to read input: {}", e))?;

    let choice: usize = input.trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection: {}", input.trim()))?;

    if choice < 1 || choice > targets.len() {
        return Err(anyhow::anyhow!("Selection out of range (1-{})", targets.len()));
    }

    Ok(targets[choice - 1].id.clone())
}

// ── Lightweight State Init ───────────────────────────────────

async fn init_state() -> anyhow::Result<AppState> {
    crate::config::load_dotenv();
    let config = Config::from_env()?;
    let db = db::create_pool(&config.database_url).await?;
    let broadcaster = Broadcaster::new();
    let token_key = config.token_encryption_key.as_ref()
        .and_then(|k| crypto::decode_hex_key(k).ok());
    let providers = ProviderRegistry::new(&config, None, None);
    Ok(AppState {
        db,
        config,
        broadcast: broadcaster,
        providers,
        token_key,
        telegram_client_manager: None,
        wa_client: None,
        media_http_client: reqwest::Client::new(),
        media_wreq_client: wreq::Client::new(),
    })
}

// ── User Resolution ──────────────────────────────────────────

pub(crate) async fn resolve_user(state: &AppState) -> anyhow::Result<Uuid> {
    let user = sqlx::query_scalar::<_, Uuid>(
        "SELECT u.id FROM users u WHERE EXISTS (SELECT 1 FROM integrations i WHERE i.user_id = u.id) LIMIT 1"
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(id) = user {
        return Ok(id);
    }

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users LIMIT 1")
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No user registered"))
}

// ── Token Resolution ─────────────────────────────────────────

pub(crate) async fn find_x_token(state: &AppState, user_id: Uuid) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let x_integrations: Vec<_> = integrations.into_iter()
        .filter(|i| i.provider_identifier == "x")
        .collect();

    if let Some(preferred) = x_integrations.iter().find(|i| i.access_token.starts_with('{')) {
        let token = preferred.access_token.clone();
        let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
        return Ok((token, preferred.internal_id.clone()));
    }

    if let (Some(auth_token), Some(ct0)) = (&state.config.x_auth_token, &state.config.x_ct0) {
        let token = serde_json::json!({"auth_token": auth_token, "ct0": ct0}).to_string();
        return Ok((token, String::new()));
    }

    if let Some(cookies) = crate::social::x_cookies::extract_x_cookies() {
        let token = crate::social::x_cookies::build_cookie_token(
            &cookies.auth_token, &cookies.ct0, Some(&cookies.cookie_string)
        );
        return Ok((token, String::new()));
    }

    if let Some(oauth) = x_integrations.first() {
        let token = oauth.access_token.clone();
        let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
        return Ok((token, oauth.internal_id.clone()));
    }

    anyhow::bail!("No X/Twitter integration found")
}

pub(crate) async fn find_reddit_token(state: &AppState, user_id: Uuid) -> anyhow::Result<String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let reddit_integrations: Vec<_> = integrations.into_iter()
        .filter(|i| i.provider_identifier == "reddit")
        .collect();

    if let Some(preferred) = reddit_integrations.iter().find(|i| i.access_token.starts_with('{')) {
        let token = preferred.access_token.clone();
        let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
        return Ok(token);
    }

    if let Some(cookies) = crate::social::reddit_cookies::extract_reddit_cookies() {
        return Ok(crate::social::reddit_cookies::build_cookie_token(
            &cookies.reddit_session, cookies.token_v2.as_deref(), Some(&cookies.cookie_string)
        ));
    }

    if let Some(oauth) = reddit_integrations.first() {
        let token = oauth.access_token.clone();
        let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
        return Ok(token);
    }

    anyhow::bail!("No Reddit integration found")
}

pub(crate) async fn find_linkedin_token(state: &AppState, user_id: Uuid) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let li = integrations.iter()
        .find(|i| i.provider_identifier == "linkedin")
        .ok_or_else(|| anyhow::anyhow!("No LinkedIn account connected"))?;
    let token = li.access_token.clone();
    let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
    Ok((token, li.internal_id.clone()))
}

pub(crate) async fn find_linkedin_page_token(state: &AppState, user_id: Uuid, page_id: &str) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let lip = integrations.iter()
        .find(|i| i.provider_identifier == "linkedin-page" && i.internal_id == page_id)
        .ok_or_else(|| anyhow::anyhow!("LinkedIn Page '{}' not connected", page_id))?;
    let token = lip.access_token.clone();
    let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
    Ok((token, lip.internal_id.clone()))
}

pub(crate) async fn find_facebook_page_token(state: &AppState, user_id: Uuid, page_id: &str) -> anyhow::Result<String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let page = integrations.iter()
        .find(|i| i.provider_identifier == "facebook" && i.internal_id == page_id)
        .ok_or_else(|| anyhow::anyhow!("Facebook page '{}' not connected", page_id))?;
    let token = page.access_token.clone();
    let token = crate::crypto::maybe_decrypt_token(&token, state.token_key.as_ref());
    Ok(token)
}

// ── Providers / Connect ──────────────────────────────────────

async fn handle_providers_with_state(state: &AppState) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let list: Vec<serde_json::Value> = integrations.iter().map(|i| {
        serde_json::json!({
            "provider": i.provider_identifier,
            "name": i.profile_name,
            "internal_id": i.internal_id,
            "disabled": i.disabled,
        })
    }).collect();
    output_json(&serde_json::json!({"providers": list}));
    Ok(())
}

fn handle_init() -> anyhow::Result<()> {
    let dir = crate::config::config_dir();
    std::fs::create_dir_all(&dir)?;
    let env_path = dir.join(".env");
    if env_path.exists() {
        output_json(&serde_json::json!({
            "status": "exists",
            "path": env_path.display().to_string(),
            "message": "Config already exists. Edit it with your preferred editor."
        }));
    } else {
        let template = r#"# ─── Server ──────────────────────────────────────────────
DATABASE_URL=postgres://social_forge:social_forge@localhost:5432/social_forge
JWT_SECRET=change-me-to-a-random-secret

# ─── X/Twitter ────────────────────────────────────────────
# X_AUTH_TOKEN=
# X_CT0=

# ─── Reddit ───────────────────────────────────────────────
# REDDIT_CLIENT_ID=
# REDDIT_CLIENT_SECRET=
# REDDIT_REDIRECT_URI=http://localhost:3444/api/auth/reddit/callback

# ─── LinkedIn ─────────────────────────────────────────────
# LINKEDIN_CLIENT_ID=
# LINKEDIN_CLIENT_SECRET=
# LINKEDIN_REDIRECT_URI=http://localhost:3444/api/auth/linkedin/callback

# ─── Facebook / Instagram / Threads ───────────────────────
# META_CLIENT_ID=
# META_CLIENT_SECRET=
# META_REDIRECT_URI=http://localhost:3444/api/auth/meta/callback

# ─── GitHub ───────────────────────────────────────────────
# GITHUB_TOKEN=

# ─── Dev.to ───────────────────────────────────────────────
# DEVTO_API_KEY=

# ─── Mastodon ─────────────────────────────────────────────
# MASTODON_ACCESS_TOKEN=
# MASTODON_INSTANCE_URL=

# ─── Medium ───────────────────────────────────────────────
# MEDIUM_TOKEN=

# ─── WordPress ────────────────────────────────────────────
# WORDPRESS_SITE_URL=
# WORDPRESS_USERNAME=
# WORDPRESS_PASSWORD=

# ─── YouTube ──────────────────────────────────────────────
# YOUTUBE_API_KEY=

# ─── Pinterest ────────────────────────────────────────────
# PINTEREST_APP_ID=
# PINTEREST_APP_SECRET=

# ─── TikTok ───────────────────────────────────────────────
# TIKTOK_CLIENT_KEY=
# TIKTOK_CLIENT_SECRET=

# ─── Hashnode ─────────────────────────────────────────────
# HASHNODE_PAT=

# ─── VK ───────────────────────────────────────────────────
# VK_CLIENT_ID=
# VK_CLIENT_SECRET=

# ─── Bluesky ──────────────────────────────────────────────
# BLUESKY_IDENTIFIER=
# BLUESKY_PASSWORD=

# ─── Telegram ─────────────────────────────────────────────
# TELEGRAM_API_ID=
# TELEGRAM_API_HASH=
# TELEGRAM_PHONE=
"#;
        std::fs::write(&env_path, template)?;
        output_json(&serde_json::json!({
            "status": "created",
            "path": env_path.display().to_string(),
            "message": "Config created. Edit ~/.social-forge/.env with your API keys and DATABASE_URL."
        }));
    }
    Ok(())
}

async fn handle_connect_with_state(state: &AppState, provider: &str) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;

    match provider {
        // ── X/Twitter: auto-import from browser ────────────────
        "x" => {
            // Check if already connected
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "x") {
                let existing: Vec<_> = integrations.iter().filter(|i| i.provider_identifier == "x").collect();
                output_json(&serde_json::json!({
                    "status": "already_connected",
                    "provider": "x",
                    "count": existing.len(),
                    "accounts": existing.iter().map(|i| serde_json::json!({
                        "name": i.profile_name,
                        "internal_id": i.internal_id,
                    })).collect::<Vec<_>>(),
                    "hint": "Already connected. Use 'social-forge x timeline' to verify.",
                }));
                return Ok(());
            }

            // Try auto-import from browser
            match crate::social::x_cookies::extract_x_cookies() {
                Some(cookies) => {
                    let token_str = crate::social::x_cookies::build_cookie_token(
                        &cookies.auth_token, &cookies.ct0, Some(&cookies.cookie_string)
                    );

                    // Validate by calling get_me
                    let mut provider_obj = crate::social::x::XProvider::new(&state.config);
                    provider_obj.prepare_from_token(&token_str);
                    match provider_obj.get_me(&token_str).await {
                        Ok(json) => {
                            let data = json.get("data");
                            let name = data.and_then(|d| d.get("name")).and_then(|s| s.as_str()).unwrap_or("X User");
                            let username = data.and_then(|d| d.get("username")).and_then(|s| s.as_str()).unwrap_or("");
                            let avatar = data.and_then(|d| d.get("profile_image_url")).and_then(|s| s.as_str());
                            let id = data.and_then(|d| d.get("id")).and_then(|s| s.as_str()).unwrap_or("").to_string();

                            crate::db::queries::create_integration(
                                &state.db, user_id, "x", "X (Twitter)", &id, &token_str,
                                None, None, Some(name), None, avatar, None, None,
                            ).await?;

                            output_json(&serde_json::json!({
                                "status": "connected",
                                "provider": "x",
                                "method": "browser-import",
                                "source": cookies.source,
                                "name": name,
                                "username": username,
                                "id": id,
                            }));
                        }
                        Err(e) => {
                            output_json(&serde_json::json!({
                                "status": "error",
                                "provider": "x",
                                "error": format!("Cookie import succeeded but validation failed: {e}. The cookies may be expired."),
                                "hint": "Log into x.com in your browser, then run this command again.",
                            }));
                        }
                    }
                }
                None => {
                    output_json(&serde_json::json!({
                        "status": "no_browser_cookies",
                        "provider": "x",
                        "error": "No X/Twitter cookies found in any browser.",
                        "hints": [
                            "Log into x.com in Chrome, Brave, Firefox, or Zen browser",
                            "Then run 'social-forge connect x' again",
                            "Or set X_AUTH_TOKEN + X_CT0 in ~/.social-forge/.env",
                            "Or visit http://localhost:6543/api/public/connect/x-cookies for manual entry",
                        ],
                    }));
                }
            }
        }

        // ── Reddit: auto-import from browser ──────────────────
        "reddit" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "reddit") {
                let existing: Vec<_> = integrations.iter().filter(|i| i.provider_identifier == "reddit").collect();
                output_json(&serde_json::json!({
                    "status": "already_connected",
                    "provider": "reddit",
                    "count": existing.len(),
                    "accounts": existing.iter().map(|i| serde_json::json!({
                        "name": i.profile_name,
                        "internal_id": i.internal_id,
                    })).collect::<Vec<_>>(),
                    "hint": "Already connected. Use 'social-forge reddit browse rust' to verify.",
                }));
                return Ok(());
            }

            match crate::social::reddit_cookies::extract_reddit_cookies() {
                Some(cookies) => {
                    let token_str = crate::social::reddit_cookies::build_cookie_token(
                        &cookies.reddit_session, cookies.token_v2.as_deref(), Some(&cookies.cookie_string)
                    );

                    let mut provider_obj = crate::social::reddit::RedditProvider::new(&state.config);
                    provider_obj.prepare_from_token(&token_str);
                    match provider_obj.get_www("/api/me.json", &[]).await {
                        Ok(json) => {
                            let name = json["data"]["name"].as_str().unwrap_or("Reddit User").to_string();
                            let id = json["data"]["id"].as_str().unwrap_or("").to_string();
                            let icon = json["data"]["icon_img"].as_str()
                                .and_then(|s| s.split('?').next())
                                .map(String::from);

                            crate::db::queries::create_integration(
                                &state.db, user_id, "reddit", "Reddit", &id, &token_str,
                                None, None, Some(&name), None, icon.as_deref(), None, None,
                            ).await?;

                            output_json(&serde_json::json!({
                                "status": "connected",
                                "provider": "reddit",
                                "method": "browser-import",
                                "source": cookies.source,
                                "name": name,
                                "id": id,
                            }));
                        }
                        Err(e) => {
                            output_json(&serde_json::json!({
                                "status": "error",
                                "provider": "reddit",
                                "error": format!("Cookie import succeeded but validation failed: {e}. The cookies may be expired."),
                                "hint": "Log into reddit.com in your browser, then run this command again.",
                            }));
                        }
                    }
                }
                None => {
                    output_json(&serde_json::json!({
                        "status": "no_browser_cookies",
                        "provider": "reddit",
                        "error": "No Reddit cookies found in any browser.",
                        "hints": [
                            "Log into reddit.com in Chrome, Brave, Firefox, or Zen browser",
                            "Then run 'social-forge connect reddit' again",
                            "Or visit http://localhost:6543/api/public/connect/reddit-cookies for manual entry",
                        ],
                    }));
                }
            }
        }

        // ── OAuth providers: check status and provide URL ──────
        "linkedin" | "linkedin-page" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            let connected: Vec<_> = integrations.iter()
                .filter(|i| i.provider_identifier == "linkedin" || i.provider_identifier == "linkedin-page")
                .collect();
            let app_url = &state.config.app_url;
            output_json(&serde_json::json!({
                "status": if connected.is_empty() { "not_connected" } else { "already_connected" },
                "provider": provider,
                "count": connected.len(),
                "accounts": connected.iter().map(|i| serde_json::json!({
                    "name": i.profile_name,
                    "internal_id": i.internal_id,
                    "type": i.provider_identifier,
                })).collect::<Vec<_>>(),
                "method": "oauth",
                "auth_url": format!("{}/api/public/connect/{}", app_url, provider),
                "hint": "Open the auth_url in a browser to complete OAuth authorization.",
            }));
        }
        "facebook" | "instagram" | "instagram-standalone" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            let connected: Vec<_> = integrations.iter()
                .filter(|i| i.provider_identifier == provider)
                .collect();
            let app_url = &state.config.app_url;
            output_json(&serde_json::json!({
                "status": if connected.is_empty() { "not_connected" } else { "already_connected" },
                "provider": provider,
                "count": connected.len(),
                "accounts": connected.iter().map(|i| serde_json::json!({
                    "name": i.profile_name,
                    "internal_id": i.internal_id,
                })).collect::<Vec<_>>(),
                "method": "oauth",
                "auth_url": format!("{}/api/public/connect/{}", app_url, provider),
                "hint": "Open the auth_url in a browser to complete OAuth authorization.",
            }));
        }

        // ── Direct-connect providers ──────────────────────────
        // ── Env-var credential providers ─────────────────────
        "bluesky" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "bluesky") {
                let existing: Vec<_> = integrations.iter().filter(|i| i.provider_identifier == "bluesky").collect();
                output_json(&serde_json::json!({"status": "already_connected", "provider": "bluesky", "count": existing.len()}));
            } else if state.config.bluesky_handle.is_some() && state.config.bluesky_app_password.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "bluesky", "method": "env_vars", "hint": "BLUESKY_HANDLE + BLUESKY_APP_PASSWORD are set. The provider will connect automatically on first use."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "bluesky", "requires": ["BLUESKY_HANDLE", "BLUESKY_APP_PASSWORD"], "hint": "Set these in ~/.social-forge/.env. Get an app password at Bluesky Settings > Advanced > App Passwords."}));
            }
        }
        "github" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "github") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "github"}));
            } else if state.config.github_token.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "github", "method": "env_vars", "hint": "GITHUB_TOKEN is set. The provider will connect automatically on first use."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "github", "requires": ["GITHUB_TOKEN"], "hint": "Create a PAT at https://github.com/settings/tokens and set GITHUB_TOKEN in ~/.social-forge/.env"}));
            }
        }
        "telegram-bot" | "telegram" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "telegram-bot") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "telegram-bot"}));
            } else if state.config.telegram_bot_tokens.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "telegram-bot", "method": "env_vars", "hint": "TELEGRAM_BOT_TOKENS is set. The provider will connect automatically on first use."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "telegram-bot", "requires": ["TELEGRAM_BOT_TOKENS"], "hint": "Message @BotFather on Telegram to create a bot, get the token, and set TELEGRAM_BOT_TOKENS in ~/.social-forge/.env"}));
            }
        }
        "discord" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "discord") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "discord"}));
            } else if state.config.discord_client_id.is_some() && state.config.discord_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "discord", "method": "oauth", "hint": "DISCORD_CLIENT_ID + DISCORD_CLIENT_SECRET are set. Authorize via the web UI."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "discord", "requires": ["DISCORD_CLIENT_ID", "DISCORD_CLIENT_SECRET"], "hint": "Create an app at https://discord.com/developers/applications and set credentials in ~/.social-forge/.env"}));
            }
        }
        "slack" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "slack") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "slack"}));
            } else if state.config.slack_client_id.is_some() && state.config.slack_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "slack", "method": "oauth", "hint": "SLACK_CLIENT_ID + SLACK_CLIENT_SECRET are set. Authorize via the web UI."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "slack", "requires": ["SLACK_CLIENT_ID", "SLACK_CLIENT_SECRET"], "hint": "Create an app at https://api.slack.com/apps and set credentials in ~/.social-forge/.env"}));
            }
        }
        "pinterest" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "pinterest") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "pinterest"}));
            } else if state.config.pinterest_client_id.is_some() && state.config.pinterest_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "pinterest", "method": "oauth"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "pinterest", "requires": ["PINTEREST_CLIENT_ID", "PINTEREST_CLIENT_SECRET"], "hint": "Create an app at https://developers.pinterest.com/apps/ and set credentials in ~/.social-forge/.env"}));
            }
        }
        "tiktok" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "tiktok") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "tiktok"}));
            } else if state.config.tiktok_client_id.is_some() && state.config.tiktok_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "tiktok", "method": "oauth"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "tiktok", "requires": ["TIKTOK_CLIENT_ID", "TIKTOK_CLIENT_SECRET"], "hint": "Create an app at https://developers.tiktok.com/ and set credentials in ~/.social-forge/.env"}));
            }
        }
        "mastodon" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            if integrations.iter().any(|i| i.provider_identifier == "mastodon") {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "mastodon"}));
            } else if state.config.mastodon_client_id.is_some() && state.config.mastodon_client_secret.is_some() && state.config.mastodon_instance_url.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "mastodon", "method": "oauth"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "mastodon", "requires": ["MASTODON_CLIENT_ID", "MASTODON_CLIENT_SECRET", "MASTODON_INSTANCE_URL"], "hint": "Register an app on your Mastodon instance and set credentials in ~/.social-forge/.env"}));
            }
        }
        "youtube" | "google" => {
            let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
            let connected = integrations.iter().any(|i| i.provider_identifier == "youtube" || i.provider_identifier == "google");
            if connected {
                output_json(&serde_json::json!({"status": "already_connected", "provider": "youtube"}));
            } else if state.config.youtube_client_id.is_some() && state.config.youtube_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "youtube", "method": "oauth", "hint": "YOUTUBE_CLIENT_ID + YOUTUBE_CLIENT_SECRET are set. Authorize via the web UI."}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "youtube", "requires": ["YOUTUBE_CLIENT_ID", "YOUTUBE_CLIENT_SECRET"], "hint": "Create a Google Cloud project, enable YouTube Data API v3, create OAuth credentials, and set in ~/.social-forge/.env"}));
            }
        }
        "medium" => {
            if state.config.medium_access_token.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "medium", "method": "env_vars"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "medium", "requires": ["MEDIUM_ACCESS_TOKEN"], "hint": "Get an integration token at https://medium.com/me/settings/security"}));
            }
        }
        "devto" => {
            if state.config.devto_api_key.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "devto", "method": "env_vars"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "devto", "requires": ["DEVTO_API_KEY"], "hint": "Generate an API key at https://dev.to/settings/extensions"}));
            }
        }
        "hashnode" => {
            if state.config.hashnode_api_key.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "hashnode", "method": "env_vars"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "hashnode", "requires": ["HASHNODE_API_KEY"], "hint": "Generate a PAT at https://hashnode.com/settings/integrations"}));
            }
        }
        "wordpress" => {
            output_json(&serde_json::json!({"status": "per_site", "provider": "wordpress", "hint": "WordPress uses per-site Application Passwords. Connect via the web UI with site URL, username, and app password."}));
        }
        "skool" => {
            output_json(&serde_json::json!({"status": "chrome_extension", "provider": "skool", "hint": "Skool uses Chrome extension cookie extraction. Install the Skool Chrome extension, log into skool.com, and cookies are auto-extracted."}));
        }
        "farcaster" | "lemmy" => {
            output_json(&serde_json::json!({"status": "per_user", "provider": provider, "hint": format!("{provider} uses per-user credentials stored in the integration record. Connect via the web UI.")}));
        }
        "whatsapp" => {
            output_json(&serde_json::json!({"status": "native_client", "provider": "whatsapp", "hint": "WhatsApp uses a native client. Connect via the web UI to scan the QR code."}));
        }
        "vk" => {
            if state.config.vk_client_id.is_some() && state.config.vk_client_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "vk", "method": "oauth"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "vk", "requires": ["VK_CLIENT_ID", "VK_CLIENT_SECRET"], "hint": "Create an app at https://vk.com/editapp?act=create and set credentials in ~/.social-forge/.env"}));
            }
        }
        "threads" => {
            if state.config.threads_app_id.is_some() && state.config.threads_app_secret.is_some() {
                output_json(&serde_json::json!({"status": "configured", "provider": "threads", "method": "oauth"}));
            } else {
                output_json(&serde_json::json!({"status": "not_configured", "provider": "threads", "requires": ["THREADS_APP_ID", "THREADS_APP_SECRET"], "hint": "Threads uses Meta credentials. Set THREADS_APP_ID and THREADS_APP_SECRET in ~/.social-forge/.env"}));
            }
        }

        _ => {
            output_json(&serde_json::json!({
                "status": "unknown",
                "provider": provider,
                "error": format!("Unknown provider '{provider}'. Use 'social-forge connect --help' for supported providers."),
                "hint": "Try: x, reddit, linkedin, facebook, instagram, bluesky, github, telegram, discord, slack, pinterest, tiktok, mastodon, youtube, medium, devto, hashnode, wordpress, threads, vk, skool",
            }));
        }
    }

    Ok(())
}

// ── Doctor: health check for all providers ───────────────────

async fn handle_doctor_with_state(state: &AppState) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list integrations: {e}"))?;

    let mut checks: Vec<serde_json::Value> = Vec::new();

    // Check each connected provider by making a lightweight API call
    for integration in &integrations {
        let provider_id = &integration.provider_identifier;
        let name = integration.profile_name.as_deref().unwrap_or("Unknown");
        let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

        let (status, detail) = match provider_id.as_str() {
            "x" => {
                let mut p = crate::social::x::XProvider::new(&state.config);
                p.prepare_from_token(&token);
                match p.get_me(&token).await {
                    Ok(_) => ("healthy".to_string(), None),
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            "reddit" => {
                let mut p = crate::social::reddit::RedditProvider::new(&state.config);
                p.prepare_from_token(&token);
                match p.get_www("/api/me.json", &[]).await {
                    Ok(_) => ("healthy".to_string(), None),
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            "linkedin" => {
                let p = crate::social::linkedin::LinkedInProvider::new(&state.config);
                match p.get_profile(&token).await {
                    Ok(_) => ("healthy".to_string(), None),
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            "linkedin-page" => {
                let p = crate::social::linkedin_page::LinkedInPageProvider::new(&state.config);
                match p.get_page_posts(&token, &integration.internal_id, 1).await {
                    Ok(_) => ("healthy".to_string(), None),
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            "facebook" => {
                let url = format!("https://graph.facebook.com/v19.0/{}?fields=name", integration.internal_id);
                match reqwest::Client::new().get(&url).bearer_auth(&token).send().await {
                    Ok(r) if r.status().is_success() => ("healthy".to_string(), None),
                    Ok(r) => {
                        let status = r.status();
                        let body = r.text().await.unwrap_or_default();
                        ("error".to_string(), Some(format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>())))
                    }
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            "instagram" | "instagram-standalone" => {
                let url = format!("https://graph.facebook.com/v19.0/{}?fields=id,name", integration.internal_id);
                match reqwest::Client::new().get(&url).bearer_auth(&token).send().await {
                    Ok(r) if r.status().is_success() => ("healthy".to_string(), None),
                    Ok(r) => {
                        let status = r.status();
                        let body = r.text().await.unwrap_or_default();
                        ("error".to_string(), Some(format!("HTTP {}: {}", status, body.chars().take(200).collect::<String>())))
                    }
                    Err(e) => ("error".to_string(), Some(format!("{e}"))),
                }
            }
            _ => ("skipped".to_string(), Some("No health check available for this provider".to_string())),
        };

        let mut check = serde_json::json!({
            "provider": provider_id,
            "name": name,
            "internal_id": integration.internal_id,
            "status": status,
        });
        if let Some(d) = detail {
            check["detail"] = serde_json::Value::String(d);
        }
        checks.push(check);
    }

    // Check for missing providers that have env vars configured
    let missing_providers: Vec<serde_json::Value> = vec![
        ("linkedin", "LinkedIn Personal", state.config.linkedin_client_id.is_some() && state.config.linkedin_client_secret.is_some()),
        ("facebook", "Facebook Pages", state.config.facebook_client_id.is_some() && state.config.facebook_client_secret.is_some()),
        ("instagram", "Instagram", state.config.facebook_client_id.is_some() && state.config.facebook_client_secret.is_some()),
    ]
    .into_iter()
    .filter(|(id, _, _)| !integrations.iter().any(|i| i.provider_identifier == *id))
    .filter(|(_, _, has_creds)| *has_creds)
    .map(|(id, name, _)| serde_json::json!({
        "provider": id,
        "name": name,
        "status": "needs_oauth",
        "hint": format!("Credentials configured but not connected. Run 'social-forge connect {id}' or visit the onboarding page.")
    }))
    .collect();

    let healthy = checks.iter().filter(|c| c["status"] == "healthy").count();
    let errored = checks.iter().filter(|c| c["status"] == "error").count();

    output_json(&serde_json::json!({
        "healthy": healthy,
        "errors": errored,
        "connected": checks.len(),
        "providers": checks,
        "missing_oauth": missing_providers,
    }));
    Ok(())
}

// ── Setup: Full Guided Onboarding ──────────────────────────

async fn handle_setup() -> anyhow::Result<()> {
    crate::config::load_dotenv();
    let mut steps: Vec<serde_json::Value> = Vec::new();

    // Step 1: Config file
    let env_path = crate::config::config_dir().join(".env");
    let config_exists = env_path.exists();
    let has_database_url = std::env::var("DATABASE_URL").is_ok();
    steps.push(serde_json::json!({
        "step": 1,
        "name": "config",
        "status": if config_exists && has_database_url { "ok" } else { "action_needed" },
        "detail": if !config_exists {
            "Run 'social-forge init' to create ~/.social-forge/.env"
        } else if !has_database_url {
            "DATABASE_URL not found. Set it in ~/.social-forge/.env or environment."
        } else {
            "Config file exists and DATABASE_URL is set."
        },
        "action": if !config_exists { Some("social-forge init") } else { None },
    }));

    // Step 2: Database (graceful — don't crash if DB isn't configured)
    let db_result = init_state().await;
    let db_ok = db_result.is_ok();
    steps.push(serde_json::json!({
        "step": 2,
        "name": "database",
        "status": if db_ok { "ok" } else { "error" },
        "detail": if db_ok { "PostgreSQL connection successful." } else { "Cannot connect to PostgreSQL. Check DATABASE_URL in ~/.social-forge/.env" },
    }));

    // If DB failed, we can still report config + cookie status
    let state = match db_result {
        Ok(s) => s,
        Err(e) => {
            // Add remaining steps with limited info
            let x_cookies = crate::social::x_cookies::extract_x_cookies();
            let reddit_cookies = crate::social::reddit_cookies::extract_reddit_cookies();
            steps.push(serde_json::json!({
                "step": 3, "name": "user", "status": "skipped",
                "detail": format!("Skipped: {e}"),
            }));
            steps.push(serde_json::json!({
                "step": 4, "name": "browser_cookies", "status": "info",
                "detail": format!("X cookies: {}. Reddit cookies: {}.",
                    if x_cookies.is_some() { "found" } else { "not_found" },
                    if reddit_cookies.is_some() { "found" } else { "not_found" }),
            }));
            output_json(&serde_json::json!({
                "status": "setup_incomplete",
                "steps": steps,
                "next_actions": {
                    "fix_db": "Set DATABASE_URL in ~/.social-forge/.env and ensure PostgreSQL is running",
                    "init": "social-forge init",
                },
            }));
            return Ok(());
        }
    };

    // Step 3: User
    let user_result = resolve_user(&state).await;
    let user_ok = user_result.is_ok();
    steps.push(serde_json::json!({
        "step": 3,
        "name": "user",
        "status": if user_ok { "ok" } else { "action_needed" },
        "detail": if user_ok { "User account exists." } else { "No user registered. Register via the web UI or API." },
    }));

    // Step 4: Cookie-based providers (X, Reddit)
    let x_cookies = crate::social::x_cookies::extract_x_cookies();
    let reddit_cookies = crate::social::reddit_cookies::extract_reddit_cookies();
    let x_cookie_status = if x_cookies.is_some() { "found" } else { "not_found" };
    let reddit_cookie_status = if reddit_cookies.is_some() { "found" } else { "not_found" };
    steps.push(serde_json::json!({
        "step": 4,
        "name": "browser_cookies",
        "status": "info",
        "detail": format!("X cookies: {}. Reddit cookies: {}.", x_cookie_status, reddit_cookie_status),
        "hint": "Run 'social-forge connect-all' to auto-import cookies from your browser.",
    }));

    // Step 5: Existing integrations
    if let Ok(uid) = user_result {
        let integrations = crate::db::queries::list_integrations(&state.db, uid).await.unwrap_or_default();
        let provider_summary: Vec<serde_json::Value> = integrations.iter().map(|i| {
            serde_json::json!({
                "provider": i.provider_identifier,
                "name": i.profile_name,
            })
        }).collect();
        steps.push(serde_json::json!({
            "step": 5,
            "name": "integrations",
            "status": if provider_summary.is_empty() { "action_needed" } else { "ok" },
            "connected_count": provider_summary.len(),
            "connected": provider_summary,
        }));
    }

    // Step 6: Env-var providers status
    let env_providers: Vec<serde_json::Value> = vec![
        serde_json::json!({"name": "Bluesky", "key": "BLUESKY_HANDLE", "configured": state.config.bluesky_handle.is_some()}),
        serde_json::json!({"name": "GitHub", "key": "GITHUB_TOKEN", "configured": state.config.github_token.is_some()}),
        serde_json::json!({"name": "Telegram Bot", "key": "TELEGRAM_BOT_TOKENS", "configured": state.config.telegram_bot_tokens.is_some()}),
        serde_json::json!({"name": "Discord Bot", "key": "DISCORD_BOT_TOKEN", "configured": state.config.discord_bot_token.is_some()}),
        serde_json::json!({"name": "Dev.to", "key": "DEVTO_API_KEY", "configured": state.config.devto_api_key.is_some()}),
        serde_json::json!({"name": "Medium", "key": "MEDIUM_ACCESS_TOKEN", "configured": state.config.medium_access_token.is_some()}),
        serde_json::json!({"name": "Hashnode", "key": "HASHNODE_API_KEY", "configured": state.config.hashnode_api_key.is_some()}),
    ];
    let configured_count = env_providers.iter().filter(|p| p["configured"] == true).count();
    steps.push(serde_json::json!({
        "step": 6,
        "name": "env_providers",
        "status": "info",
        "configured_count": configured_count,
        "total": env_providers.len(),
        "providers": env_providers,
        "hint": "Use 'social-forge config set KEY VALUE' to add API keys.",
    }));

    // Summary
    let action_needed = steps.iter().any(|s| s["status"] == "action_needed" || s["status"] == "error");
    output_json(&serde_json::json!({
        "status": if action_needed { "setup_incomplete" } else { "ready" },
        "steps": steps,
        "next_actions": {
            "connect_all": "social-forge connect-all  (import browser cookies for X + Reddit)",
            "doctor": "social-forge doctor  (health-check all providers)",
            "config_set": "social-forge config set KEY VALUE  (add API keys)",
        },
    }));
    Ok(())
}

// ── Connect All: Bulk Cookie Import ──────────────────────────

async fn handle_connect_all_with_state(state: &AppState) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let mut results: Vec<serde_json::Value> = Vec::new();

    // ── X/Twitter ─────────────────────────────────────────────
    let x_result = {
        let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
        if integrations.iter().any(|i| i.provider_identifier == "x") {
            serde_json::json!({"provider": "x", "status": "already_connected"})
        } else {
            match crate::social::x_cookies::extract_x_cookies() {
                Some(cookies) => {
                    let token_str = crate::social::x_cookies::build_cookie_token(
                        &cookies.auth_token, &cookies.ct0, Some(&cookies.cookie_string)
                    );
                    let mut provider_obj = crate::social::x::XProvider::new(&state.config);
                    provider_obj.prepare_from_token(&token_str);
                    match provider_obj.get_me(&token_str).await {
                        Ok(json) => {
                            let data = json.get("data");
                            let name = data.and_then(|d| d.get("name")).and_then(|s| s.as_str()).unwrap_or("X User");
                            let id = data.and_then(|d| d.get("id")).and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let avatar = data.and_then(|d| d.get("profile_image_url")).and_then(|s| s.as_str());
                            match crate::db::queries::create_integration(
                                &state.db, user_id, "x", "X (Twitter)", &id, &token_str,
                                None, None, Some(name), None, avatar, None, None,
                            ).await {
                                Ok(_) => serde_json::json!({"provider": "x", "status": "connected", "name": name, "source": cookies.source}),
                                Err(e) => serde_json::json!({"provider": "x", "status": "error", "error": format!("DB write failed: {e}")}),
                            }
                        }
                        Err(e) => serde_json::json!({"provider": "x", "status": "error", "error": format!("{e}")}),
                    }
                }
                None => serde_json::json!({"provider": "x", "status": "no_cookies", "hint": "Log into x.com in your browser first."}),
            }
        }
    };
    results.push(x_result);

    // ── Reddit ────────────────────────────────────────────────
    let reddit_result = {
        let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
        if integrations.iter().any(|i| i.provider_identifier == "reddit") {
            serde_json::json!({"provider": "reddit", "status": "already_connected"})
        } else {
            match crate::social::reddit_cookies::extract_reddit_cookies() {
                Some(cookies) => {
                    let token_str = crate::social::reddit_cookies::build_cookie_token(
                        &cookies.reddit_session, cookies.token_v2.as_deref(), Some(&cookies.cookie_string)
                    );
                    let mut provider_obj = crate::social::reddit::RedditProvider::new(&state.config);
                    provider_obj.prepare_from_token(&token_str);
                    match provider_obj.get_www("/api/me.json", &[]).await {
                        Ok(json) => {
                            let name = json["data"]["name"].as_str().unwrap_or("Reddit User").to_string();
                            let id = json["data"]["id"].as_str().unwrap_or("").to_string();
                            let icon = json["data"]["icon_img"].as_str().and_then(|s| s.split('?').next()).map(String::from);
                            match crate::db::queries::create_integration(
                                &state.db, user_id, "reddit", "Reddit", &id, &token_str,
                                None, None, Some(&name), None, icon.as_deref(), None, None,
                            ).await {
                                Ok(_) => serde_json::json!({"provider": "reddit", "status": "connected", "name": name, "source": cookies.source}),
                                Err(e) => serde_json::json!({"provider": "reddit", "status": "error", "error": format!("DB write failed: {e}")}),
                            }
                        }
                        Err(e) => serde_json::json!({"provider": "reddit", "status": "error", "error": format!("{e}")}),
                    }
                }
                None => serde_json::json!({"provider": "reddit", "status": "no_cookies", "hint": "Log into reddit.com in your browser first."}),
            }
        }
    };
    results.push(reddit_result);

    let connected = results.iter().filter(|r| r["status"] == "connected").count();
    let already = results.iter().filter(|r| r["status"] == "already_connected").count();
    let failed = results.iter().filter(|r| r["status"] == "error").count();
    let no_cookies = results.iter().filter(|r| r["status"] == "no_cookies").count();

    output_json(&serde_json::json!({
        "connected": connected,
        "already_connected": already,
        "errors": failed,
        "no_cookies": no_cookies,
        "results": results,
    }));
    Ok(())
}

// ── Config: Manage ~/.social-forge/.env ──────────────────────

fn handle_config(action: ConfigAction) -> anyhow::Result<()> {
    crate::config::load_dotenv();
    let dir = crate::config::config_dir();
    std::fs::create_dir_all(&dir)?;
    let env_path = dir.join(".env");

    match action {
        ConfigAction::Set { key, value } => {
            // Read existing content
            let content = if env_path.exists() {
                std::fs::read_to_string(&env_path)?
            } else {
                String::new()
            };

            let key_upper = key.to_uppercase();
            let new_line = format!("{key_upper}={value}");

            // Check if key already exists and replace it
            let mut found = false;
            let new_content: String = content.lines().map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with(&format!("{key_upper}=")) ||
                   trimmed.starts_with(&format!("# {key_upper}=")) ||
                   trimmed.starts_with(&format!("#{key_upper}=")) {
                    found = true;
                    new_line.clone()
                } else {
                    line.to_string()
                }
            }).collect::<Vec<_>>().join("\n");

            if found {
                std::fs::write(&env_path, &new_content)?;
            } else {
                let mut to_write = new_content;
                if !to_write.ends_with('\n') {
                    to_write.push('\n');
                }
                to_write.push_str(&new_line);
                to_write.push('\n');
                std::fs::write(&env_path, &to_write)?;
            }

            output_json(&serde_json::json!({
                "status": "set",
                "key": key_upper,
                "path": env_path.display().to_string(),
                "message": format!("Set {key_upper}. Restart social-forge to apply."),
            }));
        }
        ConfigAction::Get { key } => {
            let key_upper = key.to_uppercase();
            match std::env::var(&key_upper) {
                Ok(val) => {
                    // Redact secrets
                    let display = if key_upper.contains("SECRET") || key_upper.contains("PASSWORD") ||
                                   key_upper.contains("TOKEN") || key_upper.contains("KEY") ||
                                   key_upper.contains("PRIVATE") {
                        if val.len() > 8 {
                            format!("{}...{}", &val[..4], &val[val.len()-4..])
                        } else {
                            "****".into()
                        }
                    } else {
                        val
                    };
                    output_json(&serde_json::json!({
                        "key": key_upper,
                        "value": display,
                        "is_secret": key_upper.contains("SECRET") || key_upper.contains("PASSWORD") ||
                                       key_upper.contains("TOKEN") || key_upper.contains("KEY") ||
                                       key_upper.contains("PRIVATE"),
                    }));
                }
                Err(_) => {
                    output_json(&serde_json::json!({
                        "key": key_upper,
                        "value": null,
                        "error": format!("'{key_upper}' is not set."),
                    }));
                }
            }
        }
        ConfigAction::List => {
            // Read from .env file to show all keys (without loading into env)
            let mut entries: Vec<serde_json::Value> = Vec::new();
            if env_path.exists() {
                let content = std::fs::read_to_string(&env_path)?;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once('=') {
                        let k = k.trim().to_uppercase();
                        let is_secret = k.contains("SECRET") || k.contains("PASSWORD") ||
                                        k.contains("TOKEN") || k.contains("KEY") ||
                                        k.contains("PRIVATE");
                        let display = if is_secret {
                            if v.len() > 8 {
                                format!("{}...{}", &v[..4], &v[v.len()-4..])
                            } else if !v.is_empty() {
                                "****".into()
                            } else {
                                "(empty)".into()
                            }
                        } else {
                            v.to_string()
                        };
                        entries.push(serde_json::json!({
                            "key": k,
                            "value": display,
                            "is_secret": is_secret,
                        }));
                    }
                }
            }
            output_json(&serde_json::json!({
                "path": env_path.display().to_string(),
                "exists": env_path.exists(),
                "count": entries.len(),
                "entries": entries,
            }));
        }
    }
    Ok(())
}

// ── X (Twitter) Handler ──────────────────────────────────────






// ── Import Handler ───────────────────────────────────────────

async fn handle_import_with_state(state: &AppState, provider_name: &str, count: u32) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let integration = find_integration(&state, user_id, provider_name).await?;
    let token = crate::crypto::maybe_decrypt_token(&integration.access_token, state.token_key.as_ref());

    let provider = state.providers.get(provider_name)
        .ok_or_else(|| anyhow::anyhow!("Provider '{}' not found in registry", provider_name))?;

    let posts = provider.get_recent_posts(&token, &integration.internal_id, count).await
        .map_err(|e| anyhow::anyhow!("Failed to get posts from {provider_name}: {e}"))?;

    let mut imported = 0u32;
    for post in &posts {
        let media_val = serde_json::to_value(&post.media).unwrap_or_default();
        let metadata_val = post.metadata.clone().unwrap_or_default();
        match db::queries::insert_external_post(
            &state.db,
            user_id,
            provider_name,
            &post.platform_post_id,
            &post.text,
            post.author_name.as_deref(),
            post.author_handle.as_deref(),
            post.author_avatar.as_deref(),
            post.created_at,
            post.url.as_deref(),
            &media_val,
            &metadata_val,
        )
        .await
        {
            Ok(Some(_)) => imported += 1,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("Failed to import post {}: {e}", post.platform_post_id);
            }
        }
    }

    output_json(&serde_json::json!({
        "provider": provider_name,
        "imported": imported,
        "total": posts.len(),
        "posts": posts,
    }));
    Ok(())
}

// ── Feed Handler ────────────────────────────────────────────

async fn handle_feed_with_state(state: &AppState, provider: Option<&str>, limit: u32) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let limit = limit.min(100) as i64;

    let posts = crate::db::queries::list_all_external_posts(
        &state.db,
        user_id,
        provider,
        None,
        limit,
    )
    .await?;

    output_json(&serde_json::json!({
        "provider": provider,
        "count": posts.len(),
        "posts": posts,
    }));
    Ok(())
}

// ── Instagram Handler ────────────────────────────────────────


// ── Comment Handler ──────────────────────────────────────────

async fn handle_comment_with_state(state: &AppState, action: CommentAction) -> anyhow::Result<()> {match action {
        CommentAction::Get { integration_id, post_id, limit } => {
            let input = crate::mcp::tools_comments::GetCommentsInput {
                integration_id,
                post_id,
                limit: Some(limit),
            };
            let result = crate::mcp::tools_comments::get_comments(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        CommentAction::Reply { integration_id, comment_id, content } => {
            let input = crate::mcp::tools_comments::ReplyToCommentInput {
                integration_id,
                comment_id,
                content,
            };
            let result = crate::mcp::tools_comments::reply_to_comment(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        CommentAction::Delete { integration_id, comment_id } => {
            let input = crate::mcp::tools_comments::DeleteCommentInput {
                integration_id,
                comment_id,
            };
            let result = crate::mcp::tools_comments::delete_comment(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
    }
    Ok(())
}

// ── DM Handler ───────────────────────────────────────────────

async fn handle_dm_with_state(state: &AppState, action: DmAction) -> anyhow::Result<()> {match action {
        DmAction::Send { integration_id, recipient, content } => {
            let input = crate::mcp::tools_dm::SendDmInput {
                integration_id,
                recipient,
                content,
            };
            let result = crate::mcp::tools_dm::send_dm(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        DmAction::List { integration_id, limit } => {
            let input = crate::mcp::tools_dm::ListDmInput {
                integration_id,
                limit: Some(limit),
            };
            let result = crate::mcp::tools_dm::list_dm_conversations(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        DmAction::Messages { integration_id, conversation_id, limit } => {
            let input = crate::mcp::tools_dm::GetDmInput {
                integration_id,
                conversation_id,
                limit: Some(limit),
            };
            let result = crate::mcp::tools_dm::get_dm_messages(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
    }
    Ok(())
}

// ── Automation Handler ───────────────────────────────────────

async fn handle_automation_with_state(state: &AppState, action: AutomationAction) -> anyhow::Result<()> {match action {
        AutomationAction::Create { integration_id, name, trigger_type, response_template, response_type } => {
            let input = crate::mcp::tools_automation::CreateRuleInput {
                integration_id,
                name,
                trigger_type,
                trigger_filter: serde_json::json!({}),
                response_template,
                response_type,
                ai_model: None,
                cooldown_minutes: None,
                max_responses_per_hour: None,
            };
            let result = crate::mcp::tools_automation::create_rule(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        AutomationAction::List { integration_id } => {
            let input = crate::mcp::tools_automation::ListRulesInput {
                integration_id,
            };
            let result = crate::mcp::tools_automation::list_rules(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        AutomationAction::Update { rule_id, name, response_template, is_active } => {
            let input = crate::mcp::tools_automation::UpdateRuleInput {
                rule_id,
                name,
                trigger_filter: None,
                response_template,
                response_type: None,
                ai_model: None,
                is_active,
                cooldown_minutes: None,
                max_responses_per_hour: None,
            };
            let result = crate::mcp::tools_automation::update_rule(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        AutomationAction::Delete { rule_id } => {
            let input = crate::mcp::tools_automation::DeleteRuleInput {
                rule_id,
            };
            let result = crate::mcp::tools_automation::delete_rule(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        AutomationAction::Logs { rule_id, limit } => {
            let input = crate::mcp::tools_automation::GetLogsInput {
                rule_id,
                limit: Some(limit as i64),
            };
            let result = crate::mcp::tools_automation::get_logs(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
    }
    Ok(())
}



// Unified Post Handler
async fn handle_post_with_state(state: &AppState, text: &str,
    platforms: Option<&str>,
    media: Option<&str>,
    schedule: Option<&str>,
    first_comment: Option<&str>,) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;

    let integration_ids: Vec<String> = if let Some(platforms_str) = platforms {
        let requested: Vec<&str> = platforms_str.split(',').map(|s| s.trim()).collect();
        integrations.iter()
            .filter(|i| requested.iter().any(|p| i.provider_identifier == *p))
            .map(|i| i.id.to_string())
            .collect()
    } else {
        integrations.iter().map(|i| i.id.to_string()).collect()
    };

    if integration_ids.is_empty() {
        return Err(anyhow::anyhow!(
            "No matching integrations found. Use 'social-forge providers' to see connected accounts."
        ));
    }

    let input = crate::mcp::tools_posts::StagePostInput {
        content: text.to_string(),
        media: media.map(|m| {
            serde_json::json!(m.split(',').map(|s| s.trim()).filter(|s| !s.is_empty())
                .map(|u| serde_json::json!({"url": u})).collect::<Vec<_>>())
        }),
        integration_ids,
        settings: Some(serde_json::json!({})),
        scheduled_at: schedule.map(String::from),
        first_comment: first_comment.map(String::from),
    };

    let result = crate::mcp::tools_posts::stage_post(&state, &input).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    output_json(&serde_json::to_value(result.0).unwrap_or_default());
    Ok(())
}


// Stage Handler
async fn handle_stage_with_state(state: &AppState, text: &str,
    integrations_str: Option<&str>,
    platforms_str: Option<&str>,
    media: Option<&str>,
    schedule: Option<&str>,
    preview_only: bool,
    first_comment: Option<&str>,) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;
    let all_integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let integration_ids: Vec<Uuid> = if let Some(iids) = integrations_str {
        iids.split(',').map(|s| s.trim()).filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(s).map_err(|_| anyhow::anyhow!("Invalid integration_id: {s}")))
            .collect::<Result<_, _>>()?
    } else if let Some(plats) = platforms_str {
        let platform_names: Vec<&str> = plats.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        all_integrations.iter()
            .filter(|i| platform_names.iter().any(|p| {
                i.provider_identifier == *p || i.provider_identifier.starts_with(&format!("{p}-"))
            }))
            .map(|i| i.id)
            .collect()
    } else {
        all_integrations.iter().map(|i| i.id).collect()
    };
    if integration_ids.is_empty() {
        return Err(anyhow::anyhow!("No integration IDs specified and no connected accounts found."));
    }
    let media_json = match media {
        Some(m) => {
            let urls: Vec<String> = m.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            serde_json::json!(urls.iter().map(|u| serde_json::json!({"url": u})).collect::<Vec<_>>())
        }
        None => serde_json::json!([]),
    };
    let scheduled_at = match schedule {
        Some(s) => Some(chrono::DateTime::parse_from_rfc3339(s)
            .map_err(|_| anyhow::anyhow!("Invalid date format, use ISO8601"))?
            .with_timezone(&chrono::Utc)),
        None => None,
    };
    if preview_only {
        let previews: Vec<serde_json::Value> = integration_ids.iter().filter_map(|iid| {
            let integration = all_integrations.iter().find(|i| i.id == *iid)?;
            let segments = crate::services::content_splitter::split_content(text, &integration.provider_identifier, 4);
            Some(serde_json::json!({
                "provider": integration.provider_identifier,
                "limit": crate::services::content_splitter::platform_limit(&integration.provider_identifier),
                "segments": segments.len(),
                "posts": segments.iter().map(|s| serde_json::json!({"sequence": s.sequence, "total": s.total, "content": s.content, "char_count": s.content.len()})).collect::<Vec<_>>(),
            }))
        }).collect();
        output_json(&serde_json::json!({"preview": true, "total_integrations": previews.len(), "integrations": previews}));
        return Ok(());
    }
    let request = crate::services::staging::StagingRequest {
        content: text.to_string(),
        media: media_json,
        integration_ids,
        settings: serde_json::json!({}),
        scheduled_at,
        first_comment: first_comment.map(String::from),
    };
    crate::services::staging::validate_staging_request(&request).map_err(|e| anyhow::anyhow!("Validation failed: {e}"))?;
    let result = crate::services::staging::stage_post(&state.db, user_id, request).await.map_err(|e| anyhow::anyhow!("Staging failed: {e}"))?;
    let staged: Vec<serde_json::Value> = result.staged.into_iter().map(|s| {
        serde_json::json!({"post_id": s.post_id.to_string(), "provider": s.provider, "sequence": s.sequence, "total_segments": s.total_segments, "state": s.state})
    }).collect();
    output_json(&serde_json::json!({"status": "staged", "total_posts": result.total_posts, "warnings": result.warnings, "staged": staged}));
    Ok(())
}

// Carousel Handler
async fn handle_carousel_with_state(state: &AppState, text: &str,
    integration_id_str: &str,
    media_str: &str,
    title: Option<&str>,
    schedule: Option<&str>,) -> anyhow::Result<()> {let media_urls: Vec<String> = media_str.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if media_urls.len() < 2 {
        return Err(anyhow::anyhow!("Carousel requires at least 2 media URLs"));
    }

    let scheduled_at = match schedule {
        Some(s) => Some(chrono::DateTime::parse_from_rfc3339(s)
            .map_err(|_| anyhow::anyhow!("Invalid date format, use ISO8601"))?
            .with_timezone(&chrono::Utc)),
        None => None,
    };

    let media_json = serde_json::json!(media_urls);
    let user_id = resolve_user(&state).await?;
    let integration_id = Uuid::parse_str(integration_id_str)
        .map_err(|_| anyhow::anyhow!("Invalid integration_id format"))?;

    let post = crate::services::posts::PostService::create(
        &state.db,
        &state.broadcast,
        crate::services::posts::CreatePostInput {
            user_id,
            integration_id,
            content: text.to_string(),
            title: title.map(String::from),
            media_urls: media_json,
            scheduled_at,
            settings: serde_json::json!({}),
            first_comment: None,
            state: None,
        },
    ).await.map_err(|e| anyhow::anyhow!("Failed to create carousel: {e}"))?;

    output_json(&serde_json::json!({
        "id": post.id.to_string(),
        "state": post.state.to_string(),
        "scheduled_at": post.scheduled_at.map(|d| d.to_rfc3339()),
        "created_at": post.created_at.to_rfc3339(),
    }));
    Ok(())
}

// Media Handler
async fn handle_media_with_state(state: &AppState, action: MediaAction) -> anyhow::Result<()> {match action {
        MediaAction::Upload { path, alt: _ } => {
            let result = crate::mcp::tools_media::upload_from_path(&state, &path).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        MediaAction::List { limit, search } => {
            let input = crate::mcp::tools_media::MediaListInput {
                limit: Some(limit.min(200) as i64),
                search,
            };
            let result = crate::mcp::tools_media::list_media(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
        MediaAction::Download { url, output } => {
            let resp = reqwest::Client::new().get(&url).send().await
                .map_err(|e| anyhow::anyhow!("Failed to download: {e}"))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(anyhow::anyhow!("Download failed with HTTP {}", status));
            }
            let content_type = resp.headers().get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let bytes = resp.bytes().await
                .map_err(|e| anyhow::anyhow!("Failed to read response: {e}"))?;
            let out_path = if std::path::Path::new(&output).is_dir() {
                let filename = url.rsplit('/').next().unwrap_or("download");
                let filename = filename.split('?').next().unwrap_or(filename);
                std::path::Path::new(&output).join(filename)
            } else {
                std::path::PathBuf::from(&output)
            };
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("Failed to create directory: {e}"))?;
            }
            std::fs::write(&out_path, &bytes)
                .map_err(|e| anyhow::anyhow!("Failed to write file: {e}"))?;
            output_json(&serde_json::json!({
                "status": "downloaded",
                "url": url,
                "path": out_path.display().to_string(),
                "size": bytes.len() as i64,
                "content_type": content_type,
            }));
        }
        MediaAction::UploadBatch { paths } => {
            if paths.is_empty() {
                return Err(anyhow::anyhow!("At least one file path is required"));
            }
            let input = crate::mcp::tools_media::MediaUploadBatchInput {
                paths,
                alt: None,
            };
            let result = crate::mcp::tools_media::upload_batch(&state, &input).await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output_json(&serde_json::to_value(result.0).unwrap_or_default());
        }
    }
    Ok(())
}

// ── Posts Handler ─────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_posts_create_helper(
    state: &crate::api::AppState,
    user_id: uuid::Uuid,
    content: &str,
    integrations: &str,
    schedule: Option<&str>,
    title: Option<&str>,
    media: Option<&str>,
    first_comment: Option<&str>,
    settings: Option<&str>,
    state_override: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let integration_ids: Vec<uuid::Uuid> = integrations
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<uuid::Uuid>().ok())
        .collect();
    if integration_ids.is_empty() {
        return Err(anyhow::anyhow!("At least one valid integration UUID required"));
    }

    let scheduled_at = match schedule {
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|_| anyhow::anyhow!("Invalid schedule date, use ISO 8601 / RFC 3339"))?
                .with_timezone(&chrono::Utc)
        ),
        None => None,
    };

    let media_json: serde_json::Value = match media {
        Some(m) => {
            if m.trim_start().starts_with('[') {
                serde_json::from_str(m).unwrap_or_else(|_| serde_json::json!([]))
            } else {
                let urls: Vec<String> = m.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                serde_json::json!(urls)
            }
        },
        None => serde_json::json!([]),
    };

    let settings_json: serde_json::Value = match settings {
        Some(s) => serde_json::from_str::<serde_json::Value>(s)
            .map_err(|e| anyhow::anyhow!("Invalid settings JSON: {e}"))?,
        None => serde_json::json!({}),
    };

    let target_state = match state_override {
        Some("draft") => Some(crate::db::models::PostState::Draft),
        Some("queued") => Some(crate::db::models::PostState::Queued),
        Some("published") => Some(crate::db::models::PostState::Published),
        Some("error") => Some(crate::db::models::PostState::Error),
        Some(other) => return Err(anyhow::anyhow!("Invalid --state '{other}'. Use draft|queued|published|error.")),
        None if scheduled_at.is_some() => Some(crate::db::models::PostState::Queued),
        None => None,
    };

    let first_comment_owned = first_comment.and_then(|s| if s.is_empty() { None } else { Some(s.to_string()) });
    let title_owned = title.map(String::from);

    fn build(
        iid: uuid::Uuid,
        user_id: uuid::Uuid,
        content: &str,
        title_owned: Option<String>,
        media_json: serde_json::Value,
        scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
        settings_json: serde_json::Value,
        first_comment_owned: Option<String>,
        target_state: Option<crate::db::models::PostState>,
    ) -> crate::services::posts::CreatePostInput {
        crate::services::posts::CreatePostInput {
            user_id,
            integration_id: iid,
            content: content.to_string(),
            title: title_owned,
            media_urls: media_json,
            scheduled_at,
            settings: settings_json,
            first_comment: first_comment_owned,
            state: target_state,
        }
    }

    let mut created: Vec<serde_json::Value> = Vec::new();
    let count = integration_ids.len();
    for &iid in &integration_ids {
        let input = build(
            iid, user_id, content,
            title_owned.clone(),
            media_json.clone(),
            scheduled_at,
            settings_json.clone(),
            first_comment_owned.clone(),
            target_state.clone(),
        );
        match crate::services::posts::PostService::create(
            &state.db, &state.broadcast, input
        ).await {
            Ok(post) => created.push(serde_json::json!({
                "id": post.id.to_string(),
                "integration_id": iid.to_string(),
                "state": post.state.to_string(),
                "scheduled_at": post.scheduled_at.map(|d| d.to_rfc3339()),
                "created_at": post.created_at.to_rfc3339(),
            })),
            Err(e) => {
                if count == 1 {
                    return Err(anyhow::anyhow!("Create failed: {e}"));
                }
                tracing::warn!("Best-effort create for integration {iid} failed: {e}");
                created.push(serde_json::json!({
                    "integration_id": iid.to_string(),
                    "error": e,
                }));
            }
        }
    }
    Ok(serde_json::json!({
        "ok": true,
        "count": created.len(),
        "posts": created,
    }))
}

async fn handle_posts_with_state(state: &AppState, action: PostsAction) -> anyhow::Result<()> {let user_id = resolve_user(&state).await?;

    let result: anyhow::Result<serde_json::Value> = match action {
        PostsAction::List { state: post_state, limit, offset } => {
            let limit = limit.min(200) as i64;
            let offset = offset as i64;
            match crate::services::posts::PostService::list(
                &state.db, user_id, post_state.as_deref(),
                limit, offset, true
            ).await {
                Ok((posts, total)) => {
                    let summaries: Vec<_> = posts.iter().map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "content": p.content,
                            "state": p.state,
                            "title": p.title,
                            "integration_id": p.integration_id,
                            "scheduled_at": p.scheduled_at.map(|dt| dt.to_rfc3339()),
                            "published_at": p.published_at.map(|dt| dt.to_rfc3339()),
                            "platform_post_id": p.platform_post_id,
                            "platform_post_url": p.platform_post_url,
                            "error_message": p.error_message,
                            "first_comment": p.first_comment,
                            "media": p.media,
                            "settings": p.settings,
                            "created_at": p.created_at.to_rfc3339(),
                            "updated_at": p.updated_at.to_rfc3339(),
                        })
                    }).collect();
                    Ok(serde_json::json!({
                        "total": total.unwrap_or(0),
                        "limit": limit,
                        "offset": offset,
                        "count": summaries.len(),
                        "posts": summaries,
                    }))
                },
                Err(e) => Err(anyhow::anyhow!("DB error: {e}")),
            }
        }
        PostsAction::Get { id } => {
            match id.parse::<uuid::Uuid>() {
                Ok(post_id) => match crate::db::queries::get_post(&state.db, post_id, user_id).await {
                    Ok(Some(p)) => Ok(serde_json::json!({
                        "id": p.id,
                        "content": p.content,
                        "state": p.state,
                        "title": p.title,
                        "integration_id": p.integration_id,
                        "scheduled_at": p.scheduled_at.map(|dt| dt.to_rfc3339()),
                        "published_at": p.published_at.map(|dt| dt.to_rfc3339()),
                        "platform_post_id": p.platform_post_id,
                        "platform_post_url": p.platform_post_url,
                        "error_message": p.error_message,
                        "first_comment": p.first_comment,
                        "media": p.media,
                        "settings": p.settings,
                        "created_at": p.created_at.to_rfc3339(),
                        "updated_at": p.updated_at.to_rfc3339(),
                    })),
                    Ok(None) => Err(anyhow::anyhow!("Post {id} not found")),
                    Err(e) => Err(anyhow::anyhow!("DB error: {e}")),
                },
                Err(_) => Err(anyhow::anyhow!("Invalid post ID: {id}")),
            }
        }
        PostsAction::Create {
            content, integrations, schedule, title, media, first_comment, settings, state: state_override
        } => {
            handle_posts_create_helper(
                &state, user_id, &content, &integrations,
                schedule.as_deref(), title.as_deref(),
                media.as_deref(), first_comment.as_deref(),
                settings.as_deref(), state_override.as_deref(),
            ).await
        }
        PostsAction::Schedule { id, scheduled_at } => {
            let post_id = match id.parse::<uuid::Uuid>() {
                Ok(id) => id,
                Err(_) => return output_error("Invalid post ID"),
            };
            let dt = chrono::DateTime::parse_from_rfc3339(&scheduled_at)
                .map_err(|e: chrono::ParseError| anyhow::anyhow!("Invalid scheduled_at: {}", e))?
                .with_timezone(&chrono::Utc);
            crate::services::posts::PostService::schedule(
                &state.db, &state.broadcast, user_id, post_id, dt
            ).await
                .map(|post| serde_json::json!({
                    "ok": true,
                    "id": post.id.to_string(),
                    "state": post.state.to_string(),
                    "scheduled_at": post.scheduled_at.map(|d| d.to_rfc3339()),
                }))
                .map_err(|e| anyhow::anyhow!("Schedule failed: {e}"))
        }
        PostsAction::Publish { id } => {
            let post_id = match id.parse::<uuid::Uuid>() {
                Ok(id) => id,
                Err(_) => return output_error("Invalid post ID"),
            };
            crate::services::posts::PostService::publish(
                &state.db, &state.providers, &state.broadcast, user_id, post_id, state.token_key
            ).await
                .map(|platform_url| serde_json::json!({
                    "ok": true,
                    "id": id,
                    "state": "published",
                    "platform_post_url": platform_url,
                }))
                .map_err(|e| anyhow::anyhow!("Publish failed: {e}"))
        }
        PostsAction::Delete { id } => {
            let post_id = match id.parse::<uuid::Uuid>() {
                Ok(id) => id,
                Err(_) => return output_error("Invalid post ID"),
            };
            crate::services::posts::PostService::delete(
                &state.db, &state.broadcast, user_id, post_id
            ).await
                .map(|deleted| serde_json::json!({"ok": deleted, "id": id}))
                .map_err(|e| anyhow::anyhow!("Delete failed: {e}"))
        }
        PostsAction::FindSlot { integration } => {
            let integration_id = integration.as_deref().and_then(|s| uuid::Uuid::parse_str(s).ok());
            crate::services::posts::PostService::find_slot(
                &state.db, user_id, integration_id
            ).await
                .map(|slot| serde_json::json!({
                    "ok": true,
                    "scheduled_at": slot.to_rfc3339(),
                    "integration_id": integration,
                }))
                .map_err(|e| anyhow::anyhow!("Find-slot failed: {e}"))
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e.to_string()),
    }
    Ok(())
}

// Split Preview Handler
fn handle_split_preview(text: &str, platforms: Option<&str>) -> anyhow::Result<()> {
    let provider_list: Vec<&str> = match platforms {
        Some(p) => p.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect(),
        None => vec!["x", "bluesky", "linkedin", "reddit", "mastodon", "facebook", "instagram", "youtube"],
    };
    let preview: Vec<serde_json::Value> = provider_list.iter().map(|&provider| {
        let segments = crate::services::content_splitter::split_content(text, provider, 4);
        let limit = crate::services::content_splitter::platform_limit(provider);
        let needs_split = crate::services::content_splitter::needs_splitting(text, provider);
        serde_json::json!({
            "provider": provider, "char_limit": limit, "content_length": text.len(), "needs_splitting": needs_split, "segments": segments.len(),
            "posts": segments.iter().map(|s| serde_json::json!({"sequence": s.sequence, "total": s.total, "content": s.content, "char_count": s.content.len()})).collect::<Vec<_>>(),
        })
    }).collect();
    output_json(&serde_json::json!({"content_length": text.len(), "platforms": preview}));
    Ok(())
}

// ── Main CLI Dispatcher ──────────────────────────────────────

pub async fn run_cli(cli: Cli) -> anyhow::Result<()> {
    // Commands that don't need AppState — handle early, skip init_state().
    match cli.command {
        Command::Serve { .. } | Command::Mcp => {
            return Err(anyhow::anyhow!(
                "Serve/Mcp are handled in main.rs before calling run_cli — this is a bug"
            ));
        }
        Command::Init => return handle_init(),
        Command::Config { action } => return handle_config(action),
        Command::SplitPreview { text, platforms } => {
            return handle_split_preview(&text, platforms.as_deref());
        }
        _ => {}
    }

    // Initialize AppState once for all commands that need it.
    let state = init_state().await?;

    match cli.command {
        Command::Init | Command::Config { .. } | Command::SplitPreview { .. } | Command::Serve { .. } | Command::Mcp => {
            unreachable!("handled above")
        }

        Command::Providers => handle_providers_with_state(&state).await,
        Command::Connect { provider } => handle_connect_with_state(&state, &provider).await,
        Command::Doctor => handle_doctor_with_state(&state).await,
        Command::Setup => handle_setup().await,
        Command::ConnectAll => handle_connect_all_with_state(&state).await,

        // ── Core Platform Handlers ──────────────────────────
        Command::X { action } => crate::cli::platforms::x::handle(action, &state).await,
        Command::Reddit { action } => crate::cli::platforms::reddit::handle(action, &state).await,
        Command::Linkedin { action } => crate::cli::platforms::linkedin::handle(action, &state).await,
        Command::LinkedinPage { action } => crate::cli::platforms::linkedin_page::handle(action, &state).await,
        Command::Facebook { action } => crate::cli::platforms::facebook::handle(action, &state).await,
        Command::Instagram { action } => crate::cli::platforms::instagram::handle(action, &state).await,
        Command::Youtube { action } => crate::cli::platforms::youtube::handle(action, &state).await,
        Command::Bluesky { action } => crate::cli::platforms::bluesky::handle(action, &state).await,
        Command::Mastodon { action } => crate::cli::platforms::mastodon::handle(action, &state).await,

        // ── Unified Commands ────────────────────────────────
        Command::Import { provider, count } => handle_import_with_state(&state, &provider, count).await,
        Command::Feed { provider, limit } => handle_feed_with_state(&state, provider.as_deref(), limit).await,
        Command::Comment { action } => handle_comment_with_state(&state, action).await,
        Command::Dm { action } => handle_dm_with_state(&state, action).await,
        Command::Automation { action } => handle_automation_with_state(&state, action).await,
        Command::Posts { action } => handle_posts_with_state(&state, action).await,
        Command::Post { text, platforms, media, schedule, first_comment } => {
            handle_post_with_state(&state, &text, platforms.as_deref(), media.as_deref(), schedule.as_deref(), first_comment.as_deref()).await
        }
        Command::Stage { text, integrations, platforms, media, schedule, preview, first_comment } => {
            handle_stage_with_state(&state, &text, integrations.as_deref(), platforms.as_deref(), media.as_deref(), schedule.as_deref(), preview, first_comment.as_deref()).await
        }
        Command::Carousel { text, integration, media, title, schedule } => {
            handle_carousel_with_state(&state, &text, &integration, &media, title.as_deref(), schedule.as_deref()).await
        }
        Command::Media { action } => handle_media_with_state(&state, action).await,

        // ── New Platform Handlers (delegate to platform modules) ──
        Command::Tiktok { action } => crate::cli::platforms::tiktok::handle(action, &state).await,
        Command::Threads { action } => crate::cli::platforms::threads::handle(action, &state).await,
        Command::Discord { action } => crate::cli::platforms::discord::handle(action, &state).await,
        Command::Slack { action } => crate::cli::platforms::slack::handle(action, &state).await,
        Command::TelegramBot { action } => crate::cli::platforms::telegram_bot::handle(action, &state).await,
        Command::TelegramUser { action } => crate::cli::platforms::telegram_user::handle(action, &state).await,
        Command::Whatsapp { action } => crate::cli::platforms::whatsapp::handle(action, &state).await,
        Command::Pinterest { action } => crate::cli::platforms::pinterest::handle(action, &state).await,
        Command::Github { action } => crate::cli::platforms::github::handle(action, &state).await,
        Command::Wordpress { action } => crate::cli::platforms::wordpress::handle(action, &state).await,
        Command::Hashnode { action } => crate::cli::platforms::hashnode::handle(action, &state).await,
        Command::MediumBlog { action } => crate::cli::platforms::medium_blog::handle(action, &state).await,
        Command::Devto { action } => crate::cli::platforms::devto::handle(action, &state).await,
        Command::Skool { action } => crate::cli::platforms::skool::handle(action, &state).await,
        Command::Google { action } => crate::cli::platforms::google::handle(action, &state).await,
        Command::Gdrive { action } => crate::cli::platforms::drive::handle(action, &state).await,
        Command::Gcal { action } => crate::cli::platforms::gcal::handle(action, &state).await,
        Command::GmailOps { action } => crate::cli::platforms::gmail::handle(action, &state).await,
        Command::Webhooks { action } => crate::cli::platforms::webhooks::handle(action, &state).await,
        Command::Notifications { action } => crate::cli::platforms::notifications::handle(action, &state).await,
        Command::Tags { action } => crate::cli::platforms::tags::handle(action, &state).await,
        Command::Analytics { action } => crate::cli::platforms::analytics::handle(action, &state).await,
    }
}
