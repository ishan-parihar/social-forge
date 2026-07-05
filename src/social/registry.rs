// ─── Provider Registry ────────────────────────────────────────
// Central registry of all available social media providers.
// Used by both the API layer and the MCP layer to route requests.
//
// Also holds a per-provider concurrency limiter (`Semaphore`) so the
// scheduler can't fire 30 simultaneous posts at the same X account
// and trip per-account rate limits. The limit defaults to 1 for
// strict-serial platforms (X, Threads, Instagram — all have aggressive
// per-account rate windows) and 3 for platforms with more headroom
// (Reddit, Discord, Slack, Telegram-Bot, etc.). Override via the
// `PROVIDER_CONCURRENCY_{IDENTIFIER}` env var (uppercased, hyphens →
// underscores, e.g. `PROVIDER_CONCURRENCY_LINKEDIN_PAGE=2`).

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use super::farcaster;
use super::kick;
use super::mastodon;
use super::slack;
use crate::config::Config;
use crate::services::telegram_client::TelegramClientManager;
use crate::wa::WhaClient;

/// Default per-provider concurrent publish budget. Conservative to
/// avoid tripping per-account rate limits on the strictest platforms
/// (X free tier ≈ 17 posts / 24h; we don't want to blow through
/// that in a single scheduler tick).
const DEFAULT_PROVIDER_CONCURRENCY: usize = 1;

/// Providers that can safely handle more concurrent calls (their rate
/// limits are per-IP or per-token-with-high-ceiling, not per-account).
const HIGH_CONCURRENCY_PROVIDERS: &[&str] = &[
    "reddit",
    "discord",
    "slack",
    "telegram-bot",
    "telegram-user",
    "whatsapp",
    "github",
    "wordpress",
    "medium",
    "devto",
    "hashnode",
    "lemmy",
    "vk",
    "kick",
    "skool",
];

/// Thread-safe provider registry
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Arc<HashMap<&'static str, Arc<dyn SocialProvider>>>,
    /// Per-provider concurrency limiter. Each entry is an
    /// `Arc<Semaphore>` sized to the platform's concurrent-post budget.
    /// The scheduler acquires a permit before calling `provider.publish()`
    /// and releases it when the publish completes (success or failure).
    concurrency: Arc<HashMap<&'static str, Arc<tokio::sync::Semaphore>>>,
    /// Per-provider circuit breaker. When a provider has N consecutive
    /// failures (e.g. 5xx from the platform API), the circuit opens and
    /// subsequent publish attempts are skipped for a cooldown period
    /// (default 60s) instead of burning through all queued posts.
    /// After the cooldown, the circuit goes half-open: one request is
    /// allowed through; if it succeeds, the circuit closes; if it fails,
    /// the cooldown restarts.
    circuit_breakers: Arc<HashMap<&'static str, Arc<CircuitBreaker>>>,
}

