// ─── Discord Provider ─────────────────────────────────────────
// Uses Discord Bot API with OAuth for guild access.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct DiscordProvider {
    client_id: String,
    client_secret: String,
    bot_token: String,
    http: reqwest::Client,
}

impl DiscordProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("discord").unwrap_or_default();
        let bot_token = config.discord_bot_token.clone().unwrap_or_default();
        Self {
            client_id,
            client_secret,
            bot_token,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl SocialProvider for DiscordProvider {
    fn identifier(&self) -> &'static str {
        "discord"
    }

    fn name(&self) -> &'static str {
        "Discord"
    }

    fn scopes(&self) -> Vec<String> {
        vec!["identify".into(), "guilds".into()]
    }

    fn max_content_length(&self) -> usize {
        1980
    }

    fn editor_type(&self) -> EditorType {
        EditorType::Markdown
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", "identify guilds bot"),
            ("state", state),
            ("permissions", "377957124096"),
            ("integration_type", "0"),
        ];

        let url = url::Url::parse_with_params(
            "https://discord.com/oauth2/authorize",
            &params,
        )
        .map_err(|e| ProviderError::Auth(format!("URL parse: {e}")))?;

        Ok(AuthUrlResponse { url: url.to_string() })
    }

    async fn exchange_code(
        &self,
        code: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        let auth_bytes =
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.client_id, self.client_secret),
            );
        let auth_header = format!("Basic {auth_bytes}");

        let params: Vec<(&str, &str)> = vec![
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post("https://discord.com/api/oauth2/token")
            .header("Authorization", &auth_header)
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let refresh_token = json["refresh_token"].as_str().map(String::from);
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);
        let guild_id = json["guild"]["id"].as_str().unwrap_or("").to_string();

        let me: serde_json::Value = self
            .http
            .get("https://discord.com/api/oauth2/@me")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json()
            .await?;

        let app = &me["application"];

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: guild_id,
            name: app["name"]
                .as_str()
                .unwrap_or("Discord Server")
                .to_string(),
            username: app["bot"]["username"].as_str().unwrap_or("").to_string(),
            picture: Some(format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                app["bot"]["id"].as_str().unwrap_or(""),
                app["bot"]["avatar"].as_str().unwrap_or("")
            )),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let auth_bytes =
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                format!("{}:{}", self.client_id, self.client_secret),
            );
        let auth_header = format!("Basic {auth_bytes}");

        let params: Vec<(&str, &str)> = vec![
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let resp = self
            .http
            .post("https://discord.com/api/oauth2/token")
            .header("Authorization", &auth_header)
            .form(&params)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        Ok(AuthToken {
            access_token: json["access_token"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            refresh_token: json["refresh_token"].as_str().map(String::from),
            expires_in: json["expires_in"].as_u64().map(|v| v as u32),
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let channels: serde_json::Value = self
            .http
            .get(format!(
                "https://discord.com/api/guilds/{access_token}/channels"
            ))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await?
            .json()
            .await?;

        Ok(channels
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|c| {
                        c["type"].as_i64() == Some(0)
                            || c["type"].as_i64() == Some(5)
                            || c["type"].as_i64() == Some(15)
                    })
                    .map(|c| PageInfo {
                        id: c["id"].as_str().unwrap_or("").to_string(),
                        name: c["name"].as_str().unwrap_or("").to_string(),
                        access_token: None,
                        picture: None,
                        username: None,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api(
            "Use pages() to list Discord channels".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let channel_id = post.settings["channel"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidRequest("Missing channel in settings".into()))?;

        let mut payload = serde_json::json!({
            "content": post.content
        });

        if !post.media.is_empty() {
            payload["embeds"] = serde_json::json!([{
                "image": { "url": post.media[0].url }
            }]);
        }

        let resp = self
            .http
            .post(format!(
                "https://discord.com/api/channels/{channel_id}/messages"
            ))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&payload)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;

        Ok(PublishResult {
            platform_post_id: json["id"].as_str().unwrap_or("").to_string(),
            platform_post_url: Some(format!(
                "https://discord.com/channels/{access_token}/{channel_id}/{}",
                json["id"].as_str().unwrap_or("")
            )),
            status: "published".into(),
        })
    }

    fn map_error(&self, body: &str, _status: u16) -> Option<String> {
        if body.contains("50001") {
            Some("Bot doesn't have access to this channel".into())
        } else if body.contains("50013") {
            Some("Bot lacks permission to send messages".into())
        } else if body.contains("10003") {
            Some("Channel no longer exists".into())
        } else {
            None
        }
    }
}
