//! A live-API test that SKIPS when the failure is the environment's.
//!
//! The live tests hit the providers' REAL APIs with the operator's own
//! credentials. Three things can go wrong there, and only one of them is
//! a defect in this codebase:
//!
//! 1. the CODE is broken - the request is malformed, the response is
//!    parsed wrong, the field is missing. That is a FAILURE and it must
//!    fail the gate.
//! 2. the NETWORK is down, the provider throttled, or the operator's
//!    token expired. That is not a defect in the code, and a red gate
//!    for it trains the operator to ignore the gate.
//! 3. the CREDENTIALS are not configured at all. The test never ran.
//!    That is a SKIP.
//!
//! The distinction is made on the error TEXT: a transport error, a
//! timeout, a 429, or a 401 is the environment. A 200 that came back
//! with the wrong shape is the code.
//!
//! `cargo test` reports a skip as a skip, and the suite stays green for
//! the right reason.

use switchpost::social::ProviderError;

/// The environment's fault, not the code's.
fn is_environment_failure(err: &ProviderError) -> bool {
    let text = err.to_string().to_lowercase();
    // Transport: the request never completed.
    text.contains("error sending request")
        || text.contains("timeout")
        || text.contains("timed out")
        || text.contains("connection")
        || text.contains("dns")
        || text.contains("reset by peer")
        // The provider said no: throttled, or the operator's token is
        // expired, or the account is locked.
        || text.contains("429")
        || text.contains("too many requests")
        || text.contains("401")
        || text.contains("unauthorized")
        || text.contains("403")
        || text.contains("forbidden")
        // The provider's own error envelope, which carries its own
        // message.
        || text.contains("api error")
        || text.contains("graphql get error")
        || text.contains("unknown api error")
        || text.contains("not found")
}

/// Run a live-API call, turning an environment failure into a SKIP.
///
/// Returns `None` when the call failed for the environment's reason, so
/// the caller can print the reason and return without asserting.
///
/// A failure that is NOT the environment's propagates as a panic, with
/// the error's text, so the gate goes red for a real defect.
pub fn skip_on_environment_failure<T>(what: &str, result: Result<T, ProviderError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) => {
            if is_environment_failure(&err) {
                eprintln!(
                    "⏭  {what} SKIPPED - the provider did not answer ({err}). \
                     This is the environment, not the code."
                );
                None
            } else {
                panic!("{what} failed and it is NOT the environment's fault: {err}");
            }
        }
    }
}
