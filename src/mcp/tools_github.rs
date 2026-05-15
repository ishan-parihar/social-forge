// ─── MCP GitHub Tools ─────────────────────────────────────────
// GitHub REST API tools via GitHubProvider (Personal Access Token).
// PAT can be configured via GITHUB_TOKEN env var or stored as an integration.

use rmcp::{Json, schemars::JsonSchema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::AppState;
use crate::crypto;
use crate::social::github::GithubProvider;

// ── Input Types ───────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhGetUserInput {
    /// GitHub login username
    pub login: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListReposInput {
    /// GitHub username or org name
    pub username: String,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhGetRepoInput {
    pub owner: String,
    pub repo: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListIssuesInput {
    pub owner: String,
    pub repo: String,
    /// Issue state: open, closed, all (default open)
    pub state: Option<String>,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhGetIssueInput {
    pub owner: String,
    pub repo: String,
    pub issue_number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhCreateIssueInput {
    pub owner: String,
    pub repo: String,
    pub title: String,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListPullRequestsInput {
    pub owner: String,
    pub repo: String,
    /// PR state: open, closed, all (default open)
    pub state: Option<String>,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhGetPullRequestInput {
    pub owner: String,
    pub repo: String,
    pub pr_number: u32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListCommitsInput {
    pub owner: String,
    pub repo: String,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListBranchesInput {
    pub owner: String,
    pub repo: String,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListReleasesInput {
    pub owner: String,
    pub repo: String,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhSearchReposInput {
    /// Search query (e.g. "rust web framework")
    pub query: String,
    /// Max results (default 10, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhSearchCodeInput {
    /// Search query (e.g. "repo:user/repo function" or "use wreq")
    pub query: String,
    /// Max results (default 10, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListContributorsInput {
    pub owner: String,
    pub repo: String,
    /// Max results (default 30, max 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhGetRepoContentInput {
    pub owner: String,
    pub repo: String,
    /// File path in the repo (e.g. "src/main.rs" or "README.md")
    pub path: String,
}

// ── Helpers ───────────────────────────────────────────────────

async fn find_gh_token(state: &AppState, user_id: Uuid) -> Result<String, String> {
    let integrations = crate::db::queries::list_integrations(&state.db, user_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let gh = integrations
        .iter()
        .find(|i| i.provider_identifier == "github")
        .ok_or_else(|| "GitHub account not connected. Add GITHUB_TOKEN to .env or connect via onboarding.".to_string())?;

    let tok = gh.access_token.clone();
    let tok = state.token_key.as_ref()
        .and_then(|k| crypto::decrypt_string(&tok, k).ok())
        .unwrap_or(tok);
    Ok(tok)
}

fn create_gh_provider(state: &AppState) -> GithubProvider {
    GithubProvider::new(&state.config)
}

// ── Handlers ──────────────────────────────────────────────────

pub async fn handle_gh_get_authenticated_user(
    state: &AppState,
    _input: &(), // no params
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_authenticated_user(&token)
        .await
        .map_err(|e| format!("GitHub get authenticated user failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_get_user(
    state: &AppState,
    input: &GhGetUserInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_user(&token, &input.login)
        .await
        .map_err(|e| format!("GitHub get user failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_repos(
    state: &AppState,
    input: &GhListReposInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_repos(&token, &input.username, limit)
        .await
        .map_err(|e| format!("GitHub list repos failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_get_repo(
    state: &AppState,
    input: &GhGetRepoInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_repo(&token, &input.owner, &input.repo)
        .await
        .map_err(|e| format!("GitHub get repo failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_issues(
    state: &AppState,
    input: &GhListIssuesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let state_filter = input.state.as_deref().unwrap_or("open");
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_issues(&token, &input.owner, &input.repo, state_filter, limit)
        .await
        .map_err(|e| format!("GitHub list issues failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_get_issue(
    state: &AppState,
    input: &GhGetIssueInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_issue(&token, &input.owner, &input.repo, input.issue_number)
        .await
        .map_err(|e| format!("GitHub get issue failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_create_issue(
    state: &AppState,
    input: &GhCreateIssueInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let body = input.body.as_deref().unwrap_or("");
    let result = provider
        .create_issue(&token, &input.owner, &input.repo, &input.title, body)
        .await
        .map_err(|e| format!("GitHub create issue failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_pull_requests(
    state: &AppState,
    input: &GhListPullRequestsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let state_filter = input.state.as_deref().unwrap_or("open");
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_pull_requests(&token, &input.owner, &input.repo, state_filter, limit)
        .await
        .map_err(|e| format!("GitHub list PRs failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_get_pull_request(
    state: &AppState,
    input: &GhGetPullRequestInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_pull_request(&token, &input.owner, &input.repo, input.pr_number)
        .await
        .map_err(|e| format!("GitHub get PR failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_commits(
    state: &AppState,
    input: &GhListCommitsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_commits(&token, &input.owner, &input.repo, limit)
        .await
        .map_err(|e| format!("GitHub list commits failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_branches(
    state: &AppState,
    input: &GhListBranchesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_branches(&token, &input.owner, &input.repo, limit)
        .await
        .map_err(|e| format!("GitHub list branches failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_releases(
    state: &AppState,
    input: &GhListReleasesInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_releases(&token, &input.owner, &input.repo, limit)
        .await
        .map_err(|e| format!("GitHub list releases failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_search_repos(
    state: &AppState,
    input: &GhSearchReposInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(10);
    let result = provider
        .search_repos(&token, &input.query, limit)
        .await
        .map_err(|e| format!("GitHub search repos failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_search_code(
    state: &AppState,
    input: &GhSearchCodeInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(10);
    let result = provider
        .search_code(&token, &input.query, limit)
        .await
        .map_err(|e| format!("GitHub search code failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_list_contributors(
    state: &AppState,
    input: &GhListContributorsInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_contributors(&token, &input.owner, &input.repo, limit)
        .await
        .map_err(|e| format!("GitHub list contributors failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

pub async fn handle_gh_get_repo_content(
    state: &AppState,
    input: &GhGetRepoContentInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .get_repo_content(&token, &input.owner, &input.repo, &input.path)
        .await
        .map_err(|e| format!("GitHub get repo content failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhCloseIssueInput {
    pub owner: String,
    pub repo: String,
    pub issue_number: u32,
}

pub async fn handle_gh_close_issue(
    state: &AppState,
    input: &GhCloseIssueInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let result = provider
        .close_issue(&token, &input.owner, &input.repo, input.issue_number)
        .await
        .map_err(|e| format!("GitHub close issue failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GhListMyReposInput {
    /// Max repositories to return (default 30, max 100)
    pub limit: Option<u32>,
}

pub async fn handle_gh_list_my_repos(
    state: &AppState,
    input: &GhListMyReposInput,
) -> Result<Json<serde_json::Value>, String> {
    let user_id = super::tools_posts::resolve_first_user(state).await?;
    let token = find_gh_token(state, user_id).await?;
    let provider = create_gh_provider(state);
    let limit = input.limit.unwrap_or(30);
    let result = provider
        .list_my_repos(&token, limit)
        .await
        .map_err(|e| format!("GitHub list my repos failed: {e}"))?;
    Ok(Json(serde_json::json!({ "data": result })))
}
