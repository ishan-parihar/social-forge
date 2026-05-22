// ─── Slack Provider ──────────────────────────────────────────
// Uses Slack OAuth 2.0 + Slack Web API (chat.postMessage, conversations.*, users.*).
// Supports: OAuth flow, message sending, channel listing, conversation history, user listing.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

const SLACK_API_BASE: &str = "https://slack.com/api";

pub struct SlackProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl SlackProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("slack").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    /// Helper: check Slack API response for "ok": false and map the error.
    fn check_slack_response(&self, json: &serde_json::Value) -> Result<(), ProviderError> {
        if json.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            let error = json["error"]
                .as_str()
                .unwrap_or("unknown_error")
                .to_string();
            let detail = json["response_metadata"]
                .as_object()
                .and_then(|m| m.get("messages"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .map(|s| format!(": {s}"))
                .unwrap_or_default();
            return Err(ProviderError::Api(format!("Slack API error: {error}{detail}")));
        }
        Ok(())
    }

    /// List all channels in the workspace (public channels).
    pub async fn get_channel_list(
        &self,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{SLACK_API_BASE}/conversations.list"))
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("limit", "200"), ("types", "public_channel,private_channel")])
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            self.check_slack_response(&json)?;
            Ok(json)
        } else {
            Err(ProviderError::Api(format!(
                "Slack conversations.list failed ({}): {}",
                status,
                json["error"].as_str().unwrap_or("unknown")
            )))
        }
    }

    /// Get conversation history for a channel.
    pub async fn get_conversation_history(
        &self,
        access_token: &str,
        channel: &str,
        limit: i32,
    ) -> Result<serde_json::Value, ProviderError> {
        let clamped = limit.clamp(1, 200);
        let resp = self
            .http
            .get(format!("{SLACK_API_BASE}/conversations.history"))
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[
                ("channel", channel),
                ("limit", &clamped.to_string()),
            ])
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            self.check_slack_response(&json)?;
            Ok(json)
        } else {
            Err(ProviderError::Api(format!(
                "Slack conversations.history failed ({}): {}",
                status,
                json["error"].as_str().unwrap_or("unknown")
            )))
        }
    }

    /// List workspace users.
    pub async fn get_user_list(
        &self,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!("{SLACK_API_BASE}/users.list"))
            .header("Authorization", format!("Bearer {access_token}"))
            .query(&[("limit", "200")])
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() {
            self.check_slack_response(&json)?;
            Ok(json)
        } else {
            Err(ProviderError::Api(format!(
                "Slack users.list failed ({}): {}",
                status,
                json["error"].as_str().unwrap_or("unknown")
            )))
        }
    }
}

