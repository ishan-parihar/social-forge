// ─── CLI Handler Implementations ─────────────────────────────────
// Dispatches CLI subcommands to provider methods, resolving tokens
// the same way MCP tools do (DB > browser extraction > env vars).

use uuid::Uuid;

use crate::api::rate_limiter::AuthRateLimiter;
use crate::api::AppState;
use crate::config::Config;
use crate::crypto;
use crate::db;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;
use crate::social::SocialProvider;

use super::{Cli, Command, XAction, RedditAction, RedditModAction, LinkedinAction, LinkedinPageAction, FacebookAction, InstagramAction};
use crate::social::TargetInfo;
use crate::db::models::Integration;

// ── Output Helpers ───────────────────────────────────────────

fn output_json(value: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

fn output_error(msg: &str) -> anyhow::Result<()> {
    eprintln!("{}", serde_json::json!({"error": msg}));
    std::process::exit(1);
}

// ── Target Discovery Helpers ─────────────────────────────────

async fn fetch_targets(state: &AppState, integration: &Integration) -> anyhow::Result<Vec<TargetInfo>> {
    let provider_obj = state.providers.get(&integration.provider_identifier)
        .ok_or_else(|| anyhow::anyhow!("Provider not found in registry"))?;

    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&integration.access_token, key).ok())
        .unwrap_or_else(|| integration.access_token.clone());

    let targets = provider_obj.targets(&token).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch targets: {}", e))?;

    Ok(targets)
}

async fn find_integration(state: &AppState, user_id: Uuid, provider: &str) -> anyhow::Result<Integration> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    integrations.into_iter()
        .find(|i| i.provider_identifier == provider)
        .ok_or_else(|| anyhow::anyhow!("No {} integration found", provider))
}

fn pick_target_interactive(targets: &[TargetInfo], provider: &str) -> anyhow::Result<String> {
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
    let rate_limiter = AuthRateLimiter::new(5, 60);
    Ok(AppState {
        db,
        config,
        broadcast: broadcaster,
        providers,
        rate_limiter,
        token_key,
        telegram_client_manager: None,
        wa_client: None,
    })
}

// ── User Resolution ──────────────────────────────────────────

async fn resolve_user(state: &AppState) -> anyhow::Result<Uuid> {
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

async fn find_x_token(state: &AppState, user_id: Uuid) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let x_integrations: Vec<_> = integrations.into_iter()
        .filter(|i| i.provider_identifier == "x")
        .collect();

    if let Some(preferred) = x_integrations.iter().find(|i| i.access_token.starts_with('{')) {
        let token = preferred.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
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
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok((token, oauth.internal_id.clone()));
    }

    anyhow::bail!("No X/Twitter integration found")
}

async fn find_reddit_token(state: &AppState, user_id: Uuid) -> anyhow::Result<String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let reddit_integrations: Vec<_> = integrations.into_iter()
        .filter(|i| i.provider_identifier == "reddit")
        .collect();

    if let Some(preferred) = reddit_integrations.iter().find(|i| i.access_token.starts_with('{')) {
        let token = preferred.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok(token);
    }

    if let Some(cookies) = crate::social::reddit_cookies::extract_reddit_cookies() {
        return Ok(crate::social::reddit_cookies::build_cookie_token(
            &cookies.reddit_session, cookies.token_v2.as_deref(), Some(&cookies.cookie_string)
        ));
    }

    if let Some(oauth) = reddit_integrations.first() {
        let token = oauth.access_token.clone();
        let token = state.token_key.as_ref()
            .and_then(|key| crypto::decrypt_string(&token, key).ok())
            .unwrap_or(token);
        return Ok(token);
    }

    anyhow::bail!("No Reddit integration found")
}

async fn find_linkedin_token(state: &AppState, user_id: Uuid) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let li = integrations.iter()
        .find(|i| i.provider_identifier == "linkedin")
        .ok_or_else(|| anyhow::anyhow!("No LinkedIn account connected"))?;
    let token = li.access_token.clone();
    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&token, key).ok())
        .unwrap_or(token);
    Ok((token, li.internal_id.clone()))
}

async fn find_linkedin_page_token(state: &AppState, user_id: Uuid, page_id: &str) -> anyhow::Result<(String, String)> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let lip = integrations.iter()
        .find(|i| i.provider_identifier == "linkedin-page" && i.internal_id == page_id)
        .ok_or_else(|| anyhow::anyhow!("LinkedIn Page '{}' not connected", page_id))?;
    let token = lip.access_token.clone();
    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&token, key).ok())
        .unwrap_or(token);
    Ok((token, lip.internal_id.clone()))
}

