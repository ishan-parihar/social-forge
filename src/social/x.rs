// ─── X/Twitter Provider ───────────────────────────────────────
// Dual-path: GraphQL API (cookie auth) + OAuth 2.0 PKCE (Twitter API v2).
// GraphQL path uses wreq for Chrome TLS fingerprint emulation.
// OAuth path is the fallback used by the web onboarding flow.

use std::collections::HashMap; use std::sync::LazyLock;
const RATE_LIMIT_RETRIES: u32 = 2;
async fn rate_limit_sleep(attempt: u32) {
    tokio::time::sleep(std::time::Duration::from_secs(1u64 << attempt)).await;
}
fn is_rate_limited(body: &str) -> bool {
    body.contains("\"code\":88") || body.contains("Rate limit exceeded")
}

use std::time::Duration;

use async_trait::async_trait;
use rand::Rng;
use wreq::header::{self, HeaderMap, HeaderValue};
use wreq_util::Emulation;

use super::*;
use crate::config::Config;

// ── Constants ───────────────────────────────────────────────

static TWITTER_BEARER_TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";

static USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

// static constants removed — unused dead code

// sec_ch_ua() and sec_ch_ua_full_version_list() removed — unused dead code

// GraphQL Query IDs (hardcoded fallbacks — updated from JS bundle when stale)
static FALLBACK_QUERY_IDS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("HomeTimeline", "c-CzHF1LboFilMpsx4ZCrQ");
    m.insert("HomeLatestTimeline", "BKB7oi212Fi7kQtCBGE4zA");
    m.insert("UserByScreenName", "1VOOyvKkiI3FMmkeDNxM9A");
    m.insert("UserTweets", "q6xj5bs0hapm9309hexA_g");
    m.insert("TweetDetail", "xd_EMdYvB9hfZsZ6Idri0w");
    m.insert("Likes", "lIDpu_NWL7_VhimGGt0o6A");
    m.insert("SearchTimeline", "VhUd6vHVmLBcw0uX-6jMLA");
    m.insert("Bookmarks", "2neUNDqrrFzbLui8yallcQ");
    m.insert("CreateTweet", "IID9x6WsdMnTlXnzXGq8ng");
    m.insert("DeleteTweet", "VaenaVgh5q5ih7kvyVjgtg");
    m.insert("FavoriteTweet", "lI07N6Otwv1PhnEgXILM7A");
    m.insert("UnfavoriteTweet", "ZYKSe-w7KEslx3JhSIk5LA");
    m.insert("CreateRetweet", "ojPdsZsimiJrUGLR1sjUtA");
    m.insert("DeleteRetweet", "iQtK4dl5hBmXewYZuEOKVw");
    m.insert("CreateBookmark", "aoDbu3RHznuiSkQ9aNM67Q");
    m.insert("DeleteBookmark", "Wlmlj2-xzyS1GN3a6cj-mQ");
    m.insert("TweetResultByRestId", "7xflPyRiUxGVbJd4uWmbfg");
    m.insert("ListLatestTweetsTimeline", "RlZzktZY_9wJynoepm8ZsA");
    m
});

static GRAPHQL_FEATURES: LazyLock<serde_json::Value> = LazyLock::new(|| {
    serde_json::json!({
        "responsive_web_graphql_exclude_directive_enabled": true,
        "verified_phone_label_enabled": false,
        "creator_subscriptions_tweet_preview_api_enabled": true,
        "responsive_web_graphql_timeline_navigation_enabled": true,
        "responsive_web_graphql_skip_user_profile_image_extensions_enabled": false,
        "c9s_tweet_anatomy_moderator_badge_enabled": true,
        "tweetypie_unmention_optimization_enabled": true,
        "responsive_web_edit_tweet_api_enabled": true,
        "graphql_is_translatable_rweb_tweet_is_translatable_enabled": true,
        "view_counts_everywhere_api_enabled": true,
        "longform_notetweets_consumption_enabled": true,
        "responsive_web_twitter_article_tweet_consumption_enabled": true,
        "tweet_awards_web_tipping_enabled": false,
        "longform_notetweets_rich_text_read_enabled": true,
        "longform_notetweets_inline_media_enabled": true,
        "rweb_video_timestamps_enabled": true,
        "responsive_web_media_download_video_enabled": true,
        "freedom_of_speech_not_reach_fetch_enabled": true,
        "standardized_nudges_misinfo": true,
        "responsive_web_enhance_cards_enabled": false
    })
});

// ── XProvider ───────────────────────────────────────────────

pub struct XProvider {
    client_id: String,
    client_secret: String,
    http: wreq::Client,
    /// Additional cookies beyond auth_token+ct0 (e.g. guest_id, kdt, twid)
    cookie_string: Option<String>,
    /// Cached X-Client-Transaction-Id generator
    transaction_generator: Option<XTransactionGenerator>,
}

/// Generates X-Client-Transaction-Id headers
struct XTransactionGenerator {
    client_id: String,
}

impl XTransactionGenerator {
    fn new() -> Self {
        let id = uuid::Uuid::new_v4().to_string().replace('-', "");
        Self { client_id: id }
    }

    fn generate(&self, method: &str, path: &str) -> String {
        use sha2::{Digest, Sha256};
        let input = format!("{}{}{}", method.to_uppercase(), path, self.client_id);
        let digest = Sha256::digest(input.as_bytes());
        format!("{:x}", digest)[..32].to_string()
    }
}

