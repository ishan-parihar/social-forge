// ─── Archived Providers (v25 Tier-3) ──────────────────────────
// Providers in here compile but are NOT registered by default. The
// registry only inserts them when `ENABLE_ARCHIVE_PROVIDERS` is set —
// see `super::tier::archive_providers_enabled()` and plan §1.
//
// Rationale: each provider is OAuth drift plus its own test matrix.
// Archived ones keep working for anyone who still needs them without
// costing the default surface, `/api/providers`, or the MCP tool list.
//
// Nothing here should be imported outside `registry.rs`; if a new
// feature needs an archived provider, promote it to Tier-2 in
// `super::tier` and move the file back to `src/social/` first.

pub mod farcaster;
