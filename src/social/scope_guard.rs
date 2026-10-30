// ─── OAuth scope compatibility preflight ────────────────────
//
// WHY THIS EXISTS
//
// A provider's `scopes()` is handed to the authorize endpoint as one
// space-joined string. Most providers accept any union of their scopes.
// Google does NOT: some pairs are refused *up front*, before any consent
// screen exists, with a response the founder cannot act on.
//
// On a live deployment (2026-10-30) the Google provider requested YouTube +
// Gmail + Calendar + Drive in one URL and Google answered:
//
//   Error 400: invalid_request
//   This request contains scopes that cannot be requested together :
//   [https://www.googleapis.com/auth/youtube,
//    https://www.googleapis.com/auth/youtube.upload,
//    https://www.googleapis.com/auth/youtube.force-ssl,
//    https://www.googleapis.com/auth/drive.file]
//
// That is a dead end surfaced on GOOGLE'S page — the founder never reaches
// a consent screen, and the message names no fix. Nothing in the codebase
// checked the combination, so the registry happily generated the URL.
//
// This module is the one place that knows which combinations a provider
// refuses, so `generate_auth_url` can refuse the request LOCALLY with a
// message that names the offending scopes and what to do instead. The
// check lives beside the providers (not in the API layer) because the
// incompatibility rules are the provider's, not the HTTP layer's.
//
// It is deliberately small: a static conflict table plus one function.
// A rule that both the provider and a test enforce is pinned once here,
// and the test lives in `tests/connect_flow_redteam_test.rs`.

/// Pairs of Google scope families that Google's authorize endpoint refuses
/// when requested TOGETHER in a single consent.
///
/// The rule is by FAMILY rather than by exact string, so a scope added to
/// a provider later (e.g. `youtube.readonly`) is caught without editing
/// this table.
pub const GOOGLE_CONFLICTING_SCOPE_FAMILIES: &[(&str, &str)] = &[
    // YouTube Data API and Drive are the pair Google names in the error.
    ("https://www.googleapis.com/auth/youtube", "https://www.googleapis.com/auth/drive"),
    // A Calendar event scope beside a YouTube scope is refused the same way;
    // the single `calendar` scope covers the app's whole use of it.
    ("https://www.googleapis.com/auth/youtube", "https://www.googleapis.com/auth/calendar.events"),
];

/// Does `scope` belong to the family named by `family_prefix`?
///
/// The family is the scope's *root* (`.../auth/drive`), so the prefix match
/// covers `drive`, `drive.file` and `drive.readonly` with one entry.
fn scope_in_family(scope: &str, family_prefix: &str) -> bool {
    scope.starts_with(family_prefix)
}

/// Find the first pair of scopes in `scopes` that Google refuses together.
///
/// `Ok(())` means the combination is one Google grants. `Err((a, b))`
/// names the two offending scopes so the caller can say exactly which
/// request to change.
///
/// `provider` is only for the error message: the rule is Google's, but the
/// founder needs to know WHICH connect button produced it.
pub fn google_scope_conflict(
    scopes: &[String],
    provider: &str,
) -> Result<(), (String, String, String)> {
    for (family_a, family_b) in GOOGLE_CONFLICTING_SCOPE_FAMILIES {
        let a = scopes
            .iter()
            .find(|s| scope_in_family(s, family_a))
            .cloned();
        let b = scopes
            .iter()
            .find(|s| scope_in_family(s, family_b))
            .cloned();
        if let (Some(a), Some(b)) = (a, b) {
            return Err((a, b, provider.to_string()));
        }
    }
    Ok(())
}

/// The operator-facing message for a refused combination.
///
/// Names the two scopes, says why Google refused, and says what to do —
/// because "invalid_request" on Google's own page is not actionable and
/// the founder's only other clue would be the browser URL.
pub fn google_scope_conflict_message(a: &str, b: &str, provider: &str) -> String {
    format!(
        "Google refuses this combination of scopes at the authorize step, so \
         '{provider}' cannot be connected as configured: '{a}' and '{b}' \
         cannot be requested together in one consent. The registry's scope \
         set was narrowed to the combination Google grants; if this \
         persists, reconnect from the Channels screen so the corrected \
         scope set is used."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// The exact combination from the live deployment's Google 400.
    #[test]
    fn the_live_deployments_failing_combination_is_caught() {
        let scopes = v(&[
            "https://www.googleapis.com/auth/youtube",
            "https://www.googleapis.com/auth/youtube.upload",
            "https://www.googleapis.com/auth/youtube.force-ssl",
            "https://www.googleapis.com/auth/drive.file",
        ]);
        let (a, b, provider) =
            google_scope_conflict(&scopes, "google").expect_err("Google refuses this set");
        assert_eq!(a, "https://www.googleapis.com/auth/youtube");
        assert_eq!(b, "https://www.googleapis.com/auth/drive.file");
        assert_eq!(provider, "google");
    }

    /// The family rule, not an exact-string rule: a Drive scope Google does
    /// not enumerate in its error is still refused, because the conflict is
    /// between the FAMILIES.
    #[test]
    fn a_drive_scope_outside_the_error_message_is_still_refused() {
        let scopes = v(&[
            "https://www.googleapis.com/auth/youtube.upload",
            "https://www.googleapis.com/auth/drive.metadata.readonly",
        ]);
        assert!(google_scope_conflict(&scopes, "google").is_err());
    }

    /// The scope set that DOES work must pass — the fix's whole point is
    /// that the narrowed set reaches a consent screen.
    #[test]
    fn the_narrowed_google_scope_set_passes_the_preflight() {
        let narrowed = v(&[
            "https://www.googleapis.com/auth/youtube",
            "https://www.googleapis.com/auth/youtube.upload",
            "https://www.googleapis.com/auth/youtube.force-ssl",
            "https://www.googleapis.com/auth/userinfo.profile",
            "https://www.googleapis.com/auth/gmail.readonly",
            "https://www.googleapis.com/auth/gmail.send",
            "https://www.googleapis.com/auth/calendar",
        ]);
        assert_eq!(google_scope_conflict(&narrowed, "google"), Ok(()));
    }

    /// Drive's own provider requests ONLY Drive scopes, which Google grants
    /// on its own — so a separate Drive connect must not be blocked by a
    /// rule written for the combined flow.
    #[test]
    fn a_drive_only_scope_set_is_allowed() {
        let drive_only = v(&[
            "https://www.googleapis.com/auth/drive.readonly",
            "https://www.googleapis.com/auth/drive.file",
            "https://www.googleapis.com/auth/drive.metadata.readonly",
        ]);
        assert_eq!(google_scope_conflict(&drive_only, "drive"), Ok(()));
    }

    /// A non-Google provider's scopes are never checked against Google's
    /// table — the rule is provider-specific and must not fire for X.
    #[test]
    fn non_google_scopes_are_never_checked_against_the_google_table() {
        let x_scopes = v(&["tweet.read", "tweet.write", "users.read", "follows.read"]);
        assert_eq!(google_scope_conflict(&x_scopes, "x"), Ok(()));
    }

    /// The message names both scopes and the action, because a founder
    /// reading it must know what to change without opening the source.
    #[test]
    fn the_refusal_message_names_both_scopes_and_the_action() {
        let msg = google_scope_conflict_message(
            "https://www.googleapis.com/auth/youtube",
            "https://www.googleapis.com/auth/drive.file",
            "google",
        );
        assert!(msg.contains("youtube"), "{msg}");
        assert!(msg.contains("drive.file"), "{msg}");
        assert!(msg.contains("cannot be requested together"), "{msg}");
        assert!(msg.contains("Google"), "{msg}");
    }
}
