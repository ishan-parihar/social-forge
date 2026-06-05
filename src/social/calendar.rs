// ─── Google Calendar Provider ─────────────────────────────────
// Uses Google OAuth 2.0 + Google Calendar API v3.
// Reuses YouTube client credentials from YOUTUBE_CLIENT_ID / YOUTUBE_CLIENT_SECRET.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct CalendarProvider {
    client_id: String,
    client_secret: String,
    http: reqwest::Client,
}

impl CalendarProvider {
    pub fn new(config: &Config) -> Self {
        let (client_id, client_secret) =
            config.provider_credentials("youtube").unwrap_or_default();
        Self {
            client_id,
            client_secret,
            http: reqwest::Client::new(),
        }
    }

    /// List upcoming events from a calendar.
    pub async fn list_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        max_results: u32,
        time_min: Option<&str>,
        time_max: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let mr = max_results.clamp(1, 2500).to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("maxResults", &mr),
            ("orderBy", "startTime"),
            ("singleEvents", "true"),
        ];
        if let Some(t) = time_min {
            params.push(("timeMin", t));
        }
        if let Some(t) = time_max {
            params.push(("timeMax", t));
        }
        let resp = self
            .http
            .get(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events"
            ))
            .query(&params)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Create a new calendar event.
    pub async fn create_event(
        &self,
        access_token: &str,
        calendar_id: &str,
        summary: &str,
        start_time: &str,
        end_time: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let mut body = serde_json::json!({
            "summary": summary,
            "start": { "dateTime": start_time, "timeZone": "UTC" },
            "end": { "dateTime": end_time, "timeZone": "UTC" },
        });
        if let Some(desc) = description {
            body["description"] = serde_json::json!(desc);
        }
        let resp = self
            .http
            .post(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events"
            ))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Get a single event by ID.
    pub async fn get_event(
        &self,
        access_token: &str,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events/{event_id}"
            ))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// List all calendars the user has access to.
    pub async fn list_calendars(
        &self,
        access_token: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Update an event's summary and/or description.
    pub async fn update_event(
        &self,
        access_token: &str,
        calendar_id: &str,
        event_id: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<serde_json::Value, ProviderError> {
        let mut body = serde_json::json!({});
        if let Some(s) = summary {
            body["summary"] = serde_json::json!(s);
        }
        if let Some(d) = description {
            body["description"] = serde_json::json!(d);
        }
        let resp = self
            .http
            .patch(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events/{event_id}"
            ))
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(json)
        } else if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }

    /// Delete an event.
    pub async fn delete_event(
        &self,
        access_token: &str,
        calendar_id: &str,
        event_id: &str,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .http
            .delete(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{calendar_id}/events/{event_id}"
            ))
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() || status == 404 {
            return Ok(serde_json::json!({"deleted": true}));
        }
        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        if status == 401 {
            Err(ProviderError::TokenExpired)
        } else {
            let msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Err(ProviderError::Api(msg))
        }
    }
}

#[async_trait]
impl SocialProvider for CalendarProvider {
    fn identifier(&self) -> &'static str {
        "calendar"
    }

    fn name(&self) -> &'static str {
        "Google Calendar"
    }

    fn scopes(&self) -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/calendar.readonly".into(),
            "https://www.googleapis.com/auth/calendar.events".into(),
        ]
    }

    fn max_content_length(&self) -> usize {
        0
    }

    fn needs_cron_refresh(&self) -> bool {
        true
    }

    async fn generate_auth_url(
        &self,
        state: &str,
        _code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        let scope = self.scopes().join(" ");
        let params: Vec<(&str, &str)> = vec![
            ("response_type", "code"),
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", redirect_uri),
            ("scope", scope.as_str()),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ];
        let url = url::Url::parse_with_params(
            "https://accounts.google.com/o/oauth2/v2/auth",
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
        let params: Vec<(&str, &str)> = vec![
            ("code", code),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];
        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
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

        // Get user info
        let user: serde_json::Value = self
            .http
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(AuthToken {
            access_token,
            refresh_token,
            expires_in,
            provider_user_id: user["id"].as_str().unwrap_or("").to_string(),
            name: user["name"].as_str().unwrap_or("").to_string(),
            username: user["email"].as_str().unwrap_or("").to_string(),
            picture: user["picture"].as_str().map(String::from),
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken, ProviderError> {
        let params: Vec<(&str, &str)> = vec![
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ];
        let resp = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        let access_token = json["access_token"]
            .as_str()
            .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?
            .to_string();
        let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

        Ok(AuthToken {
            access_token,
            refresh_token: Some(refresh_token.to_string()),
            expires_in,
            provider_user_id: String::new(),
            name: String::new(),
            username: String::new(),
            picture: None,
        })
    }

    async fn publish(
        &self,
        _access_token: &str,
        _post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        Err(ProviderError::Api(
            "Calendar provider does not support publishing. Use create_event instead.".into(),
        ))
    }

    async fn fetch_page_info(
        &self,
        access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        // Fetch primary calendar info as the "page"
        let resp = self
            .http
            .get("https://www.googleapis.com/calendar/v3/users/me/calendarList/primary")
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;
        if status.is_success() {
            Ok(PageInfo {
                id: json["id"].as_str().unwrap_or("primary").to_string(),
                name: json["summary"]
                    .as_str()
                    .unwrap_or("My Calendar")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: None,
                username: json["id"].as_str().map(String::from),
            })
        } else {
            // Fallback: use userinfo
            let user: serde_json::Value = self
                .http
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .header("Authorization", format!("Bearer {access_token}"))
                .send()
                .await?
                .json()
                .await?;
            Ok(PageInfo {
                id: user["id"].as_str().unwrap_or("").to_string(),
                name: user["name"]
                    .as_str()
                    .unwrap_or("Google Calendar")
                    .to_string(),
                access_token: Some(access_token.to_string()),
                picture: user["picture"].as_str().map(String::from),
                username: user["email"].as_str().map(String::from),
            })
        }
    }

    fn map_error(&self, _body: &str, status: u16) -> Option<String> {
        if status == 401 {
            Some("Calendar token expired. Re-authenticate via Google OAuth.".into())
        } else if status == 403 {
            Some("Calendar API access forbidden. Check token scopes.".into())
        } else if status == 429 {
            Some("Calendar API rate limit exceeded. Try again later.".into())
        } else {
            None
        }
    }
}