impl XProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) = config
            .provider_credentials("x")
            .unwrap_or_default();

        let auth_token = config.x_auth_token.as_deref();
        let ct0 = config.x_ct0.as_deref();

        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, HeaderValue::from_static(USER_AGENT));
        headers.insert(header::ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert("origin", HeaderValue::from_static("https://x.com"));
        headers.insert("referer", HeaderValue::from_static("https://x.com/"));
        headers.insert("x-twitter-active-user", HeaderValue::from_static("yes"));
        headers.insert("x-twitter-auth-type", HeaderValue::from_static("OAuth2Session"));
        headers.insert("x-twitter-client-language", HeaderValue::from_static("en"));
        // Authorization header MUST include "Bearer " prefix
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", TWITTER_BEARER_TOKEN)).unwrap());

        let http = wreq::Client::builder()
            .default_headers(headers)
            .emulation(Emulation::Chrome131)
            .gzip(true)
            .brotli(true)
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .build().expect("Failed to build wreq client");

        let mut provider = Self {
            client_id,
            client_secret,
            http,
            cookie_string: None,
            transaction_generator: Some(XTransactionGenerator::new()),
        };

        // Priority 1: Env vars (X_AUTH_TOKEN + X_CT0)
        // Priority 2: Browser cookie extraction (Chrome/Brave/Firefox)
        // Priority 3: No cookies — will rely on OAuth v2 fallback
        if let (Some(at), Some(ct)) = (auth_token, ct0) {
            if !at.is_empty() && !ct.is_empty() {
                provider.set_credentials(at, ct, None);
            }
        } else if let Some(cookies) = crate::social::x_cookies::extract_x_cookies() {
            tracing::info!("X cookies extracted from browser: {}", cookies.source);
            provider.set_credentials(&cookies.auth_token, &cookies.ct0, Some(&cookies.cookie_string));
        }

        provider
    }

    // ── Cookie Token Parsing ─────────────────────────────────

    fn parse_cookie_token(token: &str) -> Option<(String, String)> {
        let v: serde_json::Value = serde_json::from_str(token).ok()?;
        let auth_token = v.get("auth_token")?.as_str()?.to_string();
        let ct0 = v.get("ct0")?.as_str()?.to_string();
        Some((auth_token, ct0))
    }

    /// Extract optional full cookie string from token JSON blob
    fn extract_cookie_string(token: &str) -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(token).ok()?;
        v.get("cookie_string")?.as_str().map(|s| s.to_string())
    }

    /// Initialize credentials from a token JSON blob (used by MCP handlers).
    /// Parses auth_token, ct0, and optional cookie_string from the token.
    pub fn prepare_from_token(&mut self, token: &str) {
        let v: serde_json::Value = serde_json::from_str(token).unwrap_or_default();
        let at = v.get("auth_token").and_then(|s| s.as_str()).unwrap_or("");
        let ct = v.get("ct0").and_then(|s| s.as_str()).unwrap_or("");
        let extras = v.get("cookie_string").and_then(|s| s.as_str());
        if !at.is_empty() && !ct.is_empty() {
            self.set_credentials(at, ct, extras);
        }
    }

    /// Set cookie credentials to be used for requests.
    ///
    /// Avoids duplicates: if `extra_cookies` already contains `auth_token=` and `ct0=`
    /// (e.g. a full Cookie header string), it is used directly without prepending.
    fn set_credentials(&mut self, auth_token: &str, ct0: &str, extra_cookies: Option<&str>) {
        self.cookie_string = Some(match extra_cookies {
            Some(extras) if !extras.is_empty() => {
                // extras is a full cookie string that already contains auth_token+ct0
                if extras.contains("auth_token=") && extras.contains("ct0=") {
                    extras.to_string()
                } else {
                    format!("auth_token={auth_token}; ct0={ct0}; {extras}")
                }
            }
            _ => format!("auth_token={auth_token}; ct0={ct0};"),
        });
    }

    /// Check if a token is a cookie auth JSON blob (public for testing)
    pub fn is_cookie_auth(token: &str) -> bool {
        Self::parse_cookie_token(token).is_some()
    }

    /// Public alias for is_cookie_auth for cross-crate test access
    pub fn is_cookie_auth_static(token: &str) -> bool {
        Self::parse_cookie_token(token).is_some()
    }

    // ── OAuth helpers (kept for SocialProvider trait) ────────

    fn oauth_token_endpoint(&self) -> &'static str {
        "https://api.twitter.com/2/oauth2/token"
    }

    fn oauth_authorize_endpoint(&self) -> &'static str {
        "https://twitter.com/i/oauth2/authorize"
    }

    // ── Core GraphQL Request ─────────────────────────────────

    fn graphql_url(&self, query_id: &str, operation: &str, variables: &serde_json::Value) -> Result<String, ProviderError> {
        use url::form_urlencoded;
        let vars_json = variables.to_string();
        let features_json = GRAPHQL_FEATURES.to_string();
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("variables", &vars_json)
            .append_pair("features", &features_json)
            .finish();
        Ok(format!("https://x.com/i/api/graphql/{query_id}/{operation}?{query}"))
    }

    /// Ensure the ct0 value in a cookie string matches the given ct0.
    /// This prevents CSRF mismatches when the stored cookies have a stale ct0.
    fn ensure_ct0_matches(cookie_str: &str, ct0: &str) -> String {
        let mut parts: Vec<String> = cookie_str
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.starts_with("ct0=") && !s.is_empty())
            .collect();
        parts.push(format!("ct0={ct0}"));
        parts.join("; ")
    }

    /// Build the effective cookie string for a GraphQL request.
    /// Prefers the access_token's embedded cookie_string over self.cookie_string,
    /// and ensures ct0 matches the header value.
    fn effective_cookie_str(&self, access_token: &str) -> (String, String) {
        // Parse ct0 from the access token JSON blob
        let ct0 = Self::parse_cookie_token(access_token)
            .map(|(_, ct)| ct)
            .unwrap_or_default();

        // Prefer the full cookie_string from the access token if available
        let base_cs = Self::extract_cookie_string(access_token)
            .filter(|s| !s.is_empty() && s.contains("auth_token="))
            .or_else(|| self.cookie_string.clone())
            .unwrap_or_default();

        // Ensure ct0 in cookie matches the header
        let cs = Self::ensure_ct0_matches(&base_cs, &ct0);
        (ct0, cs)
    }

    fn add_tx_header(&self, req: wreq::RequestBuilder, method: &str, url: &str) -> wreq::RequestBuilder {
        if let Some(ref tx) = self.transaction_generator {
            let path = url.split('?').next().unwrap_or(url);
            let tid = tx.generate(method, path);
            req.header("X-Client-Transaction-Id", &tid)
        } else {
            req
        }
    }

    async fn graphql_get(
        &self,
        query_id: &str,
        operation: &str,
        variables: &serde_json::Value,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = self.graphql_url(query_id, operation, variables)?;
        let (ct0, cs) = self.effective_cookie_str(access_token);
        let resp = self.http.get(&url)
            .header("x-csrf-token", &ct0)
            .header("Cookie", &cs)
            .send().await
            .map_err(|e| ProviderError::Api(format!("X GraphQL GET error: {e}")))?;
        let status = resp.status();
        let body = resp.text().await
            .map_err(|e| ProviderError::Api(format!("X body error: {e}")))?;
        if !status.is_success() {
            return Err(ProviderError::Api(format!(
                "X GraphQL {operation}: HTTP {status}: {body}"
            )));
        }
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| ProviderError::Api(format!("X JSON parse error: {e}: {body}")))?;
        self.check_graphql_response(status, &json)
    }

    async fn graphql_post(
        &self,
        query_id: &str,
        operation: &str,
        variables: &serde_json::Value,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("https://x.com/i/api/graphql/{query_id}/{operation}");
        let body = serde_json::json!({
            "variables": variables,
            "queryId": query_id,
            "features": *GRAPHQL_FEATURES,
        });
        let (ct0, cs) = self.effective_cookie_str(access_token);
        let resp = self.http.post(&url)
            .header("x-csrf-token", &ct0)
            .header("Cookie", &cs)
            .header("Priority", "u=1, i")
            .header("Referer", "https://x.com/compose/post")
            .json(&body)
            .send().await
            .map_err(|e| ProviderError::Api(format!("X GraphQL POST error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Api(format!("X JSON parse error: {e}")))?;
        self.check_graphql_response(status, &json)
    }

    // ── V2 REST API (OAuth Bearer fallback) ──────────────────

    async fn v1_get_with_cookies(&self, url: &str, cookie_str: &str) -> Result<serde_json::Value, ProviderError> {
        let ct0 = Self::parse_cookie_token(cookie_str)
            .map(|(_, ct)| ct)
            .unwrap_or_default();
        let resp = self
            .http
            .get(url)
            .header("x-csrf-token", &ct0)
            .header("Cookie", cookie_str)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("X cookie GET error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        self.check_v2_response(status, &json)
    }

    async fn v2_get(&self, url: &str, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("X v2 GET error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        self.check_v2_response(status, &json)
    }

    async fn v2_post(&self, url: &str, access_token: &str, body: &serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .post(url)
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("X v2 POST error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        self.check_v2_response(status, &json)
    }

    async fn v2_delete(&self, url: &str, access_token: &str) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(url)
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("X v2 DELETE error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        self.check_v2_response(status, &json)
    }

    async fn v2_form_post(
        &self,
        url: &str,
        access_token: &str,
        form: &[(&str, &str)],
    ) -> Result<serde_json::Value, ProviderError> {
        let body = form.iter().map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let resp = self
            .http
            .post(url)
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("X v2 form POST error: {e}")))?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await.map_err(|e| ProviderError::Api(e.to_string()))?;
        self.check_v2_response(status, &json)
    }

    // ── Response Checkers ────────────────────────────────────

    fn check_graphql_response(&self, status: wreq::StatusCode, json: &serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        if status.is_success() {
            // Check for GraphQL errors in response body
            if let Some(errors) = json.get("errors").and_then(|e| e.as_array()) {
                if let Some(first) = errors.first() {
                    let msg = first["message"].as_str().unwrap_or("GraphQL error");
                    if msg.contains("Not authorized") || msg.contains("authentication") {
                        return Err(ProviderError::TokenExpired);
                    }
                    return Err(ProviderError::Api(msg.to_string()));
                }
            }
            return Ok(json.clone());
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("X API rate limit".into()));
        }
        if status == 401 || status == 403 {
            return Err(ProviderError::TokenExpired);
        }
        let msg = json["errors"][0]["message"]
            .as_str()
            .unwrap_or("GraphQL request failed");
        Err(ProviderError::Api(format!("HTTP {status}: {msg}")))
    }

    fn check_v2_response(&self, status: wreq::StatusCode, json: &serde_json::Value) -> Result<serde_json::Value, ProviderError> {
        if status.is_success() {
            return Ok(json.clone());
        }
        if status == 429 {
            return Err(ProviderError::RateLimited("X API rate limit".into()));
        }
        if json.get("title").and_then(|t| t.as_str()) == Some("Unauthorized") || status == 401 {
            return Err(ProviderError::TokenExpired);
        }
        let detail = json
            .get("detail")
            .and_then(|d| d.as_str())
            .unwrap_or("Unknown API error");
        Err(ProviderError::Api(detail.to_string()))
    }

    // ── Pagination cursor extraction from GraphQL timeline ───

    fn extract_next_cursor(json: &serde_json::Value) -> Option<String> {
        // Walk common timeline paths
        let instructions = json
            .pointer("/data/home/home_timeline_urt/instructions")
            .or_else(|| json.pointer("/data/user/result/timeline/timeline/instructions"))
            .or_else(|| json.pointer("/data/user/result/timeline_v2/timeline/instructions"))
            .or_else(|| json.pointer("/data/search_by_raw_query/search_timeline/timeline/instructions"))
            .or_else(|| json.pointer("/data/bookmark_timeline_v2/timeline/instructions"))
            .or_else(|| json.pointer("/data/list/tweets_timeline/timeline/instructions"))
            .and_then(|i| i.as_array())?;

        for instruction in instructions {
            if let Some(entries) = instruction.get("entries").and_then(|e| e.as_array()) {
                for entry in entries {
                    if let Some(content) = entry.get("content") {
                        if content.get("cursorType").and_then(|c| c.as_str()) == Some("Bottom") {
                            if let Some(val) = content.get("value").and_then(|v| v.as_str()) {
                                return Some(val.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // ── Media upload (v1.1 API, used by both paths) ──────────

    async fn fetch_media_bytes(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        if url.starts_with("http://") || url.starts_with("https://") {
            // Use a separate reqwest client for media download (may not need emulation)
            let resp = reqwest::get(url)
                .await
                .map_err(|e| ProviderError::Api(format!("Failed to fetch media: {e}")))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(ProviderError::Api(format!("Failed to fetch media: HTTP {status}")));
            }
            resp.bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| ProviderError::Api(format!("Failed to read media body: {e}")))
        } else {
            tokio::fs::read(url)
                .await
                .map_err(|e| ProviderError::Api(format!("Failed to read local media {url}: {e}")))
        }
    }

    async fn upload_single_media(
        &self,
        _access_token: &str,
        media_url: &str,
        mime_type: &str,
    ) -> Result<String, ProviderError> {
        let bytes = self.fetch_media_bytes(media_url).await?;
        let total_bytes = bytes.len();

        let media_category = if mime_type.starts_with("video/") {
            "tweet_video"
        } else if mime_type == "image/gif" {
            "tweet_gif"
        } else {
            "tweet_image"
        };

        let api_base = "https://upload.twitter.com/1.1/media/upload.json";

        // INIT
        let init_body = format!(
            "command=INIT&total_bytes={total_bytes}&media_type={}&media_category={media_category}",
            urlencoding::encode(mime_type),
        );
        let init_resp: serde_json::Value = self
            .http
            .post(api_base)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(init_body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Media INIT error: {e}")))?
            .json()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let media_id = init_resp["media_id_string"]
            .as_str()
            .ok_or_else(|| ProviderError::Api(format!("Twitter INIT failed: {init_resp:?}")))?
            .to_string();

        // APPEND (base64-encoded per twitter-cli pattern)
        let b64_data = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &bytes,
        );
        let append_body = format!(
            "command=APPEND&media_id={media_id}&segment_index=0&media_data={}",
            urlencoding::encode(&b64_data),
        );
        self.http
            .post(api_base)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(append_body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Media APPEND error: {e}")))?;

        // FINALIZE
        let final_body = format!("command=FINALIZE&media_id={media_id}");
        let finalize_resp: serde_json::Value = self
            .http
            .post(api_base)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(final_body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Media FINALIZE error: {e}")))?
            .json()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        if let Some(state) = finalize_resp
            .pointer("/processing_info/state")
            .and_then(|s| s.as_str())
        {
            if state == "pending" || state == "in_progress" {
                tracing::warn!("Media processing still {state} for media_id={media_id}");
            }
        }

        Ok(media_id)
    }

    async fn upload_media(
        &self,
        access_token: &str,
        media: &[MediaAttachment],
    ) -> Result<Vec<String>, ProviderError> {
        if media.is_empty() {
            return Ok(vec![]);
        }
        let mut media_ids = Vec::with_capacity(media.len());
        for attachment in media {
            let id = self
                .upload_single_media(access_token, &attachment.url, &attachment.mime_type)
                .await?;
            media_ids.push(id);
        }
        Ok(media_ids)
    }

    // ── Page Parser ────────────────────────────────────────

    /// Parse one page of user tweets (GraphQL or v2) into ExternalPostData + cursor.
    fn parse_user_tweets_page(&self, response: &serde_json::Value) -> UserTweetsPage {
    let mut posts = Vec::new();
    let mut next_cursor = None;

    // ── OAuth v2 path: response["data"] is an array of tweet objects ──
    if let Some(data) = response["data"].as_array() {
        let media_map: std::collections::HashMap<String, MediaAttachment> = response["includes"]["media"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|m| {
                    let key = m["media_key"].as_str()?;
                    let media_type = m["type"].as_str().unwrap_or("photo");
                    if media_type == "video" {
                        let video_url = m["variants"]
                            .as_array()
                            .and_then(|variants| {
                                let mut best: Option<(&str, u64)> = None;
                                for v in variants {
                                    if v["content_type"].as_str() == Some("video/mp4") {
                                        if let (Some(url), Some(bitrate)) = (v["url"].as_str(), v["bitrate"].as_u64()) {
                                            if best.map_or(true, |(_, b)| bitrate > b) {
                                                best = Some((url, bitrate));
                                            }
                                        }
                                    }
                                }
                                best.map(|(url, _)| url.to_string())
                            })
                            .or_else(|| m["url"].as_str().map(String::from));
                        video_url.map(|url| (key.to_string(), MediaAttachment {
                            url,
                            mime_type: "video/mp4".to_string(),
                            alt: m["alt_text"].as_str().map(String::from),
                            poster_url: m["preview_image_url"].as_str().map(String::from),
                        }))
                    } else {
                        let url = m["url"].as_str()
                            .or_else(|| m["preview_image_url"].as_str())?;
                        Some((key.to_string(), MediaAttachment {
                            url: url.to_string(),
                            mime_type: if media_type == "animated_gif" { "image/gif".to_string() } else { "image/jpeg".to_string() },
                            alt: m["alt_text"].as_str().map(String::from),
                            poster_url: None,
                        }))
                    }
                }).collect()
            })
            .unwrap_or_default();

        let author_map: std::collections::HashMap<String, serde_json::Value> = response["includes"]["users"]
            .as_array()
            .map(|arr| {
                arr.iter().filter_map(|u| {
                    let uid = u["id"].as_str()?;
                    Some((uid.to_string(), u.clone()))
                }).collect()
            })
            .unwrap_or_default();

        for item in data {
            let id = item["id"].as_str().unwrap_or("").to_string();
            if id.is_empty() { continue; }
            let text = item["text"].as_str().unwrap_or("").to_string();
            let created_at = item["created_at"].as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now);

            let author_id = item["author_id"].as_str();
            let author_name = author_id
                .and_then(|aid| author_map.get(aid))
                .and_then(|u| u["name"].as_str().map(String::from));
            let author_handle = author_id
                .and_then(|aid| author_map.get(aid))
                .and_then(|u| u["username"].as_str().map(String::from));
            let author_avatar = author_id
                .and_then(|aid| author_map.get(aid))
                .and_then(|u| u["profile_image_url"].as_str().map(String::from));

            let mut media = Vec::new();
            if let Some(keys) = item["attachments"]["media_keys"].as_array() {
                for key in keys.iter().filter_map(|k| k.as_str()) {
                    if let Some(att) = media_map.get(key) {
                        media.push(att.clone());
                    }
                }
            }

            posts.push(ExternalPostData {
                platform_post_id: id,
                text,
                author_name,
                author_handle,
                author_avatar,
                created_at,
                url: None,
                media,
                metadata: Some(item.clone()),
            });
        }
        next_cursor = response["meta"]["next_token"].as_str().map(String::from);
    } else if let Some(timeline) = response["data"].as_object() {
        // ── Cookie auth (GraphQL) path: navigate instructions → entries ──
        if let Some(instructions) = timeline.get("instructions").and_then(|i| i.as_array()) {
            for entry in instructions.iter()
                .filter_map(|inst| inst["entries"].as_array())
                .flatten()
            {
                let raw_result = match entry["content"]["itemContent"]["tweet_results"]["result"].as_object() {
                    Some(r) => r,
                    None => continue,
                };
                let result = if raw_result.get("__typename").and_then(|t| t.as_str()) == Some("TweetWithState") {
                    match raw_result.get("tweet").and_then(|t| t.as_object()) {
                        Some(inner) => inner,
                        None => raw_result,
                    }
                } else {
                    raw_result
                };
                let legacy = match result.get("legacy") {
                    Some(l) => l,
                    None => continue,
                };
                let id = result["rest_id"].as_str().unwrap_or("").to_string();
                if id.is_empty() { continue; }
                let text = legacy["full_text"].as_str()
                    .or_else(|| legacy["text"].as_str())
                    .unwrap_or("").to_string();
                let created_at_str = legacy["created_at"].as_str().unwrap_or("");
                let created_at = chrono::DateTime::parse_from_str(created_at_str, "%a %b %d %H:%M:%S %z %Y")
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                let user = &result["core"]["user_results"]["result"]["legacy"];
                let author_name = user["name"].as_str().map(String::from);
                let author_handle = user["screen_name"].as_str().map(String::from);

                let mut media = Vec::new();
                Self::extract_media_from_legacy(legacy, &mut media);
                if media.is_empty() {
                    if let Some(media_details) = result.get("mediaDetails").and_then(|m| m.as_array()) {
                        for m in media_details {
                            let media_type = m["type"].as_str().unwrap_or("photo");
                            if media_type == "video" || media_type == "animated_gif" {
                                let video_url = m["video_info"]["variants"]
                                    .as_array()
                                    .and_then(|variants| {
                                        let mut best: Option<(&str, u64)> = None;
                                        for v in variants {
                                            if v["content_type"].as_str() == Some("video/mp4") {
                                                if let (Some(url), Some(bitrate)) = (v["url"].as_str(), v["bitrate"].as_u64()) {
                                                    if best.map_or(true, |(_, b)| bitrate > b) {
                                                        best = Some((url, bitrate));
                                                    }
                                                }
                                            }
                                        }
                                        best.map(|(url, _)| url.to_string())
                                    });
                                if let Some(url) = video_url {
                                    media.push(MediaAttachment { url, mime_type: "video/mp4".to_string(), alt: None, poster_url: m["media_url_https"].as_str().map(String::from) });
                                }
                            } else {
                                let url = m["media_url_https"].as_str().or_else(|| m["media_url"].as_str()).unwrap_or("").to_string();
                                if !url.is_empty() {
                                    media.push(MediaAttachment { url, mime_type: "image/jpeg".to_string(), alt: None, poster_url: None });
                                }
                            }
                        }
                    }
                }
                if media.is_empty() {
                    let rt_legacy = legacy.pointer("/retweeted_status_result/result/result/legacy")
                        .or_else(|| legacy.pointer("/retweeted_status_result/result/legacy"));
                    if let Some(rt) = rt_legacy {
                        Self::extract_media_from_legacy(rt, &mut media);
                    }
                }
                if media.is_empty() {
                    let qt_legacy = legacy.pointer("/quoted_status_result/result/result/legacy")
                        .or_else(|| legacy.pointer("/quoted_status_result/result/legacy"));
                    if let Some(qt) = qt_legacy {
                        Self::extract_media_from_legacy(qt, &mut media);
                    }
                }

                let author_avatar = user["profile_image_url_https"].as_str().map(String::from);

                posts.push(ExternalPostData {
                    platform_post_id: id,
                    text,
                    author_name,
                    author_handle,
                    author_avatar,
                    created_at,
                    url: None,
                    media,
                    metadata: Some(serde_json::Value::Object(result.clone())),
                });
            }
        }
        next_cursor = Self::extract_next_cursor(response);
    }

    UserTweetsPage { posts, next_cursor }
}

// ── Media Extraction Helper ──────────────────────────────

    /// Extract media attachments from a legacy tweet object's extended_entities or entities.
    /// Works for original tweets, retweets, and quote tweets.
    fn extract_media_from_legacy(legacy: &serde_json::Value, media: &mut Vec<MediaAttachment>) {
        let arr = legacy["extended_entities"]["media"].as_array()
            .or_else(|| legacy["entities"]["media"].as_array());
        if let Some(arr) = arr {
            for m in arr {
                let media_type = m["type"].as_str().unwrap_or("photo");
                // Thumbnail/poster URL available for all media types
                let poster = m["media_url_https"].as_str()
                    .or_else(|| m["media_url"].as_str())
                    .map(|s| s.to_string());
                if media_type == "video" || media_type == "animated_gif" {
                    // Extract video URL from video_info.variants — prefer highest-bitrate mp4
                    let video_url = m["video_info"]["variants"]
                        .as_array()
                        .and_then(|variants| {
                            let mut best: Option<(&str, u64)> = None;
                            for v in variants {
                                if v["content_type"].as_str() == Some("video/mp4") {
                                    if let (Some(url), Some(bitrate)) = (v["url"].as_str(), v["bitrate"].as_u64()) {
                                        if best.map_or(true, |(_, b)| bitrate > b) {
                                            best = Some((url, bitrate));
                                        }
                                    }
                                }
                            }
                            best.map(|(url, _)| url.to_string())
                        });
                    if let Some(video_url) = video_url {
                        media.push(MediaAttachment {
                            url: video_url,
                            mime_type: "video/mp4".to_string(),
                            alt: m["alt_text"].as_str().map(String::from),
                            poster_url: poster,
                        });
                    } else if let Some(url) = poster {
                        // Fallback to thumbnail
                        media.push(MediaAttachment {
                            url,
                            mime_type: "image/jpeg".to_string(),
                            alt: m["alt_text"].as_str().map(String::from),
                            poster_url: None,
                        });
                    }
                } else {
                    // Photo — use media_url_https
                    if let Some(url) = poster {
                        media.push(MediaAttachment {
                            url,
                            mime_type: "image/jpeg".to_string(),
                            alt: m["alt_text"].as_str().map(String::from),
                            poster_url: None,
                        });
                    }
                }
            }
        }
    }

    // ── Rate limit + write delay helpers ─────────────────────

    fn write_delay() {
        let delay = rand::thread_rng().gen_range(1.5..4.0);
        std::thread::sleep(Duration::from_secs_f64(delay));
    }
}

// ════════════════════════════════════════════════════════════════
// SocialProvider Trait Implementation
// ════════════════════════════════════════════════════════════════

/// Result of parsing a single page of user tweets.
struct UserTweetsPage {
    posts: Vec<ExternalPostData>,
    next_cursor: Option<String>,
}

#[async_trait]
impl SocialProvider for XProvider {
    fn identifier(&self) -> &'static str {
        "x"
    }

    fn name(&self) -> &'static str {
        "X (Twitter)"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "tweet.read".into(),
            "tweet.write".into(),
            "users.read".into(),
            "offline.access".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        4000
    }

    fn validate_media(&self, post: &PostContent) -> Result<(), String> {
        super::validate_media_limits(self.identifier(), post)
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let challenge = common::generate_code_challenge(code_verifier);
        let params = [
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("redirect_uri", redirect_uri),
            ("scope", &self.scopes().join(" ")),
            ("state", state),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
        ];
        let url = url::Url::parse_with_params(self.oauth_authorize_endpoint(), &params)
            .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;
        Ok(AuthUrlResponse {
            url: url.to_string(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );
        let form_body = format!(
            "grant_type=authorization_code&code={}&code_verifier={}&redirect_uri={}&client_id={}",
            urlencoding::encode(code),
            urlencoding::encode(code_verifier),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&self.client_id),
        );
        let json: serde_json::Value = self
            .http
            .post(self.oauth_token_endpoint())
            .header(header::AUTHORIZATION, format!("Basic {credentials}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("OAuth token exchange error: {e}")))?
            .json()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        let user_info: serde_json::Value = self
            .http
            .get("https://api.twitter.com/2/users/me?user.fields=name,username,profile_image_url")
            .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("User info error: {e}")))?
            .json()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        if user_info.get("data").is_none() {
            tracing::warn!("X /2/users/me returned no data: {user_info}");
        }

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: user_info["data"]["id"].as_str().unwrap_or("").to_string(),
            name: user_info["data"]["name"].as_str().unwrap_or("").to_string(),
            username: user_info["data"]["username"].as_str().unwrap_or("").to_string(),
            picture: user_info["data"]["profile_image_url"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", self.client_id, self.client_secret),
        );
        let form_body = format!(
            "grant_type=refresh_token&refresh_token={}&client_id={}",
            urlencoding::encode(refresh_token),
            urlencoding::encode(&self.client_id),
        );
        let json: serde_json::Value = self
            .http
            .post(self.oauth_token_endpoint())
            .header(header::AUTHORIZATION, format!("Basic {credentials}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await
            .map_err(|e| ProviderError::Api(format!("Token refresh error: {e}")))?
            .json()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        Ok(AuthToken {
            access_token: json["access_token"]
                .as_str()
                .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
                .to_string(),
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Try GraphQL path if cookie auth
        if let Some((_auth_token, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut variables = serde_json::json!({
                "tweet_text": post.content,
                "media": {
                    "media_entities": [],
                    "possibly_sensitive": false
                },
                "semantic_annotation_ids": [],
                "dark_request": false,
            });

            // Thread linking: if in_reply_to is set, add the reply field
            // so X creates this tweet as a reply to the predecessor.
            if let Some(ref reply_to_id) = post.in_reply_to {
                variables["reply"] = serde_json::json!({
                    "in_reply_to_tweet_id": reply_to_id,
                    "reply_options": [],
                });
            }

            // Upload and attach media
            if !post.media.is_empty() {
                let media_ids = self.upload_media(access_token, &post.media).await?;
                let entities: Vec<serde_json::Value> = media_ids
                    .iter()
                    .map(|id| serde_json::json!({"media_id": id, "tagged_users": []}))
                    .collect();
                variables["media"] = serde_json::json!({
                    "media_entities": entities,
                    "possibly_sensitive": false,
                });
            }

            Self::write_delay();

            let query_id = FALLBACK_QUERY_IDS
                .get("CreateTweet")
                .ok_or_else(|| ProviderError::Api("Missing CreateTweet queryId".into()))?;
            let json = self.graphql_post(query_id, "CreateTweet", &variables, access_token).await?;

            let tweet_id = json["data"]["create_tweet"]["tweet_results"]["result"]["rest_id"]
                .as_str()
                .unwrap_or("");

            return Ok(PublishResult {
                platform_post_url: if tweet_id.is_empty() {
                    None
                } else {
                    Some(format!("https://x.com/i/status/{tweet_id}"))
                },
                platform_post_id: tweet_id.to_string(),
                status: "published".into(),
            });
        }

        // Fallback: v2 API with Bearer token
        let mut body = serde_json::json!({ "text": post.content });
        if !post.media.is_empty() {
            let media_ids = self.upload_media(access_token, &post.media).await?;
            if !media_ids.is_empty() {
                body["media"] = serde_json::json!({ "media_ids": media_ids });
            }
        }
        let json = self.v2_post("https://api.twitter.com/2/tweets", access_token, &body).await?;
        let post_id = json["data"]["id"].as_str().unwrap_or("").to_string();
        Ok(PublishResult {
            platform_post_url: Some(format!("https://twitter.com/user/status/{post_id}")),
            platform_post_id: post_id,
            status: "published".into(),
        })
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api("X does not support page management".into()))
    }

    async fn reconnect(
        &self,
        access_token: &str,
        _internal_id: &str,
        _page_id: &str,
    ) -> Result<super::ReconnectResult, ProviderError> {
        // Try cookie-based auth first (access_token may be a JSON blob)
        let json = if let Some((at, ct)) = Self::parse_cookie_token(access_token) {
            let cookie_str = Self::extract_cookie_string(access_token)
                .unwrap_or_else(|| format!("auth_token={at}; ct0={ct};"));
            self.v1_get_with_cookies("https://api.twitter.com/2/users/me?user.fields=name,username,profile_image_url", &cookie_str).await?
        } else {
            self.v2_get("https://api.twitter.com/2/users/me?user.fields=name,username,profile_image_url", access_token).await?
        };
        let data = &json["data"];
        Ok(super::ReconnectResult {
            id: data["id"].as_str().unwrap_or("").to_string(),
            name: data["name"].as_str().unwrap_or("").to_string(),
            access_token: access_token.to_string(),
            picture: data["profile_image_url"].as_str().map(|s| s.to_string()),
            username: data["username"].as_str().map(|s| s.to_string()),
        })
    }

    async fn analytics(
        &self,
        access_token: &str,
        _internal_id: &str,
        _days: u32,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let json = self
            .v2_get("https://api.twitter.com/2/users/me?user.fields=public_metrics", access_token)
            .await?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let metrics = &json["data"]["public_metrics"];
        let mut result = Vec::new();
        if let Some(followers) = metrics["followers_count"].as_i64() {
            result.push(AnalyticsData {
                label: "Followers".into(),
                data: vec![AnalyticsDataPoint { total: followers.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(following) = metrics["following_count"].as_i64() {
            result.push(AnalyticsData {
                label: "Following".into(),
                data: vec![AnalyticsDataPoint { total: following.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(tweets) = metrics["tweet_count"].as_i64() {
            result.push(AnalyticsData {
                label: "Tweets".into(),
                data: vec![AnalyticsDataPoint { total: tweets.to_string(), date: today.clone() }],
                percentage_change: 0.0,
            });
        }
        if let Some(listed) = metrics["listed_count"].as_i64() {
            result.push(AnalyticsData {
                label: "Listed".into(),
                data: vec![AnalyticsDataPoint { total: listed.to_string(), date: today }],
                percentage_change: 0.0,
            });
        }
        Ok(result)
    }

    async fn post_analytics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Vec<AnalyticsData>, ProviderError> {
        let url = format!(
            "https://api.twitter.com/2/tweets/{platform_post_id}?tweet.fields=public_metrics"
        );
        let json = self.v2_get(&url, access_token).await?;
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let metrics = &json["data"]["public_metrics"];
        let mut result = Vec::new();
        for (label, key) in [
            ("Likes", "like_count"),
            ("Retweets", "retweet_count"),
            ("Replies", "reply_count"),
            ("Quotes", "quote_count"),
            ("Impressions", "impression_count"),
        ] {
            if let Some(val) = metrics.get(key).and_then(|v| v.as_i64()) {
                result.push(AnalyticsData {
                    label: label.into(),
                    data: vec![AnalyticsDataPoint { total: val.to_string(), date: today.clone() }],
                    percentage_change: 0.0,
                });
            }
        }
        Ok(result)
    }

    async fn get_recent_posts(
        &self,
        access_token: &str,
        internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        tracing::info!(
            "X get_recent_posts: internal_id='{}' limit={}",
            internal_id, limit,
        );
        let page_size = limit.min(100);
        let mut all_posts = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages_fetched = 0u32;
        const MAX_PAGES: u32 = 10;

        loop {
            let response = self.user_tweets(access_token, internal_id, page_size, cursor.as_deref()).await?;
            pages_fetched += 1;
            let data_count = response["data"].as_array().map(|a| a.len()).unwrap_or(0);
            tracing::info!(
                "X get_recent_posts page {}: data_count={}, total_so_far={}",
                pages_fetched, data_count, all_posts.len(),
            );

            let page = self.parse_user_tweets_page(&response);
            let page_post_count = page.posts.len() as u32;
            all_posts.extend(page.posts);

            cursor = page.next_cursor;

            if all_posts.len() as u32 >= limit {
                tracing::info!(
                    "X get_recent_posts: reached limit={} (got {}), stopping",
                    limit, all_posts.len(),
                );
                break;
            }
            if cursor.is_none() {
                tracing::info!(
                    "X get_recent_posts: no more pages after page {}",
                    pages_fetched,
                );
                break;
            }
            if pages_fetched >= MAX_PAGES {
                tracing::info!(
                    "X get_recent_posts: hit MAX_PAGES={} after {} posts",
                    MAX_PAGES, all_posts.len(),
                );
                break;
            }
            if page_post_count == 0 {
                tracing::info!(
                    "X get_recent_posts: empty page after {} posts",
                    all_posts.len(),
                );
                break;
            }

            // Small delay between pages to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        tracing::info!(
            "X get_recent_posts: returning {} posts (pages={})",
            all_posts.len(), pages_fetched,
        );
        Ok(all_posts)
    }

    async fn get_post_engagement(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        let detail = self.tweet_detail(access_token, platform_post_id).await?;
        let metrics = detail
            .get("data")
            .and_then(|d| d.get("public_metrics"))
            .or_else(|| {
                // GraphQL path: extract from tweet result
                detail
                    .get("data")
                    .and_then(|d| d.get("legacy"))
                    .and_then(|l| l.get("public_metrics"))
            })
            .cloned();
        Ok(metrics.map(|m| serde_json::json!({ "public_metrics": m })))
    }
}

// ════════════════════════════════════════════════════════════════
// Public API Methods (called from MCP tools)
// ════════════════════════════════════════════════════════════════

impl XProvider {
    /// Helper: dispatch to GraphQL or v2 based on token type.
    /// GraphQL path is preferred when cookie creds are available.
    async fn gql_or_v2_get<F>(
        &self,
        access_token: &str,
        gql_op: &str,
        gql_vars: &serde_json::Value,
        v2_url: &str,
        v2_construct: F,
    ) -> Result<serde_json::Value, ProviderError>
    where
        F: FnOnce(serde_json::Value) -> serde_json::Value,
    {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let qid = FALLBACK_QUERY_IDS
                .get(gql_op)
                .ok_or_else(|| ProviderError::Api(format!("Missing {gql_op} queryId")))?;
            let result = self.graphql_get(qid, gql_op, gql_vars, access_token).await?;
            return Ok(v2_construct(result));
        }
        self.v2_get(v2_url, access_token).await
    }

    // ── User / Profile ───────────────────────────────────────

    pub async fn get_me(
        &self,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_auth_token, _ct0)) = Self::parse_cookie_token(access_token) {
            let url = "https://x.com/i/api/1.1/account/multi/list.json";
            let (ct0, cs) = self.effective_cookie_str(access_token);
            let mut last_err = None;
            for attempt in 0..=RATE_LIMIT_RETRIES {
                if attempt > 0 { rate_limit_sleep(attempt - 1).await; }
                let resp = self.http.get(url)
                    .header("x-csrf-token", &ct0)
                    .header("Cookie", &cs)
                    .send().await
                    .map_err(|e| ProviderError::Api(format!("X whoami error: {e}")))?;
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    // ... parse and return below
                    let json: serde_json::Value = serde_json::from_str(&body_text)
                        .map_err(|e| ProviderError::Api(format!("JSON parse: {e}: {}", &body_text.chars().take(100).collect::<String>())))?;
                    let user = json["users"].as_array()
                        .and_then(|arr| arr.first())
                        .ok_or_else(|| ProviderError::Api("No user found".into()))?;
                    let screen_name = user["screen_name"].as_str().unwrap_or("");
                    let user_id = user["user_id"].as_str().unwrap_or("");
                    if screen_name.is_empty() && user_id.is_empty() {
                        return Ok(json);
                    }
                    let vars = serde_json::json!({"screen_name": screen_name, "withSafetyModeUserFields": true});
                    let qid = FALLBACK_QUERY_IDS.get("UserByScreenName")
                        .ok_or_else(|| ProviderError::Api("Missing UserByScreenName queryId".into()))?;
                    let profile = self.graphql_get(qid, "UserByScreenName", &vars, access_token).await?;
                    let data = profile.pointer("/data/user/result").and_then(|r| r.get("legacy")).or_else(|| profile.get("data"));
                    let name = data.and_then(|d| d.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                    let username = data.and_then(|d| d.get("screen_name")).and_then(|n| n.as_str()).unwrap_or("");
                    let avatar = profile.pointer("/data/user/result/legacy/profile_image_url_https")
                        .or_else(|| profile.pointer("/data/user/result/avatar/image_url")).and_then(|u| u.as_str()).unwrap_or("");
                    let description = data.and_then(|d| d.get("description")).and_then(|d| d.as_str()).unwrap_or("");
                    let followers = data.and_then(|d| d.get("followers_count")).and_then(|c| c.as_u64()).unwrap_or(0);
                    let following = data.and_then(|d| d.get("friends_count")).and_then(|c| c.as_u64()).unwrap_or(0);
                    return Ok(serde_json::json!({"data": {"id": user_id, "name": name, "username": username, "profile_image_url": avatar, "description": description, "public_metrics": {"followers_count": followers, "following_count": following}}}));
                }
                if status.as_u16() == 429 && is_rate_limited(&body_text) && attempt < RATE_LIMIT_RETRIES {
                    last_err = Some(format!("rate limited (attempt {})", attempt + 1));
                    continue;
                }
                let snippet = body_text.chars().take(200).collect::<String>();
                return Err(ProviderError::Api(format!("X whoami HTTP {status}: {snippet}")));
            }
            return Err(ProviderError::Api(format!("X whoami failed after {} retries: {}", RATE_LIMIT_RETRIES, last_err.unwrap_or_default())));
        }
        // Fallback: v2 REST API (OAuth Bearer) — for non-cookie auth
        let v2_url = "https://api.twitter.com/2/users/me?user.fields=profile_image_url,description,public_metrics".to_string();
        self.v2_get(&v2_url, access_token).await
    }

    pub async fn user_lookup(
        &self,
        access_token: &str,
        user_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            // Cookie auth: use x.com internal v2 API with cookie headers
            let url = format!(
                "https://x.com/i/api/2/users/{user_id}.json?user.fields=profile_image_url,description,public_metrics"
            );
            let (ct0, cs) = self.effective_cookie_str(access_token);
            let mut last_err = None;
            for attempt in 0..=RATE_LIMIT_RETRIES {
                if attempt > 0 { rate_limit_sleep(attempt - 1).await; }
                let resp = self.http.get(&url)
                    .header("x-csrf-token", &ct0)
                    .header("Cookie", &cs)
                    .send().await
                    .map_err(|e| ProviderError::Api(format!("X user lookup error: {e}")))?;
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    let json: serde_json::Value = serde_json::from_str(&body_text)
                        .map_err(|e| ProviderError::Api(format!("X JSON parse error: {e}")))?;
                    return Ok(json);
                }
                if status.as_u16() == 429 && is_rate_limited(&body_text) && attempt < RATE_LIMIT_RETRIES {
                    last_err = Some(format!("rate limited (attempt {})", attempt + 1));
                    continue;
                }
                let snippet = body_text.chars().take(200).collect::<String>();
                return Err(ProviderError::Api(format!("X user lookup failed: HTTP {status}: {snippet}")));
            }
            return Err(ProviderError::Api(format!("X user lookup failed after {} retries: {}", RATE_LIMIT_RETRIES, last_err.unwrap_or_default())));
        }
        // Fallback: v2 REST API (works with Bearer token in default_headers)
        let url = format!(
            "https://api.twitter.com/2/users/{user_id}?user.fields=profile_image_url,description,public_metrics"
        );
        self.v2_get(&url, access_token).await
    }

    pub async fn user_lookup_by_username(
        &self,
        access_token: &str,
        username: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({"screen_name": username, "withSafetyModeUserFields": true});
            let qid = FALLBACK_QUERY_IDS.get("UserByScreenName")
                .ok_or_else(|| ProviderError::Api("Missing UserByScreenName queryId".into()))?;
            let json = self.graphql_get(qid, "UserByScreenName", &vars, access_token).await?;
            let result = json.pointer("/data/user/result").cloned().unwrap_or(json);
            Ok(serde_json::json!({"data": {
                "id": result["rest_id"].as_str().unwrap_or(""),
                "name": result["legacy"]["name"].as_str().unwrap_or(""),
                "username": result["legacy"]["screen_name"].as_str().unwrap_or(""),
                "description": result["legacy"]["description"].as_str().unwrap_or(""),
                "profile_image_url": result["legacy"]["profile_image_url_https"].as_str().unwrap_or(""),
                "public_metrics": result["legacy"]["public_metrics"],
            }}))
        } else {
            let url = format!(
                "https://api.twitter.com/2/users/by/username/{username}?user.fields=profile_image_url,description,public_metrics"
            );
            self.v2_get(&url, access_token).await
        }
    }

    // ── Timeline ──────────────────────────────────────────────

    pub async fn home_timeline(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "count": max_results.min(100).max(1) as u32,
                "includePromotedContent": false,
                "latestControlAvailable": true,
                "requestContext": "launch",
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("HomeLatestTimeline")
                .ok_or_else(|| ProviderError::Api("Missing HomeLatestTimeline queryId".into()))?;
            let json = self.graphql_get(qid, "HomeLatestTimeline", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let timeline = json
                .pointer("/data/home/home_timeline_urt")
                .cloned()
                .unwrap_or(json.clone());
            let mut result = serde_json::json!({ "data": timeline });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/timelines/reverse_chronological?max_results={}&tweet.fields=created_at,public_metrics,attachments&user.fields=profile_image_url&expansions=author_id,attachments.media_keys&media.fields=url,preview_image_url",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn user_tweets(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "userId": user_id,
                "count": max_results.min(100).max(1) as u32,
                "includePromotedContent": false,
                "withQuickPromoteEligibilityTweetFields": true,
                "withVoice": true,
                "withV2Timeline": true,
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("UserTweets")
                .ok_or_else(|| ProviderError::Api("Missing UserTweets queryId".into()))?;
            let json = self.graphql_get(qid, "UserTweets", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let timeline = json
                .pointer("/data/user/result/timeline_v2/timeline")
                .or_else(|| json.pointer("/data/user/result/timeline/timeline"))
                .cloned()
                .unwrap_or(json);
            let mut result = serde_json::json!({ "data": timeline });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/tweets?max_results={}&tweet.fields=created_at,public_metrics&expansions=author_id,attachments.media_keys&media.fields=url,preview_image_url&user.fields=profile_image_url",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn tweet_detail(
        &self,
        access_token: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({
                "focalTweetId": tweet_id,
                "count": 40,
                "referrer": "tweet",
                "with_rux_injections": false,
                "includePromotedContent": false,
                "rankingMode": "Relevance",
                "withCommunity": true,
                "withQuickPromoteEligibilityTweetFields": true,
                "withBirdwatchNotes": true,
                "withVoice": true,
            });
            let qid = FALLBACK_QUERY_IDS
                .get("TweetDetail")
                .ok_or_else(|| ProviderError::Api("Missing TweetDetail queryId".into()))?;
            let json = self.graphql_get(qid, "TweetDetail", &vars, access_token).await?;
            // Extract the focal tweet from the response
            let tweet = json
                .pointer("/data/threaded_conversation_with_injections_v2/instructions")
                .and_then(|inst| inst.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("entries"))
                .and_then(|entries| entries.as_array())
                .and_then(|arr| arr.first())
                .and_then(|entry| entry.pointer("/content/itemContent/tweet_results/result"))
                .cloned()
                .unwrap_or(json);
            Ok(serde_json::json!({ "data": tweet }))
        } else {
            let url = format!(
                "https://api.twitter.com/2/tweets/{tweet_id}?tweet.fields=created_at,public_metrics,attachments,referenced_tweets&expansions=author_id,attachments.media_keys,referenced_tweets.id&user.fields=profile_image_url&media.fields=url,preview_image_url"
            );
            self.v2_get(&url, access_token).await
        }
    }

    pub async fn search_tweets(
        &self,
        access_token: &str,
        query: &str,
        max_results: u32,
        next_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "rawQuery": query,
                "count": max_results.min(100).max(1) as u32,
                "querySource": "typed_query",
                "product": "Top",
            });
            if let Some(cursor) = next_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("SearchTimeline")
                .ok_or_else(|| ProviderError::Api("Missing SearchTimeline queryId".into()))?;
            let json = self.graphql_post(qid, "SearchTimeline", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let mut result = serde_json::json!({ "data": json });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let encoded_query = urlencoding::encode(query);
        let mut url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={encoded_query}&max_results={}&tweet.fields=created_at,public_metrics,attachments&expansions=author_id,attachments.media_keys&user.fields=profile_image_url&media.fields=url,preview_image_url",
            max_results.min(100)
        );
        if let Some(token) = next_token {
            url.push_str(&format!("&next_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    // ── Tweet CRUD ────────────────────────────────────────────

    pub async fn delete_tweet(
        &self,
        access_token: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({
                "tweet_id": tweet_id,
                "dark_request": false,
            });
            let qid = FALLBACK_QUERY_IDS
                .get("DeleteTweet")
                .ok_or_else(|| ProviderError::Api("Missing DeleteTweet queryId".into()))?;
            let json = self.graphql_post(qid, "DeleteTweet", &vars, access_token).await?;
            Self::write_delay();
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/tweets/{tweet_id}");
        self.v2_delete(&url, access_token).await
    }

    pub async fn like_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({ "tweet_id": tweet_id });
            let qid = FALLBACK_QUERY_IDS
                .get("FavoriteTweet")
                .ok_or_else(|| ProviderError::Api("Missing FavoriteTweet queryId".into()))?;
            let json = self.graphql_post(qid, "FavoriteTweet", &vars, access_token).await?;
            Self::write_delay();
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/likes");
        let body = serde_json::json!({"tweet_id": tweet_id});
        self.v2_post(&url, access_token, &body).await
    }

    pub async fn unlike_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({
                "tweet_id": tweet_id,
                "dark_request": false,
            });
            let qid = FALLBACK_QUERY_IDS
                .get("UnfavoriteTweet")
                .ok_or_else(|| ProviderError::Api("Missing UnfavoriteTweet queryId".into()))?;
            let json = self.graphql_post(qid, "UnfavoriteTweet", &vars, access_token).await?;
            Self::write_delay();
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/likes/{tweet_id}");
        self.v2_delete(&url, access_token).await
    }

    pub async fn retweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({
                "tweet_id": tweet_id,
                "dark_request": false,
            });
            let qid = FALLBACK_QUERY_IDS
                .get("CreateRetweet")
                .ok_or_else(|| ProviderError::Api("Missing CreateRetweet queryId".into()))?;
            let json = self.graphql_post(qid, "CreateRetweet", &vars, access_token).await?;
            Self::write_delay();
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/retweets");
        let body = serde_json::json!({"tweet_id": tweet_id});
        self.v2_post(&url, access_token, &body).await
    }

    pub async fn unretweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({
                "source_tweet_id": tweet_id,
                "dark_request": false,
            });
            let qid = FALLBACK_QUERY_IDS
                .get("DeleteRetweet")
                .ok_or_else(|| ProviderError::Api("Missing DeleteRetweet queryId".into()))?;
            let json = self.graphql_post(qid, "DeleteRetweet", &vars, access_token).await?;
            Self::write_delay();
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/retweets/{tweet_id}");
        self.v2_delete(&url, access_token).await
    }

    // ── Bookmarks ─────────────────────────────────────────────

    pub async fn bookmarks(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "count": max_results.min(100).max(1) as u32,
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("Bookmarks")
                .ok_or_else(|| ProviderError::Api("Missing Bookmarks queryId".into()))?;
            let json = self.graphql_get(qid, "Bookmarks", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let timeline = json
                .pointer("/data/bookmark_timeline_v2/timeline")
                .or_else(|| json.pointer("/data/bookmark_timeline/timeline"))
                .cloned()
                .unwrap_or(json);
            let mut result = serde_json::json!({ "data": timeline });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/bookmarks?max_results={}&tweet.fields=created_at,public_metrics,attachments&expansions=author_id,attachments.media_keys&media.fields=url,preview_image_url&user.fields=profile_image_url",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn bookmark_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({ "tweet_id": tweet_id });
            let qid = FALLBACK_QUERY_IDS
                .get("CreateBookmark")
                .ok_or_else(|| ProviderError::Api("Missing CreateBookmark queryId".into()))?;
            let json = self.graphql_post(qid, "CreateBookmark", &vars, access_token).await?;
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/bookmarks");
        let body = serde_json::json!({"tweet_id": tweet_id});
        self.v2_post(&url, access_token, &body).await
    }

    pub async fn unbookmark_tweet(
        &self,
        access_token: &str,
        user_id: &str,
        tweet_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let vars = serde_json::json!({ "tweet_id": tweet_id });
            let qid = FALLBACK_QUERY_IDS
                .get("DeleteBookmark")
                .ok_or_else(|| ProviderError::Api("Missing DeleteBookmark queryId".into()))?;
            let json = self.graphql_post(qid, "DeleteBookmark", &vars, access_token).await?;
            return Ok(serde_json::json!({ "data": json.get("data") }));
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/bookmarks/{tweet_id}");
        self.v2_delete(&url, access_token).await
    }

    // ── Followers / Following ─────────────────────────────────

    pub async fn followers(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "userId": user_id,
                "count": max_results.min(100).max(1) as u32,
                "includePromotedContent": false,
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("UserByScreenName")
                .ok_or_else(|| ProviderError::Api("Missing UserByScreenName queryId".into()))?;
            // Followers is a POST GraphQL query
            let json = self.graphql_post(qid, "Followers", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let mut result = serde_json::json!({ "data": json });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/followers?max_results={}&user.fields=profile_image_url,description,public_metrics",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn following(
        &self,
        access_token: &str,
        user_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "userId": user_id,
                "count": max_results.min(100).max(1) as u32,
                "includePromotedContent": false,
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("UserByScreenName")
                .ok_or_else(|| ProviderError::Api("Missing UserByScreenName queryId".into()))?;
            let json = self.graphql_post(qid, "Following", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let mut result = serde_json::json!({ "data": json });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/users/{user_id}/following?max_results={}&user.fields=profile_image_url,description,public_metrics",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn follow_user(
        &self,
        access_token: &str,
        user_id: &str,
        target_user_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            // Use REST friendships/create.json (same as twitter-cli)
            let body = format!("user_id={target_user_id}");
            let url = "https://x.com/i/api/1.1/friendships/create.json";
            let (ct0, cs) = self.effective_cookie_str(access_token);
            let resp = self.http.post(url)
                .header("x-csrf-token", &ct0)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("Cookie", &cs)
                .body(body)
                .send().await
                .map_err(|e| ProviderError::Api(format!("Follow error: {e}")))?;
            let status = resp.status();
            let json: serde_json::Value = resp.json().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            Self::write_delay();
            return if status.is_success() {
                Ok(serde_json::json!({ "data": json }))
            } else {
                Err(ProviderError::Api(format!("Follow failed: HTTP {status}")))
            };
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/following");
        let body = serde_json::json!({"target_user_id": target_user_id});
        self.v2_post(&url, access_token, &body).await
    }

    pub async fn unfollow_user(
        &self,
        access_token: &str,
        user_id: &str,
        target_user_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let body = format!("user_id={target_user_id}");
            let url = "https://x.com/i/api/1.1/friendships/destroy.json";
            let (ct0, cs) = self.effective_cookie_str(access_token);
            let resp = self.http.post(url)
                .header("x-csrf-token", &ct0)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("Cookie", &cs)
                .body(body)
                .send().await
                .map_err(|e| ProviderError::Api(format!("Unfollow error: {e}")))?;
            let status = resp.status();
            let json: serde_json::Value = resp.json().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            Self::write_delay();
            return if status.is_success() {
                Ok(serde_json::json!({ "data": json }))
            } else {
                Err(ProviderError::Api(format!("Unfollow failed: HTTP {status}")))
            };
        }
        let url = format!("https://api.twitter.com/2/users/{user_id}/following/{target_user_id}");
        self.v2_delete(&url, access_token).await
    }

    // ── Lists ─────────────────────────────────────────────────

    pub async fn list_timeline(
        &self,
        access_token: &str,
        list_id: &str,
        max_results: u32,
        pagination_token: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        if let Some((_at, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut vars = serde_json::json!({
                "listId": list_id,
                "count": max_results.min(100).max(1) as u32,
            });
            if let Some(cursor) = pagination_token {
                vars["cursor"] = serde_json::json!(cursor);
            }
            let qid = FALLBACK_QUERY_IDS
                .get("ListLatestTweetsTimeline")
                .ok_or_else(|| ProviderError::Api("Missing ListLatestTweetsTimeline queryId".into()))?;
            let json = self.graphql_get(qid, "ListLatestTweetsTimeline", &vars, access_token).await?;
            let cursor = Self::extract_next_cursor(&json);
            let timeline = json
                .pointer("/data/list/tweets_timeline/timeline")
                .cloned()
                .unwrap_or(json);
            let mut result = serde_json::json!({ "data": timeline });
            if let Some(c) = cursor {
                result["meta"] = serde_json::json!({ "next_token": c });
            }
            return Ok(result);
        }
        let mut url = format!(
            "https://api.twitter.com/2/lists/{list_id}/tweets?max_results={}&tweet.fields=created_at,public_metrics&expansions=author_id&user.fields=profile_image_url",
            max_results.min(100)
        );
        if let Some(token) = pagination_token {
            url.push_str(&format!("&pagination_token={token}"));
        }
        self.v2_get(&url, access_token).await
    }

    pub async fn reply_to_comment(
        &self,
        access_token: &str,
        comment_id: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        if let Some((_auth_token, _ct0)) = Self::parse_cookie_token(access_token) {
            let mut variables = serde_json::json!({
                "tweet_text": post.content,
                "media": {
                    "media_entities": [],
                    "possibly_sensitive": false
                },
                "semantic_annotation_ids": [],
                "dark_request": false,
                "reply": {
                    "in_reply_to_tweet_id": comment_id,
                    "exclude_reply_user_ids": []
                },
            });

            if !post.media.is_empty() {
                let media_ids = self.upload_media(access_token, &post.media).await?;
                let entities: Vec<serde_json::Value> = media_ids
                    .iter()
                    .map(|id| serde_json::json!({"media_id": id, "tagged_users": []}))
                    .collect();
                variables["media"] = serde_json::json!({
                    "media_entities": entities,
                    "possibly_sensitive": false,
                });
            }

            Self::write_delay();

            let query_id = FALLBACK_QUERY_IDS
                .get("CreateTweet")
                .ok_or_else(|| ProviderError::Api("Missing CreateTweet queryId".into()))?;
            let json = self.graphql_post(query_id, "CreateTweet", &variables, access_token).await?;

            let tweet_id = json["data"]["create_tweet"]["tweet_results"]["result"]["rest_id"]
                .as_str()
                .unwrap_or("");

            return Ok(PublishResult {
                platform_post_url: if tweet_id.is_empty() {
                    None
                } else {
                    Some(format!("https://x.com/i/status/{tweet_id}"))
                },
                platform_post_id: tweet_id.to_string(),
                status: "published".into(),
            });
        }

        let mut body = serde_json::json!({
            "text": post.content,
            "reply": { "in_reply_to_tweet_id": comment_id }
        });
        if !post.media.is_empty() {
            let media_ids = self.upload_media(access_token, &post.media).await?;
            if !media_ids.is_empty() {
                body["media"] = serde_json::json!({ "media_ids": media_ids });
            }
        }
        let json = self.v2_post("https://api.twitter.com/2/tweets", access_token, &body).await?;
        let post_id = json["data"]["id"].as_str().unwrap_or("").to_string();
        Ok(PublishResult {
            platform_post_url: Some(format!("https://twitter.com/user/status/{post_id}")),
            platform_post_id: post_id,
            status: "published".into(),
        })
    }

    pub async fn send_dm(
        &self,
        access_token: &str,
        recipient: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let body = serde_json::json!({
            "event": {
                "type": "message_create",
                "message_create": {
                    "target": {
                        "recipient_id": recipient
                    },
                    "message_data": {
                        "text": post.content
                    }
                }
            }
        });
        let json = self.v2_post("https://api.twitter.com/2/dm_conversations/with/{recipient_id}/messages", access_token, &body).await?;
        let message_id = json["data"]["dm_event_id"].as_str().unwrap_or("").to_string();
        Ok(PublishResult {
            platform_post_id: message_id,
            platform_post_url: None,
            status: "sent".into(),
        })
    }

    pub async fn get_dm_conversations(
        &self,
        access_token: &str,
        limit: u32,
    ) -> Result<Vec<super::DmConversation>, ProviderError> {
        let url = format!(
            "https://api.twitter.com/2/dm_conversations?max_results={}&dm_event.fields=created_at,message_create,text",
            limit.min(50)
        );
        let json = self.v2_get(&url, access_token).await?;
        let mut conversations = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for conv in data {
                let id = conv["dm_conversation_id"].as_str().unwrap_or("").to_string();
                let participant = conv["participants"]
                    .as_array()
                    .and_then(|p| p.first())
                    .and_then(|p| p["user_id"].as_str())
                    .unwrap_or("")
                    .to_string();
                let last_message = conv["dm_events"]
                    .as_array()
                    .and_then(|e| e.last())
                    .and_then(|e| e["text"].as_str())
                    .map(|s| s.to_string());
                let last_message_at = conv["dm_events"]
                    .as_array()
                    .and_then(|e| e.last())
                    .and_then(|e| e["created_at"].as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));
                conversations.push(super::DmConversation {
                    id,
                    participant,
                    participant_name: None,
                    participant_avatar: None,
                    last_message,
                    last_message_at,
                    unread_count: 0,
                });
            }
        }
        Ok(conversations)
    }

    pub async fn get_dm_messages(
        &self,
        access_token: &str,
        conversation_id: &str,
        limit: u32,
    ) -> Result<Vec<super::DmMessage>, ProviderError> {
        let url = format!(
            "https://api.twitter.com/2/dm_conversations/{}/messages?max_results={}&dm_event.fields=created_at,message_create,text,sender_id",
            conversation_id,
            limit.min(50)
        );
        let json = self.v2_get(&url, access_token).await?;
        let mut messages = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for msg in data {
                let id = msg["dm_event_id"].as_str().unwrap_or("").to_string();
                let sender = msg["sender_id"].as_str().unwrap_or("").to_string();
                let content = msg["text"].as_str().unwrap_or("").to_string();
                let created_at = msg["created_at"].as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now);
                messages.push(super::DmMessage {
                    id,
                    conversation_id: conversation_id.to_string(),
                    sender,
                    sender_name: None,
                    content,
                    media: vec![],
                    created_at,
                    read: true,
                });
            }
        }
        Ok(messages)
    }
}