/// Per-provider circuit breaker with three states: closed, open, half-open.
///
/// - **Closed**: all requests pass through. Failure count tracked.
/// - **Open**: all requests rejected immediately for `cooldown_secs`.
///   After cooldown, transitions to half-open.
/// - **Half-open**: one request allowed through. If it succeeds → closed.
///   If it fails → back to open with full cooldown.
///
/// This prevents a platform outage (e.g. X 5xx for 10 minutes) from
/// cascading to every queued post. Without it, 50 queued X posts would
/// each independently fail their 3 retries = 150 doomed API calls.
pub struct CircuitBreaker {
    state: std::sync::atomic::AtomicU8, // 0=closed, 1=open, 2=half-open
    failure_count: std::sync::atomic::AtomicU32,
    opened_at: std::sync::atomic::AtomicI64, // unix timestamp
    /// Number of consecutive failures before opening.
    failure_threshold: u32,
    /// Seconds to wait before transitioning from open to half-open.
    cooldown_secs: i64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_secs: i64) -> Self {
        Self {
            state: std::sync::atomic::AtomicU8::new(0), // closed
            failure_count: std::sync::atomic::AtomicU32::new(0),
            opened_at: std::sync::atomic::AtomicI64::new(0),
            failure_threshold,
            cooldown_secs,
        }
    }

    /// Returns `true` if the request should be allowed through,
    /// `false` if the circuit is open (request should be skipped).
    ///
    /// If the circuit is open but the cooldown has elapsed, this
    /// transitions to half-open and allows one request through.
    pub fn allow_request(&self) -> bool {
        use std::sync::atomic::Ordering;
        let state = self.state.load(Ordering::SeqCst);
        match state {
            0 => true, // closed
            1 => {
                // open — check if cooldown elapsed
                let now = chrono::Utc::now().timestamp();
                let opened = self.opened_at.load(Ordering::SeqCst);
                if now - opened >= self.cooldown_secs {
                    // transition to half-open
                    self.state.store(2, Ordering::SeqCst);
                    tracing::info!("Circuit breaker → half-open (cooldown elapsed)");
                    true
                } else {
                    false
                }
            }
            2 => true, // half-open — allow one request
            _ => true,
        }
    }

    /// Record a successful request. Resets failure count and closes
    /// the circuit (if it was open or half-open).
    pub fn record_success(&self) {
        use std::sync::atomic::Ordering;
        let prev = self.state.swap(0, Ordering::SeqCst); // closed
        self.failure_count.store(0, Ordering::SeqCst);
        if prev != 0 {
            tracing::info!("Circuit breaker → closed (success recorded)");
        }
    }

    /// Record a failed request. Increments failure count; if it
    /// reaches the threshold, opens the circuit.
    pub fn record_failure(&self) {
        use std::sync::atomic::Ordering;
        let count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        if count >= self.failure_threshold {
            let prev = self.state.swap(1, Ordering::SeqCst); // open
            self.opened_at.store(chrono::Utc::now().timestamp(), Ordering::SeqCst);
            if prev != 1 {
                tracing::warn!(
                    "Circuit breaker → open ({} consecutive failures)",
                    count
                );
            }
        }
    }

    /// Current state as a string for metrics/debugging.
    pub fn state_str(&self) -> &'static str {
        use std::sync::atomic::Ordering;
        match self.state.load(Ordering::SeqCst) {
            0 => "closed",
            1 => "open",
            2 => "half-open",
            _ => "unknown",
        }
    }
}

