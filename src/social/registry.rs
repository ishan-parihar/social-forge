// ─── Provider Registry ────────────────────────────────────────
// Central registry of all available social media providers.
// Used by both the API layer and the MCP layer to route requests.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::config::Config;

/// Thread-safe provider registry
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Arc<HashMap<&'static str, Arc<dyn SocialProvider>>>,
}

impl ProviderRegistry {
    /// Build registry with all providers, given app config for credentials
    pub fn new(config: &Config) -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn SocialProvider>> = HashMap::new();

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

        tracing::info!(
            "Provider registry initialized with: {}",
            providers.keys().cloned().collect::<Vec<_>>().join(", ")
        );

        Self {
            providers: Arc::new(providers),
        }
    }

    /// Get a provider by identifier
    pub fn get(&self, identifier: &str) -> Option<Arc<dyn SocialProvider>> {
        self.providers.get(identifier).cloned()
    }

    /// List all registered provider identifiers
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }

    /// Get all providers
    #[allow(dead_code)]
    pub fn all(&self) -> Vec<Arc<dyn SocialProvider>> {
        self.providers.values().cloned().collect()
    }
}
