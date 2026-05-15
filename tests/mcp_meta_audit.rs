// ─── Meta MCP Full Functional Audit ──────────────────────────────────
// This test performs a cascading audit of all Facebook and Instagram tools.
// It uses the real AppState and DB to verify end-to-end functionality.

use std::sync::Arc;
use postiz_rust::config::Config;
use postiz_rust::db;
use postiz_rust::api::AppState;
use postiz_rust::realtime::Broadcaster;
use postiz_rust::social::registry::ProviderRegistry;
use postiz_rust::api::rate_limiter::AuthRateLimiter;
use rmcp::Json;

use postiz_rust::mcp::tools_facebook as fb;
use postiz_rust::mcp::tools_instagram as ig;

// ─── Test Fixtures ───────────────────────────────────────────────────

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config from .env")
}

async fn setup_state() -> AppState {
    let config = get_config();
    let db = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to DB");
    
    let broadcaster = Broadcaster::new();
    let providers = ProviderRegistry::new(&config, None, None);
    let rate_limiter = AuthRateLimiter::new(100, 60); // High limit for audit

    AppState {
        db,
        config,
        broadcast: broadcaster,
        providers,
        rate_limiter,
        token_key: None,
        telegram_client_manager: None,
        wa_client: None,
    }
}

// ─── Facebook Audit ──────────────────────────────────────────────────

#[tokio::test]
async fn audit_facebook_tools() {
    let state = setup_state().await;
    let fb_page_id = "4372074126446140"; // Golden User test page

    println!("Starting Facebook Audit for page: {}", fb_page_id);

    // 1. Feed & Post Cascade
    let feed_res = fb::handle_fb_get_feed(&state, &fb::FbGetFeedInput {
        page_id: fb_page_id.to_string(),
        limit: Some(5),
        since: None,
        until: None,
    }).await;

    match feed_res {
        Ok(Json(val)) => {
            println!("✅ fb_get_feed: Success");
            let data = val.get("data").and_then(|d| d.as_array());
            if let Some(posts) = data {
                if let Some(post) = posts.first() {
                    let post_id = post.get("id").and_then(|id| id.as_str()).unwrap_or("");
                    println!("Found post_id for cascade: {}", post_id);

                    // Cascade: Post -> Comments -> React -> Comment -> Delete
                    let get_post = fb::handle_fb_get_post(&state, &fb::FbGetPostInput {
                        page_id: fb_page_id.to_string(),
                        post_id: post_id.to_string(),
                    }).await;
                    println!("fb_get_post: {}", if get_post.is_ok() { "✅" } else { "❌" });

                    let get_comments = fb::handle_fb_get_comments(&state, &fb::FbGetCommentsInput {
                        page_id: fb_page_id.to_string(),
                        post_id: post_id.to_string(),
                    }).await;
                    println!("fb_get_comments: {}", if get_comments.is_ok() { "✅" } else { "❌" });

                    let react = fb::handle_fb_react(&state, &fb::FbReactInput {
                        page_id: fb_page_id.to_string(),
                        post_id: post_id.to_string(),
                        reaction_type: "LIKE".to_string(),
                    }).await;
                    println!("fb_react: {}", if react.is_ok() { "✅" } else { "❌" });

                    let comment = fb::handle_fb_comment(&state, &fb::FbCommentInput {
                        page_id: fb_page_id.to_string(),
                        post_id: post_id.to_string(),
                        message: "Audit test comment".to_string(),
                    }).await;
                    println!("fb_comment: {}", if comment.is_ok() { "✅" } else { "❌" });

                    let delete = fb::handle_fb_delete_post(&state, &fb::FbDeletePostInput {
                        page_id: fb_page_id.to_string(),
                        post_id: post_id.to_string(),
                    }).await;
                    println!("fb_delete_post: {}", if delete.is_ok() { "✅" } else { "❌" });
                } else {
                    println!("⚠️ Feed empty, skipping post-specific tools");
                }
            }
        }
        Err(e) => println!("❌ fb_get_feed failed: {}", e),
    }

    // Independent Tools
    let create_post = fb::handle_fb_create_post(&state, &fb::FbCreatePostInput {
        page_id: fb_page_id.to_string(),
        message: "Full Audit Test Post".to_string(),
        link: None,
    }).await;
    println!("fb_create_post: {}", if create_post.is_ok() { "✅" } else { "❌" });

    let create_photo = fb::handle_fb_create_photo(&state, &fb::FbCreatePhotoInput {
        page_id: fb_page_id.to_string(),
        url: "https://placehold.co/600x400".to_string(),
        caption: Some("Audit photo".to_string()),
    }).await;
    println!("fb_create_photo: {}", if create_photo.is_ok() { "✅" } else { "❌" });

    let create_video = fb::handle_fb_create_video(&state, &fb::FbCreateVideoInput {
        page_id: fb_page_id.to_string(),
        file_url: "https://www.w3schools.com/html/mov_bbb.mp4".to_string(),
        title: Some("Audit Video".to_string()),
        description: Some("Testing video upload".to_string()),
    }).await;
    println!("fb_create_video: {}", if create_video.is_ok() { "✅" } else { "❌" });

    let insights = fb::handle_fb_page_insights(&state, &fb::FbPageInsightsInput {
        page_id: fb_page_id.to_string(),
        metric: "page_post_engagements".to_string(),
        period: Some("week".to_string()),
        since: None,
        until: None,
    }).await;
    println!("fb_page_insights: {}", if insights.is_ok() { "✅" } else { "❌" });

    let albums = fb::handle_fb_albums(&state, &fb::FbAlbumsInput {
        page_id: fb_page_id.to_string(),
    }).await;
    println!("fb_albums: {}", if albums.is_ok() { "✅" } else { "❌" });

    let search = fb::handle_fb_search_pages(&state, &fb::FbSearchPagesInput {
        query: "Postiz".to_string(),
    }).await;
    println!("fb_search_pages: {}", if search.is_ok() { "✅" } else { "❌" });

    let convs = fb::handle_fb_conversations(&state, &fb::FbConversationsInput {
        page_id: fb_page_id.to_string(),
    }).await;
    println!("fb_conversations: {}", if convs.is_ok() { "✅" } else { "❌" });
}

