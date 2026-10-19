// ─── Activity Log API Routes ────────────────────────────────
// Read-only, cursor-paginated agent activity feed.
//
// Three append-only tables record "what the app did on the operator's
// behalf", and none of them is surfaced as a single timeline:
//
//   events_log       — publish / failed / comment events persisted by
//                      `Broadcaster::send_and_log` (src/realtime/mod.rs)
//   automation_logs  — one row per automation-rule execution
//   publish_attempts — one row per provider.publish() call
//
// This module merges them into one newest-first feed. It is strictly
// READ-ONLY: there is no POST/PUT/DELETE handler here, and every row is
// scoped to the caller's own `user_id` — `events_log` carries `user_id`
// directly, and the other two reach their owner through an FK join, which
// is the ownership filter and not an optimisation.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;

use super::AppState;

/// Rows returned per source per page. The three scans each return at most
/// this many, and the merged result is truncated to the same cap, so the
/// page is bounded regardless of how the sources are sized relative to
/// each other.
const PAGE_SIZE: i64 = 50;

// ── Query Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    /// `all` (default) | `event` | `automation` | `publish`
    #[serde(default = "default_kind")]
    pub kind: String,
    /// `24h` (default) | `7d` | `30d` — window ending at now.
    #[serde(default = "default_range")]
    pub range: String,
    /// Omit or `all` for no provider filter.
    pub provider: Option<String>,
    /// Cursor: return only rows strictly older than this epoch second.
    /// Absent means "newest page".
    pub before: Option<i64>,
}

fn default_kind() -> String {
    "all".to_string()
}

fn default_range() -> String {
    "24h".to_string()
}

impl ActivityQuery {
    /// Window length in seconds. An unknown value falls back to the default
    /// rather than erroring: a typo in a URL should show the last day, not
    /// a 400.
    fn window_secs(&self) -> i64 {
        match self.range.as_str() {
            "7d" => 7 * 86_400,
            "30d" => 30 * 86_400,
            _ => 86_400,
        }
    }

    /// `None` means "no kind filter".
    fn kind_filter(&self) -> Option<&str> {
        match self.kind.as_str() {
            "all" | "" => None,
            other => Some(other),
        }
    }
}

// ── Response Types ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ActivityRow {
    /// `<kind>:<source-id>`, so the frontend has a stable key across pages.
    pub id: String,
    /// `event` | `automation` | `publish`
    pub kind: String,
    /// Epoch seconds (UTC).
    pub at: i64,
    /// Human sentence, rendered server-side.
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Whether the underlying action succeeded.
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct ActivityResponse {
    pub rows: Vec<ActivityRow>,
    /// Epoch seconds of the oldest returned row; pass as `before` for the
    /// next page. `null` when there is nothing further back.
    pub next_before: Option<i64>,
}

// ── Raw row shapes ───────────────────────────────────────────
//
// All three columns are TEXT/INTEGER in SQLite, and the runtime
// `sqlx::query_as` cannot apply the macro's `"col: EpochUtc"` override
// (that syntax exists only inside `query!`). So the epoch column is
// decoded as a plain i64 and the payload as a String; the handler does the
// conversion.

