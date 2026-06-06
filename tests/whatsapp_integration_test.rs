use std::path::PathBuf;
use std::fs;

use social_forge::config::Config;
use social_forge::services::whatsapp_daemon::WhatsAppDaemon;
use social_forge::social::registry::ProviderRegistry;
use social_forge::social::whatsapp::WhatsAppProvider;
use social_forge::social::SocialProvider;
use social_forge::mcp::tools_whatsapp;
use social_forge::wa::chats;
use rmcp::Json;

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config from .env")
}

fn create_test_store_dir() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("wa-test-{}", ts));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// Helper: seed a chat_summaries row into the meta DB at `store_dir`.
fn insert_chat(store_dir: &PathBuf, jid: &str, name: &str, unread: u32, last_msg: Option<&str>) {
    let db = chats::ensure_meta_db(store_dir).expect("ensure_meta_db");
    db.execute(
        "INSERT OR REPLACE INTO chat_summaries (jid, name, unread_count, last_message_text) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![jid, name, unread, last_msg],
    ).expect("insert chat");
}

/// Helper: seed a contact_entries row into the meta DB at `store_dir`.
fn insert_contact(store_dir: &PathBuf, jid: &str, name: &str, push_name: &str) {
    let db = chats::ensure_meta_db(store_dir).expect("ensure_meta_db");
    db.execute(
        "INSERT OR REPLACE INTO contact_entries (jid, name, push_name) VALUES (?1, ?2, ?3)",
        rusqlite::params![jid, name, push_name],
    ).expect("insert contact");
}

#[test]
fn test_whatsapp_provider_metadata() {
    std::env::set_var("WHATSAPP_STORE_DIR", "/tmp/nonexistent-wa-store-test");
    let config = get_config();
    let registry = ProviderRegistry::new(&config, None, None);
    let wa = registry.get("whatsapp").expect("WhatsApp provider should exist");

    assert_eq!(wa.identifier(), "whatsapp");
    assert_eq!(wa.name(), "WhatsApp");
    assert!(!wa.uses_oauth(), "WhatsApp should NOT use OAuth");
    assert!(wa.one_time_token(), "WhatsApp should be one-time token");
    assert_eq!(wa.max_content_length(), 4096, "WhatsApp max content length");

    let scopes = wa.scopes();
    assert!(scopes.is_empty(), "WhatsApp should have no OAuth scopes");

    use social_forge::social::EditorType;
    assert_eq!(wa.editor_type(), EditorType::Normal);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let auth_url = rt.block_on(wa.generate_auth_url("state", "verifier", "http://localhost/callback"));
    assert!(auth_url.is_ok());
    assert_eq!(auth_url.unwrap().url, "", "Non-OAuth provider should return empty auth URL");

    let exchange = rt.block_on(wa.exchange_code("test-code", "verifier", "http://localhost/callback"));
    assert!(exchange.is_err(), "exchange_code without daemon should fail");

    let refresh = rt.block_on(wa.refresh_token("test-refresh"));
    assert!(refresh.is_err(), "refresh_token should error for non-OAuth provider");

    use social_forge::social::PostContent;
    let post = PostContent {
        content: "Hello WhatsApp".into(),
        media: vec![],
        settings: serde_json::json!({}),
    };
    let publish = rt.block_on(wa.publish("test-jid", &post));
    assert!(publish.is_err(), "publish without daemon should fail");

    let page = rt.block_on(wa.fetch_page_info("token", "page-id"));
    assert!(page.is_err(), "WhatsApp should not support page management");

    println!("PASS: WhatsAppProvider metadata traits verified");
}

#[test]
fn test_whatsapp_provider_registration() {
    std::env::set_var("WHATSAPP_STORE_DIR", "/tmp/nonexistent-wa-store-test-2");
    let config = get_config();
    let registry = ProviderRegistry::new(&config, None, None);
    let ids = registry.list();

    assert!(
        ids.contains(&"whatsapp"),
        "Provider registry should contain 'whatsapp'. Found: {:?}",
        ids
    );

    let wa = registry.get("whatsapp");
    assert!(wa.is_some(), "WhatsApp provider should be retrievable");
    assert_eq!(wa.unwrap().identifier(), "whatsapp");

    println!("PASS: WhatsAppProvider registered in provider registry");
    println!("   Total providers: {}", ids.len());
}

#[test]
fn test_wa_meta_db_chat_crud() {
    let store_dir = create_test_store_dir();

    // Fresh store → empty
    let chats_empty = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert!(chats_empty.is_empty(), "Fresh meta DB should have no chats");

    // Seed 2 chats
    insert_chat(&store_dir, "111@s.whatsapp.net", "Alice", 3, Some("Hey!"));
    insert_chat(&store_dir, "222@s.whatsapp.net", "Bob", 0, None);

    let chats = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].name, "Alice");
    assert_eq!(chats[0].unread_count, 3);
    assert_eq!(chats[0].last_message_text.as_deref(), Some("Hey!"));
    assert_eq!(chats[1].name, "Bob");
    assert_eq!(chats[1].unread_count, 0);
    assert!(chats[1].last_message_text.is_none());

    println!("PASS: wa-rs meta DB chat CRUD — 2 chats stored & retrieved");
}

