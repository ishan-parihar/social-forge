pub mod analytics;
pub mod bluesky;
pub mod devto;
pub mod discord;
pub mod drive;
pub mod facebook;
pub mod gcal;
pub mod github;
pub mod gmail;
pub mod google;
pub mod hashnode;
pub mod instagram;
pub mod linkedin;
pub mod linkedin_page;
pub mod mastodon;
pub mod medium_blog;
pub mod notifications;
pub mod pinterest;
pub mod reddit;
pub mod skool;
pub mod slack;
pub mod tags;
pub mod telegram_bot;
pub mod telegram_user;
pub mod threads;
pub mod tiktok;
pub mod webhooks;
pub mod whatsapp;
pub mod wordpress;
pub mod x;
pub mod youtube;

use serde::Serialize;

use super::run::{format_text, truncate_json_strings};

/// Maximum chars for string values in list outputs (AXI §3: truncation).
const LIST_TRUNCATE_CHARS: usize = 500;

/// Shared result-printing footer for every CLI platform shim.
///
/// Uses TOON format (AXI §1) with optional truncation for list payloads.
pub fn emit_result<T: Serialize>(result: Result<T, String>) -> anyhow::Result<()> {
    match result {
        Ok(v) => {
            let val = serde_json::to_value(&v).unwrap_or_default();
            // AXI §3: truncate string fields in list-like outputs
            let truncated = truncate_json_strings(&val, LIST_TRUNCATE_CHARS);
            println!("{}", format_text(&truncated, "toon"));
            Ok(())
        }
        Err(e) => {
            let err = serde_json::json!({"error": e, "hint": "Run `social-forge doctor` to check provider health."});
            println!("{}", format_text(&err, "toon"));
            std::process::exit(2);
        }
    }
}