// Every `id` here is declared TEXT in the schema, but every writer binds a
// `Uuid`, and sqlx encodes that as a 16-byte BLOB in SQLite. Decoding the
// column as `String` therefore fails at runtime on real rows (the table is
// TEXT-affinity, so an inserted blob stays a blob), so the raw structs read
// `Uuid` and the handler renders it as its canonical string form.
#[derive(Debug, sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    event_type: String,
    payload: String,
    /// Epoch seconds.
    created_at: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AutomationRow {
    id: Uuid,
    rule_id: Uuid,
    trigger_type: String,
    status: String,
    error_message: Option<String>,
    created_at: i64,
    /// Provider of the integration the rule acts on, via the join.
    provider: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PublishRow {
    id: Uuid,
    post_id: Uuid,
    attempt_number: Option<i64>,
    status: String,
    error_message: Option<String>,
    created_at: i64,
    integration_id: Uuid,
    provider: String,
}

// ── Summary sentences ────────────────────────────────────────
//
// `events_log` rows are JSON payloads written by the emitting call site
// (the scheduler, mostly). Read the common keys and fall back to a bare
// event name, so an event type added later degrades to a readable
// sentence instead of a `{}` blob.

/// Pull a non-empty string out of a JSON payload.
fn str_at<'a>(payload: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

/// The one sentence for one `events_log` row.
///
/// This is the SINGLE OWNER of the event-to-sentence mapping. The
/// analytics page's "Recent activity" widget renders the same rows, so it
/// consumes this function's output (via `/api/events/recent`, which now
/// returns a `summary` field) instead of carrying a second copy of the
/// map that drifts.
pub fn event_summary(event_type: &str, payload: &serde_json::Value) -> String {
    let title = str_at(payload, "title")
        .or_else(|| str_at(payload, "content"))
        .or_else(|| str_at(payload, "post_title"));
    let provider = str_at(payload, "provider");

    match event_type {
        "post_published" => match (title, provider) {
            (Some(t), Some(p)) => format!("Published “{t}” to {}", channel_label(p)),
            (Some(t), None) => format!("Published “{t}”"),
            (None, Some(p)) => format!("Published a post to {}", channel_label(p)),
            (None, None) => "Published a post".into(),
        },
        "post_failed" => match (str_at(payload, "error"), provider) {
            // Name the channel. The failure payload carries the provider
            // now, so the log says WHICH channel failed instead of the
            // "Failed on a channel" the frontend's own fallback
            // produced.
            (Some(e), Some(p)) => format!("Publish failed on {}: {e}", channel_label(p)),
            (Some(e), None) => format!("Publish failed: {e}"),
            (None, Some(p)) => format!("Publish failed on {}", channel_label(p)),
            (None, None) => "Publish failed".into(),
        },
        "post_scheduled" => match (title, provider) {
            (Some(t), Some(p)) => format!("Scheduled “{t}” for {}", channel_label(p)),
            (Some(t), None) => format!("Scheduled “{t}”"),
            (None, Some(p)) => format!("Scheduled a post for {}", channel_label(p)),
            (None, None) => "Scheduled a post".into(),
        },
        "post_created" => match provider {
            Some(p) => format!("Draft created for {}", channel_label(p)),
            None => "Draft created".into(),
        },
        "post_deleted" => "Post deleted".into(),
        "post_stage_changed" => "Post moved back to drafts".into(),
        "integration_connected" => match str_at(payload, "name").or(provider) {
            Some(p) => format!("{} connected", channel_label(p)),
            None => "A channel connected".into(),
        },
        "integration_disconnected" => match str_at(payload, "name").or(provider) {
            Some(p) => format!("{} disconnected", channel_label(p)),
            None => "A channel disconnected".into(),
        },
        "first_comment_failed" => "First comment could not be posted".into(),
        other => {
            // Colons are dropped so a `post.published` namespace reads as
            // words rather than a half-rendered log line.
            let name = other.replace(['_', '.'], " ");
            format!("Event: {name}")
        }
    }
}

/// A provider id rendered for a human.
///
/// The provider registry is not reachable from a pure function, and the
/// ids the emitters write are the registry's keys (`x`, `linkedin`,
/// `bluesky`). Capitalising them gives the same word a reader expects
/// without a second copy of the display-name table.
fn channel_label(provider: &str) -> String {
    let p = provider.trim();
    if p.is_empty() {
        return "a channel".into();
    }
    // A payload that already carries a display name ("X (Twitter)") is
    // used verbatim.
    if p.chars().any(|c| c.is_uppercase()) {
        return p.to_string();
    }
    let mut out = String::with_capacity(p.len());
    for (i, ch) in p.chars().enumerate() {
        if i == 0 {
            out.extend(ch.to_uppercase());
        } else if ch == '_' || ch == '-' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn automation_summary(trigger_type: &str, status: &str, error: Option<&str>) -> String {
    match status {
        "sent" => format!("Automation rule replied to a {trigger_type}"),
        "failed" => match error.filter(|e| !e.is_empty()) {
            Some(e) => format!("Automation failed on a {trigger_type}: {e}"),
            None => format!("Automation failed on a {trigger_type}"),
        },
        "skipped_cooldown" => format!("Automation skipped a {trigger_type} (cooldown)"),
        "skipped_limit" => format!("Automation skipped a {trigger_type} (hourly limit)"),
        _ => format!("Automation {status} on a {trigger_type}"),
    }
}

fn publish_summary(attempt_number: i64, status: &str, error: Option<&str>) -> String {
    // `attempt_number` is NOT NULL in the schema; the guard only keeps a
    // hand-seeded test row from rendering "attempt 0"s as "attempt 0".
    let n = attempt_number.max(1);
    match status {
        "success" => format!("Publish attempt {n} succeeded"),
        "failed" => match error.filter(|e| !e.is_empty()) {
            Some(e) => format!("Publish attempt {n} failed: {e}"),
            None => format!("Publish attempt {n} failed"),
        },
        _ => format!("Publish attempt {n} is {status}"),
    }
}

// ── Handlers ─────────────────────────────────────────────────

/// GET /api/activity?kind=&range=&provider=&before=
///
/// Read-only merge of `events_log`, `automation_logs` and
/// `publish_attempts`, newest first. See [`ActivityRow`] for the shape.
pub async fn list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<ActivityResponse>, AppError> {
    let kind_filter = query.kind_filter();
    let provider_filter = query
        .provider
        .as_deref()
        .filter(|p| !p.is_empty() && *p != "all");

    // Upper bound for the cursor. Without a cursor it is positive infinity,
    // so every row is eligible and the range window alone decides.
    //
    // The cursor is EXCLUSIVE: the queries filter `created_at <= ?`, and
    // `next_before` is the OLDEST row's second, so passing it straight
    // back re-included that row on the next page. The frontend appends
    // pages into a `{#each rows as row (row.id)}` keyed list, so the
    // repeated id threw `each_key_duplicate` and the Activity page
    // CRASHED on "Load more". The cursor is one second earlier, so the
    // boundary row is never returned twice.
    let cursor = query.before.unwrap_or(i64::MAX).saturating_sub(1);
    let floor = epoch_now() - query.window_secs();

    let mut rows: Vec<ActivityRow> = Vec::new();

    // ── events_log ──
    // Scoped by `user_id` directly. `payload` is stored as TEXT and bound
    // back out as a String; the runtime `query_as` cannot apply the macro's
    // `"col: Type"` override, so the epoch column is decoded as an i64 here.
    if kind_filter.is_none_or(|k| k == "event") {
        let event_rows: Vec<EventRow> = sqlx::query_as(
            r#"SELECT id, event_type, payload, created_at
               FROM events_log
               WHERE user_id = ? AND created_at <= ? AND created_at >= ?
               ORDER BY created_at DESC, id DESC
               LIMIT ?"#,
        )
        .bind(auth.user_id)
        .bind(cursor)
        .bind(floor)
        .bind(PAGE_SIZE)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read activity: {e}")))?;

        for e in event_rows {
            let payload: serde_json::Value =
                serde_json::from_str(&e.payload).unwrap_or(serde_json::Value::Null);
            rows.push(ActivityRow {
                id: format!("event:{id}", id = e.id),
                kind: "event".into(),
                at: e.created_at,
                summary: event_summary(&e.event_type, &payload),
                post_id: str_at(&payload, "post_id").map(String::from),
                rule_id: None,
                integration_id: str_at(&payload, "integration_id").map(String::from),
                provider: str_at(&payload, "provider").map(String::from),
                // The scheduler logs both branches, so the type itself is
                // the answer; an unmapped type is reported as ok.
                ok: !matches!(e.event_type.as_str(), "post_failed" | "first_comment_failed"),
            });
        }
    }

    // ── automation_logs ──
    // No `user_id` column: the row reaches the caller only through the
    // `automation_rules` FK, so that join is the ownership filter. The
    // `integrations` join brings the provider in at no extra cost.
    if kind_filter.is_none_or(|k| k == "automation") {
        let auto_rows: Vec<AutomationRow> = sqlx::query_as(
            r#"SELECT l.id, l.rule_id, l.trigger_type, l.status, l.error_message,
                      l.created_at, i.provider_identifier AS provider
               FROM automation_logs l
               JOIN automation_rules r ON r.id = l.rule_id
               JOIN integrations i ON i.id = r.integration_id
               WHERE r.user_id = ? AND l.created_at <= ? AND l.created_at >= ?
               ORDER BY l.created_at DESC, l.id DESC
               LIMIT ?"#,
        )
        .bind(auth.user_id)
        .bind(cursor)
        .bind(floor)
        .bind(PAGE_SIZE)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read activity: {e}")))?;

        for a in auto_rows {
            rows.push(ActivityRow {
                id: format!("automation:{id}", id = a.id),
                kind: "automation".into(),
                at: a.created_at,
                summary: automation_summary(
                    &a.trigger_type,
                    &a.status,
                    a.error_message.as_deref(),
                ),
                post_id: None,
                rule_id: Some(a.rule_id.to_string()),
                integration_id: None,
                provider: Some(a.provider),
                ok: a.status == "sent",
            });
        }
    }

    // ── publish_attempts ──
    // Same ownership shape as automation: `posts` carries the user and the
    // integration, and `integrations` carries the provider.
    if kind_filter.is_none_or(|k| k == "publish") {
        let pub_rows: Vec<PublishRow> = sqlx::query_as(
            r#"SELECT pa.id, pa.post_id, pa.attempt_number, pa.status, pa.error_message,
                      pa.created_at, i.id AS integration_id, i.provider_identifier AS provider
               FROM publish_attempts pa
               JOIN posts p ON p.id = pa.post_id
               JOIN integrations i ON i.id = p.integration_id
               WHERE p.user_id = ? AND pa.created_at <= ? AND pa.created_at >= ?
               ORDER BY pa.created_at DESC, pa.id DESC
               LIMIT ?"#,
        )
        .bind(auth.user_id)
        .bind(cursor)
        .bind(floor)
        .bind(PAGE_SIZE)
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read activity: {e}")))?;

        for pa in pub_rows {
            rows.push(ActivityRow {
                id: format!("publish:{id}", id = pa.id),
                kind: "publish".into(),
                at: pa.created_at,
                summary: publish_summary(
                    pa.attempt_number.unwrap_or(1),
                    &pa.status,
                    pa.error_message.as_deref(),
                ),
                post_id: Some(pa.post_id.to_string()),
                rule_id: None,
                integration_id: Some(pa.integration_id.to_string()),
                provider: Some(pa.provider),
                ok: pa.status == "success",
            });
        }
    }

    rows.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| b.id.cmp(&a.id)));

    if let Some(p) = provider_filter {
        rows.retain(|r| r.provider.as_deref() == Some(p));
    }

    rows.truncate(PAGE_SIZE as usize);

    // The cursor for the next page: one second before the oldest row
    // returned, because the queries filter `created_at <= cursor`. The
    // row AT `r.at` must not come back.
    let next_before = rows.last().map(|r| r.at.saturating_sub(1));

    Ok(Json(ActivityResponse { rows, next_before }))
}

/// `unixepoch()` in Rust, so the range window's lower bound is computed
/// before the first query rather than after.
fn epoch_now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::event_summary;
    use serde_json::json;

    #[test]
    fn published_sentence_names_the_channel() {
        let p = json!({"title": "Launch", "provider": "x"});
        assert_eq!(event_summary("post_published", &p), "Published “Launch” to X");
    }

    #[test]
    fn every_persisted_event_type_has_a_handled_sentence() {
        // The scheduler persists exactly these three types
        // (`send_and_log` in src/scheduler/mod.rs). If one is added there
        // without a case here, the sentence degrades to "Event: ..." and
        // the dashboard shows a log line instead of a sentence.
        for t in ["post_published", "post_failed", "first_comment_failed"] {
            let s = event_summary(t, &json!({"title": "T", "provider": "x"}));
            assert!(!s.starts_with("Event: "), "{t} has no handled sentence: {s}");
        }
    }

    #[test]
    fn unknown_event_type_degrades_to_words_not_a_log_line() {
        let s = event_summary("post.created", &json!({}));
        assert_eq!(s, "Event: post created");
    }

    #[test]
    fn channel_label_keeps_a_real_display_name_verbatim() {
        let p = json!({"provider": "X (Twitter)"});
        assert_eq!(
            event_summary("integration_connected", &p),
            "X (Twitter) connected"
        );
    }
}
