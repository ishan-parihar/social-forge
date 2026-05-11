// ─── Webhook Dispatcher Service ─────────────────────────────────
// Sends outbound webhook events asynchronously with HMAC signing.
// Records delivery results in the webhook_deliveries table.

use chrono::Utc;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::api::AppState;

/// Compute HMAC-SHA256 manually using sha2 crate.
/// Uses the standard HMAC construction: HMAC(K, m) = H((K' ⊕ opad) || H((K' ⊕ ipad) || m))
fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;

    let key = if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    } else {
        key.to_vec()
    };

    // Pad key to block size
    let mut key_padded = key.clone();
    key_padded.resize(BLOCK_SIZE, 0);

    // Inner hash: H((K ⊕ ipad) || message)
    let mut inner = Vec::with_capacity(BLOCK_SIZE + message.len());
    for &k in &key_padded {
        inner.push(k ^ 0x36);
    }
    inner.extend_from_slice(message);
    let inner_hash = Sha256::digest(&inner);

    // Outer hash: H((K ⊕ opad) || inner_hash)
    let mut outer = Vec::with_capacity(BLOCK_SIZE + 32);
    for &k in &key_padded {
        outer.push(k ^ 0x5c);
    }
    outer.extend_from_slice(&inner_hash);
    Sha256::digest(&outer).to_vec()
}

/// Send a webhook HTTP POST with optional HMAC-SHA256 signature.
/// Returns (status_code, response_body).
pub async fn send_webhook(
    url: &str,
    secret: Option<&str>,
    event_type: &str,
    payload: &Value,
) -> Result<(u16, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let body = serde_json::to_string(payload)
        .map_err(|e| format!("Failed to serialize payload: {e}"))?;

    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", event_type);

    if let Some(secret_key) = secret {
        let signature = hmac_sha256(secret_key.as_bytes(), body.as_bytes());
        let sig_hex = hex::encode(signature);
        request = request.header("X-Webhook-Signature", format!("sha256={sig_hex}"));
    }

    let response = request
        .body(body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let status_code = response.status().as_u16();
    let response_body = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read response body".into());

    Ok((status_code, response_body))
}

/// Dispatch an event to all active webhooks matching the event type for a user.
/// Runs synchronously (caller should spawn in background task if needed).
pub async fn dispatch_event(
    state: &AppState,
    event_type: &str,
    payload: &Value,
    user_id: Uuid,
) {
    let webhooks = match sqlx::query_as::<_, WebhookDispatchRow>(
        r#"
        SELECT id, url, secret, event_types
        FROM webhooks
        WHERE user_id = $1
          AND is_active = true
          AND $2 = ANY(event_types)
        "#,
    )
    .bind(user_id)
    .bind(event_type)
    .fetch_all(&state.db)
    .await
    {
        Ok(wh) => wh,
        Err(e) => {
            tracing::error!("Failed to fetch webhooks for dispatch: {e}");
            return;
        }
    };

    if webhooks.is_empty() {
        return;
    }

    let payload_json = serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({}));

    for wh in webhooks {
        let url = wh.url.clone();
        let secret = wh.secret.clone();
        let webhook_id = wh.id;

        let result = send_webhook(&url, secret.as_deref(), event_type, &payload_json).await;

        let (status, status_code, response_body) = match &result {
            Ok((code, body)) => {
                if *code == 200 || *code == 201 {
                    ("delivered", Some(*code as i32), Some(body.clone()))
                } else {
                    ("failed", Some(*code as i32), Some(body.clone()))
                }
            }
            Err(e) => ("failed", None, Some(e.clone())),
        };

        let delivered_at = if status == "delivered" {
            Some(Utc::now())
        } else {
            None
        };

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (webhook_id, event_type, payload, status, status_code, response_body, delivered_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(webhook_id)
        .bind(event_type)
        .bind(&payload_json)
        .bind(status)
        .bind(status_code)
        .bind(&response_body)
        .bind(delivered_at)
        .execute(&state.db)
        .await
        {
            tracing::error!("Failed to record webhook delivery: {e}");
        }

        if let Err(e) = sqlx::query("UPDATE webhooks SET last_triggered_at = now() WHERE id = $1")
            .bind(webhook_id)
            .execute(&state.db)
            .await
        {
            tracing::error!("Failed to update webhook last_triggered_at: {e}");
        }
    }
}

// ── Internal Row Type ────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct WebhookDispatchRow {
    id: Uuid,
    url: String,
    secret: Option<String>,
    event_types: Vec<String>,
}
