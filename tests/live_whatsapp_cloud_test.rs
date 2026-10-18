// Live WhatsApp Cloud API tests (E6). Skip-guarded: without
// WHATSAPP_CLOUD_TOKEN + PHONE_NUMBER_ID + WABA_ID the tests return early.
// The template CRUD round-trip creates a utility template, lists it, then
// deletes it — safe on any WABA (no send is performed without
// WHATSAPP_CLOUD_TEST_TO).

use social_forge::config::Config;
use social_forge::social::whatsapp_cloud::WhatsAppCloudProvider;

fn cloud(config: &Config) -> Option<WhatsAppCloudProvider> {
    let p = WhatsAppCloudProvider::from_config(config);
    if p.is_none() {
        eprintln!("⏭ skipped: WHATSAPP_CLOUD_TOKEN/_PHONE_NUMBER_ID/_WABA_ID not set");
    }
    p
}

fn config() -> Config {
    social_forge::config::load_dotenv_for_live_tests();
    Config::from_env().expect("Failed to load config")
}

const TEMPLATE_NAME: &str = "social_forge_e6_probe";

#[tokio::test]
async fn test_wc_template_crud_roundtrip() {
    let config = config();
    let Some(provider) = cloud(&config) else { return };

    // Clean slate: a leftover from an aborted run must not fail the create.
    let _ = provider.delete_template(TEMPLATE_NAME).await;

    // Create — Meta validates names to lowercase+underscores.
    let created = provider
        .create_template(
            TEMPLATE_NAME,
            "en",
            "UTILITY",
            serde_json::json!([
                { "type": "BODY", "text": "E6 round-trip probe {{1}}", }
            ]),
        )
        .await
        .unwrap_or_else(|e| panic!("create_template failed: {e}"));
    assert!(
        created["id"].is_string() || created["status"].is_string(),
        "create returns an id or status: {created}"
    );
    eprintln!("✅ template created: {created}");

    // Read — it must appear in the list.
    let listed = provider
        .list_templates(100, None)
        .await
        .unwrap_or_else(|e| panic!("list_templates failed: {e}"));
    let names: Vec<&str> = listed["data"]
        .as_array()
        .map(|d| {
            d.iter()
                .filter_map(|t| t["name"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        names.contains(&TEMPLATE_NAME),
        "created template must be listed: found {names:?}"
    );
    eprintln!("✅ template listed among {} total", names.len());

    // Delete — and confirm gone.
    let deleted = provider
        .delete_template(TEMPLATE_NAME)
        .await
        .unwrap_or_else(|e| panic!("delete_template failed: {e}"));
    assert_eq!(
        deleted["success"].as_bool(),
        Some(true),
        "delete reports success: {deleted}"
    );
    let relisted = provider
        .list_templates(100, None)
        .await
        .unwrap_or_else(|e| panic!("re-list failed: {e}"));
    let names: Vec<&str> = relisted["data"]
        .as_array()
        .map(|d| {
            d.iter()
                .filter_map(|t| t["name"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        !names.contains(&TEMPLATE_NAME),
        "deleted template must be gone: {names:?}"
    );
    eprintln!("✅ template deleted and gone from list");
}

#[tokio::test]
async fn test_wc_send_template_to_test_number() {
    let config = config();
    let (Some(provider), Some(to)) = (
        cloud(&config),
        std::env::var("WHATSAPP_CLOUD_TEST_TO").ok().filter(|v| !v.is_empty()),
    ) else {
        eprintln!("⏭ skipped: WHATSAPP_CLOUD_TEST_TO not set (no live send)");
        return;
    };

    // hello_world ships pre-approved on every sandbox WABA.
    let sent = provider
        .send_template(&to, "hello_world", "en_US", serde_json::json!([]))
        .await
        .unwrap_or_else(|e| panic!("send_template failed: {e}"));
    let id = sent["messages"][0]["id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(id.starts_with("wamid."), "send returns a wamid: {sent}");
    eprintln!("✅ template message sent: {id}");
}

#[tokio::test]
async fn test_wc_webhook_signature_helpers() {
    // Pure, but lives with the live suite: the webhook parse+verify path.
    let sig = social_forge::api::whatsapp_webhook::signature_hex("secret", b"{}");
    assert!(social_forge::api::whatsapp_webhook::verify_signature(
        "secret",
        b"{}",
        &format!("sha256={sig}")
    ));
    assert!(!social_forge::api::whatsapp_webhook::verify_signature(
        "secret",
        b"{}",
        "sha256=tampered"
    ));
    eprintln!("✅ webhook signature helpers verified");
}
