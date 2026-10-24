// ─── Browser-driven OAuth: the real protocol, end to end ─────────
//
// The live browseros-neo server speaks a TEXT `tabs list` format and
// reports the driver's OWN tab under "Other agents' tabs:". Both facts
// caused real failures that a shape-only test cannot catch:
//
//   - a JSON parse of `tabs list` finds nothing;
//   - a parser that stops at "Other agents' tabs:" never sees its own
//     redirect, and every login times out.
//
// This test drives the REAL MCP client and the REAL `wait_for_redirect`
// against a mock server that reproduces the live wire format: the same
// SSE framing, the same session-id header, the same text listing, the
// same section headers, and the same "owned by <agent>" annotation.
//
// The mock reads the authorization URL the driver opens, extracts the
// `state` the driver minted, and answers with a callback URL carrying
// THAT state — exactly what the provider's redirect does.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use switchpost::cli::browser::Browser;
use switchpost::cli::oauth_browser::wait_for_redirect;

/// What the mock browser observed.
#[derive(Default)]
struct Observed {
    /// The authorization URL the driver opened.
    auth_url: Option<String>,
    /// The callback URL the mock handed back in the listing.
    callback_url: Option<String>,
    /// The page the driver closed.
    closed_page: Option<u32>,
}

type Shared = Arc<Mutex<Observed>>;

/// The text listing, in the LIVE server's shape.
///
/// The driver's own tab is under "Other agents' tabs:" — the live
/// server annotates it "owned by switchpost-cli". A parser that stops
/// at that header would never find the callback.
fn tabs_listing(callback_url: &str) -> String {
    format!(
        "Your tabs:\n\
[1] chrome://newtab/ (BrowserOS neo)\n\
\n\
User's tabs:\n\
[2] https://example.com/ (Example Domain)\n\
\n\
Other agents' tabs:\n\
[7] {callback_url} (callback), owned by switchpost-cli [group AAA]\n\
[9] https://other-agent.test/auth?code=THEIRS&state=THEIRS (Other), owned by someone-else [group BBB]\n"
    )
}

/// Serve one MCP endpoint that behaves like browseros-neo.
async fn mock_browser(State(observed): State<Shared>, body: String) -> Response {
    let request: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let id = request.get("id").cloned();

    let mut observed = observed.lock().unwrap();

    // A notification has no id and gets no response body.
    let Some(id) = id else {
        return Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": {"name": "browseros-neo-mock", "version": "0"}
        }),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let args = &request["params"]["arguments"];
            match (name, args.get("action").and_then(Value::as_str).unwrap_or("")) {
                ("tabs", "new") => {
                    let url = args.get("url").and_then(Value::as_str).unwrap_or("");
                    observed.auth_url = Some(url.to_string());
                    // The live server answers with the page id AND a
                    // snapshot block. The driver parses the id.
                    json!({"content": [
                        {"type": "text", "text": "opened page 7"},
                        {"type": "text", "text": "\n--- Additional context (auto-included) ---"},
                        {"type": "text", "text": "[Page 7 snapshot]\n[UNTRUSTED_PAGE_CONTENT nonce=abc origin=https://example.test]\n- heading \"Sign in\"\n[END_UNTRUSTED_PAGE_CONTENT]"}
                    ]})
                }
                ("tabs", "list") => {
                    let auth_url = observed.auth_url.clone().unwrap_or_default();
                    // The provider redirects to the callback with the
                    // state the driver minted.
                    let state = extract_query_param(&auth_url, "state").unwrap_or_default();
                    let callback = format!(
                        "https://localhost:6544/api/auth/callback?code=MOCK_CODE&state={state}"
                    );
                    observed.callback_url = Some(callback.clone());
                    json!({"content": [
                        {"type": "text", "text": tabs_listing(&callback)}
                    ]})
                }
                ("tabs", "close") => {
                    observed.closed_page = args.get("page").and_then(Value::as_u64).map(|p| p as u32);
                    json!({"content": [{"type": "text", "text": "closed page 7"}]})
                }
                _ => json!({"content": [{"type": "text", "text": "unknown tool"}], "isError": true}),
            }
        }
        _ => json!({}),
    };

    let payload = json!({"jsonrpc": "2.0", "id": id, "result": result});
    let sse = format!("event: message\ndata: {payload}\n\n");
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("mcp-session-id", "mock-session-1")
        .body(axum::body::Body::from(sse))
        .unwrap()
}

