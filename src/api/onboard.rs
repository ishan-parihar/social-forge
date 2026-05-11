// ─── Public Onboarding Page ─────────────────────────────────────
// Browser-accessible OAuth flow: no JWT header required.
// Shows all providers with config status and clickable connect buttons.
// Auto-creates a dev user on first access.

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::jwt;
use crate::db::queries;
use crate::error::AppError;

use super::AppState;

const DEV_EMAIL: &str = "dev@postiz.dev";
const DEV_PASSWORD: &str = "devdev123";
const DEV_NAME: &str = "Dev User";

/// Query params for public connect
#[derive(Debug, Deserialize)]
pub struct PublicConnectQuery {
    pub token: Option<String>,
    pub redirect_uri: Option<String>,
}

/// Query params for the onboarding page (page-picker)
#[derive(Debug, Default, Deserialize)]
pub struct OnboardQuery {
    pub pending: Option<String>,
    pub integration_id: Option<String>,
    pub token: Option<String>,
    pub connected: Option<String>,
    pub error: Option<String>,
    pub name: Option<String>,
}

/// GET / — public onboarding HTML dashboard
/// Supports query params:
///   ?pending={provider}&integration_id={id}&token={jwt} — shows page-picker for multi-step providers
///   ?connected={provider}&name={name} — shows success banner
pub async fn onboard_page(
    State(state): State<AppState>,
    Query(query): Query<OnboardQuery>,
) -> Result<Html<String>, AppError> {
    let user = get_or_create_dev_user(&state).await?;
    let token = jwt::create_token(user.id, &state.config.jwt_secret)?;

    // ── Fetch existing integrations ───────────────────────────
    let integrations = queries::list_integrations(&state.db, user.id).await.unwrap_or_default();

    let mut connected_cards = String::new();
    for integration in &integrations {
        let pid = &integration.provider_identifier;
        let icon = provider_icon(pid);
        let name = integration.profile_name.as_deref().unwrap_or(&integration.provider_name);
        let pic_html = if let Some(ref pic) = integration.profile_picture {
            format!(r#"<img class="profile-pic" src="{}" alt="" onerror="this.style.display='none'" />"#, pic)
        } else {
            format!(r#"<div class="profile-pic profile-pic-placeholder">{}</div>"#, &icon)
        };
        let connected_at = integration.created_at.format("%b %e, %Y").to_string();

        // Provider display name for the identifier
        let provider_display = match pid.as_str() {
            "x" => "𝕏 (Twitter)",
            "linkedin" => "LinkedIn",
            "facebook" => "Facebook",
            "instagram" => "Instagram",
            "instagram-standalone" => "Instagram Standalone",
            "threads" => "Threads",
            "youtube" => "YouTube",
            "telegram-bot" => "Telegram Bot",
            "telegram-user" => "Telegram User",
            "telegram" => "Telegram",
            "linkedin-page" => "LinkedIn Page",
            "bluesky" => "Bluesky",
            "skool" => "Skool",
            _ => pid.as_str(),
        };

        connected_cards.push_str(&format!(
            r#"<div class="connected-card">
                <div class="connected-avatar">{pic_html}</div>
                <div class="connected-body">
                    <div class="connected-name">{name}</div>
                    <div class="connected-provider">{provider_display}</div>
                    <div class="connected-since">Connected {connected_at}</div>
                </div>
                <div class="connected-status-badge">
                    <span class="badge badge-connected">✅ Connected</span>
                </div>
                <div class="connected-meta">
                    <code class="connected-id">{pid}</code>
                </div>
            </div>"#,
        ));
    }

    let has_connected = !integrations.is_empty();

    // ── All providers grid ────────────────────────────────────
    let all_providers = state.providers.all();
    let mut sorted: Vec<_> = all_providers.into_iter().collect();
    sorted.sort_by(|a, b| a.name().cmp(b.name()));

    let mut cards = String::new();

    // Count integrations per provider for multi-account display
    let mut connected_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for integration in &integrations {
        *connected_counts.entry(integration.provider_identifier.clone()).or_insert(0) += 1;
    }

    for provider in &sorted {
        let id = provider.identifier();
        let name = provider.name();
        let has_creds = state.config.provider_credentials(id).is_some();
        let uses_oauth = provider.uses_oauth();
        let is_onetime = provider.one_time_token();
        let redirect_uri = format!("{}/api/auth/callback", state.config.app_url);
        let connected_count = connected_counts.get(id).copied().unwrap_or(0);

        // ── Status badge + action ─────────────────────────────
        let badge_class: &str;
        let badge_text: String;
        let hint_text: String;
        let action_html: String;

        if connected_count > 0 {
            badge_class = "badge-connected";
            badge_text = format!("Connected ({})", connected_count);
            hint_text = format!("{} account(s) connected — click Add Another to connect more", connected_count);
            action_html = format!(
                r#"<a href="/api/public/connect/{}?token={}" class="btn btn-primary">+ Add Another</a>"#,
                id, token
            );
        } else if !has_creds {
            badge_class = "badge-error";
            badge_text = "Not Configured".into();
            hint_text = match id {
                "x" => "Requires: X_CLIENT_ID + X_CLIENT_SECRET",
                "linkedin" | "linkedin-page" => "Requires: LINKEDIN_CLIENT_ID + LINKEDIN_CLIENT_SECRET",
                "facebook" | "instagram" => "Requires: FACEBOOK_CLIENT_ID + FACEBOOK_CLIENT_SECRET",
                "instagram-standalone" => "Requires: INSTAGRAM_APP_ID + INSTAGRAM_APP_SECRET",
                "threads" => "Requires: THREADS_CLIENT_ID + THREADS_CLIENT_SECRET",
                "youtube" => "Requires: YOUTUBE_CLIENT_ID + YOUTUBE_CLIENT_SECRET",
                "telegram-bot" => "Requires: TELEGRAM_BOT_TOKENS",
                "telegram-user" => "Requires: TELEGRAM_CLI_PATH (or tg in PATH)",
                "telegram" => "Requires: TELEGRAM_TOKEN",
                "reddit" => "Requires: REDDIT_CLIENT_ID + REDDIT_CLIENT_SECRET + REDDIT_USERNAME + REDDIT_PASSWORD",
                "bluesky" => "Requires: BLUESKY_HANDLE + BLUESKY_APP_PASSWORD",
                "skool" => "Requires Chrome extension — install, login to Skool, extract auth_token cookie",
                _ => "Missing environment variables",
            }.into();
            action_html = format!(r#"<a href="/api/public/connect/{}" class="btn btn-disabled">Not Available</a>"#, id);
        } else if is_onetime {
            badge_class = "badge-info";
            badge_text = "One-Time Token".into();
            hint_text = match id {
                "telegram-bot" => "Send /connect {code} to the Telegram bot".into(),
                "telegram-user" => "Login via telegram-cli when prompted".into(),
                "telegram" => "Send /connect {code} to the Telegram bot".into(),
                "skool" => "Install Chrome extension, login to Skool, extract auth_token cookie".into(),
                _ => "One-time token provider — follow provider instructions".into(),
            };
            action_html = format!(
                r#"<a href="/api/public/connect/{}?token={}" class="btn btn-primary">Connect ➜</a>"#,
                id, token
            );
        } else if uses_oauth {
            badge_class = "badge-success";
            badge_text = "OAuth 2.0".into();
            hint_text = "Opens provider OAuth page in browser".into();
            action_html = format!(
                r#"<a href="/api/public/connect/{}?token={}" class="btn btn-primary">Connect ➜</a>"#,
                id, token
            );
        } else {
            badge_class = "badge-warning";
            badge_text = "Direct Connect".into();
            hint_text = "Connects using credentials from .env".into();
            action_html = format!(
                r#"<a href="/api/public/connect/{}?token={}" class="btn btn-primary">Connect ➜</a>"#,
                id, token
            );
        };

        // ── Provider icon ──────────────────────────────────────
        let icon = provider_icon(id);

        cards.push_str(&format!(
            r#"<div class="card{css_connected}">
                <div class="card-icon">{icon}</div>
                <div class="card-body">
                    <div class="card-title">{name}</div>
                    <div class="card-id"><code>{id}</code></div>
                    <div class="card-status"><span class="badge {badge_class}">{badge_text}</span></div>
                    <div class="card-hint">{hint_text}</div>
                    <div class="card-redirect"><small>Redirect: <code>{redirect_uri}</code></small></div>
                </div>
                <div class="card-action">{action_html}</div>
            </div>"#,
            icon = icon,
            name = name,
            id = id,
            badge_class = badge_class,
            badge_text = badge_text,
            hint_text = hint_text,
            redirect_uri = redirect_uri,
            action_html = action_html,
            css_connected = if connected_count > 0 { " card-connected" } else { "" },
        ));
    }

    let app_url = &state.config.app_url;
    let frontend_url = &state.config.frontend_url;

    // ── Page-picker for multi-step providers ───────────────
    let page_picker_html = build_page_picker_html(
        query.pending.as_deref(),
        query.integration_id.as_deref(),
        query.token.as_deref(),
    );

    // ── Success/Error banner ───────────────────────────────
    let banner_html = if let Some(connected) = &query.connected {
        let name = query.name.as_deref().unwrap_or(connected);
        format!(
            r#"<div class="banner banner-success" id="banner">
                ✅ Connected to <strong>{connected}</strong> as <strong>{name}</strong>!
                <button onclick="this.parentElement.style.display='none'" style="margin-left:12px;background:none;border:none;cursor:pointer;font-size:16px;">&times;</button>
            </div>
            <script>setTimeout(function(){{var b=document.getElementById('banner');if(b)b.style.display='none';}},8000);</script>"#,
            connected = connected,
            name = name,
        )
    } else if let Some(error) = &query.error {
        format!(
            r#"<div class="banner banner-error" id="banner">
                ❌ Connection failed: <strong>{error}</strong>
                <button onclick="this.parentElement.style.display='none'" style="margin-left:12px;background:none;border:none;cursor:pointer;font-size:16px;">&times;</button>
            </div>"#,
            error = error,
        )
    } else {
        String::new()
    };

    let connected_section = if has_connected {
        format!(
            r#"<h2 class="section-title">✅ Connected Channels <span class="count-badge">{count}</span></h2>
            <p class="section-subtitle">These social accounts are already linked to your Postiz Rust account.</p>
            <div class="connected-grid">{cards}</div>
            <h2 class="section-title" style="margin-top:28px;">🔌 Available Providers</h2>
            <p class="section-subtitle">Click <strong>Connect</strong> to add more channels.</p>"#,
            count = integrations.len(),
            cards = connected_cards,
        )
    } else {
        String::from("<h2 class=\"section-title\">🔌 Available Providers</h2><p class=\"section-subtitle\">Click <strong>Connect</strong> to authorize a social media account. You'll be redirected to the provider's OAuth page.</p>")
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Postiz Rust — Channel Onboarding</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f0f2f5; color: #1a1a2e; padding: 40px 20px;
        }}
        .container {{ max-width: 960px; margin: 0 auto; }}
        h1 {{ font-size: 28px; margin-bottom: 4px; }}
        .subtitle {{ color: #666; margin-bottom: 20px; font-size: 15px; }}
        .section-title {{ font-size: 20px; margin: 0 0 4px 0; display: flex; align-items: center; gap: 8px; }}
        .section-subtitle {{ color: #666; margin-bottom: 14px; font-size: 14px; }}
        .count-badge {{
            display: inline-flex; align-items: center; justify-content: center;
            background: #4361ee; color: white; font-size: 12px; font-weight: 600;
            border-radius: 10px; padding: 1px 9px; min-width: 22px; height: 20px;
        }}
        .user-info {{
            font-size: 13px; color: #555; margin-bottom: 20px;
            padding: 12px 16px; background: #e8f4f8; border-radius: 8px;
            border: 1px solid #b8daff; display: flex; flex-wrap: wrap; gap: 16px;
            align-items: center;
        }}
        .user-info .label {{ color: #888; }}
        .user-info code {{ font-size: 12px; background: #fff; padding: 2px 6px; border-radius: 3px; }}
        /* ── Connected accounts ────────────────────── */
        .connected-grid {{ display: flex; flex-direction: column; gap: 8px; margin-bottom: 24px; }}
        .connected-card {{
            background: white; border-radius: 10px; padding: 14px 18px;
            display: flex; align-items: center; gap: 14px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.08); border: 1px solid #d4edda;
            transition: box-shadow 0.15s;
        }}
        .connected-card:hover {{ box-shadow: 0 2px 8px rgba(0,0,0,0.12); }}
        .connected-avatar {{ flex-shrink: 0; }}
        .profile-pic {{ width: 44px; height: 44px; border-radius: 50%; object-fit: cover; border: 2px solid #d4edda; }}
        .profile-pic-placeholder {{
            width: 44px; height: 44px; border-radius: 50%;
            display: flex; align-items: center; justify-content: center;
            font-size: 22px; background: #f0f2f5; border: 2px solid #e8e8e8;
        }}
        .connected-body {{ flex: 1; min-width: 0; }}
        .connected-name {{ font-weight: 600; font-size: 15px; }}
        .connected-provider {{ font-size: 13px; color: #666; }}
        .connected-since {{ font-size: 11px; color: #999; margin-top: 1px; }}
        .connected-status-badge {{ flex-shrink: 0; }}
        .connected-meta {{ flex-shrink: 0; margin-left: 8px; }}
        .connected-meta code {{ font-size: 11px; color: #aaa; }}
        /* ── Provider grid ─────────────────────────── */
        .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(400px, 1fr)); gap: 14px; }}
        .card {{
            background: white; border-radius: 12px; padding: 18px 20px;
            display: flex; align-items: flex-start; gap: 16px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.08); border: 1px solid #e8e8e8;
            transition: box-shadow 0.15s;
        }}
        .card:hover {{ box-shadow: 0 2px 8px rgba(0,0,0,0.12); }}
        .card-connected {{ border-color: #b7e4c7; background: #f6fef9; }}
        .card-icon {{ font-size: 34px; line-height: 1; flex-shrink: 0; width: 48px; text-align: center; padding-top: 2px; }}
        .card-body {{ flex: 1; min-width: 0; }}
        .card-title {{ font-weight: 600; font-size: 16px; margin-bottom: 1px; }}
        .card-id {{ margin-bottom: 5px; }}
        .card-id code {{ font-size: 12px; color: #888; }}
        .card-status {{ margin-bottom: 3px; }}
        .card-hint {{ font-size: 11px; color: #999; margin-bottom: 4px; }}
        .card-redirect code {{ font-size: 11px; color: #aaa; word-break: break-all; }}
        .card-action {{ flex-shrink: 0; padding-top: 4px; }}
        .badge {{ display: inline-block; padding: 3px 9px; border-radius: 4px; font-size: 11px; font-weight: 600; }}
        .badge-success {{ background: #d4edda; color: #155724; }}
        .badge-error {{ background: #f8d7da; color: #721c24; }}
        .badge-info {{ background: #d1ecf1; color: #0c5460; }}
        .badge-warning {{ background: #fff3cd; color: #856404; }}
        .badge-connected {{ background: #d4edda; color: #155724; }}
        .btn {{ display: inline-block; padding: 8px 18px; border-radius: 6px; text-decoration: none; font-size: 13px; font-weight: 500; transition: all 0.15s; text-align: center; white-space: nowrap; }}
        .btn-primary {{ background: #4361ee; color: white; }}
        .btn-primary:hover {{ background: #3a56d4; }}
        .btn-connected {{ background: #d4edda; color: #155724; cursor: default; border: 1px solid #b7e4c7; }}
        .btn-disabled {{ background: #e9ecef; color: #adb5bd; cursor: default; pointer-events: none; }}
        /* ── Page-picker ─────────────────────────── */
        .page-picker {{ display: none; margin-bottom: 24px; }}
        .page-picker.active {{ display: block; }}
        .page-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; margin-top: 12px; }}
        .page-card {{ background: white; border-radius: 10px; padding: 14px; display: flex; align-items: center; gap: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.08); border: 1px solid #e8e8e8; }}
        .page-card:hover {{ box-shadow: 0 2px 8px rgba(0,0,0,0.12); }}
        .page-card img {{ width: 44px; height: 44px; border-radius: 50%; object-fit: cover; }}
        .page-card .info {{ flex: 1; min-width: 0; }}
        .page-card .name {{ font-weight: 600; font-size: 14px; }}
        .page-card .btn-connect {{ padding: 7px 16px; border-radius: 6px; border: none; background: #4361ee; color: white; font-size: 12px; font-weight: 500; cursor: pointer; flex-shrink: 0; }}
        .page-card .btn-connect:disabled {{ background: #d4edda; color: #155724; cursor: default; }}
        .page-card .btn-connect.done {{ background: #d4edda; color: #155724; cursor: default; }}
        /* ── Banner ──────────────────────────────── */
        .banner {{ padding: 14px 18px; border-radius: 10px; margin-bottom: 16px; font-size: 14px; display: flex; align-items: center; }}
        .banner-success {{ background: #d4edda; border: 1px solid #b7e4c7; color: #155724; }}
        .banner-error {{ background: #f8d7da; border: 1px solid #f5c6cb; color: #721c24; }}
        .note {{
            margin-top: 20px; font-size: 13px; color: #666;
            background: #fffbe6; padding: 10px 14px; border-radius: 8px;
            border: 1px solid #ffe58f; line-height: 1.5;
        }}
        .note code {{ font-size: 12px; background: #fff3cd; padding: 1px 5px; border-radius: 3px; }}
        .footer {{ margin-top: 28px; text-align: center; font-size: 12px; color: #aaa; border-top: 1px solid #e8e8e8; padding-top: 14px; }}
        @media (max-width: 640px) {{ .grid {{ grid-template-columns: 1fr; }} }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔗 Postiz Rust — Channel Onboarding</h1>

        <div class="user-info">
            <span>👤 <span class="label">User:</span> <strong>{dev_email}</strong></span>
            <span>🔗 <span class="label">App URL:</span> <code>{app_url}</code></span>
            <span>🎯 <span class="label">Frontend:</span> <code>{frontend_url}</code></span>
        </div>

        {banner_html}

        {page_picker_html}

        {connected_section}

        <div class="grid">
            {cards}
        </div>

        <div class="note">
            <strong>ℹ️ After authorizing</strong>, you'll be redirected back here — the connected channel
            appears in the <strong>Connected Channels</strong> section above. Verify with:
            <code>curl -H "Authorization: Bearer {token}" {app_url}/api/integrations</code>
        </div>

        <div class="footer">
            Postiz Rust v{version} · server: {app_url} · MCP stdio available with --mcp flag
        </div>
    </div>
</body>
</html>"#,
        dev_email = DEV_EMAIL,
        app_url = app_url,
        frontend_url = frontend_url,
        banner_html = banner_html,
        page_picker_html = page_picker_html,
        connected_section = connected_section,
        cards = cards,
        token = token,
        version = env!("CARGO_PKG_VERSION"),
    );

    Ok(Html(html))
}

fn provider_icon(id: &str) -> &'static str {
    match id {
        "x" => "𝕏",
        "linkedin" | "linkedin-page" => "💼",
        "facebook" | "instagram" => "📘",
        "instagram-standalone" => "📸",
        "threads" => "🧵",
        "youtube" => "▶️",
        "telegram-bot" | "telegram-user" | "telegram" => "✈️",
        "bluesky" => "🦋",
        "skool" => "🎓",
        _ => "🔗",
    }
}

/// GET /api/public/connect/{provider} — initiate OAuth from browser
///
/// Uses `?token=` query param for auth (no Bearer header needed).
/// Redirects browser directly to the OAuth provider's authorization page.
///
/// For non-OAuth providers (Telegram), returns instructions as JSON.
pub async fn public_connect(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(query): Query<PublicConnectQuery>,
) -> Result<Response, AppError> {
    let user_id = if let Some(token_str) = &query.token {
        let claims = jwt::validate_token(token_str, &state.config.jwt_secret)
            .map_err(|_| AppError::BadRequest(
                "Invalid or expired token. Visit http://localhost:3000/ to get a fresh one.".into()
            ))?;
        Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::BadRequest("Invalid user ID in token".into()))?
    } else {
        let user = get_or_create_dev_user(&state).await?;
        let t = jwt::create_token(user.id, &state.config.jwt_secret)?;
        let ru = query.redirect_uri.as_deref().unwrap_or("");
        let extra = if ru.is_empty() { String::new() } else { format!("&redirect_uri={}", urlencoding::encode(ru)) };
        return Ok(Redirect::to(&format!("/api/public/connect/{provider}?token={t}{extra}")).into_response());
    };

    if state.config.provider_credentials(&provider).is_none() {
        let needed = match provider.as_str() {
            "x" => "X_CLIENT_ID + X_CLIENT_SECRET",
            "linkedin" | "linkedin-page" => "LINKEDIN_CLIENT_ID + LINKEDIN_CLIENT_SECRET",
            "facebook" | "instagram" => "FACEBOOK_CLIENT_ID + FACEBOOK_CLIENT_SECRET",
            "instagram-standalone" => "INSTAGRAM_APP_ID + INSTAGRAM_APP_SECRET",
            "threads" => "THREADS_CLIENT_ID + THREADS_CLIENT_SECRET",
            "youtube" => "YOUTUBE_CLIENT_ID + YOUTUBE_CLIENT_SECRET",
            "telegram-bot" => "TELEGRAM_BOT_TOKENS (comma-separated)",
            "telegram-user" => "TELEGRAM_CLI_PATH",
            "telegram" => "TELEGRAM_TOKEN",
                "bluesky" => "BLUESKY_HANDLE + BLUESKY_APP_PASSWORD",
                "skool" => "Chrome extension — install, login to Skool, extract auth_token cookie",
            _ => "Unknown provider. Check server logs.",
        };
        return Err(AppError::BadRequest(format!(
            "❌ Provider '{provider}' is not configured.\n\nSet these in your .env:\n  {needed}\n\nThen restart the server."
        )));
    }

    let provider_obj = state
        .providers
        .get(&provider)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown provider: {provider}")))?;

    let redirect_uri = query
        .redirect_uri
        .unwrap_or_else(|| format!("{}/api/auth/callback", state.config.app_url));

    if !provider_obj.uses_oauth() {
        if provider_obj.one_time_token() {
            let code = Uuid::new_v4().to_string();
            return Ok(Json(serde_json::json!({
                "type": "one_time_token",
                "provider": provider,
                "instructions": format!(
                    "Open Telegram, start a chat with the bot, and send: /connect {}",
                    code
                ),
                "code": code,
            }))
            .into_response());
        }

        let token = provider_obj
            .exchange_code("", "", &redirect_uri)
            .await?;

        queries::create_integration(
            &state.db,
            user_id,
            &provider,
            &token.name,
            &token.provider_user_id,
            &token.access_token,
            token.refresh_token.as_deref(),
            token.expires_in.map(|exp| {
                chrono::Utc::now() + chrono::Duration::seconds(exp as i64)
            }),
            Some(&token.name),
            token.picture.as_deref(),
            None,
            None,
        )
        .await?;

        state.broadcast.send(
            "integration_connected",
            &serde_json::json!({
                "provider": provider,
                "name": token.name,
            }),
        );

        return Ok(Json(serde_json::json!({
            "type": "connected",
            "provider": provider,
            "name": token.name,
            "message": format!("Connected to {} as {}", provider, token.name),
        }))
        .into_response());
    }

    let code_verifier = crate::social::common::generate_code_verifier();
    let oauth_state = crate::social::common::generate_state();

    let auth_url = provider_obj
        .generate_auth_url(&oauth_state, &code_verifier, &redirect_uri)
        .await?;

    crate::db::queries::save_oauth_state(
        &state.db,
        &oauth_state,
        &provider,
        &code_verifier,
        Some(&format!("{}:{}", user_id, redirect_uri)),
    )
    .await?;

    tracing::info!(
        "Public connect: {provider} for user {user_id} — redirecting to OAuth"
    );

    Ok(Redirect::to(&auth_url.url).into_response())
}

// ── Page-picker HTML builder ─────────────────────────────────

/// Build the page-picker section HTML (CSS + body + JS) for multi-step providers.
/// Shows available pages/accounts for the user to select and connect.
fn build_page_picker_html(pending: Option<&str>, integration_id: Option<&str>, pending_token: Option<&str>) -> String {
    let (prov, iid, tok) = match (pending, integration_id, pending_token) {
        (Some(p), Some(i), Some(t)) => (p, i, t),
        _ => return String::new(),
    };

    format!(
        r#"<div id="page-picker" style="margin-bottom:24px;">
    <h2 class="section-title"> Select which pages to connect</h2>
    <p class="section-subtitle">These pages are linked to your {prov} account. Click <strong>Connect</strong> for each one to add.</p>
    <div id="page-grid" class="page-grid"><p style="color:#888;font-size:14px;">Loading available pages...</p></div>
    <p style="margin-top:12px;"><a href="/" style="font-size:13px;color:#4361ee;"> Done back to main page</a></p>
</div>
<script>
(function(){{
var iid='{iid}';
var tok='{tok}';
var grid=document.getElementById('page-grid');
fetch('/api/integrations/'+iid+'/available-pages',{{headers:{{'Authorization':'Bearer '+tok}}}})
.then(function(r){{return r.ok?r.json():Promise.reject('HTTP '+r.status)}})
.then(function(d){{
if(!d.pages||!d.pages.length){{grid.innerHTML='<div class=\"note\">No pages found. <a href=\"/\">Go back</a></div>';return;}}
var h='';
d.pages.forEach(function(p){{
var icon=p.picture?'<img src=\"'+p.picture+'\" style=\"width:44px;height:44px;border-radius:50%;object-fit:cover;\" onerror=\"this.style.display=\'none\'\"/>':'F';
h+='<div class=\"page-card\"><div class=\"card-icon\">'+icon+'</div><div class=\"card-body\"><div class=\"card-title\">'+p.name.replace(/</g,'&lt;')+'</div></div><button class=\"btn btn-primary cp\" data-id=\"'+p.id+'\">Connect</button></div>';
}});
grid.innerHTML=h;
var btns=grid.querySelectorAll('.cp');
for(var i=0;i<btns.length;i++){{(function(btn){{btn.addEventListener('click',function(){{
btn.disabled=true;btn.textContent='Connecting...';
fetch('/api/integrations/'+iid+'/connect-page/'+btn.getAttribute('data-id'),{{method:'POST',headers:{{'Authorization':'Bearer '+tok}}}})
.then(function(r){{if(!r.ok)return r.text().then(function(t){{throw new Error(t)}});return r.json()}})
.then(function(){{btn.textContent='Connected';btn.className='btn btn-connected';}})
.catch(function(e){{btn.disabled=false;btn.textContent='Connect';alert('Failed: '+e.message);}});
}})}})(btns[i]);}}
}})
.catch(function(e){{grid.innerHTML='<div class=\"note\" style=\"background:#f8d7da;color:#721c24;\">Error: '+e.message+'</div>';}});
}})();
</script>"#,
        prov = prov,
        iid = iid,
        tok = tok,
    )
}

// ── Dev user helpers ──────────────────────────────────────────

/// Find or create the dev user for public onboarding
async fn get_or_create_dev_user(state: &AppState) -> Result<crate::db::models::User, AppError> {
    if let Some(user) = queries::get_user_by_email(&state.db, DEV_EMAIL).await? {
        return Ok(user);
    }

    let hash = jwt::hash_password(DEV_PASSWORD)?;
    let user = queries::create_user(&state.db, DEV_EMAIL, &hash, DEV_NAME).await?;
    tracing::info!("Created dev user: {} ({})", DEV_EMAIL, user.id);
    Ok(user)
}
