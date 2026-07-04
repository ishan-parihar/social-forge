// ── LinkedIn Page CLI Handler ─────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::LinkedinPageAction;
use crate::cli::platforms::emit_result;
use crate::cli::run::{find_linkedin_page_token, resolve_user};
use crate::social::SocialProvider;

pub async fn handle(action: LinkedinPageAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;

    let result: Result<serde_json::Value, String> = match action {
        LinkedinPageAction::List => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Ok(integrations) => {
                    let pages: Vec<serde_json::Value> = integrations.iter()
                        .filter(|i| i.provider_identifier == "linkedin-page")
                        .map(|i| serde_json::json!({"id": i.internal_id, "name": i.profile_name}))
                        .collect();
                    Ok(serde_json::json!({"pages": pages}))
                }
                Err(e) => Err(format!("DB error: {e}")),
            }
        }
        LinkedinPageAction::Post { page_id, text } => {
            match find_linkedin_page_token(state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let body = serde_json::json!({
                        "author": format!("urn:li:organization:{page_id}"),
                        "commentary": text,
                        "visibility": "PUBLIC",
                        "distribution": {"feedDistribution": "MAIN_FEED"},
                        "lifecycleState": "PUBLISHED",
                    });
                    match reqwest::Client::new()
                        .post("https://api.linkedin.com/v2/rest/posts")
                        .header("Authorization", format!("Bearer {token}"))
                        .header("X-Restli-Protocol-Version", "2.0.0")
                        .header("LinkedIn-Version", "202401")
                        .header("Content-Type", "application/json")
                        .json(&body).send().await
                    {
                        Err(e) => Err(format!("LinkedIn post failed: {e}")),
                        Ok(resp) => {
                            let post_id = resp.headers().get("x-restli-id")
                                .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                            if resp.status().is_success() {
                                Ok(serde_json::json!({"post_id": post_id}))
                            } else {
                                Err(format!("LinkedIn page post failed ({})", resp.status()))
                            }
                        }
                    }
                }
            }
        }
        LinkedinPageAction::Analytics { page_id } => {
            match find_linkedin_page_token(state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let provider = crate::social::linkedin_page::LinkedInPageProvider::new(&state.config);
                    provider.analytics(&token, &page_id, 30).await
                        .map(|d| serde_json::json!({"data": d}))
                        .map_err(|e| e.to_string())
                }
            }
        }
        LinkedinPageAction::Followers { page_id } => {
            match find_linkedin_page_token(state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((token, _)) => {
                    let org_urn = format!("urn:li:organization:{page_id}");
                    let url = format!(
                        "https://api.linkedin.com/rest/networkSizes/{org_urn}?edgeType=CompanyFollowedByMember"
                    );
                    match reqwest::Client::new()
                        .get(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .header("LinkedIn-Version", "202401")
                        .header("X-Restli-Protocol-Version", "2.0.0")
                        .send().await
                    {
                        Err(e) => Err(format!("LinkedIn followers failed: {e}")),
                        Ok(resp) => match resp.json::<serde_json::Value>().await {
                            Err(e) => Err(format!("Parse error: {e}")),
                            Ok(json) => {
                                let count = json["firstDegreeSize"].as_u64().unwrap_or(0);
                                Ok(serde_json::json!({"follower_count": count}))
                            }
                        }
                    }
                }
            }
        }
        LinkedinPageAction::Page { page_id } => {
            match find_linkedin_page_token(state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((_token, lip_id)) => {
                    let input = crate::mcp::tools_linkedin_page::LipGetPageInput {
                        lip_id, page_id,
                    };
                    crate::mcp::tools_linkedin_page::handle_lip_get_page(state, &input).await
                        .map(|v| v.0).map_err(|e| e.to_string())
                }
            }
        }
        LinkedinPageAction::Feed { page_id, limit } => {
            match find_linkedin_page_token(state, user_id, &page_id).await {
                Err(e) => Err(e.to_string()),
                Ok((_token, lip_id)) => {
                    let input = crate::mcp::tools_linkedin_page::LipGetPagePostsInput {
                        lip_id, page_id, limit,
                    };
                    crate::mcp::tools_linkedin_page::handle_lip_get_page_posts(state, &input).await
                        .map(|v| v.0).map_err(|e| e.to_string())
                }
            }
        }
        LinkedinPageAction::CreateComment { post_urn, page_urn, text } => {
            match find_linkedin_page_token(state, user_id, &page_urn).await {
                Err(e) => Err(e.to_string()),
                Ok((_token, lip_id)) => {
                    let input = crate::mcp::tools_linkedin_page::LipCreateCommentInput {
                        lip_id, post_urn, page_urn, message: text,
                    };
                    crate::mcp::tools_linkedin_page::handle_lip_create_comment(state, &input).await
                        .map(|v| v.0).map_err(|e| e.to_string())
                }
            }
        }
        LinkedinPageAction::Delete { post_urn } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let li = integrations.iter().find(|i| i.provider_identifier == "linkedin-page");
                    match li {
                        None => Err("No LinkedIn Page connected".to_string()),
                        Some(li) => {
                            let input = crate::mcp::tools_linkedin_page::LipDeletePostInput {
                                lip_id: li.internal_id.clone(), post_urn,
                            };
                            crate::mcp::tools_linkedin_page::handle_lip_delete_post(state, &input).await
                                .map(|v| v.0).map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
        LinkedinPageAction::Reactions { post_urn } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let li = integrations.iter().find(|i| i.provider_identifier == "linkedin-page");
                    match li {
                        None => Err("No LinkedIn Page connected".to_string()),
                        Some(li) => {
                            let input = crate::mcp::tools_linkedin_page::LipGetReactionsInput {
                                lip_id: li.internal_id.clone(), post_urn,
                            };
                            crate::mcp::tools_linkedin_page::handle_lip_get_reactions(state, &input).await
                                .map(|v| v.0).map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
        LinkedinPageAction::Shares { post_urn } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let li = integrations.iter().find(|i| i.provider_identifier == "linkedin-page");
                    match li {
                        None => Err("No LinkedIn Page connected".to_string()),
                        Some(li) => {
                            let input = crate::mcp::tools_linkedin_page::LipGetSharesInput {
                                lip_id: li.internal_id.clone(), post_urn,
                            };
                            crate::mcp::tools_linkedin_page::handle_lip_get_shares(state, &input).await
                                .map(|v| v.0).map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
        LinkedinPageAction::PostAnalytics { post_urn } => {
            match crate::db::queries::list_integrations(&state.db, user_id).await {
                Err(e) => Err(format!("DB error: {e}")),
                Ok(integrations) => {
                    let li = integrations.iter().find(|i| i.provider_identifier == "linkedin-page");
                    match li {
                        None => Err("No LinkedIn Page connected".to_string()),
                        Some(li) => {
                            let input = crate::mcp::tools_linkedin_page::LipGetPostAnalyticsInput {
                                lip_id: li.internal_id.clone(), post_urn,
                            };
                            crate::mcp::tools_linkedin_page::handle_lip_get_post_analytics(state, &input).await
                                .map(|v| serde_json::to_value(v.0).unwrap_or_default()).map_err(|e| e.to_string())
                        }
                    }
                }
            }
        }
    };

    emit_result(result)
}
