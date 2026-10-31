// ─── Browser-driven OAuth login (`connect --browser`) ───────────
//
// Drives the platform's OWN consent page in the operator's real browser
// (browseros-neo), then completes the code exchange in-process.
//
// The server owns the callback; this driver does NOT need the server to
// be running. The flow is:
//
//   1. `IntegrationService::initiate_connect` mints the auth URL and
//      stores the state + code_verifier in `oauth_states`.
//   2. The browser is driven to the auth URL. The operator signs in and
//      consents (the browser may already hold the session — most OAuth
//      providers then skip the login form entirely).
//   3. The provider redirects to `{app_url}/api/auth/callback?code=..&state=..`.
//      The CLI reads the redirect URL out of the browser and redeems the
//      code ITSELF via `IntegrationService::complete_connect`.
//
// Step 3 is why there is no callback daemon: the redirect target is
// unreachable when the server is down, but browseros-neo PRESERVES the
// requested URL in `tabs list` even for a failed navigation, so the code
// is still recoverable.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};

use crate::api::AppState;
use crate::cli::browser::{strip_untrusted, Browser};

/// The default browseros-neo MCP endpoint.
pub const DEFAULT_BROWSER_ENDPOINT: &str = "http://127.0.0.1:9012/mcp";

/// What the browser driver achieved.
pub struct BrowserLogin {
    pub provider: String,
    pub account_name: Option<String>,
    pub internal_id: Option<String>,
    /// The redirect URL the browser ended on (evidence for the agent).
    pub final_url: String,
}

/// Run the whole browser-driven login for one provider.
///
/// `timeout_secs` bounds the wait for the operator to finish the consent
/// screen; the code exchange itself happens as soon as the redirect URL
/// carries a `code`.
pub async fn drive_oauth_login(
    state: &AppState,
    provider: &str,
    endpoint: &str,
    timeout_secs: u64,
) -> Result<BrowserLogin> {
    let user_id = crate::cli::run::resolve_user(state).await?;
    let app_url = state.config.app_url.trim_end_matches('/').to_string();

    // Already connected? Say so and stop — no browser, no prompt.
    let existing: Vec<_> = crate::db::queries::list_integrations(&state.db, user_id)
        .await?
        .into_iter()
        .filter(|i| i.provider_identifier == provider)
        .filter(|i| crate::cli::run::is_credential_row(i))
        .collect();
    if let Some(row) = existing.first() {
        return Ok(BrowserLogin {
            provider: provider.to_string(),
            account_name: row.profile_name.clone(),
            internal_id: Some(row.internal_id.clone()),
            final_url: String::new(),
        });
    }

    // Mint the provider's authorization URL. This stores the state and
    // the code verifier, so the exchange below can find them.
    let initiated = crate::services::integrations::IntegrationService::initiate_connect(
        &state.db,
        &state.providers,
        user_id,
        provider,
        &app_url,
    )
    .await
    .map_err(|e| anyhow!("Failed to build {provider} authorization URL: {e}"))?;

    let mut browser = Browser::connect(endpoint)
        .await
        .with_context(|| format!("Failed to reach browseros-neo at {endpoint}"))?;

    let page = open_auth_page(&mut browser, &initiated.auth_url).await?;

    // Progress goes to STDERR: stdout carries the JSON result, and a
    // silent multi-minute wait leaves an agent with nothing to report.
    eprintln!(
        "Opened the {provider} consent page in the operator's browser (page {page}). \
         Complete it there; waiting up to {timeout_secs}s for the redirect."
    );

    let redirect_url = match wait_for_redirect(
        &mut browser,
        &app_url,
        &initiated.state,
        timeout_secs,
    )
    .await
    {
        Ok(url) => url,
        Err(e) => {
            // On a TIMEOUT the operator may still be mid-consent, so the
            // tab stays open — closing it would throw away their work.
            // The error names the page so they can find it.
            return Err(e.context(format!(
                "The {provider} consent page is still open in the operator's browser (page {page}). \
                 Finish it there and run 'switchpost connect {provider} --wait' to redeem the result."
            )));
        }
    };

    // Redeem the code ourselves — the server does not have to be running.
    let code = extract_param(&redirect_url, "code").ok_or_else(|| {
        let error = extract_param(&redirect_url, "error").unwrap_or_default();
        anyhow!(
            "The browser finished without an authorization code{}. Final URL: {redirect_url}",
            if error.is_empty() {
                String::new()
            } else {
                format!(" (provider error: {error})")
            }
        )
    })?;
    let oauth_state = extract_param(&redirect_url, "state")
        .ok_or_else(|| anyhow!("The callback URL carried no `state` parameter: {redirect_url}"))?;

    let exchange = crate::services::integrations::IntegrationService::complete_connect(
        &state.db,
        &state.providers,
        &state.broadcast,
        &oauth_state,
        &code,
        state.token_key.as_ref(),
    )
    .await;

    // The consent tab has served its purpose either way. Closing it on
    // FAILURE matters too: a timeout used to leave the operator's browser
    // holding a consent tab for a login that is over. A failure to close
    // must not fail the login.
    let _ = browser
        .call("tabs", json!({"action": "close", "page": page}))
        .await;

    let integration = exchange.map_err(|e| anyhow!("OAuth exchange failed: {e}"))?;

    Ok(BrowserLogin {
        provider: provider.to_string(),
        account_name: integration.profile_name.clone(),
        internal_id: Some(integration.internal_id.clone()),
        final_url: redirect_url,
    })
}

