// ─── Live Integration Tests: Threads + Instagram Standalone ──
// Run: cargo test --test live_threads_ias_test -- --nocapture
// Requires: Docker (postgres + redis), .env with INSTAGRAM_APP_ID,
//   THREADS_APP_ID, and real OAuth tokens in the DB for dev@postiz.dev.

use postiz_rust::config::Config;
use postiz_rust::db;
use postiz_rust::db::queries;
use postiz_rust::social::instagram_standalone::InstagramStandaloneProvider;
use postiz_rust::social::threads::ThreadsProvider;
use postiz_rust::social::SocialProvider;

// ── Threads Tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_threads_get_profile() {
    let (provider, token) = get_threads_creds().await;
    let result = provider.get_profile(&token).await;
    println!("Threads get_profile: {:?}", result);
    assert!(result.is_ok());
    let json = result.unwrap();
    assert!(json["id"].as_str().is_some());
    println!("Threads user: id={:?}, username={:?}", json["id"], json["username"]);
}

#[tokio::test]
async fn test_threads_social_provider_meta() {
    let (provider, _) = get_threads_creds().await;
    assert_eq!(provider.identifier(), "threads");
    assert_eq!(provider.name(), "Threads");
    assert!(provider.uses_oauth());
    let scopes = provider.scopes();
    assert!(scopes.contains(&"threads_basic".to_string()));
    println!("Threads scopes: {:?}", scopes);
}

#[tokio::test]
async fn test_ias_get_media() {
    let (provider, token, internal_id) = get_ias_creds().await;
    let result = provider.get_media(&token, &internal_id, 5).await;
    println!("IAS get_media: {:?}", result);
    if let Ok(json) = &result {
        println!("IAS media count: {:?}", json["data"].as_array().map(|a| a.len()));
    }
}

#[tokio::test]
async fn test_ias_social_provider_meta() {
    let (provider, _, _) = get_ias_creds().await;
    assert_eq!(provider.identifier(), "instagram-standalone");
    assert_eq!(provider.name(), "Instagram (Standalone)");
    assert!(provider.max_content_length() > 0);
    assert!(provider.uses_oauth());
    let scopes = provider.scopes();
    assert!(scopes.contains(&"instagram_business_content_publish".to_string()));
    println!("IAS scopes: {:?}", scopes);
}

// ── Helpers ──────────────────────────────────────────────────

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config")
}

fn get_threads_provider(config: &Config) -> ThreadsProvider {
    ThreadsProvider::new(config)
}

fn get_ias_provider(config: &Config) -> InstagramStandaloneProvider {
    InstagramStandaloneProvider::new(config)
}

async fn get_token(config: &Config, pool: &sqlx::PgPool, provider_identifier: &str) -> (String, String) {
    let user = queries::get_user_by_email(pool, "dev@postiz.dev")
        .await
        .expect("DB query")
        .expect("dev@postiz.dev not found");

    let integrations = queries::list_integrations(pool, user.id)
        .await
        .expect("list integrations");

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == provider_identifier)
        .unwrap_or_else(|| panic!("No {provider_identifier} integration found"));

    let token = config.token_encryption_key
        .as_ref()
        .and_then(|k| {
            let key = postiz_rust::crypto::decode_hex_key(k).ok()?;
            postiz_rust::crypto::decrypt_string(&integration.access_token, &key).ok()
        })
        .unwrap_or_else(|| integration.access_token.clone());

    (token, integration.internal_id.clone())
}

async fn get_threads_creds() -> (ThreadsProvider, String) {
    let config = get_config();
    let provider = get_threads_provider(&config);
    let pool = db::create_pool(&config.database_url).await.expect("DB pool");
    let (token, _) = get_token(&config, &pool, "threads").await;
    (provider, token)
}

async fn get_ias_creds() -> (InstagramStandaloneProvider, String, String) {
    let config = get_config();
    let provider = get_ias_provider(&config);
    let pool = db::create_pool(&config.database_url).await.expect("DB pool");
    let (token, internal_id) = get_token(&config, &pool, "instagram-standalone").await;
    (provider, token, internal_id)
}
