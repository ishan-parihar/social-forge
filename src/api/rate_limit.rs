// ─── Rate Limiting (in-process governor) ─────────────────────────
// Per-client fixed-window counters for the routes that are cheap to
// brute-force and expensive to abuse: password login, provider connect
// (credential POSTs + OAuth redirects), and media upload.
//
// Why hand-rolled and not `tower-governor`: tower-governor is a new
// dependency (governor + key-extractor) and its `GovernorLayer` is
// attached per route, which would mean editing route-registration lines
// owned by another agent. The single-binary constraint (AGENTS.md §0.5.3)
// also rules out Redis, so the state has to be in-process either way.
// A `Mutex<HashMap>` is ~40 lines and has no dep tree.
//
// State is per-process and lost on restart. That is the intended
// trade-off: a single self-hosted binary is the whole deployment.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::AppError;

const MINUTE: Duration = Duration::from_secs(60);

/// Strict: 10 login attempts per client per minute. The 11th is a 429.
/// Sized above the worst realistic human typo burst (double-submit,
/// password-manager retry) and far below an offline-cracking pace.
const LOGIN_MAX: u32 = 10;

/// Moderate: OAuth connect redirects plus the credential-accepting
/// POSTs under the same prefix. A provider reconnect loop walks several
/// of these routes, so this is ~2x the login budget.
const CONNECT_MAX: u32 = 20;

/// Moderate: uploads are large and bursty in normal use (a batch of 10
/// images is one drag-and-drop), so this sits well above login.
const MEDIA_MAX: u32 = 30;

/// Ceiling on tracked (bucket, client) cells. Without it, an attacker
/// rotating client identifiers grows the map until the process OOMs.
/// Purging expired cells is O(n) but only runs at the ceiling.
const MAX_TRACKED_CELLS: usize = 10_000;

/// The route families under rate limiting. Each carries its own budget.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Bucket {
    Login,
    Connect,
    MediaUpload,
}

impl Bucket {
    /// `(max hits, window length)` for this bucket.
    const fn limit(self) -> (u32, Duration) {
        match self {
            Self::Login => (LOGIN_MAX, MINUTE),
            Self::Connect => (CONNECT_MAX, MINUTE),
            Self::MediaUpload => (MEDIA_MAX, MINUTE),
        }
    }
}

/// One client's hit count inside the current window.
#[derive(Clone, Copy, Debug)]
struct Window {
    /// When the current window opened.
    start: Instant,
    /// Hits consumed in this window.
    count: u32,
}

/// Fixed-window per-(bucket, client) counter.
#[derive(Default)]
struct RateLimiter {
    cells: Mutex<HashMap<(Bucket, String), Window>>,
}

impl RateLimiter {
    /// Consume one hit. `Ok(())` when allowed; `Err(retry_after)` when the
    /// window is exhausted, carrying how long until it resets.
    ///
    /// `now` is a parameter rather than an `Instant::now()` call so tests
    /// can drive the window deterministically without sleeping.
    fn check_at(&self, bucket: Bucket, client: &str, now: Instant) -> Result<(), Duration> {
        let (max, window_len) = bucket.limit();
        // `into_inner` recovers a poisoned lock: a panic in a *different*
        // request must not take rate limiting down for the whole process.
        let mut cells = self.cells.lock().unwrap_or_else(|e| e.into_inner());

        if cells.len() >= MAX_TRACKED_CELLS {
            cells.retain(|_, w| now.saturating_duration_since(w.start) < window_len);
        }

        let window = cells
            .entry((bucket, client.to_owned()))
            .or_insert(Window { start: now, count: 0 });

        let elapsed = now.saturating_duration_since(window.start);
        if elapsed >= window_len {
            // Window rolled over — reset before judging this hit.
            window.start = now;
            window.count = 0;
        }

        if window.count >= max {
            return Err(window_len.saturating_sub(elapsed));
        }

        window.count += 1;
        Ok(())
    }

    /// `check_at` against the wall clock.
    fn check(&self, bucket: Bucket, client: &str) -> Result<(), Duration> {
        self.check_at(bucket, client, Instant::now())
    }
}

