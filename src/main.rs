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

use std::path::PathBuf;
use std::sync::Arc;

use postiz_rust::api;
use postiz_rust::config;
use postiz_rust::db;
use postiz_rust::mcp;
use postiz_rust::scheduler;

use anyhow::Context;

use postiz_rust::api::AppState;
use postiz_rust::realtime::Broadcaster;
use postiz_rust::services::telegram_client::TelegramClientManager;
use postiz_rust::social::registry::ProviderRegistry;
use postiz_rust::wa::WhaClient;

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

    // ── Telegram user client (Grammers) ──────────────────────
    let telegram_client_manager = if let (Some(api_id_str), Some(api_hash)) =
        (&config.telegram_api_id, &config.telegram_api_hash)
    {
        let api_id: i32 = api_id_str.parse().map_err(|e| {
            anyhow::anyhow!("Invalid TELEGRAM_API_ID: expected numeric, got {api_id_str}: {e}")
        })?;
        let session_dir = config
            .telegram_session_dir
            .clone()
            .unwrap_or_else(|| "./data/telegram".into());
        Some(Arc::new(TelegramClientManager::new(
            api_id,
            api_hash.clone(),
            PathBuf::from(session_dir),
        )))
    } else {
        tracing::warn!(
            "TELEGRAM_API_ID / TELEGRAM_API_HASH not set — Telegram user client disabled"
        );
        None
    };

    // ── WhatsApp Web client (wa-rs replaces Go wacli sidecar) ──
    let wa_client: postiz_rust::wa::OptionalWhaClient =
        if let Some(dir) = &config.whatsapp_store_dir {
            let store_dir = PathBuf::from(dir);
            match WhaClient::new(store_dir).await {
                Ok(client) => {
                    tracing::info!("WhatsApp Web client (wa-rs): initialized");
                    Some(Arc::new(tokio::sync::Mutex::new(client)))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize WhatsApp client: {e}");
                    None
                }
            }
        } else {
            tracing::warn!("WHATSAPP_STORE_DIR not set -- WhatsApp tools disabled");
            None
        };

    // ── Provider registry ─────────────────────────────────────
    let providers = ProviderRegistry::new(
        &config,
        telegram_client_manager.clone(),
        wa_client.clone(),
    );
    let providers_arc = Arc::new(providers);

    // ── Shared app state ─────────────────────────────────────
    let rate_limiter = api::rate_limiter::AuthRateLimiter::new(5, 60); // 5 attempts per 60 seconds

    // Parse optional token encryption key (64 hex chars = 32 bytes)
    let token_key = config
        .token_encryption_key
        .as_ref()
        .and_then(|k| postiz_rust::crypto::decode_hex_key(k).ok());
    if token_key.is_some() {
        tracing::info!("Token encryption at rest: ENABLED");
    } else {
        tracing::warn!("Token encryption at rest: DISABLED (set TOKEN_ENCRYPTION_KEY for production)");
    }

    // ── Build shared state (clone for MCP if needed) ──────────
    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*providers_arc).clone(),
        rate_limiter,
        token_key,
        telegram_client_manager: telegram_client_manager.clone(),
        wa_client,
    };
    let state_for_mcp = state.clone();

    // ── Start scheduler ───────────────────────────────────────
    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    scheduler::start_scheduler(
        db.clone(),
        providers_arc.clone(),
        broadcaster.clone(),
        token_key,
        shutdown_rx,
    );

    // ── Build HTTP router ─────────────────────────────────────
    let app = api::build_router(state);

    // ── Start HTTP server ────────────────────────────────────
    // HTTP on 3443 (internal). HTTPS on 3000 via socat TLS proxy.
    let http_addr = "0.0.0.0:3443".to_string();

    let listener = tokio::net::TcpListener::bind(&http_addr)
        .await
        .context("Failed to bind HTTP listener")?;

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("HTTP server error: {e}");
        }
    });

    tracing::info!("REST API: http://{http_addr}/api/");
    tracing::info!("SSE events: http://{http_addr}/api/events");
    tracing::info!("Health check: http://{http_addr}/health");
    tracing::info!("Frontend URL: {}", config.frontend_url);

    // ── MCP mode: also start MCP stdio server ────────────────
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--mcp".to_string()) {
        tracing::info!("Starting in MERGED mode: HTTP on {http_addr} + MCP on stdio");
        tokio::spawn(async move {
            tracing::info!("MCP server started on stdio");
            if let Err(e) = mcp::run_mcp_stdio(state_for_mcp).await {
                tracing::error!("MCP server error: {e}");
            }
        });
    }

    // ── Keep process alive ───────────────────────────────────
    // Spawned tasks (HTTP + scheduler + optional MCP) keep the
    // tokio runtime alive. In interactive terminals, Ctrl+C
    // aborts the process. In Docker/CI, orchestrator sends
    // SIGTERM. Graceful shutdown is not used here because
    // tokio::signal::ctrl_c() does not work in non-TTY shells.
    let () = std::future::pending().await;

    tracing::info!("Server shut down gracefully");
    Ok(())
}
