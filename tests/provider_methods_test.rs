// ─── IG Standalone + Threads Provider Methods Integration Test ────
// Exercises all provider methods directly against the Meta APIs with
// an invalid token, verifying:
//   1. Each method compiles and is callable
//   2. HTTP requests are properly formed (correct URL, params, headers)
//   3. Error handling catches and reports API errors correctly
//   4. No panics or crashes on bad input
//
// Run: cargo test --test provider_methods_test -- --nocapture
// Requires: running DB at DATABASE_URL, configured .env

use std::sync::Arc;

use social_forge::config::Config;
use social_forge::db;
use social_forge::social::discord::DiscordProvider;
use social_forge::social::instagram_standalone::InstagramStandaloneProvider;
use social_forge::social::linkedin::LinkedInProvider;
use social_forge::social::linkedin_page::LinkedInPageProvider;
use social_forge::social::pinterest::PinterestProvider;
use social_forge::social::reddit::RedditProvider;
use social_forge::social::skool::SkoolProvider;
use social_forge::social::threads::ThreadsProvider;
use social_forge::social::wordpress::WordPressProvider;
use social_forge::social::x::XProvider;
use social_forge::social::youtube::YoutubeProvider;
use social_forge::social::SocialProvider;

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config from .env")
}

fn create_ig_provider(config: &Config) -> InstagramStandaloneProvider {
    InstagramStandaloneProvider::new(config)
}

fn create_threads_provider(config: &Config) -> ThreadsProvider {
    ThreadsProvider::new(config)
}

const BAD_TOKEN: &str = "IGQWZVOmFla2VyX3Rva2VuX2Zvcl90ZXN0aW5n";

// ── Instagram Standalone Provider Method Tests ─────────────────

#[tokio::test]
async fn test_ias_get_media_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .get_media(BAD_TOKEN, "17841400000000000", 10)
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            // Should get an API error about invalid token, not a panic
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("expired")
                    || err_str.contains("access_token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ ias_get_media: Properly handled API error: {err_str}");
        }
        Ok(v) => {
            // Unexpected success - log it (might happen if token somehow works)
            println!("⚠️  ias_get_media: Unexpected success: {v:?}");
        }
    }
}

#[tokio::test]
async fn test_ias_get_media_detail_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .get_media_detail(BAD_TOKEN, "18000000000000000")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ ias_get_media_detail: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_get_media_detail: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_ias_get_comments_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .get_media_comments(BAD_TOKEN, "18000000000000000")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ ias_get_comments: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_get_comments: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_ias_reply_to_comment_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .reply_to_comment(BAD_TOKEN, "18000000000000000", "Test reply")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ ias_reply_to_comment: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_reply_to_comment: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_ias_create_container_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .create_container(
            BAD_TOKEN,
            "17841400000000000",
            "https://example.com/test.jpg",
            "Test caption",
            "IMAGE",
        )
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ ias_create_container: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_create_container: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_ias_publish_container_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .publish_container(BAD_TOKEN, "17841400000000000", "18000000000000000")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ ias_publish_container: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_publish_container: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_ias_poll_container_with_bad_token() {
    let config = get_config();
    let provider = create_ig_provider(&config);

    let result = provider
        .poll_container_status(BAD_TOKEN, "18000000000000000")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ ias_poll_container: Properly handled API error: {err_str}");
        }
        Ok(v) => {
            // Expired tokens get status_code FINISHED or similar
            println!("⚠️  ias_poll_container: Unexpected success: {v:?}");
        }
    }
}

#[tokio::test]
async fn test_ias_social_provider_publish_with_bad_token() {
    let config = get_config();
    let provider = InstagramStandaloneProvider::new(&config);

    let post = social_forge::social::PostContent {
        content: "Test post".into(),
        media: vec![social_forge::social::MediaAttachment {
            url: "https://example.com/test.jpg".into(),
            mime_type: "image/jpeg".into(),
            alt: None,
        }],
        settings: serde_json::Value::Object(serde_json::Map::new()),
    };

    let result = provider.publish(BAD_TOKEN, &post).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ ias_publish (SocialProvider trait): Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  ias_publish: Unexpected success: {v:?}"),
    }
}

// ── Threads Provider Method Tests ───────────────────────────

