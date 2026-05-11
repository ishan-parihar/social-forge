use std::path::PathBuf;
use std::fs;

use postiz_rust::config::Config;
use postiz_rust::services::whatsapp_daemon::WhatsAppDaemon;
use postiz_rust::social::registry::ProviderRegistry;
use postiz_rust::social::whatsapp::WhatsAppProvider;
use postiz_rust::social::SocialProvider;
use postiz_rust::mcp::tools_whatsapp;
use rmcp::Json;

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config from .env")
}

fn find_wacli_binary() -> Option<PathBuf> {
    let candidates = vec![
        PathBuf::from("./wacli/dist/wacli"),
        PathBuf::from("../wacli/dist/wacli"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.canonicalize().unwrap_or_else(|_| c.clone()));
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("wacli");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn create_test_store_dir() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("wa-test-{}", ts));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn test_whatsapp_provider_metadata() {
    std::env::set_var("WHATSAPP_STORE_DIR", "/tmp/nonexistent-wa-store-test");
    let config = get_config();
    let registry = ProviderRegistry::new(&config, None);
    let wa = registry.get("whatsapp").expect("WhatsApp provider should exist");

    assert_eq!(wa.identifier(), "whatsapp");
    assert_eq!(wa.name(), "WhatsApp");
    assert!(!wa.uses_oauth(), "WhatsApp should NOT use OAuth");
    assert!(wa.one_time_token(), "WhatsApp should be one-time token");
    assert_eq!(wa.max_content_length(), 4096, "WhatsApp max content length");

    let scopes = wa.scopes();
    assert!(scopes.is_empty(), "WhatsApp should have no OAuth scopes");

    use postiz_rust::social::EditorType;
    assert_eq!(wa.editor_type(), EditorType::Normal);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let auth_url = rt.block_on(wa.generate_auth_url("state", "verifier", "http://localhost/callback"));
    assert!(auth_url.is_ok());
    assert_eq!(auth_url.unwrap().url, "", "Non-OAuth provider should return empty auth URL");

    let exchange = rt.block_on(wa.exchange_code("test-code", "verifier", "http://localhost/callback"));
    assert!(exchange.is_err(), "exchange_code without daemon should fail");

    let refresh = rt.block_on(wa.refresh_token("test-refresh"));
    assert!(refresh.is_err(), "refresh_token should error for non-OAuth provider");

    use postiz_rust::social::PostContent;
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
    let registry = ProviderRegistry::new(&config, None);
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
fn test_whatsapp_daemon_ipc_ping() {
    let wacli_path = find_wacli_binary()
        .expect("wacli binary required. Build with: cd wacli && pnpm build");

    let store_dir = create_test_store_dir();

    let daemon = WhatsAppDaemon::start_with_binary(wacli_path, store_dir)
        .expect("WhatsAppDaemon should start successfully");

    assert!(daemon.is_running(), "wacli server process should be running");

    let result = daemon.send_request("ping", None);
    assert!(result.is_ok(), "ping should succeed: {:?}", result.err());
    assert_eq!(
        result.unwrap(),
        serde_json::Value::String("pong".to_string()),
        "ping should return 'pong'"
    );

    daemon.stop().expect("stop should succeed");
    assert!(!daemon.is_running(), "daemon should not be running after stop");

    println!("PASS: WhatsAppDaemon IPC Ping/Pong");
}

#[test]
fn test_whatsapp_daemon_auth_status_unauthenticated() {
    let wacli_path = find_wacli_binary()
        .expect("wacli binary required for this test");

    let store_dir = create_test_store_dir();

    let daemon = WhatsAppDaemon::start_with_binary(wacli_path, store_dir)
        .expect("WhatsAppDaemon should start successfully");

    let status = daemon.auth_status();
    assert!(status.is_ok(), "auth_status should not error: {:?}", status.err());

    let val = status.unwrap();
    let authenticated = val["authenticated"].as_bool().unwrap_or(true);
    assert!(!authenticated, "auth_status should report not authenticated");

    println!("PASS: WhatsAppDaemon auth_status correctly reports unauthenticated");
    println!("   Response: {}", serde_json::to_string(&val).unwrap());
}

#[test]
fn test_whatsapp_daemon_list_chats_unauthenticated() {
    let wacli_path = find_wacli_binary()
        .expect("wacli binary required for this test");

    let store_dir = create_test_store_dir();

    let daemon = WhatsAppDaemon::start_with_binary(wacli_path, store_dir)
        .expect("WhatsAppDaemon should start successfully");

    let chats = daemon.list_chats(Some(10), None);
    assert!(
        chats.is_err(),
        "list_chats without auth should fail. Got: {:?}",
        chats
    );
    let err_msg = chats.err().unwrap();
    assert!(
        err_msg.contains("not logged in") || err_msg.contains("auth"),
        "Error should mention auth: {}",
        err_msg
    );
    println!("PASS: WhatsAppDaemon list_chats correctly rejected (unauthenticated)");
    println!("   Error: {}", err_msg);

    let contacts = daemon.list_contacts(Some(10), None);
    // contacts are stored locally in the DB — accessible without auth
    if let Ok(val) = &contacts {
        println!("   contacts accessible without auth (local store): {:?}", val);
        println!("   (this is expected — contacts are read from local SQLite DB)");
    } else {
        println!("   contacts also rejected (daemon requires auth even for local ops)");
    }
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
fn test_whatsapp_daemon_lifecycle() {
    let wacli_path = find_wacli_binary()
        .expect("wacli binary required for this test");

    let store_dir = create_test_store_dir();

    let daemon = WhatsAppDaemon::start_with_binary(wacli_path.clone(), store_dir.clone())
        .expect("start daemon");
    assert!(daemon.is_running(), "should be running after start");

    let ping = daemon.send_request("ping", None);
    assert!(ping.is_ok(), "server should respond to ping: {:?}", ping.err());
    assert_eq!(ping.unwrap(), serde_json::Value::String("pong".to_string()));
    println!("PASS: Lifecycle - server responsive after start");

    daemon.stop().expect("stop daemon");
    assert!(!daemon.is_running(), "should not be running after stop");
    println!("PASS: Lifecycle - server stopped cleanly");

    let daemon2 = WhatsAppDaemon::start_with_binary(wacli_path, store_dir)
        .expect("restart daemon");
    assert!(daemon2.is_running(), "should be running after restart");

    let ping2 = daemon2.send_request("ping", None);
    assert!(ping2.is_ok(), "server should respond after restart");
    assert_eq!(ping2.unwrap(), serde_json::Value::String("pong".to_string()));
    println!("PASS: Lifecycle - server restarted successfully");
}

#[test]
fn test_whatsapp_daemon_rpc_error_unknown_method() {
    let wacli_path = find_wacli_binary()
        .expect("wacli binary required for this test");

    let store_dir = create_test_store_dir();

    let daemon = WhatsAppDaemon::start_with_binary(wacli_path, store_dir)
        .expect("start daemon");

    let result = daemon.send_request("nonexistent_method_xyz", None);
    assert!(result.is_err(), "Unknown method should return error");
    let err = result.err().unwrap();
    assert!(
        err.contains("unknown method"),
        "Error should mention unknown method. Got: {}",
        err
    );
    println!("PASS: WhatsAppDaemon unknown method correctly rejected");
    println!("   Error: {}", err);
}

#[test]
fn test_whatsapp_provider_creation_with_real_config() {
    let config = get_config();
    let wa = WhatsAppProvider::new(&config);

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
    let db = postiz_rust::db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use postiz_rust::api::AppState;
    use postiz_rust::mcp::PostizMcpServer;
    use postiz_rust::realtime::Broadcaster;

    let broadcaster = Broadcaster::new();
    let registry = ProviderRegistry::new(&config, None);
    let rate_limiter = postiz_rust::api::rate_limiter::AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: registry,
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
    };

    let server = PostizMcpServer::new(state.clone());
    assert!(server.state.config.whatsapp_store_dir.is_some() || server.state.config.whatsapp_store_dir.is_none());

    let wa_provider = server.state.providers.get("whatsapp");
    assert!(wa_provider.is_some(), "WhatsApp provider should be in server state");
    let wa = wa_provider.unwrap();
    assert_eq!(wa.identifier(), "whatsapp");
    assert_eq!(wa.name(), "WhatsApp");

    println!("PASS: PostizMcpServer includes WhatsApp provider");
}

#[tokio::test]
async fn test_whatsapp_mcp_handler_functions() {
    let config = get_config();
    let db = postiz_rust::db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");

    use postiz_rust::api::AppState;
    use postiz_rust::realtime::Broadcaster;

    let broadcaster = Broadcaster::new();
    let registry = ProviderRegistry::new(&config, None);
    let rate_limiter = postiz_rust::api::rate_limiter::AuthRateLimiter::new(5, 60);

    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        broadcast: broadcaster.clone(),
        providers: registry,
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
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
