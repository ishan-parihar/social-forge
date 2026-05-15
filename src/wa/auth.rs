use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::wa::WhaClient;

/// Options passed to [`pair_with_code`].
pub struct PairOptions {
    /// Full phone number in international format, e.g. `+1234567890`.
    pub phone_number: String,
    /// If `true`, WhatsApp will show a push notification on the device.
    pub show_push_notification: bool,
}

/// Request a 8-character pair code for linking a phone without scanning a QR
/// code.
///
/// The returned `String` is the code displayed to the user.  They open
/// WhatsApp → Linked Devices → Link a Device → enter this code.
///
/// # Prerequisites
///
/// The client must be in a non-authenticated, connected state (WebSocket
/// established but no session).  Call [`WhaClient::connect`] first.
///
/// # Errors
///
/// - `Connection(NotConnected)` — transport is not active
/// - wa-rs internal error while requesting the code
pub async fn pair_with_code(
    client: &Arc<Mutex<WhaClient>>,
    options: PairOptions,
) -> anyhow::Result<String> {
    let locked = client.lock().await;
    let wa = locked.inner();

    if options.phone_number.trim().is_empty() {
        anyhow::bail!("phone_number cannot be empty");
    }

    let pair_opts = wa_rs::pair_code::PairCodeOptions {
        phone_number: options.phone_number,
        show_push_notification: options.show_push_notification,
        custom_code: None,
        ..Default::default()
    };

    let code = wa
        .pair_with_code(pair_opts)
        .await
        .context("wa-rs pair_with_code failed")?;
    Ok(code)
}

/// Poll [`WhaClient::is_authenticated`] up to `timeout` seconds.
///
/// Returns `true` as soon as the user scans the QR code or confirms the pair
/// code.  Returns `false` if the timeout elapses without authentication.
///
/// ## Usage
///
/// ```ignore
/// let ok = wait_for_authentication(&client, Duration::from_secs(120)).await;
/// if !ok {
///     return Err(anyhow::anyhow!("User did not authenticate within 120 s"));
/// }
/// ```
pub async fn wait_for_authentication(
    client: &Arc<Mutex<WhaClient>>,
    timeout: Duration,
) -> bool {
    let start = tokio::time::Instant::now();
    loop {
        {
            let locked = client.lock().await;
            if locked.is_authenticated() {
                info!("WhatsApp Web authentication confirmed");
                return true;
            }
        }
        if start.elapsed() >= timeout {
            warn!("WhatsApp Web authentication timed out after {timeout:?}");
            return false;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
