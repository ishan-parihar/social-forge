pub mod auth;
pub mod messages;
pub mod chats;
pub mod groups;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use tokio::sync::Mutex;
use tracing::{info, warn};
use wa_rs::client::Client as WaClient;
use wa_rs::http::HttpClient;
use wa_rs::store::persistence_manager::PersistenceManager;
use wa_rs::store::Backend;
use wa_rs::transport::TransportFactory;
use wa_rs_sqlite_storage::SqliteStore;

/// High-level wrapper around `wa_rs::Client`.
///
/// Handles lifecycle: creation, connection, session persistence,
/// and exposes typed methods for the operations the application needs.
///
/// ## Lifecycle
///
/// 1. `WhaClient::new(store_dir)` — creates the client and initialises the
///    wa-rs SQLite store (Signal protocol keys, prekeys, sessions).
/// 2. `connect()` — establishes the WebSocket transport and, if a prior
///    session exists, resumes it automatically.
/// 3. Use [`is_authenticated`](Self::is_authenticated) to check whether the
///    client has a logged-in session (QR-scan or pair-code already done).
/// 4. For tools that need chat / contact meta-data, see [`chats`] module.
pub struct WhaClient {
    /// Underlying wa-rs client (requires `Arc` for `run()` / `connect()`).
    inner: Arc<WaClient>,
    /// Path to the SQLite store directory for session persistence.
    store_dir: PathBuf,
    /// Whether the client has been connected at least once.
    connected: bool,
}

/// Shared, lazily-initialised WhatsApp client.
///
/// Wired into [`AppState`](crate::api::AppState) as an optional field. When
/// `None`, the legacy Go wacli sidecar is used as a fallback.
pub type OptionalWhaClient = Option<Arc<Mutex<WhaClient>>>;

impl WhaClient {
    /// Create a new `WhaClient` backed by a wa-rs SQLite store at `store_dir`.
    ///
    /// The store directory is created if it does not exist.  Inside it
    /// `wa-rs` manages:
    /// - `wa-store.db` — Signal protocol identity, prekeys, sessions
    /// - `wa-meta.db` — application-level chat summaries / contacts (managed
    ///    by [`crate::wa::chats`])
    ///
    /// # Errors
    ///
    /// - I/O failure creating `store_dir`
    /// - SQLite store initialisation failure
    /// - wa-rs client creation failure (e.g. missing tokio runtime)
    pub async fn new(store_dir: PathBuf) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&store_dir)
            .await
            .with_context(|| format!("failed to create wa-rs store dir {store_dir:?}"))?;

        let db_path = store_dir.join("wa-store.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
        let store: Arc<dyn Backend> = Arc::new(
            SqliteStore::new(&db_url)
                .await
                .with_context(|| format!("failed to open wa-rs SQLite store at {db_url}"))?,
        );
        let persistence = PersistenceManager::new(store)
            .await
            .context("PersistenceManager::new failed")?;
        let transport: Arc<dyn TransportFactory> =
            Arc::new(wa_rs::transport::TokioWebSocketTransportFactory::new());
        let http: Arc<dyn HttpClient> = Arc::new(wa_rs::transport::UreqHttpClient::new());

        let (client, _sync_rx) = WaClient::new(Arc::new(persistence), transport, http, None).await;

        Ok(Self {
            inner: client,
            store_dir,
            connected: false,
        })
    }

    /// Establish (or resume) the WhatsApp Web WebSocket connection.
    ///
    /// If a valid session exists in the store the client will authenticate
    /// silently.  Otherwise the WebSocket is opened but stays in a
    /// waiting-for-scan state — use [`auth::pair_with_code`] or scan the QR
    /// code printed by wa-rs to complete authentication.
    ///
    /// It is safe to call this multiple times; already-connected calls are a
    /// no-op at the wa-rs level.
    ///
    /// # Errors
    ///
    /// - WebSocket connection failure (network / DNS)
    /// - wa-rs internal protocol error during reconnection
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        if self.connected && self.inner.is_connected() {
            return Ok(());
        }
        self.connected = false;
        let inner = Arc::clone(&self.inner);
        // Spawn run() which handles connect + message loop + auto-reconnect
        tokio::spawn(async move { inner.run().await });
        // Wait for connection to establish
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if self.inner.is_connected() {
                break;
            }
        }
        self.connected = self.inner.is_connected();
        if self.inner.is_logged_in() {
            info!("WhatsApp Web client connected (resumed session)");
        } else if self.connected {
            info!("WhatsApp Web socket established - awaiting authentication");
        } else {
            anyhow::bail!("WhatsApp Web connection timed out");
        }
        Ok(())
    }

    /// Force disconnect and reconnect fresh (for pair code retries).
    pub async fn reconnect(&mut self) -> anyhow::Result<()> {
        self.inner.disconnect().await;
        self.connected = false;
        // Brief pause for cleanup
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        self.connect().await
    }

    /// Returns `true` when a WhatsApp Web session has been fully
    /// authenticated (QR scanned or pair code confirmed).
    pub fn is_authenticated(&self) -> bool {
        self.inner.is_logged_in()
    }

    /// Returns `true` when the WebSocket transport is active.
    ///
    /// A connected-but-not-authenticated client can still receive QR /
    /// pair-code events.
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// Borrow the underlying wa-rs `Client` for advanced operations.
    ///
    /// Prefer the typed helpers in [`auth`], [`messages`], [`chats`], and
    /// [`groups`] over calling this directly.
    pub fn inner(&self) -> &Arc<WaClient> {
        &self.inner
    }

    /// Path to the store directory used by this client.
    pub fn store_dir(&self) -> &PathBuf {
        &self.store_dir
    }
}

impl Drop for WhaClient {
    fn drop(&mut self) {
        if self.connected {
            warn!("WhaClient dropped while connected - disconnecting");
        }
    }
}