// ─── Instagram Audit ──────────────────────────────────────────────────

#[tokio::test]
async fn audit_instagram_tools() {
    let state = setup_state().await;
    let ig_id = "17841400680408909"; // designaesthetics.co.in

    println!("Starting Instagram Audit for account: {}", ig_id);

    // 1. Media Cascade
    let media_res = ig::handle_ig_get_media(&state, &ig::IgGetMediaInput {
        ig_id: ig_id.to_string(),
        limit: Some(5),
    }).await;

    match media_res {
        Ok(Json(val)) => {
            println!("✅ ig_get_media: Success");
            let data = val.get("data").and_then(|d| d.as_array());
            if let Some(media) = data.and_then(|m| m.first()) {
                let media_id = media.get("id").and_then(|id| id.as_str()).unwrap_or("");
                println!("Found media_id for cascade: {}", media_id);

                let detail = ig::handle_ig_get_media_detail(&state, &ig::IgGetMediaDetailInput {
                    ig_id: ig_id.to_string(),
                    media_id: media_id.to_string(),
                }).await;
                println!("ig_get_media_detail: {}", if detail.is_ok() { "✅" } else { "❌" });

                let comments = ig::handle_ig_get_comments(&state, &ig::IgGetCommentsInput {
                    ig_id: ig_id.to_string(),
                    media_id: media_id.to_string(),
                }).await;
                println!("ig_get_comments: {}", if comments.is_ok() { "✅" } else { "❌" });

                if let Ok(Json(c_val)) = comments {
                    let c_data = c_val.get("data").and_then(|d| d.as_array());
                    if let Some(comment) = c_data.and_then(|c| c.first()) {
                        let comment_id = comment.get("id").and_then(|id| id.as_str()).unwrap_or("");
                        let reply = ig::handle_ig_reply_to_comment(&state, &ig::IgReplyToCommentInput {
                            ig_id: ig_id.to_string(),
                            comment_id: comment_id.to_string(),
                            message: "Audit reply".to_string(),
                        }).await;
                        println!("ig_reply_to_comment: {}", if reply.is_ok() { "✅" } else { "❌" });
                    } else {
                        println!("⚠️ No comments found to test reply");
                    }
                }
            } else {
                println!("⚠️ No media found in IG feed");
            }
        }
        Err(e) => println!("❌ ig_get_media failed: {}", e),
    }

    // Independent Tools
    let reels = ig::handle_ig_get_reels(&state, &ig::IgGetReelsInput {
        ig_id: ig_id.to_string(),
    }).await;
    println!("ig_get_reels: {}", if reels.is_ok() { "✅" } else { "❌" });

    let stories = ig::handle_ig_get_stories(&state, &ig::IgGetStoriesInput {
        ig_id: ig_id.to_string(),
    }).await;
    println!("ig_get_stories: {}", if stories.is_ok() { "✅" } else { "❌" });

    let followers = ig::handle_ig_get_followers(&state, &ig::IgGetFollowersInput {
        ig_id: ig_id.to_string(),
    }).await;
    println!("ig_get_followers: {}", if followers.is_ok() { "✅" } else { "❌" });

    let discovery = ig::handle_ig_business_discovery(&state, &ig::IgBusinessDiscoveryInput {
        ig_id: ig_id.to_string(),
        target_username: "instagram".to_string(),
    }).await;
    println!("ig_business_discovery: {}", if discovery.is_ok() { "✅" } else { "❌" });

    let insights = ig::handle_ig_get_insights(&state, &ig::IgGetInsightsInput {
        ig_id: ig_id.to_string(),
        metric: "reach".to_string(),
        period: Some("day".to_string()),
    }).await;
    println!("ig_get_insights: {}", if insights.is_ok() { "✅" } else { "❌" });

    let audience = ig::handle_ig_get_insights_audience(&state, &ig::IgGetInsightsAudienceInput {
        ig_id: ig_id.to_string(),
    }).await;
    println!("ig_get_insights_audience: {}", if audience.is_ok() { "✅" } else { "❌" });

    let tagged = ig::handle_ig_get_tagged(&state, &ig::IgGetTaggedInput {
        ig_id: ig_id.to_string(),
    }).await;
    println!("ig_get_tagged: {}", if tagged.is_ok() { "✅" } else { "❌" });

    // Container Cascade
    let container = ig::handle_ig_create_container(&state, &ig::IgCreateContainerInput {
        ig_id: ig_id.to_string(),
        media_type: "IMAGE".to_string(),
        media_url: "https://placehold.co/600x400".to_string(),
        caption: "Audit post".to_string(),
    }).await;

    if let Ok(Json(c_val)) = container {
        let creation_id = c_val.get("data").and_then(|d| d.as_str()).unwrap_or("");
        if !creation_id.is_empty() {
            let publish = ig::handle_ig_publish_container(&state, &ig::IgPublishContainerInput {
                ig_id: ig_id.to_string(),
                creation_id: creation_id.to_string(),
            }).await;
            println!("ig_publish_container: {}", if publish.is_ok() { "✅" } else { "❌" });
        } else {
            println!("⚠️ Container created but no ID returned");
        }
    } else {
        println!("ig_create_container: ❌");
    }
}
