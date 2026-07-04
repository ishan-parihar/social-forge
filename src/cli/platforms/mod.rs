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

/// Shared result-printing footer for every CLI platform shim.
///
/// Replaces the 5-line `match result { Ok(v) => println!(...), Err(e) =>
/// { eprintln!(...); std::process::exit(1); } } Ok(())` block that was
/// duplicated across all 22 `cli/platforms/*.rs` files.
pub fn emit_result<T: Serialize>(result: Result<T, String>) -> anyhow::Result<()> {
    match result {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into()));
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", serde_json::json!({"error": e}));
            std::process::exit(1);
        }
    }
}
