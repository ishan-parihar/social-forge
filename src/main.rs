// ─── Social Forge ──────────────────────────────────────────────
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

use clap::Parser;

use social_forge::api;
use social_forge::cli::{self, Cli, Command};
use social_forge::config;
use social_forge::db;
use social_forge::mcp;
use social_forge::rss;
use social_forge::scheduler;

use anyhow::Context;

use social_forge::api::AppState;
use social_forge::realtime::Broadcaster;
use social_forge::services::telegram_client::TelegramClientManager;
use social_forge::social::registry::ProviderRegistry;
use social_forge::wa::WhaClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install rustls crypto provider for TLS
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli_args = Cli::parse();

    // Extract port for server mode
    let port = match &cli_args.command {
        Command::Serve { port } => *port,
        _ => 6543,
    };

    // Dispatch non-server commands to CLI handler
    match &cli_args.command {
        Command::Serve { .. } | Command::Mcp => {
            // Fall through to server startup below
        }
        _ => {
            return cli::run_cli(cli_args).await;
        }
    }

    // ── Init logging ──────────────────────────────────────────
    social_forge::config::load_dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,social_forge=debug".into()),
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

    // Ensure the single local user row exists (needed for FK constraints)
    if let Err(e) = db::ensure_local_user(&db).await {
        tracing::warn!("Failed to ensure local user row: {e} — DB inserts may fail");
    }

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
    let wa_client: social_forge::wa::OptionalWhaClient =
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
        .and_then(|k| social_forge::crypto::decode_hex_key(k).ok());
    if token_key.is_some() {
        tracing::info!("Token encryption at rest: ENABLED");
    } else {
        tracing::warn!("Token encryption at rest: DISABLED (set TOKEN_ENCRYPTION_KEY for production)");
    }

    // ── Build shared state (clone for MCP if needed) ──────────
    let media_http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        // No total timeout — needed for streaming large video files via proxy.
        // The browser manages video playback timeout independently.
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // wreq client with Chrome TLS fingerprinting for X/Twitter CDN.
    // video.twimg.com blocks requests that don't match a real browser's TLS fingerprint.
    let media_wreq_client = wreq::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .emulation(wreq_util::Emulation::Chrome131)
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap_or_else(|_| wreq::Client::new());

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*providers_arc).clone(),
        rate_limiter,
        token_key,
        telegram_client_manager: telegram_client_manager.clone(),
        wa_client,
        media_http_client,
        media_wreq_client,
    };
    let state_for_mcp = state.clone();

    // ── Start scheduler ───────────────────────────────────────
    let (shutdown_tx, _) = tokio::sync::watch::channel(false);
    let scheduler_rx = shutdown_tx.subscribe();
    scheduler::start_scheduler(
        db.clone(),
        providers_arc.clone(),
        broadcaster.clone(),
        token_key,
        scheduler_rx,
    );

    // ── Start RSS poller ─────────────────────────────────────
    let rss_rx = shutdown_tx.subscribe();
    rss::start_rss_poller(
        db.clone(),
        providers_arc.clone(),
        Arc::new(config.clone()),
        rss_rx,
    );

    // ── Start analytics cache refresh ─────────────────────────
    let cache_db = db.clone();
    let cache_providers = providers_arc.clone();
    let cache_shutdown = shutdown_tx.subscribe();
    let cache_token_key = token_key;
    tokio::spawn(async move {
        scheduler::run_analytics_cache_refresh(
            cache_db,
            cache_providers,
            cache_token_key,
            cache_shutdown,
        )
        .await;
    });

    // ── Start feed refresher ────────────────────────────────────
    let feed_rx = shutdown_tx.subscribe();
    social_forge::feed::start_feed_refresher(
        db.clone(),
        providers_arc.clone(),
        broadcaster.clone(),
        token_key,
        feed_rx,
    );

    // ── Build HTTP router ─────────────────────────────────────
    let app = api::build_router(state);

    // ── Start HTTP/HTTPS server ─────────────────────────────
    let http_addr = format!("0.0.0.0:{port}");
    let use_tls = config.app_url.starts_with("https://");

    if use_tls {
        // Generate self-signed cert for localhost (or use mkcert certs if available)
        let cert_path = std::path::Path::new("data/tls/cert.pem");
        let key_path = std::path::Path::new("data/tls/key.pem");

        if !cert_path.exists() {
            std::fs::create_dir_all("data/tls")?;
            let mut params = rcgen::CertificateParams::new(vec![
                "localhost".into(),
                "127.0.0.1".into(),
            ])?;
            params.subject_alt_names.push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)));
            let key_pair = rcgen::KeyPair::generate()?;
            let cert = params.self_signed(&key_pair)?;
            std::fs::write(cert_path, cert.pem())?;
            std::fs::write(key_path, key_pair.serialize_pem())?;
            tracing::info!("Generated self-signed TLS cert at data/tls/");
            tracing::warn!("⚠️  For browsers: trust the cert or use mkcert. See README.");
        }

        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .context("Failed to load TLS config")?;

        let addr: std::net::SocketAddr = http_addr.parse().unwrap();
        tokio::spawn(async move {
            if let Err(e) = axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await
            {
                tracing::error!("HTTPS server error: {e}");
            }
        });
        tracing::info!("HTTPS server: https://localhost:{port}/");
    } else {
        let listener = tokio::net::TcpListener::bind(&http_addr)
            .await
            .context("Failed to bind HTTP listener")?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("HTTP server error: {e}");
            }
        });
        tracing::info!("HTTP server: http://{http_addr}/");
    }

    let scheme = if use_tls { "https" } else { "http" };
    tracing::info!("Dashboard: {scheme}://localhost:{port}/");
    tracing::info!("Setup: {scheme}://localhost:{port}/setup");
    tracing::info!("OAuth redirect_uri: {}/api/auth/callback", config.app_url);

    // ── MCP mode: also start MCP stdio server ────────────────
    if matches!(cli_args.command, Command::Mcp) {
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