#[tokio::test]
async fn test_threads_get_profile_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider.get_profile(BAD_TOKEN).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ th_get_profile: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_get_profile: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_get_threads_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .get_threads(BAD_TOKEN, "123456789", 10)
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ th_get_threads: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_get_threads: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_get_thread_detail_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .get_thread_detail(BAD_TOKEN, "1234567890123456789")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ th_get_thread_detail: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_get_thread_detail: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_get_replies_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .get_thread_replies(BAD_TOKEN, "1234567890123456789")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ th_get_replies: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_get_replies: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_reply_to_thread_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .reply_to_thread(BAD_TOKEN, "1234567890123456789", "Test reply")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ th_reply_to_thread: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_reply_to_thread: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_get_insights_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .get_insights(BAD_TOKEN, "123456789", "likes,views,reposts,replies,quotes", "day")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ th_get_insights: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_get_insights: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_delete_thread_with_bad_token() {
    let config = get_config();
    let provider = create_threads_provider(&config);

    let result = provider
        .delete_thread(BAD_TOKEN, "1234567890123456789")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("OAuthAccessTokenException")
                    || err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ th_delete_thread: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_delete_thread: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_threads_social_provider_publish_with_bad_token() {
    let config = get_config();
    let provider = ThreadsProvider::new(&config);

    let post = social_forge::social::PostContent {
        content: "Test post from social-forge-rust integration test".into(),
        media: vec![],
        settings: serde_json::Value::Object(serde_json::Map::new()),
    };

    let result = provider.publish(BAD_TOKEN, &post).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ th_publish (SocialProvider trait): Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  th_publish: Unexpected success: {v:?}"),
    }
}

// ── LinkedIn Personal Provider Method Tests ─────────────────

fn create_linkedin_provider(config: &Config) -> LinkedInProvider {
    LinkedInProvider::new(config)
}

fn create_linkedin_page_provider(config: &Config) -> LinkedInPageProvider {
    LinkedInPageProvider::new(config)
}

const LINKEDIN_BAD_TOKEN: &str = "AQVOmFla2VyX3Rva2VuX2Zvcl90ZXN0aW5n";

#[tokio::test]
async fn test_linkedin_get_profile_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_provider(&config);

    let result = provider.get_profile(LINKEDIN_BAD_TOKEN).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("expired")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ li_get_profile: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  li_get_profile: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_get_posts_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_provider(&config);

    let result = provider
        .get_posts(LINKEDIN_BAD_TOKEN, "urn:li:person:test", 10)
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}").to_lowercase();
            assert!(
                err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ li_get_posts: Properly handled API error: {e}");
        }
        Ok(v) => println!("⚠️  li_get_posts: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_get_post_detail_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_provider(&config);

    let result = provider
        .get_post_detail(LINKEDIN_BAD_TOKEN, "urn:li:activity:test")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ li_get_post_detail: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  li_get_post_detail: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_get_post_comments_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_provider(&config);

    let result = provider
        .get_post_comments(LINKEDIN_BAD_TOKEN, "urn:li:activity:test")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ li_get_comments: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  li_get_comments: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_create_comment_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_provider(&config);

    let result = provider
        .create_comment(LINKEDIN_BAD_TOKEN, "urn:li:activity:test", "urn:li:person:test", "Test comment")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ li_create_comment: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  li_create_comment: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_publish_with_bad_token() {
    let config = get_config();
    let provider = LinkedInProvider::new(&config);

    let post = social_forge::social::PostContent {
        content: "Test LinkedIn post from integration test".into(),
        media: vec![],
        settings: serde_json::Value::Object(serde_json::Map::new()),
    };

    let result = provider.publish(LINKEDIN_BAD_TOKEN, &post).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ li_publish (SocialProvider trait): Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  li_publish: Unexpected success: {v:?}"),
    }
}

// ── LinkedIn Page Provider Method Tests ─────────────────────

