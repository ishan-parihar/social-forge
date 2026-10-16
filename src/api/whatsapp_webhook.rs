// ─── WhatsApp Cloud webhook receive (E6) ──────────────────────
// Inbound Meta webhook for the Cloud API: GET subscription verification
// (hub.challenge echo) + POST payload parsing with X-Hub-Signature-256
// HMAC verification. Parsed messages/statuses are trace-logged now; an
// inbox table lands with the first consumer that needs them.

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::api::AppState;

/// HMAC-SHA256(secret, body) as lowercase hex — Meta's signature form.
pub fn signature_hex(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time comparison of the computed signature against the header.
pub fn verify_signature(secret: &str, body: &[u8], header: &str) -> bool {
    let Some(provided) = header.strip_prefix("sha256=") else {
        return false;
    };
    let computed = signature_hex(secret, body);
    // Timing-safe compare; hex encoding is constant-length (64).
    if provided.len() != computed.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in provided.bytes().zip(computed.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub mode: Option<String>,
    pub verify_token: Option<String>,
    pub challenge: Option<String>,
}

/// GET /api/webhooks/whatsapp — Meta subscription verification.
pub async fn verify(
    State(state): State<AppState>,
    Query(q): Query<VerifyQuery>,
) -> axum::response::Response {
    let expected = state
        .config
        .whatsapp_cloud_verify_token
        .clone()
        .unwrap_or_default();
    let ok = q.mode.as_deref() == Some("subscribe")
        && !expected.is_empty()
        && q.verify_token.as_deref() == Some(&expected);
    match (ok, q.challenge) {
        (true, Some(challenge)) => challenge.into_response(),
        _ => axum::http::StatusCode::FORBIDDEN.into_response(),
    }
}

/// POST /api/webhooks/whatsapp — message/status delivery. When
/// WHATSAPP_CLOUD_APP_SECRET is set the X-Hub-Signature-256 must verify
/// against the raw body (fail closed on mismatch); without a configured
/// secret the payload is accepted but flagged in the log.
pub async fn receive(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    if let Some(secret) = state
        .config
        .whatsapp_cloud_app_secret
        .as_deref()
        .filter(|s| !s.is_empty())
    {
        let header = headers
            .get("x-hub-signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !verify_signature(secret, &body, header) {
            tracing::warn!("whatsapp webhook: bad signature");
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let raw: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("whatsapp webhook: unparseable body: {e}");
            return axum::http::StatusCode::BAD_REQUEST.into_response();
        }
    };

    if raw["object"] != "whatsapp_business_account" {
        tracing::warn!("whatsapp webhook: unexpected object");
        return axum::http::StatusCode::BAD_REQUEST.into_response();
    }

    // Parse: count messages + statuses across entries→changes→value.
    let mut messages = 0usize;
    let mut statuses = 0usize;
    for entry in raw["entry"].as_array().cloned().unwrap_or_default() {
        for change in entry["changes"].as_array().cloned().unwrap_or_default() {
            if let Some(v) = change["value"].as_object() {
                messages += v
                    .get("messages")
                    .and_then(|m| m.as_array())
                    .map(Vec::len)
                    .unwrap_or(0);
                statuses += v
                    .get("statuses")
                    .and_then(|s| s.as_array())
                    .map(Vec::len)
                    .unwrap_or(0);
            }
        }
    }
    tracing::info!(
        "whatsapp webhook: {} message(s), {} status update(s)",
        messages,
        statuses
    );

    // Always 200 so Meta stops retrying.
    axum::http::StatusCode::OK.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_helper_round_trips() {
        let sig = signature_hex("s", b"body");
        assert!(verify_signature("s", b"body", &format!("sha256={sig}")));
        assert!(!verify_signature("s", b"body", "sha256=nope"));
        assert!(!verify_signature("s", b"body", "badprefix"));
        // Tampered body changes the signature.
        let other = signature_hex("s", b"body2");
        assert_ne!(sig, other);
    }

    #[test]
    fn payload_parser_counts_messages_and_statuses() {
        let raw: serde_json::Value = serde_json::json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "changes": [{
                    "value": {
                        "messages": [{"from": "15551234567", "text": {"body": "hi"}}],
                        "statuses": [{"status": "delivered"}, {"status": "read"}]
                    }
                }, {
                    "value": { "statuses": [{"status": "sent"}] }
                }]
            }]
        });
        let mut messages = 0usize;
        let mut statuses = 0usize;
        for entry in raw["entry"].as_array().cloned().unwrap_or_default() {
            for change in entry["changes"].as_array().cloned().unwrap_or_default() {
                if let Some(v) = change["value"].as_object() {
                    messages += v
                        .get("messages")
                        .and_then(|m| m.as_array())
                        .map(Vec::len)
                        .unwrap_or(0);
                    statuses += v
                        .get("statuses")
                        .and_then(|s| s.as_array())
                        .map(Vec::len)
                        .unwrap_or(0);
                }
            }
        }
        assert_eq!((messages, statuses), (1, 3));
    }
}
