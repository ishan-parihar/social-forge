// ─── Telegram Bot Provider (Multi-Account) ───────────────────────

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct TelegramBotProvider {
    tokens: Vec<String>,
    http: reqwest::Client,
}

impl TelegramBotProvider {
    pub fn new(config: &Config) -> Self {
        let tokens: Vec<String> = config
            .provider_credentials("telegram-bot")
            .map(|(_, raw_tokens)| {
                raw_tokens
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            tokens,
            http: reqwest::Client::new(),
        }
    }

    fn api_url_for(&self, token: &str, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", token, method)
    }

    fn primary_api_url(&self, method: &str) -> Option<String> {
        self.tokens.first().map(|t| self.api_url_for(t, method))
    }
}

#[async_trait]
impl SocialProvider for TelegramBotProvider {
    fn identifier(&self) -> &'static str {
        "telegram-bot"
    }

    fn name(&self) -> &'static str {
        "Telegram Bot"
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
        let expected = format!("/connect {}", code);

        for token in &self.tokens {
            let resp = self
                .http
                .post(self.api_url_for(token, "getUpdates"))
                .json(&serde_json::json!({
                    "allowed_updates": ["message", "channel_post"]
                }))
                .send()
                .await?;

            let json: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };

            let results = json["result"].as_array().cloned().unwrap_or_default();

            for update in &results {
                let msg = update
                    .get("message")
                    .or_else(|| update.get("channel_post"));

                if let Some(m) = msg {
                    let text = m["text"].as_str().unwrap_or("");

                    if text == expected {
                        let chat_id = m["chat"]["id"].as_i64().unwrap_or(0);
                        let chat_title = m["chat"]["title"]
                            .as_str()
                            .unwrap_or("Telegram Bot Chat");
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
        }

        Err(ProviderError::Auth(
            "No matching /connect message found. Send /connect <code> to any configured Telegram bot."
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
        let text = post
            .content
            .replace("<p>", "")
            .replace("</p>", "\n")
            .replace("<strong>", "<b>")
            .replace("</strong>", "</b>");

        if post.media.is_empty() {
            let api_url = self
                .primary_api_url("sendMessage")
                .ok_or_else(|| ProviderError::Api("No Telegram bot token configured".into()))?;

            let resp = self
                .http
                .post(&api_url)
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
                    "https://t.me/c/{}",
                    chat_id.replace("-100", ""),
                )),
                status: "published".into(),
            })
        } else if post.media.len() == 1 {
            let is_video = post.media[0].url.contains(".mp4");
            let method = if is_video { "sendVideo" } else { "sendPhoto" };
            let media_key = if is_video { "video" } else { "photo" };

            let api_url = self
                .primary_api_url(method)
                .ok_or_else(|| ProviderError::Api("No Telegram bot token configured".into()))?;

            let resp = self
                .http
                .post(&api_url)
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
                    "https://t.me/c/{}",
                    chat_id.replace("-100", ""),
                )),
                status: "published".into(),
            })
        } else {
            let media: Vec<serde_json::Value> = post
                .media
                .iter()
                .map(|m| serde_json::json!({"type": "photo", "media": m.url }))
                .collect();

            let api_url = self
                .primary_api_url("sendMediaGroup")
                .ok_or_else(|| ProviderError::Api("No Telegram bot token configured".into()))?;

            let resp = self
                .http
                .post(&api_url)
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
                    "https://t.me/c/{}",
                    chat_id.replace("-100", ""),
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