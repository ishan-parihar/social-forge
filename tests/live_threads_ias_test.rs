// ─── Live Integration Tests: Threads + Instagram Standalone ──
// Run: cargo test --test live_threads_ias_test -- --nocapture
// Requires: SQLite file, .env with INSTAGRAM_APP_ID,
//   THREADS_APP_ID, and real OAuth tokens in the DB for dev@social-forge.dev.

use social_forge::config::Config;
use social_forge::db;
use social_forge::db::queries;
use social_forge::social::instagram_standalone::InstagramStandaloneProvider;
use social_forge::social::threads::ThreadsProvider;
use social_forge::social::SocialProvider;

// ── Threads Tests ──────────────────────────────────────────────

#[tokio::test]
async fn test_threads_get_profile() {
    let Some((provider, token)) = get_threads_creds().await else {
        println!("⏭ threads_get_profile skipped (no threads integration in DB)");
        return;
    };
    let result = provider.get_profile(&token).await;
    println!("Threads get_profile: {:?}", result);
    assert!(result.is_ok());
    let json = result.unwrap();
    assert!(json["id"].as_str().is_some());
    println!(
        "Threads user: id={:?}, username={:?}",
        json["id"], json["username"]
    );
}

#[tokio::test]
async fn test_threads_social_provider_meta() {
    let Some((provider, _)) = get_threads_creds().await else {
        println!("⏭ threads_social_provider_meta skipped (no threads integration in DB)");
        return;
    };
    assert_eq!(provider.identifier(), "threads");
    assert_eq!(provider.name(), "Threads");
    assert!(provider.uses_oauth());
    let scopes = provider.scopes();
    assert!(scopes.contains(&"threads_basic".to_string()));
    println!("Threads scopes: {:?}", scopes);
}

#[tokio::test]
async fn test_ias_get_media() {
    let Some((provider, token, internal_id)) = get_ias_creds().await else {
        println!("⏭ ias_get_media skipped (no instagram-standalone integration in DB)");
        return;
    };
    let result = provider.get_media(&token, &internal_id, 5).await;
    println!("IAS get_media: {:?}", result);
    if let Ok(json) = &result {
        println!(
            "IAS media count: {:?}",
            json["data"].as_array().map(|a| a.len())
        );
    }
}

#[tokio::test]
async fn test_ias_social_provider_meta() {
    let Some((provider, _, _)) = get_ias_creds().await else {
        println!("⏭ ias_social_provider_meta skipped (no instagram-standalone integration in DB)");
        return;
    };
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

async fn get_token(
    config: &Config,
    pool: &sqlx::SqlitePool,
    provider_identifier: &str,
) -> Option<(String, String)> {
    let user = queries::get_user_by_email(pool, "dev@social-forge.dev")
        .await
        .expect("DB query")?;

    let integrations = queries::list_integrations(pool, user.id)
        .await
        .expect("list integrations");

    let integration = integrations
        .iter()
        .find(|i| i.provider_identifier == provider_identifier)?;

    let token = config
        .token_encryption_key
        .as_ref()
        .and_then(|k| {
            let key = social_forge::crypto::decode_hex_key(k).ok()?;
            social_forge::crypto::decrypt_string(&integration.access_token, &key).ok()
        })
        .unwrap_or_else(|| integration.access_token.clone());

    Some((token, integration.internal_id.clone()))
}

async fn get_threads_creds() -> Option<(ThreadsProvider, String)> {
    let config = get_config();
    let provider = get_threads_provider(&config);
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("DB pool");
    let (token, _) = get_token(&config, &pool, "threads").await?;
    Some((provider, token))
}

async fn get_ias_creds() -> Option<(InstagramStandaloneProvider, String, String)> {
    let config = get_config();
    let provider = get_ias_provider(&config);
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("DB pool");
    let (token, internal_id) = get_token(&config, &pool, "instagram-standalone").await?;
    Some((provider, token, internal_id))
}