/// Open the authorization URL and return the new page id.
async fn open_auth_page(browser: &mut Browser, auth_url: &str) -> Result<u32> {
    let result = browser
        .call("tabs", json!({"action": "new", "url": auth_url}))
        .await?;
    let text = Browser::text(&result);
    parse_page_id(&text).ok_or_else(|| anyhow!("browseros-neo did not report a page id: {text}"))
}

/// Wait until the browser lands on the OAuth callback with a `code`.
///
/// Two things can happen:
///   - the server IS running, so the callback page loads and shows
///     "connected" — the browser URL still carries the code;
///   - the server is NOT running, so the navigation fails, but
///     browseros-neo keeps the requested URL in `tabs list`.
///
/// Both are read the same way: list the tabs, find the one whose URL is
/// the callback, and read its query string.
pub async fn wait_for_redirect(
    browser: &mut Browser,
    app_url: &str,
    oauth_state: &str,
    timeout_secs: u64,
) -> Result<String> {
    let callback_prefix = format!("{app_url}/api/auth/callback");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    loop {
        if let Some(url) = find_callback_url(browser, &callback_prefix, oauth_state).await {
            // A callback that carries `code` or `error` is the answer.
            if extract_param(&url, "code").is_some() || extract_param(&url, "error").is_some() {
                return Ok(url);
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "No OAuth callback within {timeout_secs}s. Complete the login in the browser window and run the command again."
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    }
}

/// Find the URL of the callback tab.
///
/// `tabs list` answers with TEXT, not JSON — the live server's shape is:
///
/// ```text
/// User's tabs:
/// [1] chrome://newtab/ (BrowserOS neo)
/// [10] https://example.com/ (example.com)
///
/// Other agents' tabs:
/// [9] https://127.0.0.1:59999/api/auth/callback?code=… (127.0.0.1)
/// ```
///
/// Two things follow from that shape, and both are load-bearing:
///
///   - the tabs are TEXT, so a JSON parse finds nothing;
///   - OTHER AGENTS' tabs are listed too, so matching on the callback
///     URL alone would read somebody else's authorization code.
///
/// The match is therefore on the `state` THIS run minted — unique per
/// login, and it is the value `complete_connect` looks up. The tab this
/// driver opens is itself reported under "Other agents' tabs:", so the
/// parse does NOT stop at that header; the state is the guard.
async fn find_callback_url(
    browser: &mut Browser,
    callback_prefix: &str,
    oauth_state: &str,
) -> Option<String> {
    let result = browser.call("tabs", json!({"action": "list"})).await.ok()?;
    let text = Browser::text(&result);
    find_matching_callback(&parse_tab_urls(&text), callback_prefix, oauth_state)
}

/// Parse the tab URLs out of browseros-neo's `tabs list` text.
///
/// The server splits the listing into sections ("Your tabs:", "User's
/// tabs:", "Other agents' tabs:") and the tab THIS driver opens is
/// reported under "Other agents' tabs:" — annotated "owned by <agent>" —
/// not necessarily in the user's section. An earlier version read only
/// the user's section and worked only because the server also MIRRORED
/// the tab there; that mirror is not a contract.
///
/// Safety does not come from the section. It comes from the `state`:
/// `find_matching_callback` only accepts a callback carrying the state
/// THIS run minted, and that value is unique per login. Another agent's
/// callback carries a different state and can never match, wherever it
/// is listed. So every section is read and the state does the isolating.
fn parse_tab_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Section headers are not tab lines.
        if line.ends_with("tabs:") {
            continue;
        }
        if !line.starts_with('[') {
            continue;
        }
        // "[13] https://example.com/x?code=Q (Title) [group …]"
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((_, rest)) = rest.split_once(']') else {
            continue;
        };
        let Some(url) = rest.split_whitespace().next() else {
            continue;
        };
        if url.starts_with("http://") || url.starts_with("https://") {
            urls.push(url.to_string());
        }
    }
    urls
}

