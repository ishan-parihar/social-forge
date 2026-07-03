// ─── API Router ────────────────────────────────────────────────
// axum HTTP router combining all route modules.
// Protected routes use the auth middleware chain.

use axum::{body::Body, http::{Request, Response, StatusCode}, middleware, Router};
use rust_embed::RustEmbed;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Embedded SvelteKit static build, compiled into the binary at build time.
/// If `frontend/build/` doesn't exist at compile time, the set is empty.
/// Disable serving at runtime by setting `SERVE_FRONTEND=false`.
#[derive(RustEmbed)]
#[folder = "frontend/build"]
#[include = "*"]
struct FrontendAssets;

use crate::auth::middleware::{auth_middleware, AuthState};
use crate::config::Config;
use crate::db::PgPool;
use crate::realtime::Broadcaster;
use crate::services::telegram_client::OptionalTelegramClient;
use crate::social::registry::ProviderRegistry;
use crate::wa::OptionalWhaClient;

use self::rate_limiter::AuthRateLimiter;

mod analytics;
mod auth;
mod billing;
mod calendar;
mod comments;
mod feed;
mod integrations;
mod media;
mod notifications;
mod onboard;
mod posts;
pub mod rate_limiter;
mod rss;
mod sse;
mod signatures;
mod tags;
mod teams;
mod developer;
mod webhooks;
mod dms;
mod automation;

/// Shared application state available to all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub broadcast: Broadcaster,
    pub providers: ProviderRegistry,
    pub rate_limiter: AuthRateLimiter,
    /// Optional AES-256 key for token encryption at rest (32 bytes)
    pub token_key: Option<[u8; 32]>,
    /// Shared Telegram user client (Grammers-based, lazy init)
    pub telegram_client_manager: OptionalTelegramClient,
    /// Shared WhatsApp Web client (wa-rs-based, replaces Go wacli sidecar)
    pub wa_client: OptionalWhaClient,
    /// Shared HTTP client for media proxying (CDN bypass)
    pub media_http_client: reqwest::Client,
    /// Shared HTTP client with Chrome TLS fingerprinting for X/Twitter CDN
    pub media_wreq_client: wreq::Client,
}

/// Build the axum router with all routes
pub fn build_router(state: AppState) -> Router {
    // Extract allowed origin for CORS
    let cors_origin = state.config.frontend_url.clone();
    let cors_layer = if cors_origin == "*" || cors_origin.is_empty() {
        CorsLayer::permissive()
    } else {
        CorsLayer::new()
            .allow_origin(tower_http::cors::AllowOrigin::exact(
                cors_origin.parse::<axum::http::HeaderValue>().unwrap_or(axum::http::HeaderValue::from_static("*")),
            ))
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS])
            .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION])
    };

    // Public routes — no auth required
    let public_routes = Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route("/api/auth/callback", axum::routing::get(integrations::oauth_callback))
        .route("/api/auth/callback/{provider}", axum::routing::get(integrations::oauth_callback))
        .route("/integrations/social/{provider}", axum::routing::get(integrations::oauth_callback))
        .route("/api/events", axum::routing::get(sse::sse_handler))
        .route("/api/media/{id}", axum::routing::get(media::serve_media))
        .route("/api/proxy-media", axum::routing::get(media::proxy_media))
        // Public onboarding — browser-accessible OAuth flow (no JWT header needed)
        .route("/setup", axum::routing::get(onboard::onboard_page))
        .route("/api/public/connect/x-cookies", axum::routing::get(onboard::x_cookies_form).post(onboard::x_cookies_submit))
