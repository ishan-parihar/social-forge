// ─── Shared Browser Cookie Utilities ────────────────────────────
// Common helpers for browser cookie extraction (Chrome, Brave, Firefox, Zen).
// Used by x_cookies.rs and reddit_cookies.rs to avoid duplication.

use std::path::{Path, PathBuf};

/// Get the user's home directory
pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Build the default Chrome/Brave profile path
pub fn chrome_default_profile(home: &Path, config_dir: &str) -> PathBuf {
    home.join(".config").join(config_dir).join("Default")
}

/// Copy a cookie database to a temp location for safe reading.
/// Returns the temp path (caller should clean up).
pub fn copy_cookie_db(src: &Path, label: &str) -> Option<PathBuf> {
    let tmp = std::env::temp_dir().join(format!("{label}.db"));
    let _ = std::fs::remove_file(&tmp);
    std::fs::copy(src, &tmp).ok()?;
    Some(tmp)
}
