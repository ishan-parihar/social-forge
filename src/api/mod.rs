// ─── API Router ────────────────────────────────────────────────
// axum HTTP router combining all route modules.
// Protected routes use the auth middleware chain.

use axum::{body::Body, extract::State, http::{Request, Response, StatusCode}, middleware::{self, Next}, Router};
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

mod analytics;
mod ai;
mod auth;
mod billing;  // only stripe_webhook (public, signature-verified) is used
mod calendar;
mod comments;
mod feed;
mod integrations;
mod media;
mod music;
mod notifications;
mod onboard;
mod posts;
mod rss;
mod sse;
mod sets;
mod signatures;
mod tags;
mod developer;
mod webhooks;
mod dms;
mod automation;
mod campaigns;

/// Shared application state available to all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub broadcast: Broadcaster,
    pub providers: ProviderRegistry,
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
        .route("/ready", axum::routing::get({
            let db_for_ready = state.db.clone();
            move || ready_check(db_for_ready)
        }))
        .route("/api/metrics", axum::routing::get({
            let db_for_metrics = state.db.clone();
            move || metrics_check(db_for_metrics)
        }))
        .route("/api/streak", axum::routing::get({
            let db_for_streak = state.db.clone();
            move || streak_check(db_for_streak)
        }))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route("/api/auth/callback", axum::routing::get(integrations::oauth_callback))
        .route("/api/auth/callback/{provider}", axum::routing::get(integrations::oauth_callback))
        .route("/integrations/social/{provider}", axum::routing::get(integrations::oauth_callback))
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
        // SSE realtime stream — auth-gated to prevent event leakage on
        // networked deployments (BUG #19). The auth_middleware validates
        // the sf_session cookie; the SSE handler then subscribes.
        .route("/api/events", axum::routing::get(sse::sse_handler))
        .route("/api/posts", axum::routing::get(posts::list).post(posts::create))
        .route("/api/posts/thread", axum::routing::post(posts::create_thread))
        .route("/api/posts/validate", axum::routing::post(posts::validate))
        .route("/api/ai/generate-post", axum::routing::post(ai::generate_post))
        .route("/api/ai/generate-bulk", axum::routing::post(ai::generate_bulk))
        .route("/api/ai/improve-writing", axum::routing::post(ai::improve_writing))
        .route("/api/ai/suggest-hashtags", axum::routing::post(ai::suggest_hashtags))
        .route("/api/ai/change-tone", axum::routing::post(ai::change_tone))
        .route("/api/ai/summarize", axum::routing::post(ai::summarize))
        .route("/api/providers", axum::routing::get(integrations::list_providers))
        .route(
            "/api/posts/{id}",
            axum::routing::get(posts::get)
                .put(posts::update)
                .delete(posts::delete),
        )
        .route("/api/posts/{id}/schedule", axum::routing::post(posts::schedule))
        .route("/api/posts/{id}/unschedule", axum::routing::post(posts::unschedule))
        .route("/api/posts/{id}/date", axum::routing::put(posts::reschedule))
        .route("/api/posts/{id}/publish", axum::routing::post(posts::publish_post))
        .route("/api/posts/{id}/repeat", axum::routing::post(posts::repeat_post))
        .route("/api/posts/{id}/tags", axum::routing::put(posts::set_post_tags))
        .route("/api/posts/{id}/stage", axum::routing::patch(campaigns::update_stage))
        .route("/api/posts/find-slot", axum::routing::get(posts::find_slot))
        .route("/api/posts/group/{group_id}", axum::routing::get(posts::get_group))
        // Phase 7: Campaigns + Kanban
        .route("/api/campaigns", axum::routing::get(campaigns::list).post(campaigns::create))
        .route("/api/campaigns/{id}", axum::routing::put(campaigns::update).delete(campaigns::delete))
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
        .route("/api/integrations/{id}/mentions", axum::routing::get(integrations::search_mentions))
        .route("/api/integrations/{id}/music", axum::routing::get(music::search_music))
        .route("/api/calendar", axum::routing::get(calendar::get))
        .route("/api/feed", axum::routing::get(feed::get))
        .route("/api/feed/import", axum::routing::post(feed::import))
        .route("/api/feed/accounts", axum::routing::get(feed::accounts))
        .route("/api/feed/analytics", axum::routing::get(feed::analytics))
        .route("/api/feed/{post_id}/comments", axum::routing::get(feed::get_comments))
        .route("/api/feed/{post_id}/repurpose", axum::routing::post(feed::repurpose_post))
        .route("/api/feed/{post_id}/save", axum::routing::post(feed::save_post).delete(feed::unsave_post))
        .route(
            "/api/feed/{post_id}",
            axum::routing::put(feed::update_post).delete(feed::delete_post),
        )
        .route("/api/comments", axum::routing::get(comments::list))
        .route("/api/comments/{id}/resolve", axum::routing::post(comments::resolve))
        .route("/api/comments/{id}/reply", axum::routing::post(comments::reply))
        .route("/api/media", axum::routing::get(media::list).post(media::upload))
        .route("/api/media/{id}", axum::routing::delete(media::delete))
        .route("/api/analytics", axum::routing::get(analytics::get))
        .route("/api/analytics/summary", axum::routing::get(analytics::get_summary))
        .route("/api/analytics/engagement", axum::routing::get(analytics::get_engagement))
        .route("/api/analytics/adherence", axum::routing::get(analytics::get_adherence))
        .route("/api/analytics/cadence", axum::routing::get(analytics::get_cadence))
        .route("/api/analytics/post/{id}", axum::routing::get(analytics::get_post))
        .route("/api/events/recent", axum::routing::get(analytics::get_recent_events))
        .route("/api/tags", axum::routing::get(tags::list).post(tags::create))
        .route("/api/tags/{id}", axum::routing::get(tags::get).put(tags::update).delete(tags::delete))
        .route("/api/signatures", axum::routing::get(signatures::list).post(signatures::create))
        .route("/api/signatures/{id}", axum::routing::put(signatures::update).delete(signatures::delete))
        .route("/api/signatures/{id}/set-default", axum::routing::post(signatures::set_default))
        .route("/api/signatures/default", axum::routing::get(signatures::get_default))
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
        // RSS autopost
        .route("/api/rss/feeds", axum::routing::get(rss::list_feeds).post(rss::create_feed))
        .route("/api/rss/feeds/{id}", axum::routing::delete(rss::delete_feed))
        .route("/api/rss/feeds/{id}/toggle", axum::routing::put(rss::toggle_feed))
        .route("/api/rss/feeds/{id}/poll", axum::routing::post(rss::poll_feed))
        .route("/api/rss/feeds/{id}/items", axum::routing::get(rss::list_feed_items))
        .route("/api/rss/feeds/{id}/items/{guid}/import", axum::routing::post(rss::import_item))
        .route("/api/sets", axum::routing::get(sets::list_sets).post(sets::create_set))
        .route("/api/sets/{id}", axum::routing::delete(sets::delete_set))
        // DMs
        .route("/api/dms/conversations", axum::routing::get(dms::list_conversations))
        .route("/api/dms/send", axum::routing::post(dms::send_dm))
        .route("/api/dms/{conversation_id}/messages", axum::routing::get(dms::get_messages))
        // Automation
        .route("/api/automation/rules", axum::routing::get(automation::list_rules).post(automation::create_rule))
        .route("/api/automation/rules/{id}", axum::routing::put(automation::update_rule).delete(automation::delete_rule))
        .route("/api/automation/rules/{id}/logs", axum::routing::get(automation::get_logs))
        // CSRF defense-in-depth: validates Origin (or Referer fallback)
        // on all state-changing requests. SameSite=Lax already blocks
        // cross-site POST/PUT/DELETE cookie sends, but this catches:
        //   (a) any future GET-with-side-effects route (Lax allows GET),
        //   (b) attacks if SameSite is later relaxed to None for HTTPS.
        // The check is a no-op for same-origin requests (which is the
        // only legitimate origin in single-user mode).
        .layer(middleware::from_fn_with_state(
            CsrfState { allowed_origin: state.config.frontend_url.clone() },
            csrf_origin_check,
        ))
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
            // Use the resolved asset key for MIME guessing, not the original
            // request path — empty path "/" would guess octet-stream instead
            // of text/html for the index.html fallback.
            let resolved_key = if FrontendAssets::get(path).is_some() {
                path.to_string()
            } else if path.is_empty() || path.ends_with('/') {
                format!("{path}index.html")
            } else {
                path.to_string()
            };
            let mime = mime_guess::from_path(&resolved_key).first_or_octet_stream();
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