.route("/api/public/connect/x-cookies/import", axum::routing::post(onboard::x_cookies_import))
.route("/api/public/connect/reddit-cookies", axum::routing::get(onboard::reddit_cookies_form).post(onboard::reddit_cookies_submit))
.route("/api/public/connect/reddit-cookies/import", axum::routing::post(onboard::reddit_cookies_import))
.route("/api/public/connect/telegram-bot-token", axum::routing::get(onboard::telegram_bot_token_form).post(onboard::telegram_bot_token_submit))
        .route("/api/public/connect/{provider}", axum::routing::get(onboard::public_connect))
        // Stripe webhook — no auth (signature verification in handler)
        .route("/api/billing/webhook", axum::routing::post(billing::stripe_webhook));

    // Protected routes — auth required
    let protected_routes = Router::new()
        .route("/api/auth/me", axum::routing::get(auth::me))
        .route("/api/auth/logout", axum::routing::post(auth::logout))
        .route("/api/posts", axum::routing::get(posts::list).post(posts::create))
        .route("/api/posts/thread", axum::routing::post(posts::create_thread))
        .route("/api/providers", axum::routing::get(integrations::list_providers))
        .route(
            "/api/posts/{id}",
            axum::routing::get(posts::get)
                .put(posts::update)
                .delete(posts::delete),
        )
        .route("/api/posts/{id}/schedule", axum::routing::post(posts::schedule))
        .route("/api/posts/{id}/publish", axum::routing::post(posts::publish_post))
        .route("/api/posts/{id}/repeat", axum::routing::post(posts::repeat_post))
        .route("/api/posts/{id}/tags", axum::routing::put(posts::set_post_tags))
        .route("/api/posts/find-slot", axum::routing::get(posts::find_slot))
        .route("/api/integrations", axum::routing::get(integrations::list))
        .route("/api/integrations/connect/{provider}", axum::routing::get(integrations::connect))
        .route("/api/integrations/connect/{provider}/verify", axum::routing::post(integrations::verify_one_time_token))
        .route("/api/integrations/connect/whatsapp/pair", axum::routing::post(integrations::whatsapp_pair))
        .route("/api/integrations/connect/whatsapp/status", axum::routing::get(integrations::whatsapp_status))
        .route("/api/integrations/connect/telegram-user/request-code", axum::routing::post(integrations::telegram_user_request_code))
        .route("/api/integrations/connect/telegram-user/sign-in", axum::routing::post(integrations::telegram_user_sign_in))
        .route("/api/integrations/connect/telegram-bot/token", axum::routing::post(integrations::connect_telegram_bot_token))
        .route("/api/integrations/connect/api-key", axum::routing::post(integrations::connect_api_key))
        .route("/api/integrations/connect/x-cookie", axum::routing::post(integrations::connect_x_cookie))
        .route("/api/integrations/connect/reddit-cookie", axum::routing::post(integrations::connect_reddit_cookie))
        .route("/api/integrations/connect/github-pat", axum::routing::post(integrations::connect_github_pat))
        .route("/api/integrations/connect/web3", axum::routing::post(integrations::connect_web3))
        .route("/api/integrations/{id}", axum::routing::delete(integrations::delete))
        .route("/api/integrations/{id}/available-pages", axum::routing::get(integrations::available_pages))
        .route("/api/integrations/{parent_id}/connect-page/{page_id}", axum::routing::post(integrations::connect_page))
        .route("/api/integrations/{id}/timeslots", axum::routing::put(integrations::update_timeslots))
        .route("/api/integrations/{id}/disable", axum::routing::put(integrations::toggle_disable))
    .route("/api/integrations/{id}/refresh", axum::routing::post(integrations::refresh))
    .route("/api/integrations/{id}/targets", axum::routing::get(integrations::list_targets))
        .route("/api/calendar", axum::routing::get(calendar::get))
        .route("/api/feed", axum::routing::get(feed::get))
        .route("/api/feed/import", axum::routing::post(feed::import))
        .route("/api/feed/accounts", axum::routing::get(feed::accounts))
        .route("/api/feed/analytics", axum::routing::get(feed::analytics))
        .route("/api/feed/{post_id}/comments", axum::routing::get(feed::get_comments))
        .route("/api/comments", axum::routing::get(comments::list))
        .route("/api/comments/{id}/resolve", axum::routing::post(comments::resolve))
        .route("/api/comments/{id}/reply", axum::routing::post(comments::reply))
        .route("/api/media", axum::routing::get(media::list).post(media::upload))
        .route("/api/media/{id}", axum::routing::delete(media::delete))
        .route("/api/analytics", axum::routing::get(analytics::get))
        .route("/api/analytics/summary", axum::routing::get(analytics::get_summary))
        .route("/api/analytics/post/{id}", axum::routing::get(analytics::get_post))
        .route("/api/tags", axum::routing::get(tags::list).post(tags::create))
        .route("/api/tags/{id}", axum::routing::get(tags::get).put(tags::update).delete(tags::delete))
        .route("/api/teams", axum::routing::get(teams::list).post(teams::create))
        .route("/api/teams/accept", axum::routing::post(teams::accept_invite))
        .route("/api/teams/{id}", axum::routing::get(teams::get).put(teams::update).delete(teams::delete))
        .route("/api/teams/{id}/invite", axum::routing::post(teams::invite))
        .route("/api/teams/{id}/members", axum::routing::get(teams::members))
        .route("/api/teams/{id}/members/{user_id}", axum::routing::delete(teams::remove_member))
        .route("/api/signatures", axum::routing::get(signatures::list).post(signatures::create))
        .route("/api/signatures/{id}", axum::routing::put(signatures::update).delete(signatures::delete))
        .route("/api/developer/api-keys", axum::routing::get(developer::list).post(developer::create))
        .route("/api/developer/api-keys/{id}", axum::routing::delete(developer::revoke))
        .route("/api/developer/api-keys/{id}/regenerate", axum::routing::post(developer::regenerate))
        .route("/api/webhooks", axum::routing::get(webhooks::list).post(webhooks::create))
        .route("/api/webhooks/{id}", axum::routing::get(webhooks::get).put(webhooks::update).delete(webhooks::delete))
        .route("/api/webhooks/{id}/test", axum::routing::post(webhooks::test))
        .route("/api/webhooks/{id}/deliveries", axum::routing::get(webhooks::deliveries))
        .route("/api/notifications", axum::routing::get(notifications::list))
        .route("/api/notifications/unread-count", axum::routing::get(notifications::unread_count))
        .route("/api/notifications/prefs", axum::routing::get(notifications::get_prefs).put(notifications::update_prefs))
        .route("/api/notifications/{id}/read", axum::routing::put(notifications::mark_read))
        .route("/api/notifications/read-all", axum::routing::put(notifications::mark_all_read))
        .route("/api/notifications/{id}", axum::routing::delete(notifications::delete))
        // Billing / Stripe subscriptions
        .route("/api/billing/create-checkout", axum::routing::post(billing::create_checkout_session))
        .route("/api/billing/subscription", axum::routing::get(billing::get_subscription))
        .route("/api/billing/invoices", axum::routing::get(billing::get_invoices))
        .route("/api/billing/portal-session", axum::routing::post(billing::create_portal_session))
        // RSS autopost
        .route("/api/rss/feeds", axum::routing::get(rss::list_feeds).post(rss::create_feed))
        .route("/api/rss/feeds/{id}", axum::routing::delete(rss::delete_feed))
        .route("/api/rss/feeds/{id}/toggle", axum::routing::put(rss::toggle_feed))
        .route("/api/rss/feeds/{id}/poll", axum::routing::post(rss::poll_feed))
        .route("/api/rss/feeds/{id}/items", axum::routing::get(rss::list_feed_items))
        .route("/api/rss/feeds/{id}/items/{guid}/import", axum::routing::post(rss::import_item))
        // DMs
        .route("/api/dms/conversations", axum::routing::get(dms::list_conversations))
        .route("/api/dms/send", axum::routing::post(dms::send_dm))
        .route("/api/dms/{conversation_id}/messages", axum::routing::get(dms::get_messages))
        // Automation
        .route("/api/automation/rules", axum::routing::get(automation::list_rules).post(automation::create_rule))
        .route("/api/automation/rules/{id}", axum::routing::put(automation::update_rule).delete(automation::delete_rule))
        .route("/api/automation/rules/{id}/logs", axum::routing::get(automation::get_logs))
        // Auth middleware: validates `sf_session` cookie against the
        // JWT secret derived from `APP_PASSWORD`. Injects
        // `AuthenticatedUser { user_id: DEFAULT_USER_ID }` on success.
        .layer(middleware::from_fn_with_state(
            AuthState { session_secret: state.config.jwt_secret.clone() },
            auth_middleware,
        ));

    // Global middleware stack
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(
            10 * 1024 * 1024, // 10 MB limit
        ))
        .with_state(state);

    // ── Frontend serving ──────────────────────────────────────────────────
    // Priority order:
    //   1. SERVE_FRONTEND=false  → disabled entirely (API-only mode)
    //   2. FRONTEND_DIR env var  → serve from filesystem path (dev mode / custom path)
    //   3. Embedded assets       → serve from binary-embedded frontend/build (default)
    //
    // In all disabled/missing cases, /api/* routes still function normally.

    let serve_frontend = std::env::var("SERVE_FRONTEND")
        .map(|v| v.to_lowercase() != "false" && v != "0")
        .unwrap_or(true);

    if !serve_frontend {
        return app;
    }

    // Check for explicit filesystem override first (dev mode)
    let fs_override = std::env::var("FRONTEND_DIR").ok()
        .filter(|d| !d.is_empty());

    if let Some(dir) = fs_override {
        let frontend_path = std::path::Path::new(&dir);
        if frontend_path.join("index.html").exists() {
            let index = frontend_path.join("index.html");
            return app.fallback_service(
                ServeDir::new(frontend_path)
                    .not_found_service(ServeFile::new(index)),
            );
        }
        tracing::warn!("FRONTEND_DIR={dir} has no index.html — falling back to embedded assets");
    }

    // Use embedded assets (compiled into the binary)
    if FrontendAssets::get("index.html").is_some() {
        app.fallback(embedded_frontend_handler)
    } else {
        tracing::info!("No frontend assets embedded or found — running in API-only mode");
        app
    }
}

