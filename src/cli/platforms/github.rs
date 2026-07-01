use crate::api::AppState;
use crate::cli::GithubAction;

pub async fn handle(action: GithubAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        GithubAction::User { login } => {
            let input = crate::mcp::tools_github::GhGetUserInput { login };
            crate::mcp::tools_github::handle_gh_get_user(state, &input).await.map(|v| v.0)
        }
        GithubAction::Repos { username, limit } => {
            let input = crate::mcp::tools_github::GhListReposInput { username, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_repos(state, &input).await.map(|v| v.0)
        }
        GithubAction::Issues { owner, repo, state_filter, limit } => {
            let input = crate::mcp::tools_github::GhListIssuesInput { owner, repo, state: state_filter, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_issues(state, &input).await.map(|v| v.0)
        }
        GithubAction::Prs { owner, repo, state_filter, limit } => {
            let input = crate::mcp::tools_github::GhListPullRequestsInput { owner, repo, state: state_filter, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_pull_requests(state, &input).await.map(|v| v.0)
        }
        GithubAction::CreateIssue { owner, repo, title, body } => {
            let input = crate::mcp::tools_github::GhCreateIssueInput { owner, repo, title, body };
            crate::mcp::tools_github::handle_gh_create_issue(state, &input).await.map(|v| v.0)
        }
        GithubAction::CloseIssue { owner, repo, number } => {
            let input = crate::mcp::tools_github::GhCloseIssueInput { owner, repo, issue_number: number };
            crate::mcp::tools_github::handle_gh_close_issue(state, &input).await.map(|v| v.0)
        }
        GithubAction::Commits { owner, repo, limit } => {
            let input = crate::mcp::tools_github::GhListCommitsInput { owner, repo, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_commits(state, &input).await.map(|v| v.0)
        }
        GithubAction::Branches { owner, repo, limit } => {
            let input = crate::mcp::tools_github::GhListBranchesInput { owner, repo, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_branches(state, &input).await.map(|v| v.0)
        }
        GithubAction::Search { query, limit } => {
            let input = crate::mcp::tools_github::GhSearchReposInput { query, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_search_repos(state, &input).await.map(|v| v.0)
        }
        GithubAction::Releases { owner, repo, limit } => {
            let input = crate::mcp::tools_github::GhListReleasesInput { owner, repo, limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_releases(state, &input).await.map(|v| v.0)
        }
        GithubAction::Me => {
            crate::mcp::tools_github::handle_gh_get_authenticated_user(state, &()).await.map(|v| v.0)
        }
        GithubAction::MyRepos { limit } => {
            let input = crate::mcp::tools_github::GhListMyReposInput { limit: Some(limit) };
            crate::mcp::tools_github::handle_gh_list_my_repos(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
