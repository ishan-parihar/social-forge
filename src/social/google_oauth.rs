// ─── Google OAuth (shared) ─────────────────────────────────────
// F2: the ONE Google OAuth2 refresh implementation. Every Google-family
// provider (google, gmail, calendar, drive, youtube, google_my_business)
// delegates here — previously each carried a byte-identical copy that
// drifted independently (some surfaced error_description, some didn't).

use super::common::extract_access_token;
use super::{AuthToken, ProviderError};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// The Google account's identity, as the OpenID userinfo endpoint reports it.
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

/// The exact form body every Google-family CODE EXCHANGE sends. Extracted
/// as a pure function so the shared argument shape is testable offline.
pub fn exchange_params<'a>(
    code: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    redirect_uri: &'a str,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ]
}

/// Exchange an authorization code for a Google access token AND the
/// account's identity.
///
/// The ONE Google-family code exchange. All five Google-family providers
/// (google, youtube, drive, gmail, calendar) carried a byte-identical
/// copy of this function - five copies of the same POST, the same
/// userinfo read, and the same field mapping, differing only in the
/// provider label passed to `extract_access_token` for error text. That
/// is the same failure shape the file header records for refresh: copies
/// that drift independently.
///
/// `provider` is the label for error messages ONLY. It does not change
/// which token endpoint is called or which scopes are requested - those
/// live in each provider's own `authorize_url`, which is where a
/// Google-family provider legitimately differs.
pub async fn exchange_code(
    http: &reqwest::Client,
    provider: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<AuthToken, ProviderError> {
    let resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&exchange_params(
            code,
            client_id,
            client_secret,
            redirect_uri,
        ))
        .send()
        .await?;

    let (access_token, json) = extract_access_token(resp, provider)
        .await
        .map_err(ProviderError::Auth)?;
    let refresh_token = json["refresh_token"].as_str().map(String::from);
    let expires_in = json["expires_in"].as_u64().map(|v| v as u32);

    // Who this account is. The resource endpoint each provider identifies a
    // channel by (a mailbox, a calendar, a channel) carries no name and no
    // photo, so the identity comes from the account behind it.
    let user: serde_json::Value = http
        .get(GOOGLE_USERINFO_URL)
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

/// The Google account's profile photo, as an absolute URL.
///
/// Every Google-family provider has the same problem: the resource
/// endpoint it identifies a channel by (gmail's `users/me/profile`,
/// calendar's `users/me/calendarList/primary`, GMB's account management)
/// carries the resource's NAME and no photo, because a mailbox and a
/// calendar are not things with faces. The ACCOUNT behind them has a photo,
/// published at the OpenID userinfo endpoint.
///
/// Several providers had been hardcoding `picture: None` on that path while
/// their own `exchange_code` read the very same endpoint for the initial
/// connect - so one provider answered "who is this channel" two different
/// ways, and the answer used at CONNECT time had an avatar while the answer
/// used to REFRESH the row never could. A channel connected before the
/// avatar existed stayed avatar-less for good, because refresh is the only
/// repair path and `profile_picture = COALESCE(?, profile_picture)` keeps a
/// NULL when the provider hands back nothing.
///
/// Returns `None` rather than an error: a Google account may have no photo,
/// the OpenID scopes may not have been granted, or userinfo may simply
/// fail. None of that makes the channel unusable - it still reads mail and
/// still shows events - so failing the whole profile lookup over a photo
/// would trade a cosmetic gap for a broken channel.
pub async fn profile_picture(http: &reqwest::Client, access_token: &str) -> Option<String> {
    let resp = http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json["picture"]
        .as_str()
        .filter(|p| !p.is_empty())
        .map(String::from)
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

    /// The five Google-family providers each declared their own copy of
    /// `exchange_code` before they delegated here. This pins the shared form
    /// body so a future edit that re-splits the helper - or that adds a
    /// `scope` to one caller's copy alone - fails here rather than drifting
    /// the way the six refresh copies did.
    #[test]
    fn exchange_params_is_the_google_grant_shape() {
        let params = exchange_params("the_code", "cid", "sec", "https://app/api/auth/callback");
        assert_eq!(
            params,
            vec![
                ("code", "the_code"),
                ("client_id", "cid"),
                ("client_secret", "sec"),
                ("redirect_uri", "https://app/api/auth/callback"),
                ("grant_type", "authorization_code"),
            ]
        );
    }
}
