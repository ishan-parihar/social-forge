// ─── API Router ────────────────────────────────────────────────
// axum HTTP router combining all route modules.
// Protected routes use the auth middleware chain.

use axum::{middleware, Router};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::auth::middleware::auth_middleware;
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
        .route("/api/auth/register", axum::routing::post(auth::register))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .route("/api/auth/callback", axum::routing::get(integrations::oauth_callback))
        .route("/api/auth/callback/{provider}", axum::routing::get(integrations::oauth_callback))
        .route("/integrations/social/{provider}", axum::routing::get(integrations::oauth_callback))
        .route("/api/events", axum::routing::get(sse::sse_handler))
        .route("/api/media/{id}", axum::routing::get(media::serve_media))
        // Public onboarding — browser-accessible OAuth flow (no JWT header needed)
        .route("/", axum::routing::get(onboard::onboard_page))
        .route("/api/public/connect/x-cookies", axum::routing::get(onboard::x_cookies_form).post(onboard::x_cookies_submit))
.route("/api/public/connect/x-cookies/import", axum::routing::post(onboard::x_cookies_import))
        .route("/api/public/connect/{provider}", axum::routing::get(onboard::public_connect))
        // Stripe webhook — no auth (signature verification in handler)
        .route("/api/billing/webhook", axum::routing::post(billing::stripe_webhook));

    // Protected routes — auth required
    let protected_routes = Router::new()
        .route("/api/auth/me", axum::routing::get(auth::me))
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
        .route("/api/integrations/connect/github-pat", axum::routing::post(integrations::connect_github_pat))
        .route("/api/integrations/connect/web3", axum::routing::post(integrations::connect_web3))
        .route("/api/integrations/{id}", axum::routing::delete(integrations::delete))
        .route("/api/integrations/{id}/available-pages", axum::routing::get(integrations::available_pages))
        .route("/api/integrations/{parent_id}/connect-page/{page_id}", axum::routing::post(integrations::connect_page))
        .route("/api/integrations/{id}/timeslots", axum::routing::put(integrations::update_timeslots))
        .route("/api/integrations/{id}/disable", axum::routing::put(integrations::toggle_disable))
.route("/api/integrations/{id}/refresh", axum::routing::post(integrations::refresh))
        .route("/api/calendar", axum::routing::get(calendar::get))
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
        // Auth middleware: injects DEFAULT_USER_ID for single-user mode
        .layer(middleware::from_fn(auth_middleware));

    // Global middleware stack
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(
            10 * 1024 * 1024, // 10 MB limit
        ))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}