async fn find_facebook_page_token(state: &AppState, user_id: Uuid, page_id: &str) -> anyhow::Result<String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let page = integrations.iter()
        .find(|i| i.provider_identifier == "facebook" && i.internal_id == page_id)
        .ok_or_else(|| anyhow::anyhow!("Facebook page '{}' not connected", page_id))?;
    let token = page.access_token.clone();
    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&token, key).ok())
        .unwrap_or(token);
    Ok(token)
}

async fn find_instagram_token(state: &AppState, user_id: Uuid, ig_id: &str) -> anyhow::Result<String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id).await?;
    let ig = integrations.iter()
        .find(|i| (i.provider_identifier == "instagram" || i.provider_identifier == "instagram-standalone") && i.internal_id == ig_id)
        .ok_or_else(|| anyhow::anyhow!("Instagram account '{}' not connected", ig_id))?;
    let token = ig.access_token.clone();
    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&token, key).ok())
        .unwrap_or(token);
    Ok(token)
}

// ── Main Dispatcher ──────────────────────────────────────────

pub async fn run_cli(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Serve { .. } | Command::Mcp => {
            unreachable!("Serve/Mcp handled in main.rs before calling run_cli");
        }
        Command::Init => handle_init(),
        Command::Providers => handle_providers().await,
        Command::Connect { provider } => handle_connect(&provider),
        Command::X { action } => handle_x(action).await,
        Command::Reddit { action } => handle_reddit(action).await,
        Command::Linkedin { action } => handle_linkedin(action).await,
        Command::LinkedinPage { action } => handle_linkedin_page(action).await,
        Command::Facebook { action } => handle_facebook(action).await,
        Command::Instagram { action } => handle_instagram(action).await,
        Command::Import { provider, count } => handle_import(&provider, count).await,
        Command::Feed { provider, limit } => handle_feed(provider.as_deref(), limit).await,
    }
}

// ── Providers / Connect ──────────────────────────────────────

async fn handle_providers() -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
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

fn handle_connect(provider: &str) -> anyhow::Result<()> {
    let instructions = match provider {
        "x" => "Visit /api/public/connect/x-cookies to submit X/Twitter cookies, or set X_AUTH_TOKEN + X_CT0 env vars.",
        "reddit" => "Visit /api/public/connect/reddit-cookies to submit Reddit cookies.",
        "linkedin" => "Visit /api/public/connect/linkedin to start OAuth flow.",
        "facebook" => "Visit /api/public/connect/facebook to start OAuth flow.",
        "instagram" => "Visit /api/public/connect/instagram to start OAuth flow.",
        _ => "Unknown provider. Supported: x, reddit, linkedin, facebook, instagram",
    };
    output_json(&serde_json::json!({"instructions": instructions}));
    Ok(())
}

// ── X (Twitter) Handler ──────────────────────────────────────

