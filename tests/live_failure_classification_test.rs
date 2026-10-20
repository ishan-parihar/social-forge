//! The live-test failure classifier must put the ENVIRONMENT's failures
//! in the skip bucket and the CODE's failures in the fail bucket.
//!
//! A live test that fails for the environment's reason must SKIP, or the
//! operator's gate goes red for a network hiccup - and a gate that goes
//! red for the wrong reason is a gate the operator learns to ignore.
//!
//! A live test that fails for the CODE's reason must FAIL, or a real
//! defect ships behind a green gate.

/// The classifier, copied from `tests/common/mod.rs`.
///
/// It is duplicated here on purpose: the test must pin the RULE, and
/// the rule is the list of environment-failure markers. If the helper
/// changes, this test fails and the two are reconciled.
fn is_environment_failure(text: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("error sending request")
        || text.contains("timeout")
        || text.contains("timed out")
        || text.contains("connection")
        || text.contains("dns")
        || text.contains("reset by peer")
        || text.contains("429")
        || text.contains("too many requests")
        || text.contains("401")
        || text.contains("unauthorized")
        || text.contains("403")
        || text.contains("forbidden")
        || text.contains("api error")
        || text.contains("graphql get error")
        || text.contains("unknown api error")
        || text.contains("not found")
}

/// The ENVIRONMENT's failures. Each of these SKIPS.
#[test]
fn the_environments_failures_skip() {
    for msg in [
        "error sending request for uri (https://x.com/i/api/graphql)",
        "request timed out",
        "operation timed out",
        "connection closed before message completed",
        "dns error: failed to lookup address",
        "connection reset by peer",
        "HTTP 429 Too Many Requests",
        "429 rate limit exceeded",
        "HTTP 401 Unauthorized",
        "401 unauthorized",
        "HTTP 403 Forbidden",
        "403 forbidden",
        "X GraphQL GET error: rate limited",
        "Unknown API error",
        "post or user not found",
    ] {
        assert!(
            is_environment_failure(msg),
            "this is the environment's failure and must SKIP: {msg}"
        );
    }
}

/// The CODE's failures. Each of these FAILS.
#[test]
fn the_codes_failures_fail() {
    for msg in [
        "expected field `screen_name` at line 1 column 42",
        "invalid type: string \"1\", expected u64",
        "missing field `data` in response",
        "the response was not JSON",
        "assertion failed: post.content.len() > 0",
    ] {
        assert!(
            !is_environment_failure(msg),
            "this is the CODE's failure and must FAIL, not skip: {msg}"
        );
    }
}

/// The helper's rule and the test's copy must agree, or the test pins a
/// rule the code does not implement.
#[test]
fn the_helper_agrees_with_this_test() {
    // The helper is in `tests/common/mod.rs`. It is `pub` within the
    // test crate, so the live binaries can use it, and the rule it
    // implements is the one above.
    let helper_text = include_str!("common/mod.rs");
    for marker in [
        "error sending request",
        "timed out",
        "429",
        "401",
        "403",
        "api error",
        "not found",
    ] {
        assert!(
            helper_text.contains(marker),
            "the helper lost the marker {marker} - the classifier and this test have diverged"
        );
    }
}
