use std::path::Path;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::warn;

use crate::wa::WhaClient;

/// A simplified chat summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatSummary {
    pub jid: String,
    pub name: String,
    pub unread_count: u32,
    pub last_message_text: Option<String>,
}

/// A simplified contact entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ContactEntry {
    pub jid: String,
    pub name: String,
    pub push_name: String,
}

/// Open (or create) the meta database in `store_dir` and ensure schema tables.
pub fn ensure_meta_db(store_dir: &Path) -> anyhow::Result<rusqlite::Connection> {
    let db_path = store_dir.join("wa-meta.db");
    let db = rusqlite::Connection::open(&db_path)?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS chat_summaries (
            jid TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            unread_count INTEGER NOT NULL DEFAULT 0,
            last_message_text TEXT
        );
        CREATE TABLE IF NOT EXISTS contact_entries (
            jid TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            push_name TEXT NOT NULL DEFAULT ''
        );",
    )?;
    Ok(db)
}

/// Internal: query chat summaries from an already-opened meta DB connection.
fn query_chats(db: &rusqlite::Connection, limit: i64) -> anyhow::Result<Vec<ChatSummary>> {
    let mut stmt = db.prepare(
        "SELECT jid, name, unread_count, last_message_text \
         FROM chat_summaries ORDER BY name LIMIT ?1",
    )?;

    let rows = stmt.query_map([limit], |row| {
        Ok(ChatSummary {
            jid: row.get(0)?,
            name: row.get(1)?,
            unread_count: row.get::<_, i32>(2)? as u32,
            last_message_text: row.get(3)?,
        })
    })?;

    let results: Vec<ChatSummary> = rows.collect::<Result<_, _>>()?;

    if results.is_empty() {
        warn!("No chats found in wa-meta.db");
    }

    Ok(results)
}

/// Internal: query contact entries from an already-opened meta DB connection.
fn query_contacts(db: &rusqlite::Connection, limit: i64) -> anyhow::Result<Vec<ContactEntry>> {
    let mut stmt = db.prepare(
        "SELECT jid, name, push_name \
         FROM contact_entries ORDER BY name LIMIT ?1",
    )?;

    let rows = stmt.query_map([limit], |row| {
        Ok(ContactEntry {
            jid: row.get(0)?,
            name: row.get(1)?,
            push_name: row.get(2)?,
        })
    })?;

    let results: Vec<ContactEntry> = rows.collect::<Result<_, _>>()?;

    if results.is_empty() {
        warn!("No contacts found in wa-meta.db");
    }

    Ok(results)
}

/// List chats from the local wa-meta.db SQLite store.
///
/// If no data has been synced yet (e.g. wa-rs event loop hasn't run), this
/// returns an empty `Vec`.  Use `limit` to cap results (default 100).
pub async fn list_chats(
    client: &Arc<Mutex<WhaClient>>,
    limit: Option<u32>,
) -> anyhow::Result<Vec<ChatSummary>> {
    let store_dir = {
        let locked = client.lock().await;
        locked.store_dir().clone()
    };
    list_chats_store(&store_dir, limit)
}

/// Query chats directly from a store directory (bypasses WhaClient).
///
/// Useful for tests and for use when the store path is known independently.
pub fn list_chats_store(store_dir: &Path, limit: Option<u32>) -> anyhow::Result<Vec<ChatSummary>> {
    let lim = limit.unwrap_or(100) as i64;
    let db = ensure_meta_db(store_dir)?;
    query_chats(&db, lim)
}

/// List contacts from the local wa-meta.db SQLite store.
///
/// Like [`list_chats`], returns empty `Vec` if no data has been synced.
pub async fn list_contacts(
    client: &Arc<Mutex<WhaClient>>,
    limit: Option<u32>,
) -> anyhow::Result<Vec<ContactEntry>> {
    let store_dir = {
        let locked = client.lock().await;
        locked.store_dir().clone()
    };
    list_contacts_store(&store_dir, limit)
}

/// Query contacts directly from a store directory (bypasses WhaClient).
///
/// Useful for tests and for use when the store path is known independently.
pub fn list_contacts_store(
    store_dir: &Path,
    limit: Option<u32>,
) -> anyhow::Result<Vec<ContactEntry>> {
    let lim = limit.unwrap_or(100) as i64;
    let db = ensure_meta_db(store_dir)?;
    query_contacts(&db, lim)
}
