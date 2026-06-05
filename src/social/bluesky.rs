// ─── Bluesky Provider ─────────────────────────────────────────
// Uses AT Protocol (ATP) with username + app password (no OAuth).
// Simplest provider to implement — ideal for MVP validation.

use async_trait::async_trait;

use super::*;
use crate::config::Config;

pub struct BlueskyProvider {
    handle: String,
    app_password: String,
    http: reqwest::Client,
}

impl BlueskyProvider {
    pub fn new(config: &Config) -> Self {
        Self {
            handle: config.bluesky_handle.clone().unwrap_or_default(),
            app_password: config.bluesky_app_password.clone().unwrap_or_default(),
            http: reqwest::Client::new(),
        }
    }

    /// Authenticate with Bluesky and get a session token (JWT)
    async fn create_session(&self) -> Result<String, ProviderError> {
        let resp = self
            .http
            .post("https://bsky.social/xrpc/com.atproto.server.createSession")
            .json(&serde_json::json!({
                "identifier": self.handle,
                "password": self.app_password,
            }))
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 200 {
            json["accessJwt"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| ProviderError::Auth("Missing accessJwt".into()))
        } else {
            Err(ProviderError::Auth(
                json["message"]
                    .as_str()
                    .unwrap_or("Bluesky auth failed")
                    .to_string(),
            ))
        }
    }

    /// Get the user's DID (Decentralized Identifier)
    async fn resolve_handle(&self) -> Result<String, ProviderError> {
        let resp = self
            .http
            .get("https://bsky.social/xrpc/com.atproto.identity.resolveHandle")
            .query(&[("handle", &self.handle)])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        json["did"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| ProviderError::Auth("Could not resolve handle".into()))
    }
}

