// ─── X GraphQL queryId discovery (F4) ──────────────────────────
// Ported from ../twitter-lyr twitter_cli/graphql.py. X rotates its GraphQL
// queryIds without notice; a rotation breaks every hardcoded-table consumer
// until a release. Resolution chain: in-memory discovered map (hydrated from
// the x_query_ids DB table) → live JS-bundle scan (when the DB copy is older
// than the TTL) → static fallback table. Publish keeps working on the
// fallback when every live source fails.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;

use crate::db::SqlitePool;

/// How stale the DB copy may be before a refresh re-scans x.com bundles.
pub const REFRESH_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// ponytail: scanning every bundle on x.com is dozens of fetches; the
/// write-path ops (CreateTweet/DeleteTweet) live in the main bundles that
/// appear first in the HTML. Cap here, raise if an op stops resolving.
const MAX_BUNDLES: usize = 12;

// ── Static fallback table (last resort when discovery fails) ─────────────

pub static FALLBACK_QUERY_IDS: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("HomeTimeline", "c-CzHF1LboFilMpsx4ZCrQ");
        m.insert("HomeLatestTimeline", "BKB7oi212Fi7kQtCBGE4zA");
        m.insert("UserByScreenName", "1VOOyvKkiI3FMmkeDNxM9A");
        m.insert("UserTweets", "q6xj5bs0hapm9309hexA_g");
        m.insert("TweetDetail", "xd_EMdYvB9hfZsZ6Idri0w");
        m.insert("Likes", "lIDpu_NWL7_VhimGGt0o6A");
        m.insert("SearchTimeline", "VhUd6vHVmLBcw0uX-6jMLA");
        m.insert("Bookmarks", "2neUNDqrrFzbLui8yallcQ");
        m.insert("CreateTweet", "IID9x6WsdMnTlXnzXGq8ng");
        m.insert("DeleteTweet", "VaenaVgh5q5ih7kvyVjgtg");
        m.insert("FavoriteTweet", "lI07N6Otwv1PhnEgXILM7A");
        m.insert("UnfavoriteTweet", "ZYKSe-w7KEslx3JhSIk5LA");
        m.insert("CreateRetweet", "ojPdsZsimiJrUGLR1sjUtA");
        m.insert("DeleteRetweet", "iQtK4dl5hBmXewYZuEOKVw");
        m.insert("CreateBookmark", "aoDbu3RHznuiSkQ9aNM67Q");
        m.insert("DeleteBookmark", "Wlmlj2-xzyS1GN3a6cj-mQ");
        m.insert("TweetResultByRestId", "7xflPyRiUxGVbJd4uWmbfg");
        m.insert("ListLatestTweetsTimeline", "RlZzktZY_9wJynoepm8ZsA");
        m
    });

// ── In-memory discovered cache ────────────────────────────────────────────
// Owned strings: refresh replaces the whole map, the old one is dropped.
// A poisoned lock is recovered (not propagated) — a panic in one writer
// must not break every X request for the rest of the process.

static DISCOVERED: LazyLock<std::sync::RwLock<HashMap<String, String>>> =
    LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

fn discovered_read() -> std::sync::RwLockReadGuard<'static, HashMap<String, String>> {
    match DISCOVERED.read() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn discovered_write() -> std::sync::RwLockWriteGuard<'static, HashMap<String, String>> {
    match DISCOVERED.write() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Resolve the queryId for an operation: discovered cache first, static
/// fallback second. Returns an owned String — call sites pass `&qid`.
pub fn lookup(op: &str) -> Option<String> {
    if let Some(id) = discovered_read().get(op) {
        return Some(id.clone());
    }
    FALLBACK_QUERY_IDS.get(op).map(|id| id.to_string())
}

/// Replace the discovered cache wholesale (previous map is dropped).
fn set_discovered(map: HashMap<String, String>) {
    *discovered_write() = map;
}

// ── Pure extraction (unit-tested with canned fixtures) ────────────────────

/// Bundle script URLs x.com advertises in its HTML.
fn script_urls(html: &str) -> Vec<String> {
    static SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?:src|href)=["'](https://abs\.twimg\.com/responsive-web/client-web[^"']+\.js)["']"#,
        )
        .expect("valid bundle regex")
    });
    SCRIPT
        .captures_iter(html)
        .map(|c| c[1].to_string())
        .collect()
}

/// queryId/operationName pairs embedded in one JS bundle. First occurrence
/// of an operation wins (mirrors twitter-lyr's `setdefault`).
fn extract_pairs(bundle: &str) -> HashMap<String, String> {
    static OP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"queryId:\s*"([A-Za-z0-9_-]+)"[^}]{0,200}operationName:\s*"([^"]+)""#)
            .expect("valid op regex")
    });
    let mut map = HashMap::new();
    for c in OP.captures_iter(bundle) {
        map.entry(c[2].to_string())
            .or_insert_with(|| c[1].to_string());
    }
    map
}

// ── Live discovery ────────────────────────────────────────────────────────

