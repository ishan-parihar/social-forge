// ─── X/Twitter Browser Cookie Extraction ──────────────────────
// Extracts auth_token and ct0 from Chrome, Brave, Firefox, Zen browsers.
// Firefox/Zen: reads plaintext cookies from SQLite store — works.
// Chrome/Brave: cookie values are OS-encrypted (v10/v11); the value column
// is empty and this build does not decrypt them, so those browsers return
// None (B12: never fabricate a token from ciphertext).

use std::path::{Path, PathBuf};

use super::browser_cookies::{
    chrome_default_profile as shared_chrome_default_profile, home_dir as shared_home_dir,
};

/// Result of browser cookie extraction
#[derive(Debug)]
pub struct ExtractedCookies {
    pub auth_token: String,
    pub ct0: String,
    pub cookie_string: String,
    pub source: String,
}

/// Function type for browser cookie extractors
type BrowserExtractor = fn() -> Option<(String, String, String)>;

/// Try to extract X cookies from all known browsers in order
pub fn extract_x_cookies() -> Option<ExtractedCookies> {
    // Priority order: Brave, Chrome, Zen, Firefox
    let browsers: Vec<(&str, BrowserExtractor)> = vec![
        ("brave", extract_brave),
        ("chrome", extract_chrome),
        ("zen", extract_zen),
        ("firefox", extract_firefox),
    ];

    for (name, extractor) in &browsers {
        if let Some((at, ct0, cookie_str)) = extractor() {
            return Some(ExtractedCookies {
                auth_token: at,
                ct0,
                cookie_string: cookie_str,
                source: name.to_string(),
            });
        }
    }
    None
}

// ── Chrome / Brave ─────────────────────────────────────────────

fn home_dir() -> PathBuf {
    shared_home_dir()
}

fn chrome_cookie_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join("Cookies")
}

fn chrome_default_profile(home: &Path, config_dir: &str) -> PathBuf {
    shared_chrome_default_profile(home, config_dir)
}

fn extract_chrome_impl(profile_dir: &Path, browser: &str) -> Option<(String, String, String)> {
    let cookie_db = chrome_cookie_path(profile_dir);
    if !cookie_db.exists() {
        return None;
    }

    let tmp_copy = std::env::temp_dir().join(format!("x_cookies_{}.db", browser));
    let _ = std::fs::remove_file(&tmp_copy);
    if std::fs::copy(&cookie_db, &tmp_copy).is_err() {
        return None;
    }

    // Chromium-family cookies are OS-encrypted (v10/v11 on Linux, DPAPI on
    // Windows, Keychain on macOS) and the value column is empty. The old code
    // pretended to recover the key with a zero-key AES-GCM "decrypt" (which
    // never succeeds on a real install) and then fabricated cookie values via
    // from_utf8_lossy(ciphertext) — minting a broken integration that failed
    // later with a misleading 401. Read plaintext rows only; an encrypted-only
    // DB honestly yields None and the caller falls through to Firefox/Zen.
    // ponytail: implement v10/v11 PBKDF2-AES-128-CBC decrypt if Chromium
    // import ever becomes a real need (needs pbkdf2+sha1+cbc crates).
    let result = read_chrome_cookies(&tmp_copy);
    let _ = std::fs::remove_file(&tmp_copy);
    result
}

fn read_chrome_cookies(db_path: &Path) -> Option<(String, String, String)> {
    let conn = rusqlite::Connection::open(db_path).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT name, value, encrypted_value, host_key, path
             FROM cookies
             WHERE (host_key LIKE '%x.com' OR host_key LIKE '%.twitter.com' OR host_key LIKE '%twitter.com')
             AND name IN ('auth_token', 'ct0', 'guest_id', 'kdt', 'twid', 'lang', 'personalization_id')",
        )
        .ok()?;

    let mut auth_token = String::new();
    let mut ct0 = String::new();
    let mut all_cookies: Vec<(String, String)> = Vec::new();

    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            let encrypted_value: Vec<u8> = row.get(2)?;
            let host_key: String = row.get(3)?;
            let path: String = row.get(4)?;
            Ok((name, value, encrypted_value, host_key, path))
        })
        .ok()?;

    for row in rows.flatten() {
        let (name, value, _encrypted_value, _host, _path) = row;
        // B12: a real Chromium cookie DB stores the value encrypted (value
        // column empty). We do not decrypt, so an encrypted-only row is
        // unusable — skip it. NEVER fall back to from_utf8_lossy(ciphertext):
        // that minted ciphertext-as-auth_token, which "connected" fine and
        // then failed every request with a misleading 401.
        if value.is_empty() {
            continue;
        }
        let val = value;

        match name.as_str() {
            "auth_token" => auth_token = val.clone(),
            "ct0" => ct0 = val.clone(),
            _ => {}
        }
        all_cookies.push((name, val));
    }

    if auth_token.is_empty() || ct0.is_empty() {
        return None;
    }

    let cookie_string = all_cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");

    Some((auth_token, ct0, cookie_string))
}

