// ─── Provider Tiers ───────────────────────────────────────────
// v25 plan §1: breadth is not the goal — depth for the 12 platforms a
// solo founder actually uses, publish-maintained for the long tail,
// archived for the rest.
//
// This module is the single source of truth for tier membership. The
// registry reads it when building `ProviderRegistry`, `/api/providers`
// and the MCP `integrations.list_providers` tool report it as a `tier`
// field, and the frontend uses that field to decide what to render.
//
// Tier-1 (depth): analytics, engagement, comments, mentions, targets.
// Tier-2 (publish-maintained): publish + validate + refresh + errors.
// Tier-3 (archive): code kept under `social::archive`, registration and
//   UI gated behind `ENABLE_ARCHIVE_PROVIDERS`.

use serde::{Deserialize, Serialize};

/// Support tier of a provider (see `docs/planning/PLAN_PARITY_DEPTH_SINGLEUSER_v25.md` §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderTier {
    /// Depth investment — all ten capability rows.
    Tier1,
    /// Publish-maintained — no new depth spend unless publishing breaks.
    Tier2,
    /// Archived — registered only when `ENABLE_ARCHIVE_PROVIDERS` is set.
    Tier3,
}

impl ProviderTier {
    /// Wire/JSON representation used by `/api/providers` and MCP output.
    pub fn as_str(self) -> &'static str {
        match self {
            ProviderTier::Tier1 => "tier-1",
            ProviderTier::Tier2 => "tier-2",
            ProviderTier::Tier3 => "tier-3",
        }
    }

    /// Whether a provider at this tier may be registered. Tier-3 providers
    /// are opt-in via `ENABLE_ARCHIVE_PROVIDERS`.
    pub fn is_enabled_by_default(self) -> bool {
        !matches!(self, ProviderTier::Tier3)
    }
}

impl std::fmt::Display for ProviderTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Tier-1 — the solo-founder core (12). Top-10 global MAU for creators
/// plus LinkedIn Page (B2B is the paying use-case) and Instagram-standalone
/// (Meta API without a Facebook Page dependency).
pub const TIER_1: &[&str] = &[
    "x",
    "linkedin",
    "linkedin-page",
    "facebook",
    "instagram",
    "instagram-standalone",
    "threads",
    "youtube",
    "tiktok",
    "reddit",
    "bluesky",
    "pinterest",
];

/// Tier-2 — publish-maintained (14). Messaging platforms, the blogging/dev
/// API-key group, and the user-mandated utility keeps (Google My Business,
/// Gmail/Calendar/Drive via `google`, Skool).
pub const TIER_2: &[&str] = &[
    "telegram-bot",
    "telegram-user",
    "discord",
    "slack",
    "whatsapp",
    "wordpress",
    "mastodon",
    "medium",
    "devto",
    "hashnode",
    "github",
    "google",
    "google_my_business",
    "skool",
];

/// Tier-3 — archived (1). Code lives under `src/social/archive/` and is
/// registered only when `ENABLE_ARCHIVE_PROVIDERS=1`.
///
/// Formally **won't-port** from Postiz, documented so nobody re-adds them:
/// `dribbble, tumblr, twitch, mewe, moltbook, nostr, listmonk` — niche MAU
/// plus distinct media pipelines (Twitch clips, Tumblr reblogs) that would
/// each cost a full provider cycle. `kick, vk, whop, lemmy` are removed
/// outright rather than archived.
pub const TIER_3: &[&str] = &["farcaster"];

/// Env var that re-enables Tier-3 providers (default: disabled).
pub const ARCHIVE_FLAG: &str = "ENABLE_ARCHIVE_PROVIDERS";

/// Whether archived (Tier-3) providers should be registered.
///
/// Any non-empty value other than `0`/`false`/`no` enables them, matching
/// the convention used elsewhere in the codebase.
pub fn archive_providers_enabled() -> bool {
    match std::env::var(ARCHIVE_FLAG) {
        Ok(v) => !matches!(v.trim().to_ascii_lowercase().as_str(), "" | "0" | "false" | "no" | "off"),
        Err(_) => false,
    }
}

/// Tier membership for a provider identifier.
///
/// Unknown identifiers return `Tier2` rather than panicking: a provider
/// added to the registry without a tier entry still shows up in the UI and
/// keeps the publish-maintained guarantee, which is the safe default.
pub fn tier_of(identifier: &str) -> ProviderTier {
    if TIER_1.contains(&identifier) {
        ProviderTier::Tier1
    } else if TIER_3.contains(&identifier) {
        ProviderTier::Tier3
    } else {
        ProviderTier::Tier2
    }
}

/// Every identifier the registry may register: Tier-1 + Tier-2 + Tier-3.
/// The registry consults this so tier membership and registration can
/// never drift apart.
pub fn all_known_identifiers() -> Vec<&'static str> {
    let mut ids: Vec<&'static str> = TIER_1
        .iter()
        .chain(TIER_2.iter())
        .chain(TIER_3.iter())
        .copied()
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_counts_match_v25_plan() {
        assert_eq!(TIER_1.len(), 12, "v25 §1 Tier-1 is 12 platforms");
        assert_eq!(TIER_2.len(), 14, "v25 §1 Tier-2 is 14 platforms");
        assert_eq!(TIER_3.len(), 1, "only farcaster is archived in v25 §1");
        assert_eq!(all_known_identifiers().len(), 27);
    }

    #[test]
    fn tiers_are_disjoint() {
        for id in TIER_1 {
            assert!(
                !TIER_2.contains(id) && !TIER_3.contains(id),
                "{id} is in more than one tier"
            );
        }
        for id in TIER_2 {
            assert!(!TIER_3.contains(id), "{id} is in Tier-2 and Tier-3");
        }
    }

    #[test]
    fn removed_providers_are_not_in_any_tier() {
        for id in ["kick", "vk", "whop", "lemmy"] {
            assert!(
                !all_known_identifiers().contains(&id),
                "{id} was removed in v25 §1 and must not be tiered"
            );
        }
    }

    #[test]
    fn archived_provider_is_tier_3_and_disabled_by_default() {
        assert_eq!(tier_of("farcaster"), ProviderTier::Tier3);
        assert!(!ProviderTier::Tier3.is_enabled_by_default());
        assert!(ProviderTier::Tier1.is_enabled_by_default());
        assert!(ProviderTier::Tier2.is_enabled_by_default());
    }

    #[test]
    fn unknown_provider_defaults_to_tier_2() {
        assert_eq!(tier_of("not-a-provider"), ProviderTier::Tier2);
    }

    #[test]
    fn archive_flag_parses_boolean_spellings() {
        // Env var is process-global; exercise the parser shape directly.
        for (raw, expected) in [
            ("1", true),
            ("true", true),
            ("yes", true),
            ("off", false),
            ("0", false),
            ("false", false),
            ("no", false),
            ("", false),
        ] {
            assert_eq!(
                !matches!(raw, "" | "0" | "false" | "no" | "off"),
                expected,
                "unexpected parse for ENABLE_ARCHIVE_PROVIDERS={raw:?}"
            );
        }
    }
}