const X_HOME: &str = "https://x.com";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Fetch x.com, scan its JS bundles, and return every queryId found.
/// Best-effort: network errors skip that bundle; an empty result is
/// returned (never an error) so callers fall back to the static table.
pub async fn discover(http: &wreq::Client) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let html = match http
        .get(X_HOME)
        .header("user-agent", USER_AGENT)
        .send()
        .await
    {
        Ok(resp) => match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("x queryId discovery: x.com body read failed: {e}");
                return found;
            }
        },
        Err(e) => {
            tracing::warn!("x queryId discovery: x.com fetch failed: {e}");
            return found;
        }
    };

    for url in script_urls(&html).into_iter().take(MAX_BUNDLES) {
        let Ok(resp) = http.get(&url).send().await else {
            continue;
        };
        let Ok(bundle) = resp.text().await else {
            continue;
        };
        for (op, id) in extract_pairs(&bundle) {
            found.entry(op).or_insert(id);
        }
    }

    tracing::info!(
        "x queryId discovery: {} operations scanned from up to {} bundles",
        found.len(),
        MAX_BUNDLES
    );
    found
}

// ── DB hydration + TTL refresh ────────────────────────────────────────────

/// Load the DB-cached map into memory. Fast, no network — both the daemon
/// and the one-shot CLI call this at startup so the first X request of the
/// process benefits from the last discovery.
pub async fn hydrate_from_db(pool: &SqlitePool) {
    match crate::db::queries::list_x_query_ids(pool).await {
        Ok(rows) if !rows.is_empty() => {
            set_discovered(rows.into_iter().map(|(op, id, _)| (op, id)).collect());
        }
        Ok(_) => {}
        Err(e) => tracing::warn!("x queryId hydrate: DB read failed: {e}"),
    }
}

/// Re-scan x.com when the DB copy is older than [`REFRESH_TTL`], then
/// persist + hydrate. Best-effort: on any failure the existing cache and
/// the static table keep serving lookups (the fallback path).
pub async fn refresh_if_stale(pool: &SqlitePool, http: &wreq::Client) {
    let newest = crate::db::queries::list_x_query_ids(pool)
        .await
        .ok()
        .and_then(|rows| rows.iter().map(|r| r.2).max())
        .unwrap_or(0);
    let age = chrono::Utc::now().timestamp() - newest;
    if age > 0 && Duration::from_secs(age as u64) < REFRESH_TTL {
        return;
    }

    let found = discover(http).await;
    if found.is_empty() {
        tracing::warn!("x queryId refresh: discovery returned nothing; keeping DB cache");
        return;
    }

    let now = chrono::Utc::now().timestamp();
    for (op, id) in &found {
        if let Err(e) = crate::db::queries::upsert_x_query_id(pool, op, id, now).await {
            tracing::warn!("x queryId refresh: upsert {op} failed: {e}");
        }
    }
    set_discovered(found);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises the tests that share the PROCESS-WIDE discovery cache.
    fn test_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        &LOCK
    }
    const CANNED_HTML: &str = r#"
        <html><head>
        <script src="https://abs.twimg.com/responsive-web/client-web/main.abc123.js"></script>
        <link href="https://abs.twimg.com/not-a-bundle.css" rel="stylesheet">
        </head></html>
    "#;

    const CANNED_BUNDLE: &str = r#"
        {queryId:"NewTweetId123",operationName:"CreateTweet",operationType:"mutation"},
        {queryId:"HomeFeedId456",operationName:"HomeTimeline",operationType:"query"},
        {operationName:"NoQueryIdAbove",operationType:"query"}
    "#;

    #[test]
    fn script_urls_picks_only_twimg_client_bundles() {
        let urls = script_urls(CANNED_HTML);
        assert_eq!(urls.len(), 1);
        assert!(urls[0].ends_with("main.abc123.js"));
    }

    #[test]
    fn canned_bundle_extracts_query_ids() {
        let pairs = extract_pairs(CANNED_BUNDLE);
        assert_eq!(
            pairs.get("CreateTweet").map(String::as_str),
            Some("NewTweetId123")
        );
        assert_eq!(
            pairs.get("HomeTimeline").map(String::as_str),
            Some("HomeFeedId456")
        );
        assert!(!pairs.contains_key("NoQueryIdAbove"));
    }

    #[test]
    fn first_occurrence_wins_for_duplicate_ops() {
        let bundle = r#"
            {queryId:"FirstId",operationName:"CreateTweet"},
            {queryId:"SecondId",operationName:"CreateTweet"}
        "#;
        let pairs = extract_pairs(bundle);
        assert_eq!(
            pairs.get("CreateTweet").map(String::as_str),
            Some("FirstId")
        );
    }

    #[test]
    fn lookup_falls_back_to_static_table() {
        // The discovery cache is a PROCESS-WIDE global, and the tests
        // that read and write it run in PARALLEL - so one test could
        // read another's write and fail. The mutex serialises them.
        let _guard = test_lock().lock().unwrap_or_else(|e| e.into_inner());

        // Discovery cache is empty in a fresh process.
        assert!(discovered_read().is_empty());
        assert_eq!(
            lookup("CreateTweet").as_deref(),
            Some("IID9x6WsdMnTlXnzXGq8ng")
        );
        assert_eq!(lookup("NoSuchOperation"), None);
    }

    #[test]
    fn discovered_ids_take_precedence_over_fallback() {
        let _guard = test_lock().lock().unwrap_or_else(|e| e.into_inner());

        let mut fresh = HashMap::new();
        fresh.insert("CreateTweet".to_string(), "RotatedId999".to_string());
        set_discovered(fresh);
        assert_eq!(lookup("CreateTweet").as_deref(), Some("RotatedId999"));
        assert_eq!(
            lookup("DeleteTweet").as_deref(),
            Some("VaenaVgh5q5ih7kvyVjgtg")
        );
        set_discovered(HashMap::new());
    }
}