impl ProviderRegistry {
    /// Build registry with all providers, given app config for credentials
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &Config,
        telegram_client_manager: Option<Arc<TelegramClientManager>>,
        wa_client: Option<Arc<tokio::sync::Mutex<WhaClient>>>,
    ) -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn SocialProvider>> = HashMap::new();

        // Current providers
        providers.insert("x", Arc::new(x::XProvider::new(config)));
        providers.insert(
            "linkedin",
            Arc::new(linkedin::LinkedInProvider::new(config)),
        );
        providers.insert("bluesky", Arc::new(bluesky::BlueskyProvider::new(config)));
        providers.insert(
            "facebook",
            Arc::new(facebook::FacebookProvider::new(config)),
        );
        providers.insert(
            "instagram",
            Arc::new(instagram::InstagramProvider::new(config)),
        );

        // New providers (with credentials)
        let linkedin_page = linkedin_page::LinkedInPageProvider::new(config);
        // Only add if credential check passes — LinkedIn page uses same credentials as LinkedIn
        if config.linkedin_client_id.is_some() {
            providers.insert("linkedin-page", Arc::new(linkedin_page));
        }

        if config.instagram_app_id.is_some() {
            providers.insert(
                "instagram-standalone",
                Arc::new(instagram_standalone::InstagramStandaloneProvider::new(
                    config,
                )),
            );
        }

        if config.threads_app_id.is_some() {
            providers.insert("threads", Arc::new(threads::ThreadsProvider::new(config)));
        }

        // Always registered (show on frontend even without credentials)
        providers.insert("reddit", Arc::new(reddit::RedditProvider::new(config)));

        if config.discord_client_id.is_some() {
            providers.insert("discord", Arc::new(discord::DiscordProvider::new(config)));
        }

        // Telegram Bot — token-based accounts (comma-separated TELEGRAM_BOT_TOKENS)
        if config.telegram_bot_tokens.is_some() {
            providers.insert(
                "telegram-bot",
                Arc::new(telegram_bot::TelegramBotProvider::new(config)),
            );
        }

        // Telegram User — Grammers-based MTProto client (always registered)
        providers.insert(
            "telegram-user",
            Arc::new(telegram_user::TelegramUserProvider::new(config, telegram_client_manager.clone())),
        );

        // Always registered (show on frontend even without credentials)
        providers.insert("pinterest", Arc::new(pinterest::PinterestProvider::new(config)));

        // WhatsApp — native wa-rs client with wacli fallback
        providers.insert("whatsapp", Arc::new(whatsapp::WhatsAppProvider::new(config, wa_client.clone())));

        // TikTok — OAuth-based video platform
        if config.tiktok_client_id.is_some() {
            providers.insert("tiktok", Arc::new(tiktok::TikTokProvider::new(config)));
        }

        // VK — OAuth-based social network
        if config.vk_client_id.is_some() {
            providers.insert("vk", Arc::new(vk::VkProvider::new(config)));
        }

        // Google My Business — uses same Google OAuth credentials
        if config.youtube_client_id.is_some() && config.youtube_client_secret.is_some() {
            providers.insert(
                "google_my_business",
                Arc::new(google_my_business::GoogleMyBusinessProvider::new(config)),
            );
        }

        // Whop — OAuth-based community commerce platform
        if config.whop_client_id.is_some() {
            providers.insert("whop", Arc::new(whop::WhopProvider::new(config)));
        }

        // Kick — OAuth-based streaming platform
        if config.kick_client_id.is_some() {
            providers.insert("kick", Arc::new(kick::KickProvider::new(config)));
        }

        // Mastodon — OAuth-based microblogging (with app registration)
        if config.mastodon_client_id.is_some() {
            providers.insert("mastodon", Arc::new(mastodon::MastodonProvider::new(config)));
        }

        // Medium — API key-based publishing
        if config.medium_access_token.is_some() {
            providers.insert("medium", Arc::new(medium::MediumProvider::new(config)));
        }

        // Dev.to — API key-based publishing
        if config.devto_api_key.is_some() {
            providers.insert("devto", Arc::new(devto::DevtoProvider::new(config)));
        }

        // Hashnode — API key-based blogging
        if config.hashnode_api_key.is_some() {
            providers.insert("hashnode", Arc::new(hashnode::HashnodeProvider::new(config)));
        }

        // GitHub — PAT-based (always registered, shows as configured if GITHUB_TOKEN is set)
        if config.github_token.is_some() {
            providers.insert("github", Arc::new(github::GithubProvider::new(config)));
        }

        // YouTube — dedicated provider for importing recent videos
        // Uses YOUTUBE_CLIENT_ID / YOUTUBE_CLIENT_SECRET
        if config.youtube_client_id.is_some() {
            providers.insert("youtube", Arc::new(youtube::YoutubeProvider::new(config)));
        }

        // Google Suite — unified provider for YouTube, Gmail, Calendar, Drive
        // Uses YOUTUBE_CLIENT_ID / YOUTUBE_CLIENT_SECRET for all Google OAuth scopes
        if config.youtube_client_id.is_some() {
            providers.insert("google", Arc::new(google::GoogleProvider::new(config)));
        }

        // Chrome extension-based provider (no OAuth credentials needed)
        providers.insert("skool", Arc::new(skool::SkoolProvider::new()));

        // WordPress — REST API + Application Password (always registered, no global credentials)
        providers.insert("wordpress", Arc::new(wordpress::WordPressProvider::new(config)));

        // Farcaster — Web3-based (always registered, no OAuth)
        providers.insert("farcaster", Arc::new(farcaster::FarcasterProvider::new(config)));

        // Lemmy — API key-based (always registered, no global credentials — per-user in integration record)
        providers.insert("lemmy", Arc::new(lemmy::LemmyProvider::new(config)));

        // Slack — OAuth-based messaging workspace provider
        if config.slack_client_id.is_some() {
            providers.insert("slack", Arc::new(slack::SlackProvider::new(config)));
        }

        tracing::info!(
            "Provider registry initialized with: {}",
            providers.keys().cloned().collect::<Vec<_>>().join(", ")
        );

        // Build per-provider concurrency semaphores. Each provider
        // gets a budget based on its platform's rate-limit profile.
        let mut concurrency: HashMap<&'static str, Arc<tokio::sync::Semaphore>> = HashMap::new();
        let mut circuit_breakers: HashMap<&'static str, Arc<CircuitBreaker>> = HashMap::new();
        for &id in providers.keys() {
            let default = if HIGH_CONCURRENCY_PROVIDERS.contains(&id) {
                3
            } else {
                DEFAULT_PROVIDER_CONCURRENCY
            };
            // Allow env override: PROVIDER_CONCURRENCY_X=2 etc.
            let env_key = format!(
                "PROVIDER_CONCURRENCY_{}",
                id.to_uppercase().replace('-', "_")
            );
            let limit = std::env::var(&env_key)
                .ok()
                .and_then(|v| v.parse().ok())
                .filter(|v: &usize| *v >= 1 && *v <= 20)
                .unwrap_or(default);
            concurrency.insert(id, Arc::new(tokio::sync::Semaphore::new(limit)));

            // Circuit breaker: 5 consecutive failures → open for 60s.
            // Configurable via PROVIDER_CB_THRESHOLD_{ID} and
            // PROVIDER_CB_COOLDOWN_{ID} env vars.
            let threshold: u32 = std::env::var(format!(
                "PROVIDER_CB_THRESHOLD_{}",
                id.to_uppercase().replace('-', "_")
            ))
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|v| *v >= 1 && *v <= 50)
            .unwrap_or(5);
            let cooldown: i64 = std::env::var(format!(
                "PROVIDER_CB_COOLDOWN_{}",
                id.to_uppercase().replace('-', "_")
            ))
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|v| *v >= 10 && *v <= 3600)
            .unwrap_or(60);
            circuit_breakers.insert(id, Arc::new(CircuitBreaker::new(threshold, cooldown)));
        }

        Self {
            providers: Arc::new(providers),
            concurrency: Arc::new(concurrency),
            circuit_breakers: Arc::new(circuit_breakers),
        }
    }

    /// Get a provider by identifier
    pub fn get(&self, identifier: &str) -> Option<Arc<dyn SocialProvider>> {
        self.providers.get(identifier).cloned()
    }

    /// Get the per-provider concurrency semaphore for the given
    /// provider. Returns `None` if the provider isn't registered.
    /// The caller should `.acquire()` before making the platform API
    /// call and hold the permit until the call completes.
    pub fn concurrency(&self, identifier: &str) -> Option<Arc<tokio::sync::Semaphore>> {
        self.concurrency.get(identifier).cloned()
    }

    /// Get the per-provider circuit breaker. The scheduler calls
    /// `allow_request()` before publishing — if it returns `false`,
    /// the post is left in `queued` state (not marked as error) and
    /// retried on the next tick after the cooldown elapses.
    pub fn circuit_breaker(&self, identifier: &str) -> Option<Arc<CircuitBreaker>> {
        self.circuit_breakers.get(identifier).cloned()
    }

    /// List all registered provider identifiers
    pub fn list(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }

    /// Get all providers
    pub fn all(&self) -> Vec<Arc<dyn SocialProvider>> {
        self.providers.values().cloned().collect()
    }
}