async fn handle_x(action: XAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
    let (token, my_id) = match find_x_token(&state, user_id).await {
        Ok(t) => t,
        Err(e) => return output_error(&e.to_string()),
    };

    let mut provider = crate::social::x::XProvider::new(&state.config);
    provider.prepare_from_token(&token);

    let result: Result<serde_json::Value, String> = match action {
        XAction::Post { text } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            };
            provider.publish(&token, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        XAction::Timeline { count } => {
            provider.home_timeline(&token, &my_id, count, None).await
                .map_err(|e| e.to_string())
        }
        XAction::Search { query } => {
            provider.search_tweets(&token, &query, 20, None).await
                .map_err(|e| e.to_string())
        }
        XAction::Like { tweet_id } => {
            provider.like_tweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Retweet { tweet_id } => {
            provider.retweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Delete { tweet_id } => {
            provider.delete_tweet(&token, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::Bookmark { tweet_id } => {
            provider.bookmark_tweet(&token, &my_id, &tweet_id).await
                .map_err(|e| e.to_string())
        }
        XAction::User { username } => {
            provider.user_lookup_by_username(&token, &username).await
                .map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}

// ── Reddit Handler ───────────────────────────────────────────

async fn handle_reddit(action: RedditAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
    let token = match find_reddit_token(&state, user_id).await {
        Ok(t) => t,
        Err(e) => return output_error(&e.to_string()),
    };

    let mut provider = crate::social::reddit::RedditProvider::new(&state.config);
    provider.prepare_from_token(&token);

    let result: Result<serde_json::Value, String> = match action {
        RedditAction::Browse { subreddit, sort, limit } => {
            provider.browse(&token, &subreddit, &sort, limit, "all").await
                .map_err(|e| e.to_string())
        }
        RedditAction::Search { query, subreddit, sort } => {
            provider.search(&token, &query, subreddit.as_deref(), &sort, 25, "all").await
                .map_err(|e| e.to_string())
        }
        RedditAction::Post { title, text, url, target, targets } => {
            let subreddits: Vec<String> = if let Some(ref t) = targets {
                t.split(',').map(|s| s.trim().replace("r/", "")).filter(|s| !s.is_empty()).collect()
            } else if let Some(ref t) = target {
                vec![t.trim().replace("r/", "")]
            } else {
                let integration = find_integration(&state, user_id, "reddit").await?;
                match fetch_targets(&state, &integration).await {
                    Ok(targets) => {
                        let selected = pick_target_interactive(&targets, "reddit")?;
                        vec![selected.replace("r/", "")]
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not fetch targets ({}).", e);
                        eprintln!("Please specify a subreddit with --target or --targets.");
                        return Err(anyhow::anyhow!("No subreddit specified and target discovery failed"));
                    }
                }
            };

            let mut results = Vec::new();
            for sub in &subreddits {
                let kind = if url.is_some() { "link" } else { "self" };
                let text_val = text.as_deref().unwrap_or("");
                let url_val = url.as_deref().unwrap_or("");
                let mut form: Vec<(&str, &str)> = vec![
                    ("api_type", "json"), ("sr", sub), ("title", &title), ("kind", kind),
                ];
                if kind == "self" { form.push(("text", text_val)); }
                else { form.push(("url", url_val)); }

                let resp = reqwest::Client::new()
                    .post("https://oauth.reddit.com/api/submit")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
                    .form(&form).send().await
                    .map_err(|e| format!("Reddit submit failed: {e}"));
                let result = match resp {
                    Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse failed: {e}")),
                    Err(e) => Err(e),
                };
                results.push((sub.clone(), result));
            }

            if results.len() == 1 {
                let (_, result) = results.remove(0);
                result
            } else {
                let output: Vec<serde_json::Value> = results.into_iter().map(|(sub, result)| {
                    match result {
                        Ok(v) => serde_json::json!({"subreddit": sub, "status": "success", "result": v}),
                        Err(e) => serde_json::json!({"subreddit": sub, "status": "error", "error": e}),
                    }
                }).collect();
                Ok(serde_json::json!({"posts": output}))
            }
        }
        RedditAction::Comment { thing_id, text } => {
            let tid = if thing_id.starts_with("t3_") || thing_id.starts_with("t1_") {
                thing_id
            } else {
                format!("t3_{}", thing_id)
            };
            let resp = reqwest::Client::new()
                .post("https://oauth.reddit.com/api/comment")
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "social-forge:v0.1.0 (by /u/social_forge)")
                .form(&[("api_type", "json"), ("thing_id", &*tid), ("text", &*text)])
                .send().await
                .map_err(|e| format!("Reddit comment failed: {e}"));
            match resp {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse failed: {e}")),
                Err(e) => Err(e),
            }
        }
        RedditAction::Vote { thing_id, direction } => {
            let dir: i8 = match direction.as_str() {
                "up" => 1, "down" => -1, _ => 0,
            };
            provider.vote(&thing_id, dir).await.map_err(|e| e.to_string())
        }
        RedditAction::Save { thing_id } => {
            provider.save(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::Unsave { thing_id } => {
            provider.unsave(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::Delete { thing_id } => {
            provider.delete(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::User { username } => {
            provider.user_info(&token, &username, true, false).await
                .map_err(|e| e.to_string())
        }
        RedditAction::Inbox { folder } => {
            provider.inbox(&token, &folder, 25).await
                .map_err(|e| e.to_string())
        }
        RedditAction::Mod { action: mod_action } => {
            match mod_action {
                RedditModAction::Remove { thing_id, spam } => {
                    provider.mod_remove(&thing_id, spam).await.map_err(|e| e.to_string())
                }
                RedditModAction::Approve { thing_id } => {
                    provider.mod_approve(&thing_id).await.map_err(|e| e.to_string())
                }
                RedditModAction::Lock { thing_id } => {
                    provider.mod_lock(&thing_id).await.map_err(|e| e.to_string())
                }
                RedditModAction::Unlock { thing_id } => {
                    provider.mod_unlock(&thing_id).await.map_err(|e| e.to_string())
                }
            }
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}

// ── LinkedIn Personal Handler ────────────────────────────────

async fn handle_linkedin(action: LinkedinAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
    let (token, li_id) = match find_linkedin_token(&state, user_id).await {
        Ok(t) => t,
        Err(e) => return output_error(&e.to_string()),
    };

    let provider = crate::social::linkedin::LinkedInProvider::new(&state.config);

    let result: Result<serde_json::Value, String> = match action {
        LinkedinAction::Profile => {
            provider.get_profile(&token).await.map_err(|e| e.to_string())
        }
        LinkedinAction::Posts { limit } => {
            let author_urn = format!("urn:li:person:{li_id}");
            provider.get_posts(&token, &author_urn, limit).await.map_err(|e| e.to_string())
        }
        LinkedinAction::Post { text } => {
            let post = crate::social::PostContent {
                content: text,
                media: vec![],
                settings: serde_json::Value::Object(serde_json::Map::new()),
            };
            provider.publish(&token, &post).await
                .map(|r| serde_json::json!({"id": r.platform_post_id, "url": r.platform_post_url, "status": r.status}))
                .map_err(|e| e.to_string())
        }
        LinkedinAction::Delete { post_urn } => {
            let url = format!(
                "https://api.linkedin.com/v2/rest/posts/{}",
                urlencoding::encode(&post_urn)
            );
            let resp = reqwest::Client::new()
                .delete(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202601")
                .send().await
                .map_err(|e| format!("LinkedIn delete failed: {e}"));
            match resp {
                Ok(r) if r.status().is_success() => Ok(serde_json::json!({"deleted": true})),
                Ok(r) => Err(format!("LinkedIn delete failed ({})", r.status())),
                Err(e) => Err(e),
            }
        }
        LinkedinAction::Reactions { post_urn } => {
            let url = format!(
                "https://api.linkedin.com/v2/rest/reactions/(entity:{})",
                urlencoding::encode(&post_urn)
            );
            let resp = reqwest::Client::new()
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Restli-Protocol-Version", "2.0.0")
                .header("LinkedIn-Version", "202601")
                .send().await
                .map_err(|e| format!("LinkedIn reactions failed: {e}"));
            match resp {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}")),
                Err(e) => Err(e),
            }
        }
        LinkedinAction::Analytics => {
            let author_urn = format!("urn:li:person:{li_id}");
            provider.get_posts(&token, &author_urn, 5).await
                .map(|posts| serde_json::json!({"analytics_summary": posts}))
                .map_err(|e| e.to_string())
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}

// ── LinkedIn Page Handler ────────────────────────────────────

async fn handle_linkedin_page(action: LinkedinPageAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;

    let result: Result<serde_json::Value, String> = match action {
        LinkedinPageAction::List => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Ok(integrations) => {
                    let pages: Vec<serde_json::Value> = integrations.iter()
                        .filter(|i| i.provider_identifier == "linkedin-page")
                        .map(|i| serde_json::json!({"id": i.internal_id, "name": i.profile_name}))
                        .collect();
                    Ok(serde_json::json!({"pages": pages}))
                }
                Err(e) => Err(format!("DB error: {e}")),
            }
        }
        LinkedinPageAction::Post { page_id, text } => {
            match find_linkedin_page_token(&state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let body = serde_json::json!({
                        "author": format!("urn:li:organization:{page_id}"),
                        "commentary": text,
                        "visibility": "PUBLIC",
                        "distribution": {"feedDistribution": "MAIN_FEED"},
                        "lifecycleState": "PUBLISHED",
                    });
                    match reqwest::Client::new()
                        .post("https://api.linkedin.com/v2/rest/posts")
                        .header("Authorization", format!("Bearer {token}"))
                        .header("X-Restli-Protocol-Version", "2.0.0")
                        .header("LinkedIn-Version", "202601")
                        .header("Content-Type", "application/json")
                        .json(&body).send().await
                    {
                        Err(e) => Err(format!("LinkedIn post failed: {e}")),
                        Ok(resp) => {
                            let post_id = resp.headers().get("x-restli-id")
                                .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                            if resp.status().is_success() {
                                Ok(serde_json::json!({"post_id": post_id}))
                            } else {
                                Err(format!("LinkedIn page post failed ({})", resp.status()))
                            }
                        }
                    }
                }
            }
        }
        LinkedinPageAction::Analytics { page_id } => {
            match find_linkedin_page_token(&state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let provider = crate::social::linkedin_page::LinkedInPageProvider::new(&state.config);
                    provider.get_page_posts(&token, &page_id, 10).await
                        .map(|d| serde_json::json!({"data": d}))
                        .map_err(|e| e.to_string())
                }
            }
        }
        LinkedinPageAction::Followers { page_id } => {
            match find_linkedin_page_token(&state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let org_urn = format!("urn:li:organization:{page_id}");
                    let url = format!(
                        "https://api.linkedin.com/rest/networkSizes/{org_urn}?edgeType=CompanyFollowedByMember"
                    );
                    match reqwest::Client::new()
                        .get(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .header("LinkedIn-Version", "202601")
                        .header("X-Restli-Protocol-Version", "2.0.0")
                        .send().await
                    {
                        Err(e) => Err(format!("LinkedIn followers failed: {e}")),
                        Ok(resp) => match resp.json::<serde_json::Value>().await {
                            Err(e) => Err(format!("Parse error: {e}")),
                            Ok(json) => {
                                let count = json["firstDegreeSize"].as_u64().unwrap_or(0);
                                Ok(serde_json::json!({"follower_count": count}))
                            }
                        }
                    }
                }
            }
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}

// ── Facebook Handler ─────────────────────────────────────────

async fn handle_facebook(action: FacebookAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;

    let result: Result<serde_json::Value, String> = match action {
        FacebookAction::Posts { page_id } => {
            match find_facebook_page_token(&state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok(token) => {
                    let provider = crate::social::facebook::FacebookProvider::new(&state.config);
                    provider.get_page_feed(&token, &page_id, 20, None, None).await
                        .map_err(|e| e.to_string())
                }
            }
        }
        FacebookAction::Insights { page_id } => {
            match find_facebook_page_token(&state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok(token) => {
                    let provider = crate::social::facebook::FacebookProvider::new(&state.config);
                    provider.get_page_insights(&token, &page_id, "page_impressions,page_engaged_users", "day", None, None).await
                        .map_err(|e| e.to_string())
                }
            }
        }
        FacebookAction::Comment { post_id, text } => {
            let page_id = post_id.split('_').next().unwrap_or(&post_id);
            match find_facebook_page_token(&state, user_id, page_id).await {
                Err(e) => Err(e.to_string()),
                Ok(token) => {
                    let provider = crate::social::facebook::FacebookProvider::new(&state.config);
                    provider.comment_on_post(&token, &post_id, &text).await
                        .map_err(|e| e.to_string())
                }
            }
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}

// ── Import Handler ───────────────────────────────────────────

async fn handle_import(provider_name: &str, count: u32) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
    let integration = find_integration(&state, user_id, provider_name).await?;
    let token = state.token_key.as_ref()
        .and_then(|key| crypto::decrypt_string(&integration.access_token, key).ok())
        .unwrap_or_else(|| integration.access_token.clone());

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

async fn handle_feed(provider: Option<&str>, limit: u32) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;
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

async fn handle_instagram(action: InstagramAction) -> anyhow::Result<()> {
    let state = init_state().await?;
    let user_id = resolve_user(&state).await?;

    let result: Result<serde_json::Value, String> = match action {
        InstagramAction::Posts { account_id } => {
            match find_instagram_token(&state, user_id, &account_id).await {
                Err(e) => Err(e.to_string()),
                Ok(token) => {
                    let provider = crate::social::instagram::InstagramProvider::new(&state.config);
                    provider.get_ig_media(&token, &account_id, 20).await
                        .map_err(|e| e.to_string())
                }
            }
        }
        InstagramAction::Insights { account_id } => {
            match find_instagram_token(&state, user_id, &account_id).await {
                Err(e) => Err(e.to_string()),
                Ok(token) => {
                    let provider = crate::social::instagram::InstagramProvider::new(&state.config);
                    provider.get_ig_insights(&token, &account_id, "impressions,reach,profile_views", "day").await
                        .map_err(|e| e.to_string())
                }
            }
        }
        InstagramAction::Comment { media_id, text } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let ig = integrations.iter()
                        .find(|i| i.provider_identifier == "instagram" || i.provider_identifier == "instagram-standalone");
                    match ig {
                        None => Err("No Instagram account connected".to_string()),
                        Some(ig) => {
                            let token = ig.access_token.clone();
                            let token = state.token_key.as_ref()
                                .and_then(|key| crypto::decrypt_string(&token, key).ok())
                                .unwrap_or(token);
                            let provider = crate::social::instagram::InstagramProvider::new(&state.config);
                            provider.reply_to_ig_comment(&token, &media_id, &text).await
                                .map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
    };

    match result {
        Ok(v) => output_json(&v),
        Err(e) => return output_error(&e),
    }
    Ok(())
}
