// ─── Short Link Service ──────────────────────────────────────
// Auto-shortens URLs in post content using Dub.co API.
// Also supports stripping links from X posts (X downranks external links).

use regex::Regex;
use std::sync::OnceLock;

static URL_REGEX: OnceLock<Regex> = OnceLock::new();

fn url_regex() -> &'static Regex {
    URL_REGEX.get_or_init(|| {
        // Match http(s):// URLs that are not already shortened
        Regex::new(r#"https?://[^\s<>"']+"#).unwrap()
    })
}

/// Strip all URLs from text. Used for X posts when
/// STRIP_LINKS_FROM_X is enabled.
pub fn strip_links(text: &str) -> String {
    url_regex().replace_all(text, "").trim().to_string()
}

/// Check if the text contains any URLs that could be shortened.
pub fn has_urls(text: &str) -> bool {
    url_regex().is_match(text)
}

/// Shorten all URLs in the text using Dub.co API.
/// Returns the text with URLs replaced by short links.
/// If the API key is not set or the request fails, returns the original text.
pub async fn shorten_urls(
    text: &str,
    api_key: &str,
    workspace: Option<&str>,
) -> String {
    let urls: Vec<&str> = url_regex().find_iter(text).map(|m| m.as_str()).collect();
    if urls.is_empty() {
        return text.to_string();
    }

    let client = reqwest::Client::new();
    let mut result = text.to_string();

    for url in urls {
        // Skip URLs that are already short (e.g. dub.sh links)
        if url.contains("dub.sh") || url.len() < 30 {
            continue;
        }

        match shorten_single_url(&client, url, api_key, workspace).await {
            Ok(short) => {
                result = result.replace(url, &short);
            }
            Err(e) => {
                tracing::warn!("Failed to shorten URL {url}: {e}");
                // Leave original URL in place
            }
        }
    }

    result
}

async fn shorten_single_url(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    workspace: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut body = serde_json::json!({
        "url": url,
    });
    if let Some(ws) = workspace {
        body["workspaceId"] = serde_json::json!(ws);
    }

    let response = client
        .post("https://api.dub.co/links")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Dub.co returned status {}", response.status()).into());
    }

    let json: serde_json::Value = response.json().await?;
    let short_url = json["shortLink"]
        .as_str()
        .ok_or("Dub.co response missing shortLink")?
        .to_string();

    Ok(short_url)
}