#[async_trait]
impl SocialProvider for BlueskyProvider {
    fn identifier(&self) -> &'static str {
        "bluesky"
    }

    fn name(&self) -> &'static str {
        "Bluesky"
    }

    fn scopes(&self) -> Vec<String> {
        vec![] // Bluesky uses app password, not OAuth scopes
    }

    fn max_content_length(&self) -> usize {
        300
    }

    /// Bluesky doesn't use OAuth; this returns an error instructing the user
    fn uses_oauth(&self) -> bool {
        false // Bluesky uses app passwords instead of OAuth
    }

    async fn generate_auth_url(
        &self,
        _state: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthUrlResponse, ProviderError> {
        Err(ProviderError::Auth(
            "Bluesky uses app passwords instead of OAuth. \
             The agent will guide you through setting BLUESKY_HANDLE and \
             BLUESKY_APP_PASSWORD in your .env file."
                .into(),
        ))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<AuthToken, ProviderError> {
        // Auto-connect: create session and return token
        let session_jwt = self.create_session().await?;
        let did = self.resolve_handle().await?;

        Ok(AuthToken {
            access_token: session_jwt,
            refresh_token: None,
            expires_in: Some(7200), // 2 hours
            provider_user_id: did,
            name: self.handle.clone(),
            username: self.handle.clone(),
            picture: None,
        })
    }

    async fn refresh_token(
        &self,
        _refresh_token: &str,
    ) -> Result<AuthToken, ProviderError> {
        // Bluesky sessions are short-lived; just create a new one
        self.exchange_code("", "", "").await
    }

    async fn publish(
        &self,
        access_token: &str,
        post: &PostContent,
    ) -> Result<PublishResult, ProviderError> {
        let did = self.resolve_handle().await?;

        // Build the Bluesky post record
        let mut record = serde_json::json!({
            "$type": "app.bsky.feed.post",
            "createdAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "text": post.content,
        });

        // Add embed if media present
        if !post.media.is_empty() {
            if let Ok(embed) = self.upload_and_embed(access_token, &post.media).await {
                record["embed"] = embed;
            }
        }

        let body = serde_json::json!({
            "repo": did,
            "collection": "app.bsky.feed.post",
            "record": record,
        });

        let resp = self
            .http
            .post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = resp.json().await?;

        if status == 200 {
            let uri = json["uri"].as_str().unwrap_or("").to_string();
            let post_id = json["cid"].as_str().unwrap_or("").to_string();
            let at_uri = format!("https://bsky.app/profile/{}/post/{}",
                self.handle,
                uri.rsplit('/').next().unwrap_or(""));
            Ok(PublishResult {
                platform_post_id: post_id,
                platform_post_url: Some(at_uri),
                status: "published".into(),
            })
        } else {
            Err(ProviderError::Api(
                json["message"]
                    .as_str()
                    .unwrap_or("Bluesky publish failed")
                    .to_string(),
            ))
        }
    }

    async fn fetch_page_info(
        &self,
        _access_token: &str,
        _page_id: &str,
    ) -> Result<PageInfo, ProviderError> {
        Err(ProviderError::Api("Bluesky does not support page management".into()))
    }

    async fn get_recent_posts(
        &self,
        access_token: &str,
        _internal_id: &str,
        limit: u32,
    ) -> Result<Vec<ExternalPostData>, ProviderError> {
        // Use the passed JWT directly, or create a fresh session if expired
        let jwt = if access_token.is_empty() { self.create_session().await? } else { access_token.to_string() };
        let did = self.resolve_handle().await?;

        let resp = self.http
            .get("https://bsky.social/xrpc/app.bsky.feed.getAuthorFeed")
            .header("Authorization", format!("Bearer {jwt}"))
            .query(&[("actor", &did), ("limit", &limit.to_string())])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let feed = json["feed"].as_array().map(|a| a.to_vec()).unwrap_or_default();

        let mut posts = Vec::new();
        for item in &feed {
            let post = &item["post"];
            let record = &post["record"];

            let cid = post["cid"].as_str().unwrap_or("").to_string();
            if cid.is_empty() { continue; }

            let text = record["text"].as_str().unwrap_or("").to_string();
            let created_at = record["createdAt"].as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now);

            let author = &post["author"];
            let author_name = author["displayName"].as_str().map(String::from);
            let author_handle = author["handle"].as_str().map(String::from);
            let uri = post["uri"].as_str().map(String::from);

            let mut media = Vec::new();
            if let Some(embed) = post.get("embed") {
                let embed_type = embed["$type"].as_str().unwrap_or("");
                if embed_type == "app.bsky.embed.video" || embed_type == "app.bsky.embed.video#view" {
                    // Bluesky video — resolve blob reference to a playable URL
                    if let Some(video) = embed.get("video") {
                        let mime = video["mimeType"].as_str().unwrap_or("video/mp4");
                        // Try direct playlist URL first (video.playlist)
                        if let Some(playlist) = video["playlist"].as_str() {
                            media.push(MediaAttachment {
                                url: playlist.to_string(),
                                mime_type: mime.to_string(),
                                alt: None,
                            });
                        } else if let Some(cid) = video["ref"]["$link"].as_str()
                            .or_else(|| video["ref"]["cid"].as_str())
                        {
                            // Construct blob URL from DID + CID
                            let did = post["author"]["did"]
                                .as_str()
                                .unwrap_or("");
                            let blob_url = format!(
                                "https://bsky.social/xrpc/com.atproto.sync.getBlob?did={did}&cid={cid}"
                            );
                            media.push(MediaAttachment {
                                url: blob_url,
                                mime_type: mime.to_string(),
                                alt: None,
                            });
                        }
                    }
                } else if let Some(images) = embed["images"].as_array() {
                    for img in images {
                        if let Some(url) = img["fullsize"].as_str().or_else(|| img["thumb"].as_str()) {
                            media.push(MediaAttachment {
                                url: url.to_string(),
                                mime_type: "image/jpeg".to_string(),
                                alt: img["alt"].as_str().map(String::from),
                            });
                        }
                    }
                } else if let Some(external) = embed.get("external") {
                    // External link embed — use thumbnail if available
                    if let Some(thumb) = external["thumb"].as_str() {
                        media.push(MediaAttachment {
                            url: thumb.to_string(),
                            mime_type: "image/jpeg".to_string(),
                            alt: None,
                        });
                    }
                }
            }

            let author_avatar = author["avatar"].as_str().map(String::from);

            posts.push(ExternalPostData {
                platform_post_id: cid,
                text,
                author_name,
                author_handle,
                author_avatar,
                created_at,
                url: uri,
                media,
                metadata: Some(post.clone()),
            });
        }

        Ok(posts)
    }

    async fn get_post_engagement(
        &self,
        access_token: &str,
        _platform_post_id: &str,
    ) -> Result<Option<serde_json::Value>, ProviderError> {
        // Use getAuthorFeed to find the post by matching against recent posts
        let jwt = if access_token.is_empty() { self.create_session().await? } else { access_token.to_string() };
        let did = self.resolve_handle().await?;

        let resp = self.http
            .get("https://bsky.social/xrpc/app.bsky.feed.getAuthorFeed")
            .header("Authorization", format!("Bearer {jwt}"))
            .query(&[("actor", &did), ("limit", &"30".to_string())])
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        let feed = json["feed"].as_array().map(|a| a.to_vec()).unwrap_or_default();

        for item in &feed {
            let post = &item["post"];
            let cid = post["cid"].as_str().unwrap_or("");
            if cid == _platform_post_id {
                let like_count = post["likeCount"].as_i64().unwrap_or(0);
                let repost_count = post["repostCount"].as_i64().unwrap_or(0);
                let reply_count = post["replyCount"].as_i64().unwrap_or(0);
                let quote_count = post["quoteCount"].as_i64().unwrap_or(0);

                let result = serde_json::json!({
                    "likeCount": like_count,
                    "repostCount": repost_count,
                    "replyCount": reply_count,
                    "quoteCount": quote_count,
                });
                return Ok(Some(result));
            }
        }

        Ok(None)
    }
}

impl BlueskyProvider {
    async fn upload_and_embed(
        &self,
        access_token: &str,
        media: &[MediaAttachment],
    ) -> Result<serde_json::Value, ProviderError> {
        // For MVP: return first image as an embed if it exists
        for item in media {
            if item.mime_type.starts_with("image/") {
                // Download image and upload to Bluesky blob store
                let img_resp = self.http.get(&item.url).send().await?;
                let img_bytes = img_resp.bytes().await?;
                let mime = &item.mime_type;

                let blob_resp = self
                    .http
                    .post("https://bsky.social/xrpc/com.atproto.repo.uploadBlob")
                    .header("Authorization", format!("Bearer {access_token}"))
                    .header("Content-Type", mime)
                    .body(img_bytes)
                    .send()
                    .await?;

                let blob_json: serde_json::Value = blob_resp.json().await?;
                let blob_ref = &blob_json["blob"];

                return Ok(serde_json::json!({
                    "$type": "app.bsky.embed.images",
                    "images": [{
                        "alt": item.alt.as_deref().unwrap_or(""),
                        "image": blob_ref,
                    }]
                }));
            }
        }
        Ok(serde_json::Value::Null)
    }
}
