// Live Telegram user-session parity tests (P6.4). Skip-guarded: the
// suite constructs the same TelegramClientManager the server uses; when
// the stored session is not authenticated (or TELEGRAM_API_ID/HASH are
// absent) the test returns early instead of failing the gate.
//
// ONE test function on purpose: grammers MTProto sessions are not safe
// for concurrent clients — parallel #[tokio::test]s on the same session
// file fight over the update loop and hang. The server runs one manager;
// the test mirrors that.

use std::sync::Arc;

use social_forge::config::Config;
use social_forge::services::telegram_client::TelegramClientManager;

async fn manager() -> Option<Arc<TelegramClientManager>> {
    social_forge::config::load_dotenv_for_live_tests();
    let config = Config::from_env().expect("Failed to load config");
    let (api_id, api_hash) = (config.telegram_api_id.clone(), config.telegram_api_hash.clone());
    let Some(api_id) = api_id.and_then(|v| v.parse::<i32>().ok()).filter(|v| *v != 0) else {
        eprintln!("⏭ skipped: TELEGRAM_API_ID not set");
        return None;
    };
    let api_hash = api_hash.unwrap_or_default();
    if api_hash.is_empty() {
        eprintln!("⏭ skipped: TELEGRAM_API_HASH not set");
        return None;
    }
    let session_dir = config
        .telegram_session_dir
        .clone()
        .unwrap_or_else(|| "data/telegram-sessions".into());
    let mgr = Arc::new(TelegramClientManager::new(api_id, api_hash, session_dir.into()));
    match mgr.is_authenticated().await {
        Ok(true) => Some(mgr),
        Ok(false) => {
            eprintln!("⏭ skipped: Telegram session not authenticated (run tu_request_code/tu_sign_in first)");
            None
        }
        Err(e) => {
            eprintln!("⏭ skipped: Telegram auth check failed: {e}");
            None
        }
    }
}

fn test_peer() -> Option<String> {
    match std::env::var("TELEGRAM_TEST_PEER") {
        Ok(p) if !p.is_empty() => Some(p),
        _ => {
            eprintln!("⏭ skipped: TELEGRAM_TEST_PEER not set (a @username the session can message, or `me`)");
            None
        }
    }
}

#[tokio::test]
async fn test_tu_parity_full_roundtrip() {
    let (Some(mgr), Some(peer)) = (manager().await, test_peer()) else {
        return;
    };

    // ── history + tail (read side) ──────────────────────────────
    let history = mgr
        .fetch_history(&peer, 10, None)
        .await
        .unwrap_or_else(|e| panic!("history failed: {e}"));
    let msgs = history["messages"].as_array().cloned().unwrap_or_default();
    let ids: Vec<i64> = msgs.iter().filter_map(|m| m["id"].as_i64()).collect();
    let sorted = {
        let mut s = ids.clone();
        s.sort_unstable();
        s
    };
    assert_eq!(ids, sorted, "history must be oldest-first");

    let newest = ids.last().copied().unwrap_or(0) as i32;
    let tail = mgr
        .tail_messages(&peer, 5, Some(newest))
        .await
        .unwrap_or_else(|e| panic!("tail failed: {e}"));
    for m in tail["messages"].as_array().cloned().unwrap_or_default() {
        assert!(
            m["id"].as_i64().unwrap_or(0) > newest as i64,
            "tail must only return ids above the after cursor"
        );
    }
    eprintln!("✅ history {} msgs + tail above cursor ok", msgs.len());

    // ── send → edit → delete (write side, own messages) ──────────
    let sent = mgr
        .send_message(&peer, "social-forge P6.4 parity probe")
        .await
        .unwrap_or_else(|e| panic!("send failed: {e}"));
    let id = sent["id"].as_i64().expect("send returned an id") as i32;

    mgr.edit_message(&peer, id, "social-forge P6.4 parity probe (edited)")
        .await
        .unwrap_or_else(|e| panic!("edit failed: {e}"));

    let deleted = mgr
        .delete_message(&peer, id)
        .await
        .unwrap_or_else(|e| panic!("delete failed: {e}"));
    assert!(
        deleted["deleted"].as_u64().unwrap_or(0) >= 1,
        "delete must count"
    );
    eprintln!("✅ send/edit/delete roundtrip on message {id}");

    // ── write queue drains under the limiter ─────────────────────
    mgr.queue_send(&peer, "P6.4 queue probe 1")
        .await
        .unwrap_or_else(|e| panic!("queue_send failed: {e}"));
    mgr.queue_send(&peer, "P6.4 queue probe 2")
        .await
        .unwrap_or_else(|e| panic!("queue_send 2 failed: {e}"));
    assert!(mgr.queue_len().await <= 2, "queue bounded by what we pushed");

    tokio::time::sleep(std::time::Duration::from_secs(12)).await;
    let drained_to = mgr.queue_len().await;
    assert!(
        drained_to <= 1,
        "worker must drain at least one send (1/5s peer pacing), queue={drained_to}"
    );
    eprintln!("✅ write queue drained (remaining: {drained_to})");
}