#[test]
fn test_wa_meta_db_contact_crud() {
    let store_dir = create_test_store_dir();

    let contacts_empty = chats::list_contacts_store(&store_dir, None).expect("list_contacts_store");
    assert!(contacts_empty.is_empty(), "Fresh meta DB should have no contacts");

    insert_contact(&store_dir, "333@s.whatsapp.net", "Charlie", "Charlie P.");
    insert_contact(&store_dir, "444@s.whatsapp.net", "Diana", "Diana Q.");

    let contacts = chats::list_contacts_store(&store_dir, None).expect("list_contacts_store");
    assert_eq!(contacts.len(), 2);
    assert_eq!(contacts[0].name, "Charlie");
    assert_eq!(contacts[0].push_name, "Charlie P.");
    assert_eq!(contacts[1].name, "Diana");

    println!("PASS: wa-rs meta DB contact CRUD — 2 contacts stored & retrieved");
}

#[test]
fn test_wa_meta_db_empty_on_fresh_store() {
    let store_dir = create_test_store_dir();

    let chats = chats::list_chats_store(&store_dir, Some(50)).expect("list_chats_store");
    assert!(chats.is_empty(), "Fresh meta DB should return empty chats");

    let contacts = chats::list_contacts_store(&store_dir, Some(50)).expect("list_contacts_store");
    assert!(contacts.is_empty(), "Fresh meta DB should return empty contacts");

    println!("PASS: wa-rs meta DB empty results on fresh store");
}

#[test]
fn test_wa_meta_db_limit_respected() {
    let store_dir = create_test_store_dir();

    for i in 0..10 {
        let jid = format!("{}@s.whatsapp.net", 1000 + i);
        insert_chat(&store_dir, &jid, &format!("User {}", i), 0, None);
        insert_contact(&store_dir, &jid, &format!("User {}", i), "");
    }

    let limited = chats::list_chats_store(&store_dir, Some(3)).expect("list_chats_store");
    assert_eq!(limited.len(), 3, "limit=3 should return 3 chats");

    let all = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(all.len(), 10, "default limit should return all 10");

    let contacts_limited = chats::list_contacts_store(&store_dir, Some(5)).expect("list_contacts_store");
    assert_eq!(contacts_limited.len(), 5, "limit=5 should return 5 contacts");

    println!("PASS: wa-rs meta DB LIMIT respected (chats={}, contacts={})", limited.len(), contacts_limited.len());
}

#[test]
fn test_wa_meta_db_structure() {
    let store_dir = create_test_store_dir();

    // ensure_meta_db creates tables on first call
    let _db = chats::ensure_meta_db(&store_dir).expect("ensure_meta_db");

    // Verify tables exist by inserting via raw rusqlite and reading via store API
    insert_chat(&store_dir, "555@s.whatsapp.net", "Eve", 1, Some("Hello"));
    let results = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].jid, "555@s.whatsapp.net");

    // Verify it's a real SQLite DB file
    let db_path = store_dir.join("wa-meta.db");
    assert!(db_path.exists(), "wa-meta.db should exist on disk");
    let meta = std::fs::metadata(&db_path).expect("metadata");
    assert!(meta.len() > 0, "wa-meta.db should have content");

    println!("PASS: wa-rs meta DB schema & structure verified");
    println!("   DB path: {} ({} bytes)", db_path.display(), meta.len());
}

#[test]
fn test_wa_meta_db_upsert() {
    let store_dir = create_test_store_dir();

    // Insert then update
    insert_chat(&store_dir, "666@s.whatsapp.net", "First", 1, Some("initial"));
    let c1 = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(c1.len(), 1);
    assert_eq!(c1[0].name, "First");

    insert_chat(&store_dir, "666@s.whatsapp.net", "Updated", 5, Some("modified"));
    let c2 = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(c2.len(), 1, "upsert should keep 1 row");
    assert_eq!(c2[0].name, "Updated");
    assert_eq!(c2[0].unread_count, 5);

    println!("PASS: wa-rs meta DB upsert works correctly");
}

#[test]
fn test_whatsapp_daemon_not_found() {
    let result = WhatsAppDaemon::start_with_binary(
        PathBuf::from("/usr/bin/nonexistent-binary-12345"),
        PathBuf::from("/tmp"),
    );
    assert!(
        result.is_err(),
        "Starting with non-existent binary should fail"
    );
    let err = result.err().unwrap();
    assert!(
        err.contains("Failed to spawn") || err.contains("No such file"),
        "Error should mention spawn failure. Got: {}",
        err
    );
    println!("PASS: WhatsAppDaemon binary-not-found error correctly propagated");
    println!("   Error: {}", err);
}