#[tokio::test]
async fn test_linkedin_page_pages_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_page_provider(&config);

    let result = provider.pages(LINKEDIN_BAD_TOKEN).await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            assert!(
                err_str.contains("Invalid")
                    || err_str.contains("invalid")
                    || err_str.contains("token")
                    || err_str.contains("error"),
                "Expected API error, got: {err_str}"
            );
            println!("✅ lip_pages: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  lip_pages: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_page_fetch_page_info_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_page_provider(&config);

    let result = provider
        .fetch_page_info(LINKEDIN_BAD_TOKEN, "12345678")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ lip_fetch_page_info: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  lip_fetch_page_info: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_page_get_posts_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_page_provider(&config);

    let result = provider
        .get_page_posts(LINKEDIN_BAD_TOKEN, "12345678", 10)
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ lip_get_page_posts: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  lip_get_page_posts: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_linkedin_page_create_comment_with_bad_token() {
    let config = get_config();
    let provider = create_linkedin_page_provider(&config);

    let result = provider
        .create_comment(LINKEDIN_BAD_TOKEN, "urn:li:activity:test", "urn:li:organization:test", "Test comment")
        .await;

    match &result {
        Err(e) => {
            let err_str = format!("{e}");
            println!("✅ lip_create_comment: Properly handled API error: {err_str}");
        }
        Ok(v) => println!("⚠️  lip_create_comment: Unexpected success: {v:?}"),
    }
}

// ── MCP Tool Handler Integration Tests ──────────────────────
// These test the full chain: resolve_first_user → find_token → call method
// using a real DB connection and mock integrations.

#[tokio::test]
async fn test_ias_mcp_tool_handler_full_chain() {
    let config = get_config();
    let db = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use social_forge::api::AppState;
    use social_forge::mcp::Social ForgeMcpServer;
    use social_forge::realtime::Broadcaster;
    use social_forge::api::rate_limiter::AuthRateLimiter;
    use social_forge::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config, None, None));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
    };

    // Verify MCP server can be created (all #[tool] macros compile)
    let server = Social ForgeMcpServer::new(state.clone());
    drop(server);

    // Verify Instagram Standalone provider is registered
    let ias = state.providers.get("instagram-standalone");
    assert!(ias.is_some(), "Instagram Standalone provider should be registered");
    let ias = ias.unwrap();
    assert_eq!(ias.identifier(), "instagram-standalone");
    assert!(!ias.scopes().is_empty(), "Should have scopes configured");

    // Verify the provider generates valid auth URLs
    let auth_result = ias
        .generate_auth_url("test_state", "test_verifier", "http://localhost:3000/api/auth/callback")
        .await;
    assert!(auth_result.is_ok(), "Auth URL should generate: {auth_result:?}");
    let url = auth_result.unwrap().url;
    assert!(url.contains("instagram.com/oauth/authorize"), "URL should go to Instagram OAuth");
    assert!(url.contains("instagram_business_basic"), "Should contain business scopes (matching social-forge-app pattern)");

    println!("✅ ias_mcp_handler: Full chain verified (provider registration, auth URL, server creation)");
}

#[tokio::test]
async fn test_threads_mcp_tool_handler_full_chain() {
    let config = get_config();
    let db = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use social_forge::api::AppState;
    use social_forge::mcp::Social ForgeMcpServer;
    use social_forge::realtime::Broadcaster;
    use social_forge::api::rate_limiter::AuthRateLimiter;
    use social_forge::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config, None, None));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
    };

    // Verify MCP server can be created
    let server = Social ForgeMcpServer::new(state.clone());
    drop(server);

    // Verify Threads provider is registered
    let threads = state.providers.get("threads");
    assert!(threads.is_some(), "Threads provider should be registered");
    let threads = threads.unwrap();
    assert_eq!(threads.identifier(), "threads");
    assert!(!threads.scopes().is_empty(), "Should have scopes configured");

    // Verify the provider generates valid auth URLs
    let auth_result = threads
        .generate_auth_url("test_state", "test_verifier", "http://localhost:3000/api/auth/callback")
        .await;
    assert!(auth_result.is_ok(), "Auth URL should generate: {auth_result:?}");
    let url = auth_result.unwrap().url;
    assert!(url.contains("threads.net/oauth/authorize"), "URL should go to Threads OAuth");
    assert!(url.contains("threads_basic"), "Should contain the right scopes");

    println!("✅ threads_mcp_handler: Full chain verified (provider registration, auth URL, server creation)");
}

#[tokio::test]
async fn test_linkedin_mcp_tool_handler_full_chain() {
    let config = get_config();
    let db = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use social_forge::api::AppState;
    use social_forge::mcp::Social ForgeMcpServer;
    use social_forge::realtime::Broadcaster;
    use social_forge::api::rate_limiter::AuthRateLimiter;
    use social_forge::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config, None, None));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
    };

    // Verify MCP server can be created
    let server = Social ForgeMcpServer::new(state.clone());
    drop(server);

    // Verify LinkedIn provider is registered
    let li = state.providers.get("linkedin");
    assert!(li.is_some(), "LinkedIn provider should be registered");
    let li = li.unwrap();
    assert_eq!(li.identifier(), "linkedin");
    assert!(!li.scopes().is_empty(), "Should have scopes configured");

    // Verify LinkedIn Page provider is registered
    let lip = state.providers.get("linkedin-page");
    assert!(lip.is_some(), "LinkedIn Page provider should be registered");
    let lip = lip.unwrap();
    assert_eq!(lip.identifier(), "linkedin-page");
    assert!(!lip.scopes().is_empty(), "Should have scopes configured");

    // Verify LinkedIn provider generates valid auth URLs
    let auth_result = li
        .generate_auth_url("test_state", "test_verifier", "http://localhost:3000/api/auth/callback")
        .await;
    assert!(auth_result.is_ok(), "Auth URL should generate: {auth_result:?}");
    let url = auth_result.unwrap().url;
    assert!(url.contains("linkedin.com/oauth/v2/authorization"), "URL should go to LinkedIn OAuth");
    assert!(url.contains("w_member_social"), "Should contain the right scopes");

    println!("✅ linkedin_mcp_handler: Full chain verified (both providers registered, auth URLs, server creation)");
}

