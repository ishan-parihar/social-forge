// ─── Short Link Service ──────────────────────────────────────
// Auto-shortens URLs in post content using Dub.co API.
// Also supports stripping links from X posts (X downranks external links).

use regex::Regex;
use std::sync::LazyLock;

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match http(s):// URLs that are not already shortened
    Regex::new(r#"https?://[^\s<>"']+"#).unwrap()
});

fn url_regex() -> &'static Regex {
    &URL_REGEX
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

/// E2: append `utm_source=<provider>&utm_medium=social&utm_campaign=<campaign>`
/// to every plain URL in the text. A URL that already carries ANY utm_
/// parameter is left untouched — nothing gets double-tagged. Fragment-
/// bearing URLs are tagged before the `#` so the params reach the server.
pub fn tag_urls_with_utm(text: &str, provider: &str, campaign: &str) -> String {
    url_regex()
        .replace_all(text, |caps: &regex::Captures| {
            let url = &caps[0];
            if url.contains("utm_") {
                return url.to_string();
            }
            let (base, frag) = match url.split_once('#') {
                Some((b, f)) => (b, Some(f)),
                None => (url, None),
            };
            let sep = if base.contains('?') { '&' } else { '?' };
            let tagged = format!(
                "{base}{sep}utm_source={provider}&utm_medium=social&utm_campaign={}{}",
                urlencoding::encode(campaign),
                frag.map(|f| format!("#{f}")).unwrap_or_default()
            );
            tagged
        })
        .into_owned()
}

/// Shorten all URLs in the text using Dub.co API.
/// Returns the text with URLs replaced by short links.
/// If the API key is not set or the request fails, returns the original text.
pub async fn shorten_urls(text: &str, api_key: &str, workspace: Option<&str>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm_tags_a_plain_url() {
        let out = tag_urls_with_utm("see https://example.com/page now", "linkedin", "launch");
        assert!(
            out.contains("https://example.com/page?utm_source=linkedin&utm_medium=social&utm_campaign=launch"),
            "got: {out}"
        );
    }

    #[test]
    fn utm_appends_with_ampersand_when_query_exists() {
        let out = tag_urls_with_utm("https://example.com/?id=7", "x", "spring");
        assert!(
            out.contains("https://example.com/?id=7&utm_source=x&utm_medium=social&utm_campaign=spring"),
            "got: {out}"
        );
    }

    #[test]
    fn utm_never_double_tags_and_keeps_fragment_last() {
        let out = tag_urls_with_utm(
            "https://example.com/a?utm_campaign=old https://example.com/b#top",
            "reddit",
            "launch",
        );
        assert!(
            out.contains("https://example.com/a?utm_campaign=old"),
            "already-tagged URL must stay untouched: {out}"
        );
        assert!(
            out.contains("utm_campaign=launch#top"),
            "fragment must stay at the end: {out}"
        );
    }

    #[test]
    fn utm_encodes_campaign_spaces() {
        let out = tag_urls_with_utm("https://example.com/", "x", "spring launch");
        assert!(
            out.contains("utm_campaign=spring%20launch"),
            "campaign must be URL-encoded: {out}"
        );
    }
}