/// Health check endpoint — liveness probe (no DB call, returns 200 if
/// the process is up and the router is dispatching).
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Global drain flag — set to `true` when the process receives
/// SIGTERM/SIGINT. The `/ready` endpoint checks this and returns 503
/// so the load balancer stops routing traffic during the shutdown
/// drain window. This prevents new HTTP requests from creating posts
/// that will be killed mid-publish when the process exits.
static DRAINING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Mark the process as draining — called from main.rs on shutdown signal.
pub fn set_draining() {
    DRAINING.store(true, std::sync::atomic::Ordering::SeqCst);
    tracing::info!("Process marked as draining — /ready will return 503");
}

/// Readiness check — pings the DB with `SELECT 1` to verify the
/// connection pool is healthy. Container orchestrators should probe
/// `/ready` (not `/health`) before routing traffic, so a postgress
/// outage takes the instance out of rotation without killing the
/// process (which would prevent it from reconnecting).
///
/// Returns 503 if the process is draining (shutdown in progress) so
/// the load balancer stops sending new traffic. Existing in-flight
/// requests continue to be served — only new connections are
/// de-registered by the LB.
async fn ready_check(db: PgPool) -> Result<axum::Json<serde_json::Value>, (StatusCode, axum::Json<serde_json::Value>)> {
    // If we're draining, return 503 immediately — don't even ping the DB.
    if DRAINING.load(std::sync::atomic::Ordering::SeqCst) {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "status": "draining",
                "database": true,
                "version": env!("CARGO_PKG_VERSION"),
            })),
        ));
    }

    let db_ok = match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&db).await {
        Ok(1) => true,
        Ok(_) => false,
        Err(e) => {
            tracing::warn!("Readiness check DB ping failed: {e}");
            false
        }
    };
    let status = if db_ok { "ok" } else { "degraded" };
    Ok(axum::Json(serde_json::json!({
        "status": status,
        "database": db_ok,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

/// Metrics endpoint — returns per-platform publish counts, queue
/// depth, and token-expiry status. Designed for Prometheus scrape
/// (JSON now; can add `text/plain; version=0.0.4` format later).
///
/// This is the operator's primary observability surface for the
/// scheduler. Without it, the only way to see "X failed 47 times in
/// the last hour" was to grep logs.
///
/// Uses runtime `sqlx::query` (not the `query!` macro) so the build
/// doesn't require a live DB or the .sqlx offline cache for these
/// new aggregate queries.
async fn metrics_check(db: PgPool) -> axum::Json<serde_json::Value> {
    #[derive(Default, sqlx::FromRow)]
    struct PostCounts {
        draft: i64,
        queued: i64,
        publishing: i64,
        published: i64,
        error: i64,
    }

    #[derive(Default, sqlx::FromRow)]
    struct IntegrationCounts {
        total: i64,
        disabled: i64,
        refresh_needed: i64,
        expiring_soon: i64,
    }

    #[derive(Default, sqlx::FromRow)]
    struct AttemptCounts {
        success: i64,
        failed: i64,
    }

    let post_counts: PostCounts = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE state = 'draft') as draft,
            COUNT(*) FILTER (WHERE state = 'queued') as queued,
            COUNT(*) FILTER (WHERE state = 'publishing') as publishing,
            COUNT(*) FILTER (WHERE state = 'published') as published,
            COUNT(*) FILTER (WHERE state = 'error') as error
           FROM posts",
    )
    .fetch_one(&db)
    .await
    .unwrap_or_default();

    let integration_counts: IntegrationCounts = sqlx::query_as(
        "SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE disabled = true) as disabled,
            COUNT(*) FILTER (WHERE refresh_needed = true) as refresh_needed,
            COUNT(*) FILTER (WHERE token_expires_at IS NOT NULL AND token_expires_at < NOW() + INTERVAL '24 hours') as expiring_soon
           FROM integrations",
    )
    .fetch_one(&db)
    .await
    .unwrap_or_default();

    let recent_attempts: AttemptCounts = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'success') as success,
            COUNT(*) FILTER (WHERE status = 'failed') as failed
           FROM publish_attempts
           WHERE started_at > NOW() - INTERVAL '1 hour'",
    )
    .fetch_one(&db)
    .await
    .unwrap_or_default();

    axum::Json(serde_json::json!({
        "posts": {
            "draft": post_counts.draft,
            "queued": post_counts.queued,
            "publishing": post_counts.publishing,
            "published": post_counts.published,
            "error": post_counts.error,
        },
        "integrations": {
            "total": integration_counts.total,
            "disabled": integration_counts.disabled,
            "refresh_needed": integration_counts.refresh_needed,
            "expiring_soon": integration_counts.expiring_soon,
        },
        "publish_attempts_last_1h": {
            "success": recent_attempts.success,
            "failed": recent_attempts.failed,
        },
        "draining": DRAINING.load(std::sync::atomic::Ordering::SeqCst),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Streak endpoint — returns the user's current posting streak.
/// Used by the frontend flame icon in the top bar.
async fn streak_check(db: PgPool) -> axum::Json<serde_json::Value> {
    use sqlx::Row;
    let user_id = crate::auth::middleware::DEFAULT_USER_ID;
    let row = sqlx::query(
        "SELECT streak_days, streak_since FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&db)
    .await;

    match row {
        Ok(Some(r)) => {
            let streak_days: i32 = r.try_get("streak_days").unwrap_or(0);
            let streak_since: Option<chrono::DateTime<chrono::Utc>> = r.try_get("streak_since").ok();
            axum::Json(serde_json::json!({
                "streak_days": streak_days,
                "streak_since": streak_since.map(|d| d.to_rfc3339()),
            }))
        }
        _ => axum::Json(serde_json::json!({
            "streak_days": 0,
            "streak_since": null,
        })),
    }
}

/// State carried by the CSRF middleware.
#[derive(Clone)]
struct CsrfState {
    /// The single allowed origin for state-changing requests.
    /// Defaults to `frontend_url` from config (which itself defaults
    /// to `app_url`). Set to `*` to disable the check (not recommended).
    allowed_origin: String,
}

/// CSRF defense-in-depth: for any non-GET/non-HEAD/non-OPTIONS request,
/// verify that the `Origin` header (or `Referer` as fallback) matches
/// the configured allowed origin. Rejects with 403 otherwise.
///
/// This is a defense-in-depth layer on top of SameSite=Lax cookies:
/// Lax already blocks cross-site POST/PUT/DELETE cookie sends, but
/// (a) GET requests are still allowed cross-site under Lax, and any
/// future GET-with-side-effects route would be vulnerable, and
/// (b) if SameSite is relaxed to `None` for HTTPS deployment, this
/// becomes the primary CSRF defense.
///
/// Safe methods (GET, HEAD, OPTIONS) are not checked — they should
/// not have side effects. If a future GET route ever does have a
/// side effect, fix the route, not this middleware.
async fn csrf_origin_check(
    State(csrf): State<CsrfState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    use axum::http::{header, Method};

    // Skip check for safe methods.
    let method = req.method().clone();
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(req).await;
    }

    // If allowed_origin is `*`, the check is disabled (operator opted out).
    if csrf.allowed_origin == "*" || csrf.allowed_origin.is_empty() {
        return next.run(req).await;
    }

    // Check Origin header first, then fall back to Referer.
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Extract just the origin part of Referer (scheme://host[:port])
    // by parsing it as a URL and reading the origin string.
    let referer_origin = referer.as_deref().and_then(|r| {
        url::Url::parse(r)
            .ok()
            .map(|u| u.origin().ascii_serialization())
    });

    let request_origin = origin.clone().or(referer_origin.clone());

    // v23-9: support comma-separated allowed origins. Previously the
    // operator could only set a single origin, so a deployment reachable
    // via multiple origins (e.g. https://social-forge.example.com AND
    // http://localhost:6543 for local dev) had to set CSRF_ALLOWED_ORIGIN=*
    // (disabling the check entirely). Now the operator can set
    // FRONTEND_URL=https://a.com,http://localhost:6543 and each is checked.
    let allowed_origins: Vec<&str> = csrf.allowed_origin
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let allowed = match request_origin.as_deref() {
        Some(o) => allowed_origins.iter().any(|&allowed| {
            o == allowed || o == allowed.trim_end_matches('/')
        }),
        None => false,
    };

    if !allowed {
        tracing::warn!(
            "CSRF check failed: origin={origin:?} referer={referer:?} allowed={}",
            csrf.allowed_origin
        );
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({
                    "error": "Cross-origin request blocked",
                    "code": "csrf_origin_mismatch",
                })
                .to_string(),
            ))
            .unwrap();
    }

    next.run(req).await
}