#[async_trait]
impl SocialProvider for SlackProvider {
    fn identifier(&self) -> &'static str {
        "slack"
    }

    fn name(&self) -> &'static str {
        "Slack"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "channels:history".into(),
            "channels:read".into(),
            "channels:manage".into(),
            "chat:write".into(),
            "chat:write.customize".into(),
            "users:read".into(),
            "users:read.email".into(),
            "team:read".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        40000
    }

    fn uses_oauth(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = self.scopes().join(",");
        let params: Vec<(&str, &str)> = vec![
            ("client_id", self.client_id.as_str()),
            ("scope", scope.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", state),
        ];

        let url = url::Url::parse_with_params(
            "https://slack.com/oauth/v2/authorize",
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
        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .http
            .post(format!("{SLACK_API_BASE}/oauth.v2.access"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if !status.is_success() || json.get("ok").and_then(|v| v.as_bool()) != Some(true) {
            let error = json["error"]
                .as_str()
                .unwrap_or("Token exchange failed")
                .to_string();
            return Err(ProviderError::Auth(error));
        }

        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();

        let team_name = json["team"]["name"]
            .as_str()
            .unwrap_or("Slack Workspace")
            .to_string();

        let team_id = json["team"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let authed_user_id = json["authed_user"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(AuthToken {
            access_token,
            refresh_token: None,
            expires_in: None,
            provider_user_id: authed_user_id,
            name: team_name,
            username: team_id,
            picture: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<AuthToken, ProviderError> {
        Err(ProviderError::Auth(
            "Slack tokens do not expire. Re-connect if your token is revoked.".into(),
        ))
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // Extract channel from settings, default to "#general"
        let channel = post
            .settings
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or("#general")
            .to_string();

        let body = serde_json::json!({
            "channel": channel,
            "text": post.content,
            "mrkdwn": true,
        });

        let resp = self
            .http
            .post(format!("{SLACK_API_BASE}/chat.postMessage"))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() && json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            let ts = json["ts"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let permalink = json.get("message")
                .and_then(|m| m.get("permalink"))
                .or_else(|| json.get("permalink"))
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(PublishResult {
                platform_post_id: ts,
                platform_post_url: permalink,
                status: "published".into(),
            })
        } else {
            let error = json["error"]
                .as_str()
                .unwrap_or("Slack publish failed")
                .to_string();
            Err(ProviderError::Api(error))
        }
    }

    async fn comment(
        &self,
        access_token: &str,
        channel_ts: &str,
        _last_comment_id: Option<&str>,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        // When commenting on a message, the channel_ts is actually the channel ID
        // stored as platform_post_id. Actually in Slack, to reply in a thread,
        // we need the channel ID and the parent message timestamp (thread_ts).
        // The channel_ts param is being reused as the channel name/ID for simplicity.
        let channel = post
            .settings
            .get("channel")
            .and_then(|v| v.as_str())
            .unwrap_or(channel_ts);

        // The thread_ts is the parent message timestamp — defaults to channel_ts if provided
        let thread_ts = post.settings.get("thread_ts")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut body = serde_json::json!({
            "channel": channel,
            "text": post.content,
            "mrkdwn": true,
        });

        // If thread_ts is set, reply in thread
        if !thread_ts.is_empty() {
            body["thread_ts"] = serde_json::Value::String(thread_ts.to_string());
        }

        let resp = self
            .http
            .post(format!("{SLACK_API_BASE}/chat.postMessage"))
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() && json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            let ts = json["ts"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let permalink = json.get("message")
                .and_then(|m| m.get("permalink"))
                .or_else(|| json.get("permalink"))
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(PublishResult {
                platform_post_id: ts,
                platform_post_url: permalink,
                status: "published".into(),
            })
        } else {
            let error = json["error"]
                .as_str()
                .unwrap_or("Slack comment failed")
                .to_string();
            Err(ProviderError::Api(error))
        }
    }

    /// Return workspace info as a single page.
    async fn pages(&self, access_token: &str) -> Result<Vec<PageInfo>, ProviderError> {
        let resp = self
            .http
            .get(format!("{SLACK_API_BASE}/team.info"))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status.is_success() && json.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            let team = &json["team"];
            Ok(vec![PageInfo {
                id: team["id"].as_str().unwrap_or("").to_string(),
                name: team["name"].as_str().unwrap_or("Slack Workspace").to_string(),
                access_token: Some(access_token.to_string()),
                picture: team["icon"]["image_132"]
                    .as_str()
                    .map(String::from)
                    .or_else(|| team["icon"]["image_72"].as_str().map(String::from)),
                username: team["domain"].as_str().map(String::from),
            }])
        } else {
            let error = json["error"]
                .as_str()
                .unwrap_or("Failed to fetch Slack team info")
                .to_string();
            Err(ProviderError::Api(error))
        }
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        let mut pages = self.pages(access_token).await?;
        pages.pop().ok_or_else(|| {
            ProviderError::Api("No Slack workspace info available".into())
        })
    }

    fn map_error(&self, body: &str, _status: u16) -> Option<String> {
        // Try to parse Slack API JSON error response
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            if json.get("ok").and_then(|v| v.as_bool()) == Some(false) {
                let error = json["error"].as_str().unwrap_or("unknown_error");
                let hint = match error {
                    "not_authed" => "Invalid Slack token. Re-connect your Slack account.",
                    "invalid_auth" => "Invalid Slack token. Re-connect your Slack account.",
                    "account_inactive" => "Slack account is inactive. Re-connect.",
                    "token_revoked" => "Slack token was revoked. Re-connect.",
                    "no_permission" => "Missing Slack permission scope. Re-install the app with required scopes.",
                    "not_in_channel" => "The bot/user is not in this Slack channel.",
                    "channel_not_found" => "Slack channel not found. Verify the channel ID or name.",
                    "is_archived" => "Cannot post to an archived Slack channel.",
                    "msg_too_long" => "Message exceeds Slack's 40,000 character limit.",
                    "rate_limited" => "Slack API rate limit exceeded. Try again later.",
                    _ => "Slack API error. Check your connection and permissions.",
                };
                return Some(hint.to_string());
            }
        }
        None
    }

    async fn targets(&self, access_token: &str) -> Result<Vec<TargetInfo>, ProviderError> {
        let channels = match self.get_channel_list(access_token).await {
            Ok(channels) => channels,
            Err(_) => return Ok(vec![]),
        };

        let channels_array = channels["channels"]
            .as_array()
            .map(|arr| arr.to_vec())
            .unwrap_or_default();

        let targets: Vec<TargetInfo> = channels_array
            .iter()
            .filter_map(|ch| {
                let id = ch["id"].as_str()?.to_string();
                let name = format!("#{}", ch["name"].as_str()?);
                let is_private = ch["is_private"].as_bool().unwrap_or(false);
                let target_type = if is_private {
                    "private_channel".to_string()
                } else {
                    "public_channel".to_string()
                };

                let metadata = serde_json::json!({
                    "member_count": ch["num_members"].as_u64(),
                    "topic": ch["topic"]["value"].as_str().map(String::from),
                    "purpose": ch["purpose"]["value"].as_str().map(String::from),
                    "is_archived": ch["is_archived"].as_bool(),
                    "is_general": ch["is_general"].as_bool(),
                });

                Some(TargetInfo {
                    id,
                    name,
                    target_type,
                    picture: None,
                    metadata: Some(metadata),
                })
            })
            .collect();

        Ok(targets)
    }
}
