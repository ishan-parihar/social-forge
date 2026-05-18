// ─── Reddit Browser Cookie Extraction ─────────────────────────
// Extracts reddit_session and token_v2 from Chrome, Brave, Firefox, Zen browsers.
// Mirrors the pattern from x_cookies.rs but for reddit.com domain.

use std::path::{Path, PathBuf};

/// Result of browser cookie extraction
#[derive(Debug)]
pub struct ExtractedRedditCookies {
    pub reddit_session: String,
    pub token_v2: Option<String>,
    pub cookie_string: String,
    pub source: String,
}

/// Try to extract Reddit cookies from all known browsers in order
pub fn extract_reddit_cookies() -> Option<ExtractedRedditCookies> {
    let browsers: Vec<(&str, fn() -> Option<ExtractedRedditCookies>)> = vec![
        ("brave", extract_brave),
        ("chrome", extract_chrome),
        ("zen", extract_zen),
        ("firefox", extract_firefox),
    ];

    for (name, extractor) in &browsers {
        if let Some(mut result) = extractor() {
            result.source = name.to_string();
            return Some(result);
        }
    }
    None
}

/// Build the JSON token blob for storage in DB
pub fn build_cookie_token(reddit_session: &str, token_v2: Option<&str>, cookie_string: Option<&str>) -> String {
    let mut m = serde_json::Map::new();
    m.insert("reddit_session".into(), serde_json::Value::String(reddit_session.into()));
    if let Some(t) = token_v2 {
        m.insert("token_v2".into(), serde_json::Value::String(t.into()));
    }
    if let Some(cs) = cookie_string {
        if !cs.is_empty() {
            m.insert("cookie_string".into(), serde_json::Value::String(cs.into()));
        }
    }
    serde_json::Value::Object(m).to_string()
}

/// Parse a cookie token JSON blob back into components
pub fn parse_cookie_token(token: &str) -> Option<(String, Option<String>, Option<String>)> {
    let v: serde_json::Value = serde_json::from_str(token).ok()?;
    let session = v.get("reddit_session")?.as_str()?.to_string();
    if session.is_empty() {
        return None;
    }
    let token_v2 = v.get("token_v2").and_then(|s| s.as_str()).map(String::from);
    let cookie_string = v.get("cookie_string").and_then(|s| s.as_str()).map(String::from);
    Some((session, token_v2, cookie_string))
}

/// Check if a token string is a Reddit cookie auth JSON blob
pub fn is_cookie_auth(token: &str) -> bool {
    parse_cookie_token(token).is_some()
}

/// Parse a raw cookie header string into components
pub fn parse_cookie_string(cookie_str: &str) -> Option<ExtractedRedditCookies> {
    let mut reddit_session = None;
    let mut token_v2 = None;

    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            match k.trim() {
                "reddit_session" => reddit_session = Some(v.trim().to_string()),
                "token_v2" => token_v2 = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }

    let session = reddit_session?;
    Some(ExtractedRedditCookies {
        reddit_session: session,
        token_v2,
        cookie_string: cookie_str.to_string(),
        source: "manual".into(),
    })
}

// ── Helpers ─────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn chrome_default_profile(home: &Path, config_dir: &str) -> PathBuf {
    home.join(".config").join(config_dir).join("Default")
}

// ── Chrome / Brave ─────────────────────────────────────────

fn extract_chrome_impl(profile_dir: &Path, browser: &str) -> Option<ExtractedRedditCookies> {
    let cookie_db = profile_dir.join("Cookies");
    if !cookie_db.exists() {
        return None;
    }

    let tmp_copy = std::env::temp_dir().join(format!("reddit_cookies_{}.db", browser));
    let _ = std::fs::remove_file(&tmp_copy);
    if std::fs::copy(&cookie_db, &tmp_copy).is_err() {
        return None;
    }

    let result = read_chrome_cookies(&tmp_copy);
    let _ = std::fs::remove_file(&tmp_copy);
    result
}