/// Serve embedded frontend assets via rust-embed.
/// Falls back to `index.html` for unknown paths (SPA client-side routing).
async fn embedded_frontend_handler(req: Request<Body>) -> Response<Body> {
    let path = req.uri().path().trim_start_matches('/');

    // Try exact path first, then with index.html appended (for directory routes)
    let asset = FrontendAssets::get(path)
        .or_else(|| {
            if path.is_empty() || path.ends_with('/') {
                FrontendAssets::get(&format!("{path}index.html"))
            } else {
                None
            }
        });

    match asset {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime.as_ref())
                // Cache immutable hashed assets aggressively; HTML never
                .header("cache-control", if path.contains("/_app/immutable/") {
                    "public, max-age=31536000, immutable"
                } else {
                    "no-cache"
                })
                .body(Body::from(file.data.into_owned()))
                .unwrap_or_else(|_| not_found())
        }
        // SPA fallback — serve index.html for any unknown path (client-side routing)
        None => match FrontendAssets::get("index.html") {
            Some(index) => Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "text/html; charset=utf-8")
                .header("cache-control", "no-cache")
                .body(Body::from(index.data.into_owned()))
                .unwrap_or_else(|_| not_found()),
            None => not_found(),
        },
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap()
}

/// Health check endpoint
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}


