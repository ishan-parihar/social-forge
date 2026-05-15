use std::time::Duration;

const URL: &str = "https://x.com/i/api/graphql/1VOOyvKkiI3FMmkeDNxM9A/UserByScreenName?variables=%7B%22screen_name%22%3A%22elonmusk%22%2C%22withSafetyModeUserFields%22%3Atrue%7D&features=%7B%22responsive_web_graphql_exclude_directive_enabled%22%3Atrue%2C%22verified_phone_label_enabled%22%3Afalse%2C%22responsive_web_graphql_skip_user_profile_image_extensions_enabled%22%3Afalse%7D";
const BEARER: &str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

fn get_cookies() -> (String, String) {
    dotenvy::dotenv().ok();
    let at = std::env::var("X_AUTH_TOKEN").expect("X_AUTH_TOKEN not set");
    let ct0 = std::env::var("X_CT0").expect("X_CT0 not set");
    (at, ct0)
}

async fn test_headers(headers: &[(&str, &str)], label: &str) {
    let (at, ct0) = get_cookies();
    let cookies = format!("auth_token={at}; ct0={ct0}");
    let client = reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(15))
        .build().unwrap_or_else(|e| panic!("client: {e}"));

    let mut req = client.get(URL)
        .header("Cookie", &cookies)
        .header("x-csrf-token", &ct0)
        .header("Authorization", format!("Bearer {BEARER}"));
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = req.send().await.unwrap_or_else(|e| panic!("req: {e}"));
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let preview: String = body.chars().take(100).collect();
    if status.is_success() {
        eprintln!("  ✅ {label}");
    } else {
        eprintln!("  ❌ {label}: HTTP {status} (body: {preview})");
    }
    assert!(status.is_success(), "{label}: HTTP {status}");
}

