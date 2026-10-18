// ─── MCP parity tools (campaigns, sets, RSS, streak, resolve) ───
// The new MCP tools call the REST handlers in-process. These tests drive the
// MCP handler functions against a real (isolated) database and check the
// round trip and the argument validation.
//
// Run: cargo test --test mcp_parity_test
// Uses the scratch DB from `load_dotenv_for_tests` (never data/social-forge.db).

use social_forge::api::AppState;
use social_forge::config::Config;
use social_forge::db;
use social_forge::mcp::{tools_campaigns, tools_rss, tools_sets, tools_analytics};
use social_forge::realtime::Broadcaster;
use social_forge::social::registry::ProviderRegistry;

async fn state() -> AppState {
    social_forge::config::load_dotenv_for_tests();
    let config = Config::from_env().expect("config");
    let db = db::create_pool(&config.database_url).await.expect("pool");
    db::ensure_local_user(&db).await.expect("local user");
    let registry = ProviderRegistry::new(&config, None, None);
    AppState {
        db,
        config: config.clone(),
        broadcast: Broadcaster::new(),
        providers: registry,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
        media_http_client: reqwest::Client::new(),
        media_wreq_client: wreq::Client::new(),
    }
}

fn campaign_create(name: &str) -> tools_campaigns::CampaignCreateInput {
    tools_campaigns::CampaignCreateInput {
        name: name.into(),
        description: None,
        color: None,
        start_date: None,
        end_date: None,
        goal: None,
        status: None,
        progress_metric: None,
        progress_target: None,
        audience_persona: None,
        content_pillars: None,
        budget_cents: None,
        kpi_targets: None,
    }
}

fn campaign_id(v: &serde_json::Value) -> String {
    v["id"].as_str().expect("campaign id").to_string()
}

#[tokio::test]
async fn campaign_round_trip_through_mcp_handlers() {
    let state = state().await;
    let name = format!("parity-{}", uuid::Uuid::new_v4());

    let created = tools_campaigns::handle_campaign_create(&state, &campaign_create(&name))
        .await
        .expect("create");
    let id = campaign_id(&created.0);

    let listed = tools_campaigns::handle_campaign_list(
        &state,
        &tools_campaigns::CampaignListInput {},
    )
    .await
    .expect("list");
    let rows = listed.0.as_array().expect("list is an array");
    assert!(
        rows.iter().any(|c| c["name"] == name.as_str()),
        "created campaign must appear in the list"
    );

    let update = tools_campaigns::CampaignUpdateInput {
        campaign_id: id.clone(),
        name: None,
        description: None,
        color: None,
        start_date: None,
        end_date: None,
        goal: None,
        status: Some("paused".into()),
        progress_metric: None,
        progress_target: None,
        audience_persona: None,
        content_pillars: None,
        budget_cents: None,
        kpi_targets: None,
    };
    let updated = tools_campaigns::handle_campaign_update(&state, &update)
        .await
        .expect("update");
    assert_eq!(updated.0["status"], "paused");
    assert_eq!(updated.0["name"], name.as_str(), "omitted fields keep their value");

    tools_campaigns::handle_campaign_delete(
        &state,
        &tools_campaigns::CampaignDeleteInput { campaign_id: id },
    )
    .await
    .expect("delete");
}

#[tokio::test]
async fn parity_tools_reject_malformed_ids_before_touching_the_db() {
    let state = state().await;

    let err = tools_campaigns::handle_campaign_update(
        &state,
        &tools_campaigns::CampaignUpdateInput {
            campaign_id: "not-a-uuid".into(),
            name: None,
            description: None,
            color: None,
            start_date: None,
            end_date: None,
            goal: None,
            status: None,
            progress_metric: None,
            progress_target: None,
            audience_persona: None,
            content_pillars: None,
            budget_cents: None,
            kpi_targets: None,
        },
    )
    .await
    .err().expect("bad campaign id");
    assert!(err.contains("campaign_id must be a UUID"), "got: {err}");

    let err = tools_sets::handle_set_create(
        &state,
        &tools_sets::SetCreateInput {
            name: "t".into(),
            description: None,
            content: serde_json::json!({}),
            channel_ids: vec!["nope".into()],
        },
    )
    .await
    .err().expect("bad channel id");
    assert!(err.contains("channel_ids must be a UUID"), "got: {err}");

    let err = tools_rss::handle_rss_feed_items(
        &state,
        &tools_rss::RssFeedIdInput { feed_id: "nope".into() },
    )
    .await
    .err().expect("bad feed id");
    assert!(err.contains("feed_id must be a UUID"), "got: {err}");
}

#[tokio::test]
async fn streak_tool_reports_the_streak_fields() {
    let state = state().await;
    let out = tools_analytics::handle_streak_get(&state, &tools_analytics::StreakGetInput {})
        .await
        .expect("streak");
    assert!(out.0["streak_days"].is_number(), "streak_days present: {}", out.0);
    assert!(out.0.get("streak_since").is_some(), "streak_since present");
}

// Regression: these queries used Postgres-only SQL (`NOW()`, `make_interval`)
// and failed at runtime on SQLite. They must execute against the real schema.
#[tokio::test]
async fn admin_analytics_and_integration_toggle_run_on_sqlite() {
    use social_forge::mcp::tools_admin;

    let state = state().await;

    let summary = tools_admin::handle_analytics_summary(
        &state,
        &tools_admin::AnalyticsSummaryInput { days: Some(7) },
    )
    .await;
    assert!(summary.is_ok(), "analytics summary failed: {:?}", summary.err());

    let feed = tools_admin::handle_feed_analytics(
        &state,
        &tools_admin::FeedAnalyticsInput { days: Some(7) },
    )
    .await;
    assert!(feed.is_ok(), "feed analytics failed: {:?}", feed.err());

    // An unknown integration id is a clean "no row" result, not a SQL error.
    let matched = db::queries::set_integration_disabled(
        &state.db,
        social_forge::auth::middleware::DEFAULT_USER_ID,
        uuid::Uuid::new_v4(),
        true,
    )
    .await
    .expect("toggle SQL runs on sqlite");
    assert!(!matched);
}