fn extract_query_param(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            Some(v.to_string())
        } else {
            None
        }
    })
}

/// Start the mock browser and return its endpoint plus the observation log.
async fn start_mock() -> (String, Shared) {
    let observed: Shared = Arc::new(Mutex::new(Observed::default()));
    let app = Router::new()
        .route("/mcp", post(mock_browser))
        .with_state(observed.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/mcp"), observed)
}

/// Open a page (as the driver does) and return the page id the live
/// server reports.
async fn open_page(browser: &mut Browser, auth_url: &str) {
    let result = browser
        .call("tabs", json!({"action": "new", "url": auth_url}))
        .await
        .expect("tabs new");
    let text = Browser::text(&result);
    assert!(
        text.contains("opened page 7"),
        "the live server reports the page id in the first text block: {text}"
    );
}

#[tokio::test]
async fn the_driver_finds_its_own_callback_in_the_live_tab_listing_shape() {
    let (endpoint, observed) = start_mock().await;
    let mut browser = Browser::connect(&endpoint).await.expect("handshake");

    // The driver opens the provider's authorization URL. The state in
    // it is the one the mock will echo back on the callback.
    let minted_state = "MINTED_STATE_VALUE";
    let auth_url = format!(
        "https://provider.test/authorize?client_id=x&state={minted_state}&redirect_uri=https%3A%2F%2Flocalhost%3A6544%2Fapi%2Fauth%2Fcallback"
    );
    open_page(&mut browser, &auth_url).await;

    // The REAL wait loop, against the REAL MCP client, against the
    // live listing shape. The driver's own tab is under "Other agents'
    // tabs:" — the mock reproduces that exactly.
    let redirect = wait_for_redirect(
        &mut browser,
        "https://localhost:6544",
        minted_state,
        10,
    )
    .await
    .expect("the driver must find its own callback");

    assert!(
        redirect.contains("code=MOCK_CODE"),
        "the callback must carry the code: {redirect}"
    );
    assert!(
        redirect.contains(&format!("state={minted_state}")),
        "the callback must carry the minted state: {redirect}"
    );
    assert!(
        !redirect.contains("THEIRS"),
        "another agent's callback must never be returned: {redirect}"
    );

    let seen = observed.lock().unwrap();
    assert!(
        seen.callback_url.as_deref().unwrap_or_default().contains("MOCK_CODE"),
        "the mock must have produced the callback"
    );
}

/// The same flow, but the mock's callback carries a DIFFERENT state —
/// another agent's login. The driver must NOT pick it up.
#[tokio::test]
async fn another_agents_callback_is_never_taken() {
    let (endpoint, observed) = start_mock().await;
    let mut browser = Browser::connect(&endpoint).await.expect("handshake");

    // Overwrite the mock's auth URL so its callback carries a foreign state.
    {
        let mut seen = observed.lock().unwrap();
        seen.auth_url = Some(
            "https://provider.test/authorize?state=SOMEONE_ELSES_STATE".to_string(),
        );
    }
    open_page(&mut browser, "https://provider.test/authorize?state=OURS").await;

    // The mock builds its callback from the auth URL it was given, so
    // the driver must open the URL whose state it wants matched.
    {
        let mut seen = observed.lock().unwrap();
        seen.auth_url = Some("https://provider.test/authorize?state=OURS".to_string());
    }

    let result = wait_for_redirect(&mut browser, "https://localhost:6544", "OURS", 3).await;
    let redirect = result.expect("the driver's own callback must be found");
    assert!(
        redirect.contains("state=OURS"),
        "the driver's own state must win: {redirect}"
    );
    assert!(
        !redirect.contains("SOMEONE_ELSE"),
        "a foreign state must never be taken: {redirect}"
    );
}
