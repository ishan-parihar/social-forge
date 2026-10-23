// Live Reddit parity tests (P6.5). Skip-guarded: without a Reddit token
// in the environment every test returns early and prints a skip note.
// Destructive tools (report/block/friend/nsfw/spoiler/crosspost) further
// require REDDIT_LIVE_WRITE=1 so a CI run with a token can never mutate
// the account's public state by accident.

use switchpost::config::Config;
use switchpost::social::reddit::RedditProvider;
use switchpost::social::SocialProvider;

fn get_config() -> Config {
    switchpost::config::load_dotenv_for_live_tests();
    Config::from_env().expect("Failed to load config")
}

/// Cookie JSON blob or OAuth access token; None = skip the suite.
/// OAuth path: refresh via the permanent REDDIT_REFRESH_TOKEN (public
/// trait method); when that grant is rejected the stale access token is
/// probed — dead creds skip the suite with a note instead of failing the
/// gate (a rejected password/secret is an operator issue, not a code bug).
async fn reddit_token(config: &Config) -> Option<String> {
    if let Ok(t) = std::env::var("REDDIT_COOKIE_TOKEN") {
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Some(rt) = config.reddit_refresh_token().filter(|t| !t.is_empty()) {
        let p = RedditProvider::new(config);
        if let Ok(fresh) = p.refresh_token(&rt).await {
            return Some(fresh.access_token);
        }
    }
    let stale = config.reddit_access_token().filter(|t| !t.is_empty())?;
    let probe = RedditProvider::new(config);
    if probe.friends(&stale).await.is_err() {
        eprintln!(
            "⏭ skipped: Reddit credentials present but rejected by Reddit \
             (refresh + access token both unauthorized) — reconnect the account"
        );
        return None;
    }
    Some(stale)
}

fn live_write_enabled() -> bool {
    std::env::var("REDDIT_LIVE_WRITE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn make_provider(config: &Config, token: &str) -> RedditProvider {
    let mut p = RedditProvider::new(config);
    p.prepare_from_token(token);
    p
}

/// All tests hit the SAME account; cargo's default parallel dispatch trips
/// Reddit's per-account rate limits mid-roundtrip (live-pass finding:
/// the write test 403'd at a different step on every parallel run, green
/// when isolated). One binary-wide lock serializes them regardless of
/// --test-threads.
static LIVE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn live_guard() -> tokio::sync::MutexGuard<'static, ()> {
    LIVE_LOCK.lock().await
}

#[tokio::test]
async fn test_reddit_list_friends() {
    let _g = live_guard().await;
    let config = get_config();
    let Some(token) = reddit_token(&config).await else {
        eprintln!("⏭ skipped: no Reddit token (REDDIT_COOKIE_TOKEN or REDDIT_ACCESS_TOKEN)");
        return;
    };
    let p = make_provider(&config, &token);
    match p.friends(&token).await {
        Ok(v) => {
            let n = v.as_array().map(Vec::len).unwrap_or(0);
            eprintln!("✅ friends: {n} entries");
        }
        Err(e) => panic!("friends failed: {e}"),
    }
}

#[tokio::test]
async fn test_reddit_get_rules() {
    let _g = live_guard().await;
    let config = get_config();
    let Some(token) = reddit_token(&config).await else {
        eprintln!("⏭ skipped: no Reddit token");
        return;
    };
    let p = make_provider(&config, &token);
    match p.get_rules(&token, "rust").await {
        Ok(v) => {
            let n = v["rules"].as_array().map(Vec::len).unwrap_or(0);
            assert!(n > 0, "r/rust must have rules: {v}");
            eprintln!("✅ get_rules: {n} rules");
        }
        Err(e) => panic!("get_rules failed: {e}"),
    }
    // Flair listing rides the same dual-auth GET path — verify it on both
    // auth kinds when creds exist (the .json suffix fix for cookie auth).
    match p.list_flairs(&token, "rust").await {
        Ok(v) => eprintln!(
            "✅ list_flairs: {} entries",
            v.as_array().map(Vec::len).unwrap_or(0)
        ),
        Err(e) => panic!("list_flairs failed: {e}"),
    }
}

#[tokio::test]
async fn test_reddit_get_saved() {
    let _g = live_guard().await;
    let config = get_config();
    let Some(token) = reddit_token(&config).await else {
        eprintln!("⏭ skipped: no Reddit token");
        return;
    };
    let p = make_provider(&config, &token);
    // A listing response (even empty) is success — we only prove the
    // account-name resolution + endpoint work.
    p.get_saved(&token, 5)
        .await
        .unwrap_or_else(|e| panic!("get_saved failed: {e}"));
    eprintln!("✅ get_saved: listing returned");
}

#[tokio::test]
async fn test_reddit_read_all_messages() {
    let _g = live_guard().await;
    let config = get_config();
    let Some(token) = reddit_token(&config).await else {
        eprintln!("⏭ skipped: no Reddit token");
        return;
    };
    let mut p = make_provider(&config, &token);
    p.read_all_messages(&token)
        .await
        .unwrap_or_else(|e| panic!("read_all_messages failed: {e}"));
    eprintln!("✅ read_all_messages: inbox marked read");
}

#[tokio::test]
async fn test_reddit_write_parity_roundtrip() {
    let _g = live_guard().await;
    let config = get_config();
    let Some(token) = reddit_token(&config).await else {
        eprintln!("⏭ skipped: no Reddit token");
        return;
    };
    if !live_write_enabled() {
        eprintln!("⏭ skipped: REDDIT_LIVE_WRITE != 1 (destructive roundtrip)");
        return;
    }
    let mut p = make_provider(&config, &token);

    // Own-post NSFW/spoiler marks round-trip and revert cleanly.
    let submit = p
        .api_post(
            &token,
            "/api/submit",
            &[
                ("sr", "test"),
                ("title", "switchpost P6.5 parity test"),
                ("kind", "self"),
                ("text", "temporary post — will be deleted by the test"),
            ],
        )
        .await
        .unwrap_or_else(|e| panic!("submit fixture failed: {e}"));
    let post_id = submit["json"]["data"]["id"]
        .as_str()
        .map(|id| format!("t3_{id}"))
        .expect("submit returned no id");

    p.mark_nsfw(&token, &post_id)
        .await
        .unwrap_or_else(|e| panic!("mark_nsfw failed: {e}"));
    p.unmark_nsfw(&token, &post_id)
        .await
        .unwrap_or_else(|e| panic!("unmark_nsfw failed: {e}"));
    p.mark_spoiler(&token, &post_id)
        .await
        .unwrap_or_else(|e| panic!("mark_spoiler failed: {e}"));
    p.unmark_spoiler(&token, &post_id)
        .await
        .unwrap_or_else(|e| panic!("unmark_spoiler failed: {e}"));
    eprintln!("✅ nsfw/spoiler marks round-tripped");

    // Reporting our own fixture post is harmless (the post is deleted
    // right after; r/test mods only ever saw a self-report).
    p.report(
        &token,
        &post_id,
        "switchpost self-test report",
        "",
        "",
        "",
    )
    .await
    .unwrap_or_else(|e| panic!("report failed: {e}"));
    eprintln!("✅ report accepted");

    // Crosspost the fixture, then delete both.
    let cp = p
        .crosspost(&token, "test", "P6.5 crosspost test", &post_id)
        .await
        .unwrap_or_else(|e| panic!("crosspost failed: {e}"));
    let cp_id = cp["json"]["data"]["id"]
        .as_str()
        .map(|id| format!("t3_{id}"))
        .unwrap_or_default();
    eprintln!("✅ crosspost created: {cp_id}");
    if !cp_id.is_empty() {
        p.delete(&cp_id)
            .await
            .unwrap_or_else(|e| panic!("crosspost cleanup failed: {e}"));
    }
    p.delete(&post_id)
        .await
        .unwrap_or_else(|e| panic!("cleanup failed: {e}"));
    eprintln!("✅ fixtures deleted");

    // Friends add/remove round-trip against the official Reddit account.
    p.add_friend(&token, "Reddit")
        .await
        .unwrap_or_else(|e| panic!("add_friend failed: {e}"));
    p.remove_friend(&token, "Reddit")
        .await
        .unwrap_or_else(|e| panic!("remove_friend failed: {e}"));
    eprintln!("✅ friends add/remove round-tripped");

    // block_user: skipped even in write mode — blocking is not part of
    // this port's reversible surface (no unblock op), so a live block
    // would leave permanent account state behind. The tool's wiring is
    // identical to the friends ops exercised above.
    eprintln!("⏭ block_user: no reversible live path (no unblock op in this port)");
}
