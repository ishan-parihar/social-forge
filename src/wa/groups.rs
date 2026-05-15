use std::sync::Arc;

use anyhow::Context;
use tokio::sync::Mutex;

use wa_rs::Jid;

use crate::wa::WhaClient;

/// List all groups the authenticated user participates in.
///
/// Returns the group JIDs (e.g. `1234567890-123456@g.us`) as `String`s.
///
/// # Errors
///
/// - wa-rs store query error (e.g. missing session data).
pub async fn list_groups(
    client: &Arc<Mutex<WhaClient>>,
) -> anyhow::Result<Vec<String>> {
    let locked = client.lock().await;
    let wa = locked.inner();
    let participating = wa
        .groups()
        .get_participating()
        .await
        .context("wa-rs get_participating failed")?;
    Ok(participating.into_keys().collect())
}

/// Create a new WhatsApp group with the given subject and participants.
///
/// `participants` must contain at least one entry (the group cannot be empty).
/// Each entry should be a phone number without `+` or special characters
/// (it will be converted via [`Jid::pn`]).
///
/// # Errors
///
/// - Empty `participants` list is rejected.
/// - wa-rs group creation failure (network / invalid JID).
pub async fn create_group(
    client: &Arc<Mutex<WhaClient>>,
    subject: &str,
    participants: &[String],
) -> anyhow::Result<wa_rs::CreateGroupResult> {
    if participants.is_empty() {
        anyhow::bail!("create_group requires at least one participant");
    }
    if subject.trim().is_empty() {
        anyhow::bail!("create_group requires a non-empty subject");
    }

    let locked = client.lock().await;
    let wa = locked.inner();
    let groups = wa.groups();

    let participant_opts: Vec<wa_rs::GroupParticipantOptions> = participants
        .iter()
        .map(|p| wa_rs::GroupParticipantOptions::new(Jid::pn(p.as_str())))
        .collect();

    let opts = wa_rs::GroupCreateOptions {
        subject: subject.to_string(),
        participants: participant_opts,
        ..Default::default()
    };

    let result = groups
        .create_group(opts)
        .await
        .context("wa-rs create_group failed")?;
    Ok(result)
}

/// Obtain an invite link for an existing group.
///
/// The link format is `https://chat.whatsapp.com/<code>` and can be shared
/// with anyone.
///
/// # Errors
///
/// - wa-rs invite link retrieval failure (network / permissions).
pub async fn get_group_invite_link(
    client: &Arc<Mutex<WhaClient>>,
    group_jid: &Jid,
) -> anyhow::Result<String> {
    let locked = client.lock().await;
    let wa = locked.inner();
    let code = wa
        .groups()
        .get_invite_link(group_jid, false)
        .await
        .with_context(|| format!("wa-rs get_invite_link failed for {group_jid}"))?;
    Ok(code)
}