// ── Firefox / Zen ─────────────────────────────────────────────

fn firefox_profile_dir(home: &Path, config_dir: &str) -> Vec<PathBuf> {
    let profiles_ini = home.join(".mozilla").join(config_dir).join("profiles.ini");
    let mut profiles = Vec::new();

    if let Ok(content) = std::fs::read_to_string(&profiles_ini) {
        for line in content.lines() {
            if let Some(path) = line.strip_prefix("Path=") {
                let p = home.join(".mozilla").join(config_dir).join(path);
                if p.join("cookies.sqlite").exists() {
                    profiles.push(p);
                }
            }
        }
    }

    // Fallback: scan for .default directories
    let mozilla_dir = home.join(".mozilla").join(config_dir);
    if let Ok(entries) = std::fs::read_dir(&mozilla_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("cookies.sqlite").exists() && !profiles.contains(&path) {
                profiles.push(path);
            }
        }
    }

    profiles
}

fn extract_firefox_impl(profile_dir: &Path, _browser: &str) -> Option<(String, String, String)> {
    let cookie_db = profile_dir.join("cookies.sqlite");
    if !cookie_db.exists() {
        return None;
    }

    // Firefox cookies are NOT encrypted — plaintext in SQLite
    let tmp_copy = std::env::temp_dir().join("x_cookies_firefox.db");
    let _ = std::fs::remove_file(&tmp_copy);
    if std::fs::copy(&cookie_db, &tmp_copy).is_err() {
        return None;
    }

    let conn = rusqlite::Connection::open(&tmp_copy).ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT name, value, host, path
             FROM moz_cookies
             WHERE (host LIKE '%x.com' OR host LIKE '%.twitter.com' OR host LIKE '%twitter.com')
             AND name IN ('auth_token', 'ct0', 'guest_id', 'kdt', 'twid', 'lang', 'personalization_id')",
        )
        .ok()?;

    let mut auth_token = String::new();
    let mut ct0 = String::new();
    let mut all_cookies: Vec<(String, String)> = Vec::new();

    // Collect into two domain buckets so auth_token+ct0 are from same domain
    let mut x_domain: Vec<(String, String)> = Vec::new();
    let mut tw_domain: Vec<(String, String)> = Vec::new();

    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            let host: String = row.get(2)?;
            Ok((name, value, host))
        })
        .ok()?;

    for row in rows.flatten() {
        let (name, value, host) = row;
        if host.contains("x.com") {
            x_domain.push((name, value));
        } else {
            tw_domain.push((name, value));
        }
    }

    // Prefer x.com cookies; fall back to twitter.com
    let chosen = if !x_domain.is_empty() {
        &x_domain
    } else {
        &tw_domain
    };
    for (name, value) in chosen {
        match name.as_str() {
            "auth_token" => auth_token = value.clone(),
            "ct0" => ct0 = value.clone(),
            _ => {}
        }
        all_cookies.push((name.clone(), value.clone()));
    }

    let _ = std::fs::remove_file(&tmp_copy);

    if auth_token.is_empty() || ct0.is_empty() {
        return None;
    }

    let cookie_string = all_cookies
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");

    Some((auth_token, ct0, cookie_string))
}

// ── Browser-specific entry points ────────────────────────────

fn extract_chrome() -> Option<(String, String, String)> {
    let home = home_dir();
    let profile = chrome_default_profile(&home, "google-chrome");
    extract_chrome_impl(&profile, "chrome")
}

fn extract_brave() -> Option<(String, String, String)> {
    let home = home_dir();
    // Brave ships several profile-dir spellings; Brave-Origin is the current
    // stable layout on Linux (seen holding the only live session on this host).
    let profiles = [
        chrome_default_profile(&home, "BraveSoftware/Brave-Browser"),
        chrome_default_profile(&home, "BraveSoftware/Brave-Origin"),
        chrome_default_profile(&home, "BraveSoftware/Brave-Origin-Beta"),
    ];
    for profile in &profiles {
        if let Some(result) = extract_chrome_impl(profile, "brave") {
            return Some(result);
        }
    }
    None
}

