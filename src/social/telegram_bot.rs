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

    fn resolve_bot_token_and_chat(&self, access_token: &str) -> Result<(String, String), ProviderError> {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(access_token) {
            let bot_token = v["bot_token"].as_str().unwrap_or("").to_string();
            let chat_id = v["chat_id"].as_str().unwrap_or("").to_string();
            if !bot_token.is_empty() && !chat_id.is_empty() {
                return Ok((bot_token, chat_id));
            }
            // Token-only integration (no chat_id yet)
            if !bot_token.is_empty() {
                return Err(ProviderError::Api("This bot integration has no target chat. Use the /connect flow to link a chat.".into()));
            }
        }
        // Legacy: access_token is just chat_id, use first env token
        let bot_token = self.tokens.first()
            .ok_or_else(|| ProviderError::Api("No Telegram bot token configured".into()))?
            .clone();
        Ok((bot_token, access_token.to_string()))
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

                        // Store bot_token + chat_id as JSON
                        let access_token = serde_json::json!({
                            "bot_token": token,
                            "chat_id": chat_id.to_string(),
                        }).to_string();

                        return Ok(AuthToken {
                            access_token,
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
        let (bot_token, chat_id) = self.resolve_bot_token_and_chat(access_token)?;

        let text = post
            .content
            .replace("<p>", "")
            .replace("</p>", "\n")
            .replace("<strong>", "<b>")
            .replace("</strong>", "</b>");

        if post.media.is_empty() {
            let resp = self
                .http
                .post(self.api_url_for(&bot_token, "sendMessage"))
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

            let resp = self
                .http
                .post(self.api_url_for(&bot_token, method))
                .form(&[
                    ("chat_id", chat_id.as_str()),
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

            let resp = self
                .http
                .post(self.api_url_for(&bot_token, "sendMediaGroup"))
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
