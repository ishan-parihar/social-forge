// ─── Telegram Provider ────────────────────────────────────────
// Uses Telegram Bot API. Code-based auth (user sends /connect {code}).

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct TelegramProvider {
    token: String,
    http: reqwest::Client,
}

impl TelegramProvider {
    pub fn new(config: &Config) -> Self {
        let (_, token) = config.provider_credentials("telegram").unwrap_or_default();
        Self {
            token,
            http: reqwest::Client::new(),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }
}

#[async_trait]
impl SocialProvider for TelegramProvider {
    fn identifier(&self) -> &'static str {
        "telegram"
    }

    fn name(&self) -> &'static str {
        "Telegram"
    }

    fn scopes(&self) -> Vec<String> {
        vec![]
    }

    fn max_content_length(&self) -> usize {
        4096
    }

    fn uses_oauth(&self) -> bool {
        false
    }

    fn one_time_token(&self) -> bool {
        true
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Html
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        // Telegram uses code-based binding, not OAuth
        Ok(AuthUrlResponse {
            url: String::new(),
        })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // Poll updates to find a message with /connect {code}
        let resp = self
            .http
            .post(self.api_url("getUpdates"))
            .json(&serde_json::json!({
                "allowed_updates": ["message", "channel_post"]
            }))
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let results = json["result"].as_array().cloned().unwrap_or_default();

        for update in &results {
            let msg = update
                .get("message")
                .or_else(|| update.get("channel_post"));

            if let Some(m) = msg {
                let text = m["text"].as_str().unwrap_or("");
                let expected = format!("/connect {code}");

                if text == expected {
                    let chat_id = m["chat"]["id"].as_i64().unwrap_or(0);
                    let chat_title = m["chat"]["title"]
                        .as_str()
                        .unwrap_or("Telegram Chat");
                    let chat_username = m["chat"]["username"]
                        .as_str()
                        .unwrap_or("");

                    return Ok(AuthToken {
                        access_token: chat_id.to_string(),
                        refresh_token: None,
                        expires_in: Some(999_999_999),
                        provider_user_id: chat_id.to_string(),
                        name: chat_title.to_string(),
                        username: chat_username.to_string(),
                        picture: None,
                    });
                }
            }
        }

        Err(ProviderError::Auth(
            "No matching /connect message found. Send /connect <code> in your Telegram chat."
                .into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth("Telegram tokens do not expire".into()))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let chat_id = access_token;
        // strip simple HTML tags for Telegram compatibility
        let text = post
            .content
            .replace("<p>", "")
            .replace("</p>", "\n")
            .replace("<strong>", "<b>")
            .replace("</strong>", "</b>");

        if post.media.is_empty() {
            let resp = self
                .http
                .post(self.api_url("sendMessage"))
                .json(&serde_json::json!({
                    "chat_id": chat_id,
                    "text": text,
                    "parse_mode": "HTML"
                }))
                .send()
                .await?;

            let json: serde_json::Value = resp.json().await?;
            let msg_id = json["result"]["message_id"].as_i64().unwrap_or(0);

            Ok(PublishResult {
                platform_post_id: msg_id.to_string(),
                platform_post_url: Some(format!(
                    "https://t.me/c/{}/{}",
                    chat_id.replace("-100", ""),
                    msg_id
                )),
                status: "published".into(),
            })
        } else if post.media.len() == 1 {
            let is_video = post.media[0].url.contains(".mp4");
            let method = if is_video { "sendVideo" } else { "sendPhoto" };
            let media_key = if is_video { "video" } else { "photo" };

            let resp = self
                .http
                .post(self.api_url(method))
                .form(&[
                    ("chat_id", chat_id),
                    (media_key, post.media[0].url.as_str()),
                    ("caption", text.as_str()),
                    ("parse_mode", "HTML"),
                ])
                .send()
                .await?;

            let json: serde_json::Value = resp.json().await?;
            let msg_id = json["result"]["message_id"].as_i64().unwrap_or(0);

            Ok(PublishResult {
                platform_post_id: msg_id.to_string(),
                platform_post_url: Some(format!(
                    "https://t.me/c/{}/{}",
                    chat_id.replace("-100", ""),
                    msg_id
                )),
                status: "published".into(),
            })
        } else {
            // Media group
            let media: Vec<serde_json::Value> = post
                .media
                .iter()
                .map(|m| serde_json::json!({"type": "photo", "media": m.url}))
                .collect();

            let resp = self
                .http
                .post(self.api_url("sendMediaGroup"))
                .json(&serde_json::json!({
                    "chat_id": chat_id,
                    "media": media
                }))
                .send()
                .await?;

            let json: serde_json::Value = resp.json().await?;
            let msg_id = json["result"][0]["message_id"].as_i64().unwrap_or(0);

            Ok(PublishResult {
                platform_post_id: msg_id.to_string(),
                platform_post_url: Some(format!(
                    "https://t.me/c/{}/{}",
                    chat_id.replace("-100", ""),
                    msg_id
                )),
                status: "published".into(),
            })
        }
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Telegram does not support page management".into(),
        ))
    }
}