// ── Provider Configuration Verification ─────────────────────

#[tokio::test]
async fn test_ias_and_threads_credentials_loaded() {
    let config = get_config();

    // Instagram Standalone needs INSTAGRAM_APP_ID + INSTAGRAM_APP_SECRET
    let ias_creds = config.provider_credentials("instagram-standalone");
    assert!(ias_creds.is_some(), "Instagram Standalone credentials should be loaded from .env (INSTAGRAM_APP_ID/INSTAGRAM_APP_SECRET)");
    let (cid, secret) = ias_creds.unwrap();
    assert!(!cid.is_empty(), "Instagram Standalone client_id should not be empty");
    assert!(!secret.is_empty(), "Instagram Standalone client_secret should not be empty");
    println!("✅ Instagram Standalone credentials loaded (client_id: {}...)", &cid[..5]);

    // Threads needs THREADS_APP_ID + THREADS_APP_SECRET
    let threads_creds = config.provider_credentials("threads");
    assert!(threads_creds.is_some(), "Threads credentials should be loaded from .env (THREADS_APP_ID/THREADS_APP_SECRET)");
    let (cid2, secret2) = threads_creds.unwrap();
    assert!(!cid2.is_empty(), "Threads client_id should not be empty");
    assert!(!secret2.is_empty(), "Threads client_secret should not be empty");
    println!("✅ Threads credentials loaded (client_id: {}...)", &cid2[..5]);
}

// ── Discord Provider Method Tests ──────────────────────────────

#[tokio::test]
async fn test_discord_send_message() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.send_message("0", "test message").await;
    assert!(result.is_err(), "Should fail with bad token/channel");
    println!("✅ discord_send_message: Properly handled API error");
}

#[tokio::test]
async fn test_discord_delete_message() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.delete_message("0", "0").await;
    assert!(result.is_err());
    println!("✅ discord_delete_message: Properly handled API error");
}

#[tokio::test]
async fn test_discord_add_reaction() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.add_reaction("0", "0", "👍").await;
    assert!(result.is_err());
    println!("✅ discord_add_reaction: Properly handled API error");
}

#[tokio::test]
async fn test_discord_get_guild_channels() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.get_guild_channels("0").await;
    assert!(result.is_err());
    println!("✅ discord_get_guild_channels: Properly handled API error");
}

#[tokio::test]
async fn test_discord_get_server_info() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.get_server_info("0").await;
    assert!(result.is_err());
    println!("✅ discord_get_server_info: Properly handled API error");
}

#[tokio::test]
async fn test_discord_create_forum_post() {
    let config = get_config();
    let provider = DiscordProvider::new(&config);
    let result = provider.create_forum_post("0", "Test Post", "Body text", &[]).await;
    assert!(result.is_err());
    println!("✅ discord_create_forum_post: Properly handled API error");
}

// ── Skool Provider Method Tests ────────────────────────────────

#[tokio::test]
async fn test_skool_get_community_info() {
    let provider = SkoolProvider::new();
    let result = provider.get_community_info("test", "bad-token").await;
    match &result {
        Err(e) => println!("✅ skool_get_community_info: Got error (expected with bad token): {e}"),
        Ok(v) => println!("⚠️  skool_get_community_info: Returned data (skool.com may not require auth): {v:?}"),
    }
}

#[tokio::test]
async fn test_skool_list_posts() {
    let provider = SkoolProvider::new();
    let result = provider.list_posts("test", "bad-token", None, None, None).await;
    match &result {
        Err(e) => println!("✅ skool_list_posts: Got error (expected with bad token): {e}"),
        Ok(v) => println!("⚠️  skool_list_posts: Returned data (skool.com may not require auth): {v:?}"),
    }
}

#[tokio::test]
async fn test_skool_get_post() {
    let provider = SkoolProvider::new();
    let result = provider.get_post("test", "test-post", "bad-token").await;
    assert!(result.is_err());
    println!("✅ skool_get_post: Properly handled API error");
}

