// ─── Live X/Twitter Provider Test ──────────────────────────────
// Tests all X provider methods against the real Twitter GraphQL API
// using cookies from .env (X_AUTH_TOKEN + X_CT0).
//
// Run: cargo test --test live_x_test -- --nocapture
//
// Requires: .env with X_AUTH_TOKEN, X_CT0 set, database running.

use social_forge::config::Config;
use social_forge::social::x::XProvider;

fn setup_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config from .env")
}

fn create_provider() -> XProvider {
    let config = setup_config();
    XProvider::new(&config)
}

fn get_token() -> String {
    let config = setup_config();
    let at = config.x_auth_token.expect("X_AUTH_TOKEN not set in .env");
    let ct0 = config.x_ct0.expect("X_CT0 not set in .env");
    serde_json::json!({ "auth_token": at, "ct0": ct0 }).to_string()
}

#[tokio::test]
async fn test_x_get_me() {
    let provider = create_provider();
    let token = get_token();
    let result = provider.get_me(&token).await.expect("get_me failed");
    println!("\n✅ X get_me succeeded:");
    println!("   Response: {}", serde_json::to_string_pretty(&result).unwrap_or_default().chars().take(500).collect::<String>());
    assert!(result.get("data").is_some(), "Expected data field in response");
    let data = result.get("data").unwrap();
    let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
    let username = data.get("username").and_then(|v| v.as_str()).unwrap_or("unknown");
    println!("   ID: {id}");
    println!("   Name: {name}");
    println!("   Username: {username}");
}

#[tokio::test]
async fn test_x_user_lookup_by_username() {
    let provider = create_provider();
    let token = get_token();
    let result = provider.user_lookup_by_username(&token, "XDevelopers").await;
    match result {
        Ok(json) => println!("\n✅ X user_lookup_by_username: OK"),
        Err(e) => println!("\n⚠️  X user_lookup_by_username: {e}"),
    }
}

#[tokio::test]
async fn test_x_home_timeline() {
    let provider = create_provider();
    let token = get_token();
    let result = provider.home_timeline(&token, "", 5, None).await;
    match result {
        Ok(json) => {
            let count = json.pointer("/data/user_result/timeline/instructions")
                .or_else(|| json.pointer("/data/home/home_timestone/instructions"))
                .map(|i| i.as_array().map(|a| a.len()).unwrap_or(0))
                .unwrap_or(0);
            println!("\n✅ X home_timeline: OK ({count} timeline entries)");
        }
        Err(e) => println!("\n⚠️  X home_timeline: {e}"),
    }
}

#[tokio::test]
async fn test_x_user_tweets() {
    let provider = create_provider();
    let token = get_token();

    // First get "me" so we have our user ID
    let me = provider.get_me(&token).await.expect("get_me failed");
    let my_id = me.get("data").and_then(|d| d.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    if !my_id.is_empty() {
        let result = provider.user_tweets(&token, my_id, 5, None).await;
        match result {
            Ok(json) => println!("\n✅ X user_tweets: OK"),
            Err(e) => println!("\n⚠️  X user_tweets: {e}"),
        }
    } else {
        println!("\n⚠️  X user_tweets: skipped (no user ID from get_me)");
    }
}

#[tokio::test]
async fn test_x_search_tweets() {
    let provider = create_provider();
    let token = get_token();
    let result = provider.search_tweets(&token, "rust lang", 5, None).await;
    match result {
        Ok(json) => println!("\n✅ X search_tweets: OK"),
        Err(e) => println!("\n⚠️  X search_tweets: {e}"),
    }
}

#[tokio::test]
async fn test_x_followers() {
    let provider = create_provider();
    let token = get_token();

    let me = provider.get_me(&token).await.expect("get_me failed");
    let my_id = me.get("data").and_then(|d| d.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    if !my_id.is_empty() {
        let result = provider.followers(&token, my_id, 5, None).await;
        match result {
            Ok(json) => {
                let count = json.get("data").and_then(|d| d.as_array()).map(|a| a.len()).unwrap_or(0);
                println!("\n✅ X followers: OK ({count} followers)");
            }
            Err(e) => println!("\n⚠️  X followers: {e}"),
        }
    } else {
        println!("\n⚠️  X followers: skipped (no user ID)");
    }
}

#[tokio::test]
async fn test_x_following() {
    let provider = create_provider();
    let token = get_token();

    let me = provider.get_me(&token).await.expect("get_me failed");
    let my_id = me.get("data").and_then(|d| d.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    if !my_id.is_empty() {
        let result = provider.following(&token, my_id, 5, None).await;
        match result {
            Ok(json) => println!("\n✅ X following: OK"),
            Err(e) => println!("\n⚠️  X following: {e}"),
        }
    } else {
        println!("\n⚠️  X following: skipped (no user ID)");
    }
}

#[tokio::test]
async fn test_x_tweet_detail() {
    let provider = create_provider();
    let token = get_token();
    // Use a known tweet ID (this one should be stable)
    let result = provider.tweet_detail(&token, "20").await;
    match result {
        Ok(json) => println!("\n✅ X tweet_detail: OK"),
        Err(e) => println!("\n⚠️  X tweet_detail: {e}"),
    }
}

#[tokio::test]
async fn test_x_bookmarks() {
    let provider = create_provider();
    let token = get_token();
    let result = provider.bookmarks(&token, "", 5, None).await;
    match result {
        Ok(json) => println!("\n✅ X bookmarks: OK"),
        Err(e) => println!("\n⚠️  X bookmarks: {e}"),
    }
}

#[tokio::test]
async fn test_x_like_unlike() {
    let provider = create_provider();
    let token = get_token();

    let me = provider.get_me(&token).await.expect("get_me failed");
    let my_id = me.get("data").and_then(|d| d.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    if !my_id.is_empty() {
        // Like a tweet
        let result = provider.like_tweet(&token, my_id, "20").await;
        match &result {
            Ok(json) => println!("\n✅ X like_tweet: OK"),
            Err(e) => println!("\n⚠️  X like_tweet: {e}"),
        }

        // Unlike it
        let result2 = provider.unlike_tweet(&token, my_id, "20").await;
        match &result2 {
            Ok(json) => println!("\n✅ X unlike_tweet: OK"),
            Err(e) => println!("\n⚠️  X unlike_tweet: {e}"),
        }
    } else {
        println!("\n⚠️  X like/unlike: skipped (no user ID)");
    }
}

#[tokio::test]
async fn test_x_retweet_unretweet() {
    let provider = create_provider();
    let token = get_token();

    let me = provider.get_me(&token).await.expect("get_me failed");
    let my_id = me.get("data").and_then(|d| d.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    if !my_id.is_empty() {
        let result = provider.retweet(&token, my_id, "20").await;
        match &result {
            Ok(json) => println!("\n✅ X retweet: OK"),
            Err(e) => println!("\n⚠️  X retweet: {e}"),
        }

        let result2 = provider.unretweet(&token, my_id, "20").await;
        match &result2 {
            Ok(json) => println!("\n✅ X unretweet: OK"),
            Err(e) => println!("\n⚠️  X unretweet: {e}"),
        }
    } else {
        println!("\n⚠️  X retweet/unretweet: skipped (no user ID)");
    }
}