/// Process-wide limiter, shared by every request the router serves.
static LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::default);

/// Which bucket a request falls into, or `None` when it is not limited.
///
/// The three families are matched by path/method here rather than by
/// per-route layers, so this file stays independent of the route table.
fn classify(method: &Method, path: &str) -> Option<Bucket> {
    let is_post = method == Method::POST;
    if is_post && path == "/api/auth/login" {
        return Some(Bucket::Login);
    }
    // Trailing slash required: every connect route has a provider segment.
    if path.starts_with("/api/integrations/connect/") {
        return Some(Bucket::Connect);
    }
    // Exact path only — `GET /api/media/{id}` serves dashboard images and
    // must never be throttled.
    if is_post && path == "/api/media" {
        return Some(Bucket::MediaUpload);
    }
    None
}

/// Best-effort client identity from proxy headers.
///
/// ponytail: trusts `X-Forwarded-For`/`X-Real-IP`, which a deployment
/// reachable directly from the internet could spoof to evade the limit.
/// Acceptable under the documented posture — the server binds 127.0.0.1
/// by default (AGENTS.md §6.6) behind the operator's reverse proxy. Move
/// to `ConnectInfo<SocketAddr>` if a `BIND_HOST=0.0.0.0` deployment ever
/// needs a hard guarantee; that requires changing the serve call, not
/// this file.
fn client_key(req: &Request<Body>) -> String {
    first_forwarded_hop(req, "x-forwarded-for")
        .or_else(|| first_forwarded_hop(req, "x-real-ip"))
        // No proxy headers (direct local access): one shared cell. Stricter
        // than per-client, never looser, so it fails safe.
        .unwrap_or_else(|| "no-forwarded-for".to_owned())
}

/// First hop of a comma-separated forwarding chain: `"client, p1, p2"`.
fn first_forwarded_hop(req: &Request<Body>, name: &str) -> Option<String> {
    let raw = req.headers().get(name)?.to_str().ok()?;
    let first = raw.split(',').next()?.trim();
    (!first.is_empty()).then(|| first.to_owned())
}

/// 429 with `Retry-After`, reusing the existing `rate_limited` error shape
/// the frontend already branches on for provider 429s.
fn too_many_requests(retry_after: Duration) -> Response {
    let mut res = AppError::RateLimited("Too many requests".into()).into_response();
    // Truncating a sub-second remainder would advertise `Retry-After: 0`,
    // which tells the client to retry immediately. Round up to 1s.
    let secs = retry_after.as_secs().max(1);
    if let Ok(value) = HeaderValue::from_str(&secs.to_string()) {
        res.headers_mut().insert(header::RETRY_AFTER, value);
    }
    res
}

