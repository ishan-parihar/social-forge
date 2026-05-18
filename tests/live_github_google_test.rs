use social_forge::config::Config;
use social_forge::crypto;
use social_forge::db;
use social_forge::social::github::GithubProvider;
use social_forge::social::google::GoogleProvider;
use social_forge::social::SocialProvider;

fn get_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load config")
}

fn decrypt_token(raw: &str) -> Option<String> {
    if !raw.is_empty() && !raw.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(raw.to_string());
    }
    let cfg = get_config();
    let key = crypto::decode_hex_key(cfg.token_encryption_key.as_ref()?).ok()?;
    crypto::decrypt_string(raw, &key).ok()
}

async fn get_google_token() -> Option<String> {
    let cfg = get_config();
    let pool = db::create_pool(&cfg.database_url).await.ok()?;
    let user = sqlx::query!("SELECT id FROM users WHERE email = 'dev@postiz.dev'")
        .fetch_optional(&pool).await.ok()??;
    let row = sqlx::query!(
        "SELECT access_token FROM integrations WHERE user_id = $1 AND provider_identifier = $2 LIMIT 1",
        user.id, "google"
    ).fetch_optional(&pool).await.ok()??;
    decrypt_token(&row.access_token)
}

const GH_OWNER: &str = "ishan-parihar";
const GH_REPO: &str = "social-forge";

#[tokio::test]
async fn test_gh_user() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    let r = gh.get_user(t, GH_OWNER).await.unwrap();
    assert_eq!(r["login"], GH_OWNER);
    println!("OK gh_user: {GH_OWNER}");
}

#[tokio::test]
async fn test_gh_auth_user() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    let r = gh.get_authenticated_user(t).await.unwrap();
    println!("OK gh_auth: {}", r["login"].as_str().unwrap_or("?"));
}

#[tokio::test]
async fn test_gh_repos() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    let r = gh.list_repos(t, GH_OWNER, 50).await.unwrap();
    println!("OK gh_repos: {}", r.as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_gh_issues() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_issues(t, GH_OWNER, GH_REPO, "all", 50).await {
        Ok(r) => println!("OK gh_issues: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_issues: {e}"),
    }
}

#[tokio::test]
async fn test_gh_prs() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_pull_requests(t, GH_OWNER, GH_REPO, "all", 50).await {
        Ok(r) => println!("OK gh_prs: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_prs: {e}"),
    }
}

#[tokio::test]
async fn test_gh_commits() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_commits(t, GH_OWNER, GH_REPO, 50).await {
        Ok(r) => println!("OK gh_commits: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_commits: {e}"),
    }
}

#[tokio::test]
async fn test_gh_branches() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_branches(t, GH_OWNER, GH_REPO, 50).await {
        Ok(r) => println!("OK gh_branches: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_branches: {e}"),
    }
}

#[tokio::test]
async fn test_gh_releases() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_releases(t, GH_OWNER, GH_REPO, 50).await {
        Ok(r) => println!("OK gh_releases: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_releases: {e}"),
    }
}

#[tokio::test]
async fn test_gh_search_repos() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    let r = gh.search_repos(t, "rust", 10).await.unwrap();
    println!("OK gh_search: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_gh_search_code() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    let r = gh.search_code(t, "fn main", 50).await.unwrap();
    println!("OK gh_code: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_gh_contributors() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.list_contributors(t, GH_OWNER, GH_REPO, 50).await {
        Ok(r) => println!("OK gh_contributors: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- gh_contributors: {e}"),
    }
}

#[tokio::test]
async fn test_gh_content() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.get_repo_content(t, GH_OWNER, GH_REPO, "README.md").await {
        Ok(r) => println!("OK gh_content: {} bytes", r["size"].as_u64().unwrap_or(0)),
        Err(e) => println!("-- gh_content: {e}"),
    }
}

#[tokio::test]
async fn test_gh_repo() {
    let c = get_config(); let gh = GithubProvider::new(&c); let t = c.github_token.as_deref().unwrap_or("");
    match gh.get_repo(t, GH_OWNER, GH_REPO).await {
        Ok(r) => println!("OK gh_repo: {} {} stars", r["full_name"].as_str().unwrap_or("?"), r["stargazers_count"]),
        Err(e) => println!("-- gh_repo: {e}"),
    }
}

// Google Suite
async fn google_provider_token() -> (GoogleProvider, String) {
    let c = get_config();
    let p = GoogleProvider::new(&c);
    let t = get_google_token().await.expect("Google token not found. Connect Google Suite first.");
    (p, t)
}