fn extract_firefox() -> Option<(String, String, String)> {
    let home = home_dir();
    let profiles = firefox_profile_dir(&home, "firefox");
    for profile in profiles {
        if let Ok(c) = std::fs::read_dir(&profile) {
            for entry in c.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("cookies.sqlite").exists() {
                    if let Some(result) = extract_firefox_impl(&path, "firefox") {
                        return Some(result);
                    }
                }
            }
        }
        if let Some(result) = extract_firefox_impl(&profile, "firefox") {
            return Some(result);
        }
    }
    None
}

fn extract_zen() -> Option<(String, String, String)> {
    let home = home_dir();
    let zen_dir = home.join(".zen");
    if !zen_dir.exists() {
        return None;
    }

    // Zen stores everything in ~/.zen/, NOT ~/.mozilla/zen/
    let profiles_ini = zen_dir.join("profiles.ini");
    let mut profiles: Vec<PathBuf> = Vec::new();

    if let Ok(content) = std::fs::read_to_string(&profiles_ini) {
        for line in content.lines() {
            if let Some(path) = line.strip_prefix("Path=") {
                let p = zen_dir.join(path);
                if p.join("cookies.sqlite").exists() {
                    profiles.push(p);
                }
            }
        }
    }

    if profiles.is_empty() {
        if let Ok(entries) = std::fs::read_dir(&zen_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("cookies.sqlite").exists() {
                    profiles.push(path);
                }
            }
        }
    }

    for profile in &profiles {
        if let Some(result) = extract_firefox_impl(profile, "zen") {
            return Some(result);
        }
    }
    None
}

/// Parse a full cookie string (from browser export/curl) into components
pub fn parse_cookie_string(cookie_str: &str) -> Option<(String, String, String)> {
    let mut auth_token = None;
    let mut ct0 = None;

    for part in cookie_str.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            match k {
                "auth_token" => auth_token = Some(v.to_string()),
                "ct0" => ct0 = Some(v.to_string()),
                _ => {}
            }
        }
    }

    let at = auth_token?;
    let c = ct0?;
    Some((at, c, cookie_str.to_string()))
}

/// Build the JSON token blob for storage in DB or env
pub fn build_cookie_token(auth_token: &str, ct0: &str, cookie_string: Option<&str>) -> String {
    let mut m = serde_json::Map::new();
    m.insert(
        "auth_token".into(),
        serde_json::Value::String(auth_token.into()),
    );
    m.insert("ct0".into(), serde_json::Value::String(ct0.into()));
    if let Some(cs) = cookie_string {
        if !cs.is_empty() {
            m.insert("cookie_string".into(), serde_json::Value::String(cs.into()));
        }
    }
    serde_json::Value::Object(m).to_string()
}

#[cfg(test)]
mod b12_tests {
    use super::*;

    fn chrome_cookie_db(tag: &str, rows: &[(&str, &str, Option<&[u8]>)]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("x_cookies_b12_{tag}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "CREATE TABLE cookies (name TEXT, value TEXT, encrypted_value BLOB, host_key TEXT, path TEXT)",
            [],
        )
        .unwrap();
        for (name, value, enc) in rows {
            conn.execute(
                "INSERT INTO cookies (name, value, encrypted_value, host_key, path)
                 VALUES (?1, ?2, ?3, '.x.com', '/')",
                rusqlite::params![name, value, enc.map(<[u8]>::to_vec).unwrap_or_default()],
            )
            .unwrap();
        }
        path
    }

    /// B12 regression: a real Chromium cookie DB stores the session value
    /// encrypted with an empty `value` column. The extractor must refuse
    /// (None) — previously it minted from_utf8_lossy(ciphertext) as the
    /// auth_token, which "connected" and then failed every call with a
    /// misleading 401.
    #[test]
    fn b12_encrypted_only_chromium_rows_are_refused_not_fabricated() {
        let db = chrome_cookie_db(
            "refuse",
            &[
                ("auth_token", "", Some(&[0x76, 0x31, 0x30, 0xAA, 0xBB, 0xCC])),
                ("ct0", "plain-ct0", None),
            ],
        );
        assert!(
            read_chrome_cookies(&db).is_none(),
            "encrypted-only Chromium cookies must be refused, never fabricated"
        );
        let _ = std::fs::remove_file(&db);
    }

    /// Positive control: plaintext rows still extract.
    #[test]
    fn b12_plaintext_chromium_rows_still_read() {
        let db = chrome_cookie_db(
            "plain",
            &[
                ("auth_token", "real-token", None),
                ("ct0", "real-ct0", None),
            ],
        );
        let (at, ct0, cookie_str) =
            read_chrome_cookies(&db).expect("plaintext rows must extract");
        assert_eq!(at, "real-token");
        assert_eq!(ct0, "real-ct0");
        assert!(cookie_str.contains("auth_token=real-token"));
        let _ = std::fs::remove_file(&db);
    }
}
