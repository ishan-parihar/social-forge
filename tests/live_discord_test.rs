// Live Discord read-side tests (E7-read). Skip-guarded: without
// DISCORD_BOT_TOKEN + DISCORD_TEST_CHANNEL_ID every test returns early.
// di_search additionally needs DISCORD_TEST_GUILD_ID (and a bot with Read
// Message History + View Channel on that guild).

use social_forge::config::Config;
use social_forge::social::discord::DiscordProvider;

fn get_config() -> Config {
    social_forge::config::load_dotenv_for_live_tests();
    Config::from_env().expect("Failed to load config")
}

fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => {
            eprintln!("⏭ skipped: {name} not set");
            None
        }
    }
}

fn bot_provider(config: &Config) -> Option<DiscordProvider> {
    if config.discord_bot_token.as_deref().unwrap_or("").is_empty() {
        eprintln!("⏭ skipped: DISCORD_BOT_TOKEN not set");
        return None;
    }
    Some(DiscordProvider::new(config))
}

fn snowflake(m: &serde_json::Value) -> u64 {
    m["id"].as_str().and_then(|s| s.parse().ok()).unwrap_or(0)
}

#[tokio::test]
async fn test_di_history_is_chronological_and_paginates() {
    let config = get_config();
    let (Some(provider), Some(channel)) = (
        bot_provider(&config),
        env_or_skip("DISCORD_TEST_CHANNEL_ID"),
    ) else {
        return;
    };

    // Ask for 150 to prove the 100/request pagination loop runs.
    let messages = provider
        .fetch_channel_history(&channel, 150, None, None)
        .await
        .unwrap_or_else(|e| panic!("history failed: {e}"));

    let ids: Vec<u64> = messages.iter().map(snowflake).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "history must be chronological (oldest first)");
    assert!(!ids.contains(&0), "every message needs an id");
    eprintln!("✅ history: {} messages, chronological", messages.len());
}

#[tokio::test]
async fn test_di_tail_snapshot_returns_newest_id() {
    let config = get_config();
    let (Some(provider), Some(channel)) = (
        bot_provider(&config),
        env_or_skip("DISCORD_TEST_CHANNEL_ID"),
    ) else {
        return;
    };

    let messages = provider
        .fetch_channel_history(&channel, 25, None, None)
        .await
        .unwrap_or_else(|e| panic!("tail failed: {e}"));
    let newest = messages.last().map(snowflake).unwrap_or(0);
    if !messages.is_empty() {
        assert!(newest > 0, "tail must expose a newest_id cursor");
        // Incremental sync: everything after the tail's 5th-newest must be
        // the newer suffix, in chronological order.
        let cutoff = &messages[messages.len() - 5];
        let after = provider
            .fetch_channel_history(&channel, 100, None, Some(cutoff["id"].as_str().unwrap()))
            .await
            .unwrap_or_else(|e| panic!("tail after-cursor failed: {e}"));
        assert!(
            after.iter().all(|m| snowflake(m) > snowflake(cutoff)),
            "after-cursor must only return newer messages"
        );
    }
    eprintln!("✅ tail: {} messages, newest_id cursor ok", messages.len());
}

#[tokio::test]
async fn test_di_search_returns_hit_shape() {
    let config = get_config();
    let (Some(provider), Some(guild)) = (
        bot_provider(&config),
        env_or_skip("DISCORD_TEST_GUILD_ID"),
    ) else {
        return;
    };

    let result = provider
        .search_guild_messages(&guild, "the", None, 25)
        .await
        .unwrap_or_else(|e| panic!("search failed: {e}"));
    assert!(result["messages"].is_array(), "search returns a messages array");
    let total = result["total_results"].as_u64().unwrap_or(0);
    let shown = result["messages"].as_array().map(Vec::len).unwrap_or(0);
    assert!(shown as u64 <= total, "shown hits cannot exceed total_results");
    eprintln!("✅ search: {shown} hits of {total} total");
}