#[tokio::test]
async fn test_goog_search() {
    let (p, t) = google_provider_token().await;
    let r = p.search_videos(&t, "Rust", 3).await.unwrap();
    println!("OK yt_search: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_channel() {
    let (p, t) = google_provider_token().await;
    let pages: Vec<social_forge::social::PageInfo> = SocialProvider::pages(&p, &t).await.unwrap();
    if let Some(page) = pages.first() {
        let r = p.get_channel_stats(&t, &page.id).await.unwrap();
        let s = &r["items"][0]["statistics"];
        println!("OK channel: {} subs {}", s["subscriberCount"], s["videoCount"]);
    }
}

#[tokio::test]
async fn test_goog_video() {
    let (p, t) = google_provider_token().await;
    let s = p.search_videos(&t, "Rust", 1).await.unwrap();
    if let Some(v) = s["items"].as_array().and_then(|a| a.first()) {
        let vid = v["id"]["videoId"].as_str().unwrap_or("");
        let r = p.get_video(&t, vid).await.unwrap();
        println!("OK video: {}", r["items"][0]["snippet"]["title"].as_str().unwrap_or("?"));
    }
}

#[tokio::test]
async fn test_goog_playlists() {
    let (p, t) = google_provider_token().await;
    let pages: Vec<social_forge::social::PageInfo> = SocialProvider::pages(&p, &t).await.unwrap();
    if let Some(page) = pages.first() {
        let r = p.get_playlists(&t, &page.id, 5).await.unwrap();
        println!("OK playlists: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
    }
}

#[tokio::test]
async fn test_goog_comments() {
    let (p, t) = google_provider_token().await;
    let s = p.search_videos(&t, "Rust", 1).await.unwrap();
    if let Some(v) = s["items"].as_array().and_then(|a| a.first()) {
        let vid = v["id"]["videoId"].as_str().unwrap_or("");
        match p.get_comments(&t, vid, 3).await {
            Ok(r) => println!("OK comments: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0)),
            Err(e) => println!("-- comments: {e}"),
        }
    }
}

#[tokio::test]
async fn test_goog_find() {
    let (p, t) = google_provider_token().await;
    match p.find_creators(&t, "architecture", None, Some(3)).await {
        Ok(r) => println!("OK creators: {}", r.as_array().map(|a| a.len()).unwrap_or(0)),
        Err(e) => println!("-- find: {e}"),
    }
}

#[tokio::test]
async fn test_goog_analytics() {
    let (p, t) = google_provider_token().await;
    let pages: Vec<social_forge::social::PageInfo> = SocialProvider::pages(&p, &t).await.unwrap();
    if let Some(page) = pages.first() {
        match p.get_analytics(&t, &page.id, "views,estimatedMinutesWatched", "2025-01-01", "2025-05-15").await {
            Ok(r) => println!("OK analytics: {} rows", r["rows"].as_array().map(|a| a.len()).unwrap_or(0)),
            Err(e) => println!("-- analytics: {e}"),
        }
    }
}

#[tokio::test]
async fn test_goog_gmail() {
    let (p, t) = google_provider_token().await;
    let r = p.get_profile(&t).await.unwrap();
    println!("OK gmail: {} {} msgs", r["emailAddress"], r["messagesTotal"]);
}

#[tokio::test]
async fn test_goog_messages() {
    let (p, t) = google_provider_token().await;
    let r = p.list_messages(&t, 5, None).await.unwrap();
    println!("OK msgs: {}", r["messages"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_labels() {
    let (p, t) = google_provider_token().await;
    let r = p.list_labels(&t).await.unwrap();
    println!("OK labels: {}", r["labels"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_cal_list() {
    let (p, t) = google_provider_token().await;
    let r = p.list_calendars(&t).await.unwrap();
    println!("OK calendars: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_events() {
    let (p, t) = google_provider_token().await;
    let r = p.list_events(&t, "primary", 5, None, None).await.unwrap();
    println!("OK events: {}", r["items"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_cal_crud() {
    let (p, t) = google_provider_token().await;
    let created = p.create_event(&t, "primary", "Test", "2026-05-16T10:00:00", "2026-05-16T11:00:00", Some("test")).await.unwrap();
    let eid = created["id"].as_str().unwrap_or("").to_string();
    let _ = p.update_event(&t, "primary", &eid, Some("Updated"), Some("desc")).await;
    p.delete_event(&t, "primary", &eid).await.unwrap();
    println!("OK cal_crud");
}

#[tokio::test]
async fn test_goog_drive_files() {
    let (p, t) = google_provider_token().await;
    let r = p.list_files(&t, 10, None).await.unwrap();
    println!("OK drive: {} files", r["files"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_drive_folders() {
    let (p, t) = google_provider_token().await;
    let r = p.list_folders(&t, 10).await.unwrap();
    println!("OK folders: {}", r["files"].as_array().map(|a| a.len()).unwrap_or(0));
}

#[tokio::test]
async fn test_goog_drive_meta() {
    let (p, t) = google_provider_token().await;
    let files = p.list_files(&t, 1, None).await.unwrap();
    if let Some(f) = files["files"].as_array().and_then(|a| a.first()) {
        let m = p.get_file_metadata(&t, f["id"].as_str().unwrap_or("")).await.unwrap();
        println!("OK meta: {} ({})", m["name"], m["mimeType"]);
    }
}
