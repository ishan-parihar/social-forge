// ─── API Router ────────────────────────────────────────────────
// axum HTTP router combining all route modules.
// Protected routes use the auth middleware chain.

use axum::{middleware, Extension, Router};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::auth::middleware::{auth_middleware, JwtSecret};
use crate::config::Config;
use crate::db::PgPool;
use crate::realtime::Broadcaster;
use crate::services::telegram_client::OptionalTelegramClient;
use crate::social::registry::ProviderRegistry;

use self::rate_limiter::AuthRateLimiter;

mod analytics;
mod auth;
mod calendar;
mod integrations;
mod media;
mod onboard;
mod posts;
pub mod rate_limiter;
mod sse;
mod tags;
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
}

/// Build the axum router with all routes
pub fn build_router(state: AppState) -> Router {
    let jwt_secret = JwtSecret(state.config.jwt_secret.clone());

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
        .route("/api/public/connect/{provider}", axum::routing::get(onboard::public_connect));

    // Protected routes — auth required
    let protected_routes = Router::new()
        .route("/api/auth/me", axum::routing::get(auth::me))
        .route("/api/posts", axum::routing::get(posts::list).post(posts::create))
        .route("/api/providers", axum::routing::get(integrations::list_providers))
        .route(
            "/api/posts/{id}",
            axum::routing::get(posts::get)
                .put(posts::update)
                .delete(posts::delete),
        )
        .route("/api/posts/{id}/schedule", axum::routing::post(posts::schedule))
        .route("/api/posts/{id}/publish", axum::routing::post(posts::publish_post))
        .route("/api/posts/find-slot", axum::routing::get(posts::find_slot))
        .route("/api/integrations", axum::routing::get(integrations::list))
        .route("/api/integrations/connect/{provider}", axum::routing::get(integrations::connect))
        .route("/api/integrations/{id}", axum::routing::delete(integrations::delete))
        .route("/api/integrations/{id}/available-pages", axum::routing::get(integrations::available_pages))
        .route("/api/integrations/{parent_id}/connect-page/{page_id}", axum::routing::post(integrations::connect_page))
        .route("/api/calendar", axum::routing::get(calendar::get))
        .route("/api/media", axum::routing::get(media::list).post(media::upload))
        .route("/api/analytics", axum::routing::get(analytics::get))
        .route("/api/analytics/post/{id}", axum::routing::get(analytics::get_post))
        .route("/api/tags", axum::routing::get(tags::list).post(tags::create))
        .route("/api/tags/{id}", axum::routing::get(tags::get).put(tags::update).delete(tags::delete))
        .route("/api/webhooks", axum::routing::get(webhooks::list).post(webhooks::create))
        .route("/api/webhooks/{id}", axum::routing::get(webhooks::get).put(webhooks::update).delete(webhooks::delete))
        .route("/api/webhooks/{id}/test", axum::routing::post(webhooks::test))
        // Auth middleware chain: inject secret first, then validate
        .layer(middleware::from_fn(auth_middleware))
        .layer(Extension(jwt_secret));

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