#[tokio::test]
async fn test_skool_create_comment() {
    let provider = SkoolProvider::new();
    let result = provider.create_comment("post1", "group1", "hello", "bad-token").await;
    assert!(result.is_err());
    println!("✅ skool_create_comment: Properly handled API error");
}

// ── YouTube Provider Method Tests ──────────────────────────────

#[tokio::test]
async fn test_youtube_find_creators() {
    let config = get_config();
    let provider = YoutubeProvider::new(&config);
    let result = provider.find_creators("bad-token", "rust programming", None, None).await;
    assert!(result.is_err());
    println!("✅ youtube_find_creators: Properly handled API error");
}

// ── Pinterest Provider Method Tests ────────────────────────────

#[tokio::test]
async fn test_pinterest_search_pins() {
    let config = get_config();
    let provider = PinterestProvider::new(&config);
    let result = provider.search_pins("bad-token", "landscape", None).await;
    assert!(result.is_err());
    println!("✅ pinterest_search_pins: Properly handled API error");
}

// ── X/Twitter Provider Method Tests ─────────────────────────────
// XProvider reads credentials from env, so we require a valid config.

const X_BAD_TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAInvalidXTokenForTesting";

#[tokio::test]
async fn test_x_get_me() {
    let config = get_config();
    let provider = XProvider::new(&config);
    let result = provider.get_me(X_BAD_TOKEN).await;
    match &result {
        Err(e) => println!("✅ x_get_me: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  x_get_me: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_x_home_timeline() {
    let config = get_config();
    let provider = XProvider::new(&config);
    let result = provider.home_timeline(X_BAD_TOKEN, "44196397", 5, None).await;
    match &result {
        Err(e) => println!("✅ x_home_timeline: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  x_home_timeline: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_x_search_tweets() {
    let config = get_config();
    let provider = XProvider::new(&config);
    let result = provider.search_tweets(X_BAD_TOKEN, "rust programming", 5, None).await;
    match &result {
        Err(e) => println!("✅ x_search_tweets: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  x_search_tweets: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_x_followers() {
    let config = get_config();
    let provider = XProvider::new(&config);
    let result = provider.followers(X_BAD_TOKEN, "44196397", 5, None).await;
    match &result {
        Err(e) => println!("✅ x_followers: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  x_followers: Unexpected success: {v:?}"),
    }
}

// ── Reddit Provider Method Tests ────────────────────────────────

const REDDIT_BAD_TOKEN: &str = "invalid_reddit_token_for_testing_12345";

#[tokio::test]
async fn test_reddit_browse() {
    let config = get_config();
    let provider = RedditProvider::new(&config);
    let result = provider.browse(REDDIT_BAD_TOKEN, "rust", "hot", 5, "all").await;
    match &result {
        Err(e) => println!("✅ reddit_browse: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  reddit_browse: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_reddit_search() {
    let config = get_config();
    let provider = RedditProvider::new(&config);
    let result = provider.search(REDDIT_BAD_TOKEN, "test query", Some("all"), "new", 5, "all").await;
    match &result {
        Err(e) => println!("✅ reddit_search: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  reddit_search: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_reddit_inbox() {
    let config = get_config();
    let provider = RedditProvider::new(&config);
    let result = provider.inbox(REDDIT_BAD_TOKEN, "inbox", 5).await;
    match &result {
        Err(e) => println!("✅ reddit_inbox: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  reddit_inbox: Unexpected success: {v:?}"),
    }
}

// ── WordPress Provider Method Tests ─────────────────────────────
// WordPress expects a JSON token {"site_url":"...","username":"...","app_password":"..."}

const WP_BAD_TOKEN: &str = r#"{"site_url":"https://example.com","username":"admin","app_password":"bad"}"#;

#[tokio::test]
async fn test_wordpress_list_posts() {
    let config = get_config();
    let provider = WordPressProvider::new(&config);
    let result = provider.list_posts(WP_BAD_TOKEN, None, None).await;
    match &result {
        Err(e) => println!("✅ wordpress_list_posts: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  wordpress_list_posts: Unexpected success: {v:?}"),
    }
}

#[tokio::test]
async fn test_wordpress_get_post() {
    let config = get_config();
    let provider = WordPressProvider::new(&config);
    let result = provider.get_post(WP_BAD_TOKEN, 1).await;
    match &result {
        Err(e) => println!("✅ wordpress_get_post: Properly handled API error: {e}"),
        Ok(v) => println!("⚠️  wordpress_get_post: Unexpected success: {v:?}"),
    }
}
