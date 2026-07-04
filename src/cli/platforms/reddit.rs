// ── Reddit CLI Handler ────────────────────────────────────────
// Relocated from cli/run.rs to match the platforms/*.rs shim pattern.

use crate::api::AppState;
use crate::cli::{RedditAction, RedditModAction};
use crate::cli::platforms::emit_result;
use crate::cli::run::{find_reddit_token, resolve_user, find_integration, fetch_targets, pick_target_interactive};
use crate::social::SocialProvider;

pub async fn handle(action: RedditAction, state: &AppState) -> anyhow::Result<()> {
    let user_id = resolve_user(state).await?;
    let token = find_reddit_token(state, user_id).await?;

    let mut provider = crate::social::reddit::RedditProvider::new(&state.config);
    provider.prepare_from_token(&token);

    let result: Result<serde_json::Value, String> = match action {
        RedditAction::Browse { subreddit, sort, limit } => {
            provider.browse(&token, &subreddit, &sort, limit, "all").await
                .map_err(|e| e.to_string())
        }
        RedditAction::Search { query, subreddit, sort } => {
            provider.search(&token, &query, subreddit.as_deref(), &sort, 25, "all").await
                .map_err(|e| e.to_string())
        }
        RedditAction::Post { title, text, url, target, targets } => {
            let subreddits: Vec<String> = if let Some(ref t) = targets {
                t.split(',').map(|s| s.trim().replace("r/", "")).filter(|s| !s.is_empty()).collect()
            } else if let Some(ref t) = target {
                vec![t.trim().replace("r/", "")]
            } else {
                let integration = find_integration(state, user_id, "reddit").await?;
                match fetch_targets(state, &integration).await {
                    Ok(targets) => {
                        let selected = pick_target_interactive(&targets, "reddit")?;
                        vec![selected.replace("r/", "")]
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not fetch targets ({}).", e);
                        eprintln!("Please specify a subreddit with --target or --targets.");
                        return Err(anyhow::anyhow!("No subreddit specified and target discovery failed"));
                    }
                }
            };

            let mut results = Vec::new();
            for sub in &subreddits {
                let kind = if url.is_some() { "link" } else { "self" };
                let text_val = text.as_deref().unwrap_or("");
                let url_val = url.as_deref().unwrap_or("");
                let mut form: Vec<(&str, &str)> = vec![
                    ("api_type", "json"), ("sr", sub), ("title", &title), ("kind", kind),
                ];
                if kind == "self" { form.push(("text", text_val)); }
                else { form.push(("url", url_val)); }

                let resp = reqwest::Client::new()
                    .post("https://oauth.reddit.com/api/submit")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("User-Agent", concat!("social-forge:v", env!("CARGO_PKG_VERSION"), " (by /u/social_forge)"))
                    .form(&form).send().await
                    .map_err(|e| format!("Reddit submit failed: {e}"));
                let result = match resp {
                    Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse failed: {e}")),
                    Err(e) => Err(e),
                };
                results.push((sub.clone(), result));
            }

            if results.len() == 1 {
                let (_, result) = results.remove(0);
                result
            } else {
                let output: Vec<serde_json::Value> = results.into_iter().map(|(sub, result)| {
                    match result {
                        Ok(v) => serde_json::json!({"subreddit": sub, "status": "success", "result": v}),
                        Err(e) => serde_json::json!({"subreddit": sub, "status": "error", "error": e}),
                    }
                }).collect();
                Ok(serde_json::json!({"posts": output}))
            }
        }
        RedditAction::Comment { thing_id, text } => {
            let tid = if thing_id.starts_with("t3_") || thing_id.starts_with("t1_") {
                thing_id
            } else {
                format!("t3_{}", thing_id)
            };
            let resp = reqwest::Client::new()
                .post("https://oauth.reddit.com/api/comment")
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", concat!("social-forge:v", env!("CARGO_PKG_VERSION"), " (by /u/social_forge)"))
                .form(&[("api_type", "json"), ("thing_id", &*tid), ("text", &*text)])
                .send().await
                .map_err(|e| format!("Reddit comment failed: {e}"));
            match resp {
                Ok(r) => r.json::<serde_json::Value>().await.map_err(|e| format!("Parse failed: {e}")),
                Err(e) => Err(e),
            }
        }
        RedditAction::Vote { thing_id, direction } => {
            let dir: i8 = match direction.as_str() {
                "up" => 1, "down" => -1, _ => 0,
            };
            provider.vote(&thing_id, dir).await.map_err(|e| e.to_string())
        }
        RedditAction::Save { thing_id } => {
            provider.save(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::Unsave { thing_id } => {
            provider.unsave(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::Delete { thing_id } => {
            provider.delete(&thing_id).await.map_err(|e| e.to_string())
        }
        RedditAction::User { username } => {
            provider.user_info(&token, &username, true, false).await
                .map_err(|e| e.to_string())
        }
        RedditAction::Inbox { folder } => {
            provider.inbox(&token, &folder, 25).await
                .map_err(|e| e.to_string())
        }
        RedditAction::Mod { action: mod_action } => {
            match mod_action {
                RedditModAction::Remove { thing_id, spam } => {
                    provider.mod_remove(&thing_id, spam).await.map_err(|e| e.to_string())
                }
                RedditModAction::Approve { thing_id } => {
                    provider.mod_approve(&thing_id).await.map_err(|e| e.to_string())
                }
                RedditModAction::Lock { thing_id } => {
                    provider.mod_lock(&thing_id).await.map_err(|e| e.to_string())
                }
                RedditModAction::Unlock { thing_id } => {
                    provider.mod_unlock(&thing_id).await.map_err(|e| e.to_string())
                }
            }
        }
    };

    emit_result(result)
}