/// Read a query parameter out of a URL.
fn extract_param(url: &str, key: &str) -> Option<String> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        // A pair without `=` is not the one we want — it must not abort
        // the scan of the pairs after it.
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        if name == key {
            return Some(percent_decode(value));
        }
    }
    None
}

/// Pick the callback URL for THIS login out of the parsed tab URLs.
///
/// The match is on the minted `state`, not on the callback path alone:
/// browseros-neo lists every agent's tabs, and another agent's tab can
/// be on the same callback URL. The `state` is unique per login, so it
/// is what makes another agent's authorization code unreachable.
fn find_matching_callback(
    urls: &[String],
    callback_prefix: &str,
    oauth_state: &str,
) -> Option<String> {
    urls.iter()
        .find(|url| {
            url.starts_with(callback_prefix)
                && extract_param(url, "state").as_deref() == Some(oauth_state)
        })
        .cloned()
}

/// Decode a query value: `%xx` escapes and `+` as space.
///
/// The `%xx` half is `urlencoding::decode`, which the crate was already a
/// dependency for. Its docs are explicit that "unencoded `+` is preserved
/// literally, and _not_ changed to a space" - so the crate alone is NOT a
/// substitute for the loop this replaced, and swapping it in unchanged broke
/// the existing test with `a b/c+d`. `+`-as-space is the
/// `x-www-form-urlencoded` rule and this parses a query string, so it is
/// applied first and the crate handles the escapes.
///
/// The hand-rolled loop was not wrong - its `i + 2 < bytes.len()` bound was
/// the correct one for an escape at `i`, and the mutation run confirmed it:
/// every input below decodes identically either way. It was 31 lines doing
/// what a 3-line delegation does, which is the only reason it was cut.
///
/// A value that fails to decode comes back unchanged: this parses a callback
/// URL a browser handed us, and a malformed escape should not lose the
/// parameter. That fallback is the crate's own behaviour on the escape half,
/// so it is one rule rather than two.
fn percent_decode(value: &str) -> String {
    let space_form = value.replace('+', " ");
    urlencoding::decode(&space_form)
        .map(|c| c.into_owned())
        .unwrap_or(space_form)
}

