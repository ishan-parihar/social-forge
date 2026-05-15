use postiz_rust::config::Config;
use postiz_rust::social::x::XProvider;

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config")
}

fn log(label: &str, result: &Result<serde_json::Value, impl std::fmt::Display>) {
    match result {
        Ok(v) => {
            let preview = serde_json::to_string(v).unwrap_or_default().chars().take(80).collect::<String>();
            eprintln!("  ✅ {label}: {preview}");
        }
        Err(e) => eprintln!("  ⚠️  {label}: {e}"),
    }
}

#[tokio::test]
async fn test_x_tools_live() {
    let config = get_config();
    let provider = XProvider::new(&config);

    let token = if let (Some(at), Some(ct0)) = (&config.x_auth_token, &config.x_ct0) {
        if !at.is_empty() && !ct0.is_empty() {
            serde_json::json!({"auth_token": at, "ct0": ct0}).to_string()
        } else {
            eprintln!("X_AUTH_TOKEN or X_CT0 env var is empty");
            return;
        }
    } else {
        eprintln!("Set X_AUTH_TOKEN and X_CT0 env vars for live test");
        return;
    };

    let is_cookie = XProvider::is_cookie_auth(&token);
    eprintln!("cookie_auth={is_cookie} token_len={}", token.len());

    eprint!("1. UserByScreenName ... ");
    match provider.user_lookup_by_username(&token, "elonmusk").await {
        Ok(d) => {
            let name = d.pointer("/data/user/result/legacy/name").and_then(|v| v.as_str()).unwrap_or("?");
            eprintln!("✅ {name}");
        }
        Err(e) => eprintln!("❌ {e}"),
    }

    eprint!("2. get_me ... ");
    let my_id = match provider.get_me(&token).await {
        Ok(d) => {
            let id = d.pointer("/data/id").and_then(|v| v.as_str()).unwrap_or("44196397");
            eprintln!("✅ id={id}");
            id.to_string()
        }
        Err(e) => {
            eprintln!("❌ {e}");
            "44196397".to_string()
        }
    };

    log("home_timeline", &provider.home_timeline(&token, &my_id, 5, None).await);
    log("user_tweets", &provider.user_tweets(&token, &my_id, 5, None).await);
    log("tweet_detail", &provider.tweet_detail(&token, "20").await);
    log("search_tweets", &provider.search_tweets(&token, "rust programming", 5, None).await);
    log("user_lookup", &provider.user_lookup(&token, "44196397").await);
    log("followers", &provider.followers(&token, "44196397", 5, None).await);
    log("following", &provider.following(&token, "44196397", 5, None).await);
    log("delete_tweet", &provider.delete_tweet(&token, "999999999999999999").await);
    log("like_tweet", &provider.like_tweet(&token, &my_id, "20").await);
    log("unlike_tweet", &provider.unlike_tweet(&token, &my_id, "20").await);
    log("bookmarks", &provider.bookmarks(&token, &my_id, 5, None).await);
    log("list_timeline", &provider.list_timeline(&token, "1449448327965741056", 5, None).await);
    log("retweet", &provider.retweet(&token, &my_id, "20").await);
    log("unretweet", &provider.unretweet(&token, &my_id, "20").await);
    log("follow_user", &provider.follow_user(&token, &my_id, "44196397").await);
    log("unfollow_user", &provider.unfollow_user(&token, &my_id, "44196397").await);

    eprintln!("\n=== ALL 18 X TOOLS TESTED ===");
}
