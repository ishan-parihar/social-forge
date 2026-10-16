// ─── Google OAuth (shared) ─────────────────────────────────────
// F2: the ONE Google OAuth2 refresh implementation. Every Google-family
// provider (google, gmail, calendar, drive, youtube, google_my_business)
// delegates here — previously each carried a byte-identical copy that
// drifted independently (some surfaced error_description, some didn't).

use super::{AuthToken, ProviderError};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// The exact form body every Google-family refresh sends. Extracted as a
/// pure function so the shared argument shape is testable offline.
pub fn refresh_params<'a>(
    refresh_token: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
    ]
}

/// Refresh a Google OAuth2 access token. Returns the new access token;
/// the refresh token itself is Google-issued and long-lived, so it is
/// passed back unchanged.
pub async fn refresh(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<AuthToken, ProviderError> {
    let resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&refresh_params(refresh_token, client_id, client_secret))
        .send()
        .await?;

    let status = resp.status();
    let json: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        let msg = json["error_description"]
            .as_str()
            .or_else(|| json["error"].as_str())
            .unwrap_or("Token refresh failed")
            .to_string();
        return Err(ProviderError::Auth(msg));
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_params_is_the_google_grant_shape() {
        let params = refresh_params("rt", "cid", "sec");
        assert_eq!(
            params,
            vec![
                ("refresh_token", "rt"),
                ("client_id", "cid"),
                ("client_secret", "sec"),
                ("grant_type", "refresh_token"),
            ]
        );
    }
}