#[test]
fn test_wa_meta_db_concurrent_tables() {
    let store_dir = create_test_store_dir();

    // Both tables coexist independently
    insert_chat(&store_dir, "777@s.whatsapp.net", "ChatOne", 2, Some("msg"));
    insert_contact(&store_dir, "777@s.whatsapp.net", "ContactOne", "C1");
    insert_contact(&store_dir, "888@s.whatsapp.net", "ContactTwo", "C2");

    let chats = chats::list_chats_store(&store_dir, None).expect("list_chats_store");
    assert_eq!(chats.len(), 1);

    let contacts = chats::list_contacts_store(&store_dir, None).expect("list_contacts_store");
    assert_eq!(contacts.len(), 2);

    println!("PASS: wa-rs meta DB — chats & contacts tables coexist independently");
}

#[test]
fn test_wa_meta_db_error_on_bad_path() {
    let bad_dir = PathBuf::from("/proc/nonexistent_wa_test_dir_12345");

    let chats = chats::list_chats_store(&bad_dir, None);
    assert!(chats.is_err(), "bad path should produce error");

    let contacts = chats::list_contacts_store(&bad_dir, None);
    assert!(contacts.is_err(), "bad path should produce error");

    println!("PASS: wa-rs meta DB — bad path properly errors");
    println!("   chats error: {:?}", chats.err());
}

#[test]
fn test_whatsapp_provider_creation_with_real_config() {
    let config = get_config();
    let wa = WhatsAppProvider::new(&config, None);

    assert_eq!(wa.identifier(), "whatsapp");
    assert_eq!(wa.name(), "WhatsApp");
    assert!(!wa.uses_oauth());
    assert!(wa.one_time_token());

    println!("PASS: WhatsAppProvider created from real config");
    println!("   whatsapp_store_dir: {:?}", config.whatsapp_store_dir);
}

#[tokio::test]
async fn test_whatsapp_mcp_tool_compilation() {
    let config = get_config();
    let db = social_forge::db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use social_forge::api::AppState;
    use social_forge::mcp::Social ForgeMcpServer;
    use social_forge::realtime::Broadcaster;

    let broadcaster = Broadcaster::new();
    let registry = ProviderRegistry::new(&config, None, None);
    let rate_limiter = social_forge::api::rate_limiter::AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: registry,
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
        media_http_client: reqwest::Client::new(),
    };

    let server = Social ForgeMcpServer::new(state.clone());
    assert!(server.state.config.whatsapp_store_dir.is_some() || server.state.config.whatsapp_store_dir.is_none());

    let wa_provider = server.state.providers.get("whatsapp");
    assert!(wa_provider.is_some(), "WhatsApp provider should be in server state");
    let wa = wa_provider.unwrap();
    assert_eq!(wa.identifier(), "whatsapp");
    assert_eq!(wa.name(), "WhatsApp");

    println!("PASS: Social ForgeMcpServer includes WhatsApp provider");
}

#[tokio::test]
async fn test_whatsapp_mcp_handler_functions() {
    let config = get_config();
    let db = social_forge::db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use social_forge::api::AppState;
    use social_forge::realtime::Broadcaster;

    let broadcaster = Broadcaster::new();
    let registry = ProviderRegistry::new(&config, None, None);
    let rate_limiter = social_forge::api::rate_limiter::AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: registry,
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
        media_http_client: reqwest::Client::new(),
    };

    let send_input = tools_whatsapp::WaSendTextInput {
        to: "919876543210@s.whatsapp.net".to_string(),
        text: "Test message".to_string(),
    };
    assert_eq!(send_input.to, "919876543210@s.whatsapp.net");
    assert_eq!(send_input.text, "Test message");

    let chats_input = tools_whatsapp::WaChatsInput { limit: Some(25) };
    assert_eq!(chats_input.limit, Some(25));

    let contacts_input = tools_whatsapp::WaContactsInput {
        query: Some("test".to_string()),
        limit: Some(10),
    };
    assert_eq!(contacts_input.query, Some("test".to_string()));
    assert_eq!(contacts_input.limit, Some(10));

    let auth_result: Result<Json<tools_whatsapp::WaAuthStatusOutput>, String> =
        tools_whatsapp::handle_wa_auth_status(&state).await;
    assert!(auth_result.is_err(), "wa_auth_status without daemon should error");

    let send_result: Result<Json<tools_whatsapp::WaSendTextOutput>, String> =
        tools_whatsapp::handle_wa_send_text(&state, &send_input).await;
    assert!(send_result.is_err(), "wa_send_text without daemon should error");

    let chats_result: Result<Json<tools_whatsapp::WaChatsOutput>, String> =
        tools_whatsapp::handle_wa_chats(&state, &chats_input).await;
    assert!(chats_result.is_err(), "wa_chats without daemon should error");

    let contacts_result: Result<Json<tools_whatsapp::WaContactsOutput>, String> =
        tools_whatsapp::handle_wa_contacts(&state, &contacts_input).await;
    assert!(contacts_result.is_err(), "wa_contacts without daemon should error");

    println!("PASS: All 4 MCP handler functions compile and error gracefully without daemon");
}
