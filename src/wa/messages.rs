use std::sync::Arc;

use anyhow::Context;
use tokio::sync::Mutex;
use tracing::warn;
use wa_rs::Jid;

use crate::wa::WhaClient;
use wa_rs::wa_rs_proto;

/// Build a wa-rs `Message` containing only a `conversation` text field.
///
/// Returns `None` when `text` is empty (WhatsApp does not accept empty
/// conversation messages).
fn text_message(text: &str) -> Option<wa_rs_proto::whatsapp::Message> {
    if text.trim().is_empty() {
        warn!("attempted to send empty WhatsApp message — skipped");
        return None;
    }
    let mut msg = wa_rs_proto::whatsapp::Message::default();
    msg.conversation = Some(text.to_owned());
    Some(msg)
}

/// Send a plain-text message to a WhatsApp contact or group.
///
/// `to` is a [`Jid`] (use [`wa_rs::Jid::pn`] for phone numbers or
/// `Jid::from_str` for group JIDs).  Returns the server-assigned message ID.
///
/// # Errors
///
/// - Empty `text` is silently ignored (returns a zero-length ID).
/// - wa-rs network or send-failure error is forwarded.
pub async fn send_text(
    client: &Arc<Mutex<WhaClient>>,
    to: &Jid,
    text: &str,
) -> anyhow::Result<String> {
    let msg = text_message(text).ok_or_else(|| anyhow::anyhow!("text is empty"))?;
    let locked = client.lock().await;
    let wa = locked.inner();
    let msg_id = wa
        .send_message(to.clone(), msg)
        .await
        .with_context(|| format!("wa-rs send_message failed to {to}"))?;
    Ok(msg_id)
}

/// Edit a previously-sent message (WhatsApp supports edits within ~48 h).
///
/// `original_id` is the message ID returned by [`send_text`].  `new_text`
/// replaces the full conversation text.
///
/// # Errors
///
/// - Empty `new_text` is rejected.
/// - wa-rs network or edit-failure error is forwarded.
pub async fn edit_message(
    client: &Arc<Mutex<WhaClient>>,
    to: &Jid,
    original_id: &str,
    new_text: &str,
) -> anyhow::Result<String> {
    if new_text.trim().is_empty() {
        anyhow::bail!("edit_message requires non-empty text");
    }
    let msg = text_message(new_text).ok_or_else(|| anyhow::anyhow!("new_text is empty"))?;
    let locked = client.lock().await;
    let wa = locked.inner();
    let msg_id = wa
        .edit_message(to.clone(), original_id, msg)
        .await
        .with_context(|| format!("wa-rs edit_message failed for {original_id}"))?;
    Ok(msg_id)
}

/// Revoke (delete for everyone) a message that was sent by this client.
///
/// Uses [`wa_rs::RevokeType::Sender`], which removes the message for all
/// chat participants.
///
/// # Errors
///
/// - wa-rs network or revoke-failure error is forwarded.
/// - May fail if the message was sent more than ~48 h ago (WhatsApp
///   server-enforced limit).
pub async fn revoke_message(
    client: &Arc<Mutex<WhaClient>>,
    to: &Jid,
    message_id: &str,
) -> anyhow::Result<()> {
    let locked = client.lock().await;
    let wa = locked.inner();
    wa.revoke_message(to.clone(), message_id, wa_rs::RevokeType::Sender)
        .await
        .with_context(|| format!("wa-rs revoke_message failed for {message_id}"))?;
    Ok(())
}
