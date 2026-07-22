pub mod tiktok;
pub mod threads;
pub mod discord;
pub mod slack;
pub mod telegram_bot;
pub mod telegram_user;
pub mod whatsapp;
pub mod pinterest;
pub mod github;
pub mod wordpress;
pub mod hashnode;
pub mod medium_blog;
pub mod devto;
pub mod skool;
pub mod google;
pub mod youtube;
pub mod bluesky;
pub mod mastodon;
pub mod x;
pub mod reddit;
pub mod linkedin;
pub mod linkedin_page;
pub mod facebook;
pub mod instagram;
pub mod drive;
pub mod gcal;
pub mod gmail;
pub mod webhooks;
pub mod notifications;
pub mod tags;
pub mod analytics;

use serde::Serialize;
use toon_helper::{self, truncate_json_strings};

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
            println!("{}", toon_helper::format_text(&truncated, "toon"));
            Ok(())
        }
        Err(e) => {
            let err = serde_json::json!({"error": e, "hint": "Run `social-forge doctor` to check provider health."});
            println!("{}", toon_helper::format_text(&err, "toon"));
            std::process::exit(2);
        }
    }
}