/// axum middleware: reject over-limit requests before they reach a handler.
///
/// Sits in the global layer stack (inside CORS, outside `TraceLayer` and
/// the auth/CSRF chain) so a rejected request never reaches password
/// hashing or a provider, and the 429 still carries CORS headers.
pub(super) async fn enforce(req: Request<Body>, next: Next) -> Response {
    let Some(bucket) = classify(req.method(), req.uri().path()) else {
        return next.run(req).await;
    };

    let client = client_key(&req);
    match LIMITER.check(bucket, &client) {
        Ok(()) => next.run(req).await,
        Err(retry_after) => {
            tracing::warn!(
                "rate limit tripped: bucket={bucket:?} client={client} retry_after={}s",
                retry_after.as_secs()
            );
            too_many_requests(retry_after)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IP_A: &str = "203.0.113.7";
    const IP_B: &str = "198.51.100.4";

    #[test]
    fn login_allows_ten_per_window_then_rejects_the_eleventh() {
        let rl = RateLimiter::default();
        let now = Instant::now();

        for attempt in 1..=LOGIN_MAX {
            assert!(
                rl.check_at(Bucket::Login, IP_A, now).is_ok(),
                "attempt {attempt} of {LOGIN_MAX} should be allowed"
            );
        }

        let retry_after = rl
            .check_at(Bucket::Login, IP_A, now)
            .expect_err("attempt 11 must be rejected");
        assert!(retry_after > Duration::ZERO, "rejection must advertise a wait");
    }

    #[test]
    fn window_resets_once_the_period_elapses() {
        let rl = RateLimiter::default();
        let t0 = Instant::now();

        for _ in 0..LOGIN_MAX {
            rl.check_at(Bucket::Login, IP_A, t0).expect("within budget");
        }
        assert!(rl.check_at(Bucket::Login, IP_A, t0).is_err());

        // One second short of the window: still limited.
        assert!(rl.check_at(Bucket::Login, IP_A, t0 + Duration::from_secs(59)).is_err());
        // Window rolled over: allowed again.
        assert!(rl.check_at(Bucket::Login, IP_A, t0 + MINUTE + Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn one_exhausted_client_does_not_affect_another() {
        let rl = RateLimiter::default();
        let now = Instant::now();

        for _ in 0..LOGIN_MAX {
            rl.check_at(Bucket::Login, IP_A, now).expect("within budget");
        }
        assert!(rl.check_at(Bucket::Login, IP_A, now).is_err());
        assert!(rl.check_at(Bucket::Login, IP_B, now).is_ok());
    }

    #[test]
    fn budgets_are_tracked_per_bucket_not_globally() {
        let rl = RateLimiter::default();
        let now = Instant::now();

        for _ in 0..LOGIN_MAX {
            rl.check_at(Bucket::Login, IP_A, now).expect("within budget");
        }
        assert!(rl.check_at(Bucket::Login, IP_A, now).is_err());
        // Connect and upload budgets are untouched by login exhaustion.
        assert!(rl.check_at(Bucket::Connect, IP_A, now).is_ok());
        assert!(rl.check_at(Bucket::MediaUpload, IP_A, now).is_ok());
    }

    #[test]
    fn classify_matches_only_the_three_limited_families() {
        let post = Method::POST;
        let get = Method::GET;
        let cases: &[(Method, &str, Option<Bucket>)] = &[
            (post.clone(), "/api/auth/login", Some(Bucket::Login)),
            (get.clone(), "/api/auth/login", None),
            (post.clone(), "/api/auth/logout", None),
            (
                post.clone(),
                "/api/integrations/connect/github-pat",
                Some(Bucket::Connect),
            ),
            (
                get.clone(),
                "/api/integrations/connect/linkedin",
                Some(Bucket::Connect),
            ),
            (post.clone(), "/api/integrations", None),
            (post.clone(), "/api/media", Some(Bucket::MediaUpload)),
            // Serving existing media must never be throttled.
            (get.clone(), "/api/media/abc-123", None),
            (get.clone(), "/health", None),
            (get.clone(), "/api/posts", None),
        ];

        for (method, path, expected) in cases {
            assert_eq!(
                classify(method, path),
                *expected,
                "unexpected bucket for {method} {path}"
            );
        }
    }

    #[test]
    fn too_many_requests_emits_429_with_retry_after() {
        let res = too_many_requests(Duration::from_secs(42));

        assert_eq!(res.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res.headers().get(header::RETRY_AFTER).unwrap(), "42");
    }

    #[test]
    fn sub_second_remainder_rounds_retry_after_up() {
        let res = too_many_requests(Duration::from_millis(400));

        assert_eq!(res.headers().get(header::RETRY_AFTER).unwrap(), "1");
    }

    #[test]
    fn client_key_prefers_the_first_forwarded_hop() {
        let req = Request::builder()
            .header("x-forwarded-for", "203.0.113.7, 10.0.0.1, 10.0.0.2")
            .body(Body::empty())
            .expect("valid request");

        assert_eq!(client_key(&req), IP_A);
    }

    #[test]
    fn client_key_falls_back_when_no_proxy_headers_present() {
        let req = Request::builder().body(Body::empty()).expect("valid request");

        assert_eq!(client_key(&req), "no-forwarded-for");
    }
}