/// Parse the page id out of browseros-neo's answer.
///
/// The live format is TWO shapes:
///
/// ```text
/// opened page 12
/// [Page 12 snapshot]
/// ```
///
/// So the id is the run of DIGITS after the marker — the snapshot line
/// carries a trailing ` snapshot]`, and reading up to the `]` would
/// parse "12 snapshot" and fail.
fn parse_page_id(text: &str) -> Option<u32> {
    for marker in ["opened page ", "[Page "] {
        let Some(found) = text.find(marker) else {
            continue;
        };
        let start = found + marker.len();
        let digits: String = text[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(id) = digits.parse::<u32>() {
            return Some(id);
        }
    }
    None
}

/// Read the current URL of a page (used for diagnostics).
pub async fn current_url(browser: &mut Browser, page: u32) -> Option<String> {
    let result = browser
        .call(
            "evaluate",
            json!({"page": page, "code": "return location.href"}),
        )
        .await
        .ok()?;
    let text = Browser::text(&result);
    let value = strip_untrusted(&text);
    serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|v| v.get("href").and_then(Value::as_str).map(String::from))
        .or_else(|| Some(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_id_is_parsed_from_the_opened_page_text() {
        assert_eq!(
            parse_page_id("opened page 12\n[Page 12 snapshot]"),
            Some(12)
        );
    }

    #[test]
    fn page_id_is_parsed_from_the_snapshot_line_alone() {
        // The "opened page N" line can be absent — the snapshot line alone
        // still carries the id.
        assert_eq!(parse_page_id("[Page 7 snapshot]"), Some(7));
    }

    #[test]
    fn missing_page_id_is_none() {
        assert_eq!(parse_page_id("no page here"), None);
    }

    #[test]
    fn query_param_is_extracted() {
        let url = "https://localhost:6544/api/auth/callback?code=abc123&state=xyz";
        assert_eq!(extract_param(url, "code").as_deref(), Some("abc123"));
        assert_eq!(extract_param(url, "state").as_deref(), Some("xyz"));
        assert_eq!(extract_param(url, "missing"), None);
    }

    #[test]
    fn percent_escapes_are_decoded() {
        assert_eq!(percent_decode("a%20b%2Fc+d"), "a b/c d");
    }

    /// This is the assertion a bare `urlencoding::decode` fails: the crate's
    /// own docs say "unencoded `+` is preserved literally, and _not_ changed to
    /// a space", and swapping the delegation in without the `replace` first
    /// had this test reading `a b/c+d`. `+`-as-space is the
    /// `x-www-form-urlencoded` rule and this parses a query string.
    #[test]
    fn a_plus_still_means_space() {
        assert_eq!(percent_decode("a+b"), "a b");
        // And the two rules compose, rather than one being applied only when
        // the other did not fire.
        assert_eq!(percent_decode("a+b%2Dc"), "a b-c");
    }

    #[test]
    fn callback_prefix_matches_only_the_callback_path() {
        let url = "https://localhost:6544/api/auth/callback?code=1";
        assert!(url.starts_with("https://localhost:6544/api/auth/callback"));
        assert!(!url.starts_with("https://localhost:6544/api/auth/connect"));
    }

    // The live server's `tabs list` answer is TEXT, and it lists OTHER
    // agents' tabs too. The tab THIS driver opens is itself reported
    // under "Other agents' tabs:" (annotated "owned by <agent>"), so a
    // parser that stopped at that header would never see its own
    // redirect. Every section is read; the minted `state` is what
    // isolates one agent's callback from another's.
    const TABS_TEXT: &str = "Your tabs:\n\
[1] chrome://newtab/ (BrowserOS neo)\n\
\n\
User's tabs:\n\
[13] https://example.com/xyz?code=Q&state=S (Example Domain) [group ABC]\n\
\n\
Other agents' tabs:\n\
[21] https://localhost:6544/api/auth/callback?code=OURS&state=OURSTATE (Authorize), owned by p [group DEF]\n\
[9] https://localhost:6544/api/auth/callback?code=THEIRS&state=THEIRS (127.0.0.1) [group GHI]\n";

    #[test]
    fn tab_urls_are_parsed_from_the_text_format() {
        let urls = parse_tab_urls(TABS_TEXT);
        assert!(urls.contains(&"https://example.com/xyz?code=Q&state=S".to_string()));
        assert!(urls.contains(
            &"https://localhost:6544/api/auth/callback?code=OURS&state=OURSTATE".to_string()
        ));
    }

    #[test]
    fn a_tab_opened_by_this_driver_is_parsed_even_though_it_is_listed_under_other_agents() {
        // The live server reports the driver's OWN tab under "Other
        // agents' tabs:" with an "owned by <agent>" annotation. If the
        // parser stopped at that header it would never find its own
        // redirect, and every browser login would time out.
        let urls = parse_tab_urls(TABS_TEXT);
        assert!(
            urls.iter().any(|u| u.contains("state=OURSTATE")),
            "the driver's own tab must be a candidate: {urls:?}"
        );
    }

    #[test]
    fn another_agents_callback_is_never_matched_because_of_the_state() {
        // Their tab IS parsed, but the state keeps it from matching.
        let urls = parse_tab_urls(TABS_TEXT);
        assert!(urls.iter().any(|u| u.contains("THEIRS")));
        assert!(find_matching_callback(
            &urls,
            "https://localhost:6544/api/auth/callback",
            "OURSTATE"
        )
        .is_some());
        assert!(find_matching_callback(
            &urls,
            "https://localhost:6544/api/auth/callback",
            "THEIRS"
        )
        .is_some());
        // Neither state can reach the other's code.
        assert!(find_matching_callback(
            &urls,
            "https://localhost:6544/api/auth/callback",
            "SOMEONE_ELSE"
        )
        .is_none());
    }

    #[test]
    fn non_http_tabs_are_skipped() {
        let urls = parse_tab_urls("User's tabs:\n[1] chrome://newtab/ (BrowserOS neo)\n");
        assert!(urls.is_empty());
    }

    #[test]
    fn another_agents_callback_tab_is_not_matched() {
        // Both tabs are on a callback URL; only one carries OUR state.
        let ours = "https://localhost:6544/api/auth/callback?code=OURS&state=OURSTATE";
        let theirs = "https://localhost:6544/api/auth/callback?code=THEIRS&state=THEIRS";
        let urls = vec![theirs.to_string(), ours.to_string()];
        let found = find_matching_callback(
            &urls,
            "https://localhost:6544/api/auth/callback",
            "OURSTATE",
        );
        assert_eq!(found.as_deref(), Some(ours));
    }

    #[test]
    fn a_query_pair_without_equals_does_not_abort_the_scan() {
        let url = "https://localhost:6544/api/auth/callback?flag&code=abc&state=xyz";
        assert_eq!(extract_param(url, "code").as_deref(), Some("abc"));
        assert_eq!(extract_param(url, "state").as_deref(), Some("xyz"));
    }

    #[test]
    fn a_callback_without_our_state_is_never_matched() {
        let urls =
            vec!["https://localhost:6544/api/auth/callback?code=THEIRS&state=THEIRS".to_string()];
        assert!(find_matching_callback(
            &urls,
            "https://localhost:6544/api/auth/callback",
            "OURSTATE"
        )
        .is_none());
    }
}
