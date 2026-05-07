// ─── Postiz-Rust ──────────────────────────────────────────────
// Social Media Scheduling Platform — Rust implementation.
//
// Architecture:
//   - axum HTTP server: REST API for SvelteKit frontend
//   - rmcp MCP server: stdio-based tools for AI agents
//   - In-process scheduler: handles post publishing
//   - SSE broadcast: real-time updates for both frontend and agents
//
// Two kinds of end users:
//   1. Humans via SvelteKit frontend (HTTP + SSE)
//   2. AI agents via MCP tools (stdio/SSE)

mod api;
mod auth;
mod config;
mod db;
mod error;
mod mcp;
mod realtime;
mod scheduler;
mod social;

use std::sync::Arc;

use anyhow::Context;

use crate::api::AppState;
use crate::realtime::Broadcaster;
use crate::social::registry::ProviderRegistry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Init logging ──────────────────────────────────────────
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,postiz_rust=debug".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // ── Load config ───────────────────────────────────────────
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");

    // ── Database ──────────────────────────────────────────────
    let db = db::create_pool(&config.database_url)
        .await
        .context("Failed to create database pool")?;
    tracing::info!("Database connected");

    // ── Realtime broadcaster ──────────────────────────────────
    let broadcaster = Broadcaster::new();

    // ── Provider registry ─────────────────────────────────────
    let providers = ProviderRegistry::new(&config);
    let providers_arc = Arc::new(providers);

    // ── Shared app state ─────────────────────────────────────
    let rate_limiter = api::rate_limiter::AuthRateLimiter::new(5, 60); // 5 attempts per 60 seconds

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*providers_arc).clone(),
        rate_limiter,
    };

    // ── Start scheduler ───────────────────────────────────────
    scheduler::start_scheduler(db.clone(), providers_arc.clone(), broadcaster.clone());

    // ── Build HTTP router ─────────────────────────────────────
    let app = api::build_router(state.clone());

    // ── Start servers ─────────────────────────────────────────
    let http_port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let http_addr = format!("0.0.0.0:{http_port}");

    tracing::info!("Starting HTTP server on {http_addr}");
    tracing::info!("REST API: http://{http_addr}/api/");
    tracing::info!("SSE events: http://{http_addr}/api/events");
    tracing::info!("Health check: http://{http_addr}/health");
    tracing::info!("MCP server available on stdio (--mcp flag)");
    tracing::info!("Frontend URL: {}", config.frontend_url);

    let listener = tokio::net::TcpListener::bind(&http_addr)
        .await
        .context("Failed to bind HTTP listener")?;

    // ── Start MCP on stdio if --mcp flag ──────────────────────
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--mcp".to_string()) {
        tracing::info!("Starting in MCP stdio mode");
        mcp::run_mcp_stdio(state).await?;
    } else {
        // Normal HTTP server mode
        axum::serve(listener, app)
            .await
            .context("HTTP server error")?;
    }

    Ok(())
}