fn read_chrome_cookies(db_path: &Path) -> Option<ExtractedRedditCookies> {
    let conn = rusqlite::Connection::open(db_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT name, value, encrypted_value
             FROM cookies
             WHERE host_key LIKE '%reddit.com'
             AND name IN ('reddit_session', 'token_v2', 'csrf_token', 'loid', 'session_tracker', 'edgebucket')",
        )
        .ok()?;

    let mut reddit_session = String::new();
    let mut token_v2 = String::new();
    let mut all_cookies: Vec<(String, String)> = Vec::new();

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let value: String = row.get(1)?;
        let _encrypted_value: Vec<u8> = row.get(2)?;
        Ok((name, value))
    }).ok()?;

    for row in rows.flatten() {
        let (name, value) = row;
        if value.is_empty() {
            continue;
        }
        match name.as_str() {
            "reddit_session" => reddit_session = value.clone(),
            "token_v2" => token_v2 = value.clone(),
            _ => {}
        }
        all_cookies.push((name, value));
    }

    // Need at least reddit_session OR token_v2
    if reddit_session.is_empty() && token_v2.is_empty() {
        return None;
    }

    let cookie_string = all_cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");

    Some(ExtractedRedditCookies {
        reddit_session,
        token_v2: if token_v2.is_empty() { None } else { Some(token_v2) },
        cookie_string,
        source: String::new(),
    })
}

// ── Firefox / Zen ─────────────────────────────────────────

fn extract_firefox_impl(profile_dir: &Path) -> Option<ExtractedRedditCookies> {
    let cookie_db = profile_dir.join("cookies.sqlite");
    if !cookie_db.exists() {
        return None;
    }

    let tmp_copy = std::env::temp_dir().join("reddit_cookies_firefox.db");
    let _ = std::fs::remove_file(&tmp_copy);
    if std::fs::copy(&cookie_db, &tmp_copy).is_err() {
        return None;
    }

    let conn = rusqlite::Connection::open(&tmp_copy).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT name, value
             FROM moz_cookies
             WHERE baseDomain LIKE '%reddit.com'
             AND name IN ('reddit_session', 'token_v2', 'csrf_token', 'loid', 'session_tracker', 'edgebucket')",
        )
        .ok()?;

    let mut reddit_session = String::new();
    let mut token_v2 = String::new();
    let mut all_cookies: Vec<(String, String)> = Vec::new();

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let value: String = row.get(1)?;
        Ok((name, value))
    }).ok()?;

    for row in rows.flatten() {
        let (name, value) = row;
        if value.is_empty() {
            continue;
        }
        match name.as_str() {
            "reddit_session" => reddit_session = value.clone(),
            "token_v2" => token_v2 = value.clone(),
            _ => {}
        }
        all_cookies.push((name, value));
    }

    let _ = std::fs::remove_file(&tmp_copy);

    if reddit_session.is_empty() && token_v2.is_empty() {
        return None;
    }

    let cookie_string = all_cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");

    Some(ExtractedRedditCookies {
        reddit_session,
        token_v2: if token_v2.is_empty() { None } else { Some(token_v2) },
        cookie_string,
        source: String::new(),
    })
}

// ── Browser-specific entry points ────────────────────────────

fn extract_chrome() -> Option<ExtractedRedditCookies> {
    let home = home_dir();
    extract_chrome_impl(&chrome_default_profile(&home, "google-chrome"), "chrome")
}

fn extract_brave() -> Option<ExtractedRedditCookies> {
    let home = home_dir();
    extract_chrome_impl(&chrome_default_profile(&home, "BraveSoftware/Brave-Browser"), "brave")
}

fn extract_firefox() -> Option<ExtractedRedditCookies> {
    let home = home_dir();
    let mozilla_dir = home.join(".mozilla").join("firefox");
    if let Ok(entries) = std::fs::read_dir(&mozilla_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(result) = extract_firefox_impl(&path) {
                    return Some(result);
                }
            }
        }
    }
    None
}

fn extract_zen() -> Option<ExtractedRedditCookies> {
    let home = home_dir();
    let zen_dir = home.join(".zen");
    if !zen_dir.exists() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(&zen_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(result) = extract_firefox_impl(&path) {
                    return Some(result);
                }
            }
        }
    }
    None
}
