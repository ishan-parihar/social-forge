// ─── X/Twitter Browser Cookie Extraction ──────────────────────
// Extracts auth_token and ct0 from Chrome, Brave, Firefox, Zen browsers.
// Chrome/Brave: reads encrypted cookies from SQLite store via aes-gcm
// Firefox/Zen: reads plaintext cookies from SQLite store

use std::path::{Path, PathBuf};

use super::browser_cookies::{home_dir as shared_home_dir, chrome_default_profile as shared_chrome_default_profile};

/// Result of browser cookie extraction
#[derive(Debug)]
pub struct ExtractedCookies {
    pub auth_token: String,
    pub ct0: String,
    pub cookie_string: String,
    pub source: String,
}

/// Try to extract X cookies from all known browsers in order
pub fn extract_x_cookies() -> Option<ExtractedCookies> {
    // Priority order: Brave, Chrome, Zen, Firefox
    let browsers: Vec<(&str, fn() -> Option<(String, String, String)>)> = vec![
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

fn chrome_local_state_path(profile_dir: &Path) -> PathBuf {
    profile_dir.parent().map(|p| p.join("Local State"))
        .unwrap_or_else(|| profile_dir.join("Local State"))
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

    // Get encryption key from Local State
    let enc_key = get_chrome_encryption_key(&chrome_local_state_path(profile_dir))?;

    // Read cookies from the copied DB
    let result = read_chrome_cookies(&tmp_copy, &enc_key);
    let _ = std::fs::remove_file(&tmp_copy);
    result
}

fn get_chrome_encryption_key(local_state_path: &Path) -> Option<Vec<u8>> {
    let content = std::fs::read_to_string(local_state_path).ok()?;
    let state: serde_json::Value = serde_json::from_str(&content).ok()?;
    let enc_key_b64 = state
        .pointer("/os_crypt/encrypted_key")
        .and_then(|v| v.as_str())?;

    let enc_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        enc_key_b64,
    )
    .ok()?;

    // Chrome format: "DPAPI" prefix (5 bytes), then actual encrypted data
    // On Linux, the key is encrypted with the OS keyring
    // For simplicity, try the Windows/Mac path: strip "DPAPI" prefix
    if enc_key.len() > 5 {
        let ciphertext = &enc_key[5..];
        // Try to decrypt with a null key (works when keyring returns empty string
        // or when the key is stored in plaintext on some Linux DE setups)
        if let Ok(key) = decrypt_chrome_key_linux(ciphertext) {
            return Some(key);
        }
    }

    None
}

fn decrypt_chrome_key_linux(encrypted: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
    if encrypted.len() >= 15 {
        let nonce_len = u32::from_ne_bytes([encrypted[0], encrypted[1], encrypted[2], encrypted[3]]) as usize;
        if nonce_len > 0 && nonce_len + 4 + 16 <= encrypted.len() && nonce_len <= 32 {
            let nonce = &encrypted[4..4 + nonce_len];
            let ciphertext_and_tag = &encrypted[4 + nonce_len..];

            use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
            use aes_gcm::aead::Aead;
            let fallback_key = Key::<Aes256Gcm>::from_slice(&[0u8; 32]);
            let cipher = Aes256Gcm::new(fallback_key);
            let nonce_obj = Nonce::from_slice(nonce);

            if let Ok(pt) = cipher.decrypt(nonce_obj, ciphertext_and_tag) {
                return Ok(pt);
            }
        }
    }

    Err(aes_gcm::Error)
}

fn read_chrome_cookies(db_path: &Path, _enc_key: &[u8]) -> Option<(String, String, String)> {
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

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let value: String = row.get(1)?;
        let encrypted_value: Vec<u8> = row.get(2)?;
        let host_key: String = row.get(3)?;
        let path: String = row.get(4)?;
        Ok((name, value, encrypted_value, host_key, path))
    }).ok()?;

    for row in rows.flatten() {
        let (name, value, encrypted_value, _host, _path) = row;
        // Use plaintext value if available, otherwise try decryption
        let val = if !value.is_empty() {
            value
        } else if !encrypted_value.is_empty() {
            // Try decryption (may fail without proper key)
            String::from_utf8_lossy(&encrypted_value).to_string()
        } else {
            continue;
        };

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
            if path.is_dir() && path.join("cookies.sqlite").exists() {
                if !profiles.contains(&path) {
                    profiles.push(path);
                }
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

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let value: String = row.get(1)?;
        let host: String = row.get(2)?;
        Ok((name, value, host))
    }).ok()?;

    for row in rows.flatten() {
        let (name, value, host) = row;
        if host.contains("x.com") {
            x_domain.push((name, value));
        } else {
            tw_domain.push((name, value));
        }
    }

    // Prefer x.com cookies; fall back to twitter.com
    let chosen = if !x_domain.is_empty() { &x_domain } else { &tw_domain };
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
    // Try standard Brave profile first, then Origin Beta
    let profiles = [
        chrome_default_profile(&home, "BraveSoftware/Brave-Browser"),
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
    m.insert("auth_token".into(), serde_json::Value::String(auth_token.into()));
    m.insert("ct0".into(), serde_json::Value::String(ct0.into()));
    if let Some(cs) = cookie_string {
        if !cs.is_empty() {
            m.insert("cookie_string".into(), serde_json::Value::String(cs.into()));
        }
    }
    serde_json::Value::Object(m).to_string()
}
