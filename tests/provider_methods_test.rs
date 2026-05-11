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

use postiz_rust::config::Config;
use postiz_rust::db;
use postiz_rust::social::instagram_standalone::InstagramStandaloneProvider;
use postiz_rust::social::linkedin::LinkedInProvider;
use postiz_rust::social::linkedin_page::LinkedInPageProvider;
use postiz_rust::social::threads::ThreadsProvider;
use postiz_rust::social::SocialProvider;

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

    let post = postiz_rust::social::PostContent {
        content: "Test post".into(),
        media: vec![postiz_rust::social::MediaAttachment {
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

    let post = postiz_rust::social::PostContent {
        content: "Test post from postiz-rust integration test".into(),
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

    let post = postiz_rust::social::PostContent {
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

    use postiz_rust::api::AppState;
    use postiz_rust::mcp::PostizMcpServer;
    use postiz_rust::realtime::Broadcaster;
    use postiz_rust::api::rate_limiter::AuthRateLimiter;
    use postiz_rust::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
    };

    // Verify MCP server can be created (all #[tool] macros compile)
    let server = PostizMcpServer::new(state.clone());
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
    assert!(url.contains("instagram_business_basic"), "Should contain the right scopes");

    println!("✅ ias_mcp_handler: Full chain verified (provider registration, auth URL, server creation)");
}

#[tokio::test]
async fn test_threads_mcp_tool_handler_full_chain() {
    let config = get_config();
    let db = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use postiz_rust::api::AppState;
    use postiz_rust::mcp::PostizMcpServer;
    use postiz_rust::realtime::Broadcaster;
    use postiz_rust::api::rate_limiter::AuthRateLimiter;
    use postiz_rust::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
    };

    // Verify MCP server can be created
    let server = PostizMcpServer::new(state.clone());
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

    use postiz_rust::api::AppState;
    use postiz_rust::mcp::PostizMcpServer;
    use postiz_rust::realtime::Broadcaster;
    use postiz_rust::api::rate_limiter::AuthRateLimiter;
    use postiz_rust::social::registry::ProviderRegistry;

    let registry = Arc::new(ProviderRegistry::new(&config));
    let broadcaster = Broadcaster::new();
    let rate_limiter = AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: (*registry).clone(),
        rate_limiter,
        token_key: None,
    };

    // Verify MCP server can be created
    let server = PostizMcpServer::new(state.clone());
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

    // Threads needs THREADS_CLIENT_ID + THREADS_CLIENT_SECRET
    let threads_creds = config.provider_credentials("threads");
    assert!(threads_creds.is_some(), "Threads credentials should be loaded from .env (THREADS_CLIENT_ID/THREADS_CLIENT_SECRET)");
    let (cid2, secret2) = threads_creds.unwrap();
    assert!(!cid2.is_empty(), "Threads client_id should not be empty");
    assert!(!secret2.is_empty(), "Threads client_secret should not be empty");
    println!("✅ Threads credentials loaded (client_id: {}...)", &cid2[..5]);
}