#[tokio::test]
async fn test_x_headers_bisect() {
    eprintln!("=== Header bisect ===");

    test_headers(&[], "base (cookies+csrf+bearer+UA)").await;

    test_headers(&[
        ("x-twitter-active-user", "yes"),
        ("x-twitter-auth-type", "OAuth2Session"),
        ("x-twitter-client-language", "en"),
    ], "+ x-twitter-*").await;

    test_headers(&[
        ("origin", "https://x.com"),
        ("referer", "https://x.com/"),
    ], "+ origin+referer").await;

    test_headers(&[
        ("accept", "*/*"),
    ], "+ accept: */*").await;

    test_headers(&[
        ("accept-language", "en-US,en;q=0.9"),
    ], "+ accept-language").await;

    test_headers(&[
        ("sec-ch-ua", r#""Chromium";v="131", "Not(A:Brand";v="24", "Google Chrome";v="131""#),
    ], "+ sec-ch-ua").await;

    test_headers(&[
        ("sec-ch-ua-full-version-list", r#""Chromium";v="131", "Not(A:Brand";v="24.0.0.0", "Google Chrome";v="131.0.6778.265""#),
    ], "+ sec-ch-ua-full-version-list").await;

    test_headers(&[
        ("accept", "*/*"),
        ("accept-language", "en-US,en;q=0.9"),
        ("origin", "https://x.com"),
        ("referer", "https://x.com/"),
        ("x-twitter-active-user", "yes"),
        ("x-twitter-auth-type", "OAuth2Session"),
        ("x-twitter-client-language", "en"),
        ("sec-ch-ua", r#""Chromium";v="131", "Not(A:Brand";v="24", "Google Chrome";v="131""#),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", "Linux"),
        ("sec-ch-ua-arch", "x86"),
        ("sec-ch-ua-bitness", "64"),
        ("sec-ch-ua-full-version", "131.0.6778.265"),
        ("sec-ch-ua-full-version-list", r#""Chromium";v="131", "Not(A:Brand";v="24.0.0.0", "Google Chrome";v="131.0.6778.265""#),
        ("sec-ch-ua-model", "\"\""),
        ("sec-ch-ua-platform-version", "\"\""),
        ("Sec-Fetch-Dest", "empty"),
        ("Sec-Fetch-Mode", "cors"),
        ("Sec-Fetch-Site", "same-origin"),
    ], "+sec-ch-ua+sec-fetch family").await;

    test_headers(&[
        ("x-client-transaction-id", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ], "+ x-client-transaction-id").await;

    test_headers(&[
        ("accept", "*/*"),
        ("accept-language", "en-US,en;q=0.9"),
        ("origin", "https://x.com"),
        ("referer", "https://x.com/"),
        ("x-twitter-active-user", "yes"),
        ("x-twitter-auth-type", "OAuth2Session"),
        ("x-twitter-client-language", "en"),
        ("sec-ch-ua", r#""Chromium";v="131", "Not(A:Brand";v="24", "Google Chrome";v="131""#),
        ("sec-ch-ua-mobile", "?0"),
        ("sec-ch-ua-platform", "Linux"),
        ("sec-ch-ua-arch", "x86"),
        ("sec-ch-ua-bitness", "64"),
        ("sec-ch-ua-full-version", "131.0.6778.265"),
        ("sec-ch-ua-full-version-list", r#""Chromium";v="131", "Not(A:Brand";v="24.0.0.0", "Google Chrome";v="131.0.6778.265""#),
        ("sec-ch-ua-model", "\"\""),
        ("sec-ch-ua-platform-version", "\"\""),
        ("Sec-Fetch-Dest", "empty"),
        ("Sec-Fetch-Mode", "cors"),
        ("Sec-Fetch-Site", "same-origin"),
        ("x-client-transaction-id", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ], "+ ALL+txid").await;
}

#[tokio::test]
async fn test_wreq_no_emulation() {
    let client = wreq::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build().unwrap();

    let (at, ct0) = get_cookies();
    let cookies = format!("auth_token={at}; ct0={ct0}");
    let bearer = format!("Bearer {BEARER}");
    let resp = client.get(URL)
        .header("Cookie", &cookies)
        .header("x-csrf-token", &ct0)
        .header("Authorization", &bearer)
        .send().await.unwrap();

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    eprintln!("wreq no emulation | HTTP {status} | body_len={} | body_start={}", body.len(), &body.chars().take(100).collect::<String>());
    assert!(status.is_success(), "wreq no emulation failed: HTTP {status}: {body}");
}

#[tokio::test]
async fn test_wreq_chrome131() {
    let (at, ct0) = get_cookies();
    let cookies = format!("auth_token={at}; ct0={ct0}");
    let bearer = format!("Bearer {BEARER}");
    let client = wreq::Client::builder()
        .emulation(wreq_util::Emulation::Chrome131)
        .timeout(Duration::from_secs(15))
        .build().unwrap();

    let resp = client.get(URL)
        .header("Cookie", &cookies)
        .header("x-csrf-token", &ct0)
        .header("Authorization", &bearer)
        .send().await.unwrap();

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    eprintln!("wreq Chrome131 | HTTP {status} | body_len={} | body_start={}", body.len(), &body.chars().take(100).collect::<String>());
    assert!(status.is_success(), "wreq Chrome131 failed: HTTP {status}: {body}");
}

#[tokio::test]
async fn test_xprovider_emulation_headers() {
    // Replicate how XProvider builds its wreq client:
    // .default_headers(headers) then .emulation(Chrome131)
    let (at, ct0) = get_cookies();
    let cookies = format!("auth_token={at}; ct0={ct0}");
    let bearer = format!("Bearer {BEARER}");

    let mut headers = wreq::header::HeaderMap::new();
    headers.insert(wreq::header::ACCEPT, wreq::header::HeaderValue::from_static("*/*"));
    headers.insert("origin", wreq::header::HeaderValue::from_static("https://x.com"));
    headers.insert("referer", wreq::header::HeaderValue::from_static("https://x.com/"));
    headers.insert("x-twitter-active-user", wreq::header::HeaderValue::from_static("yes"));
    headers.insert("x-twitter-auth-type", wreq::header::HeaderValue::from_static("OAuth2Session"));
    headers.insert("x-twitter-client-language", wreq::header::HeaderValue::from_static("en"));
    headers.insert(wreq::header::AUTHORIZATION, wreq::header::HeaderValue::from_str(&bearer).unwrap());
    headers.insert("accept-language", wreq::header::HeaderValue::from_static("en-US,en;q=0.9"));
    // sec-ch-ua family
    headers.insert("sec-ch-ua", wreq::header::HeaderValue::from_static(r#""Chromium";v="131", "Not(A:Brand";v="24", "Google Chrome";v="131""#));
    headers.insert("sec-ch-ua-mobile", wreq::header::HeaderValue::from_static("?0"));
    headers.insert("sec-ch-ua-platform", wreq::header::HeaderValue::from_static("Linux"));
    headers.insert("sec-ch-ua-arch", wreq::header::HeaderValue::from_static("x86"));
    headers.insert("sec-ch-ua-bitness", wreq::header::HeaderValue::from_static("64"));
    headers.insert("sec-ch-ua-full-version", wreq::header::HeaderValue::from_static("131.0.6778.265"));
    headers.insert("sec-ch-ua-full-version-list", wreq::header::HeaderValue::from_static(r#""Chromium";v="131", "Not(A:Brand";v="24.0.0.0", "Google Chrome";v="131.0.6778.265""#));
    headers.insert("sec-ch-ua-model", wreq::header::HeaderValue::from_static("\"\""));
    headers.insert("sec-ch-ua-platform-version", wreq::header::HeaderValue::from_static(if std::env::var("OS_VERSION").unwrap_or_default().is_empty() { "\"\"" } else { "dummy" }));
    // sec-fetch family
    headers.insert("Sec-Fetch-Dest", wreq::header::HeaderValue::from_static("empty"));
    headers.insert("Sec-Fetch-Mode", wreq::header::HeaderValue::from_static("cors"));
    headers.insert("Sec-Fetch-Site", wreq::header::HeaderValue::from_static("same-origin"));

    let client = wreq::Client::builder()
        .default_headers(headers)
        .emulation(wreq_util::Emulation::Chrome131)
        .gzip(true)
        .brotli(true)
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(90))
        .build().unwrap();

    // Test GET with Cookie + x-csrf-token (like graphql_get does)
    let resp = client.get(URL)
        .header("Cookie", &cookies)
        .header("x-csrf-token", &ct0)
        .send().await.unwrap();

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    let preview: String = body.chars().take(100).collect();
    eprintln!("XProvider-like wreq | HTTP {status} | body_len={} | body_start={}", body.len(), &preview);
    assert!(status.is_success(), "XProvider-like wreq: HTTP {status}: {body}");
}
