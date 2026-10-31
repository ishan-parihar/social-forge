// ─── Common OAuth2 Helpers ────────────────────────────────────
// PKCE code generation, state generation, and token exchange.

use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin};

const CODE_VERIFIER_CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a PKCE code verifier (128 chars)
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let mut chars = Vec::with_capacity(128);
    for _ in 0..128 {
        let idx = rng.gen_range(0..CODE_VERIFIER_CHARSET.len());
        chars.push(CODE_VERIFIER_CHARSET[idx] as char);
    }
    chars.into_iter().collect()
}

/// Generate SHA256 code challenge from verifier (PKCE S256)
pub fn generate_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a random OAuth state string (hex-encoded 32 bytes)
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = vec![0u8; 32];
    RngCore::fill_bytes(&mut rng, &mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a timestamp string to DateTime<Utc>.
/// Handles RFC3339 and common non-standard formats like `+0000` (missing colon).
/// Returns `Utc::now()` as fallback on parse failure.
pub fn parse_timestamp(s: &str) -> chrono::DateTime<chrono::Utc> {
    // Legacy infallible form kept for non-feed callers (DMs, comments) that
    // already had a "now" fallback. Feed imports must use
    // parse_timestamp_opt and SKIP undated items instead.
    parse_timestamp_opt(s).unwrap_or_else(chrono::Utc::now)
}

/// Parse a provider timestamp, returning None when unparseable.
///
/// A fabricated "now" for a missing date reads as a fake fresh post in
/// the feed (the "hallucinated feed" report), so every feed-import
/// caller must SKIP the item when this returns None rather than stamp
/// it with the import time.
pub fn parse_timestamp_opt(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let s = s.trim();
    let normalized = if s.len() >= 6 {
        let (prefix, offset) = s.split_at(s.len() - 5);
        let offset_chars: Vec<char> = offset.chars().collect();
        if offset_chars.len() == 5 && (offset_chars[0] == '+' || offset_chars[0] == '-') {
            format!(
                "{}{}{}:{}",
                prefix,
                offset_chars[0],
                &offset[1..3],
                &offset[3..5]
            )
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    };

    chrono::DateTime::parse_from_rfc3339(&normalized)
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(s))
        .map(|d| d.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .map(|d| d.and_utc())
                .map_err(|_| ())
        })
        .ok()
}

/// Resolve a caller-supplied OAuth `redirect_uri` against the deployment, or
/// refuse it.
///
/// The connect endpoints accept `?redirect_uri=` (the REST connect route, the
/// public `/setup` connect route, the MCP `integrations_connect` tool). Whatever
/// they receive is passed to `generate_auth_url`, which URL-encodes it into the
/// authorize URL, and to `save_oauth_state`, which stores it for the callback.
/// So the value is not cosmetic: it is where the provider sends an
/// authorization code.
///
/// Three ways a raw value goes wrong, all of them observed:
///
///   1. A JSON-quoted string. `"https://site/api/auth/callback"` - quotes
///      included - reaches the authorize URL and the provider answers
///      `URL Blocked`. Meta then renders its consent page with the quoted value
///      visible, which reads as OUR bug and sends the operator to hunt a URL we
///      never sent. Trim the quotes.
///   2. An arbitrary host. `?redirect_uri=https://evil.test/cb` is honoured
///      verbatim, so whoever controls the query string controls where the code
///      goes. An open redirect in an OAuth flow is the worst shape of that bug:
///      the login SUCCEEDS from the user's point of view.
///   3. Empty or whitespace. Yields a malformed authorize URL.
///
/// The accepted set is DELIBERATELY narrow: `None`, empty, or the callback path
/// relative to `app_url` (`https://{app_url}/api/auth/callback`). That is the
/// one URI the deployment registers with every provider, it is what the server
/// already builds when the caller supplies nothing, and it is the value the
/// existing tree-wide tests assert on the authorize URL.
///
/// NOTE this also closes the divergence between "what we tell the operator"
/// and "what we send": the MCP guide and `/api/providers` report the
/// `app_url` callback, so refusing anything else keeps the reported value the
/// only truthful one.
pub fn resolve_redirect_uri(provided: Option<&str>, app_url: &str) -> Result<String, String> {
    let fallback = format!("{}/api/auth/callback", app_url.trim_end_matches('/'));
    let raw = match provided {
        Some(r) if !r.trim().is_empty() => r.trim(),
        // No opinion from the caller: use the deployment's own callback.
        _ => return Ok(fallback),
    };

    // Undo ONE layer of accidental JSON quoting. An agent or a curl
    // caller that round-trips a value through `serde_json::to_string` hands
    // over the quoted form, and the provider sees literal quotes around the
    // whole parameter.
    //
    // `serde_json` escapes `/` as `\/`, so the form that actually reaches
    // us from that path is
    //
    //     "https:\/\/switchpost.example.com\/api\/auth\/callback"
    //
    // which is what `?redirect_uri=%22https%3A%5C%2F%5C%2F...%22` decodes
    // to. Stripping only the quotes leaves the backslashes in place, the
    // string is then neither the callback nor a parseable URL, and the
    // caller gets a refusal instead of a connect - so the escape has to be
    // undone in the same pass.
    //
    // Only `\/`, `\\` and `\"` are undone, and ONLY inside a string that
    // was actually quoted: an unquoted value is left exactly as it was, so
    // a URL that legitimately contains a backslash (a Windows-style base,
    // or a path segment that has one) is not silently rewritten.
    let unquoted = {
        let (unquoted, was_quoted) = match raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            Some(inner) => (inner.trim(), true),
            None => (raw, false),
        };
        if was_quoted {
            unescape_json_slashes(unquoted).trim().to_string()
        } else {
            unquoted.to_string()
        }
    };

    let unquoted: &str = &unquoted;
    if unquoted.is_empty() {
        return Err(
            "redirect_uri was empty after trimming quotes and whitespace. Omit it to use \
             the deployment's own callback URI."
                .to_string(),
        );
    }

    // The bare callback path is accepted too: it is the same URI relative,
    // and a caller that appends it to a configured base has already done the
    // join correctly.
    if unquoted == "/api/auth/callback" {
        return Ok(fallback);
    }

    // Anything else must be EXACTLY the deployment's callback. A relative path
    // other than the callback, a query string, a fragment, or any other host is
    // refused, because none of those is where this deployment receives codes.
    let parsed = url::Url::parse(unquoted).map_err(|_| {
        // A relative path is the most likely mistake here (a caller that
        // knows the callback's path but not its base), so the refusal names
        // the absolute form the way the "wrong host" one does.
        format!(
            "redirect_uri is not an absolute URL: {unquoted:?}. The callback URI every provider \
             registers is {fallback} (built from APP_URL). Omit redirect_uri to use it."
        )
    })?;
    if !same_callback(&parsed, &fallback) {
        return Err(format!(
            "redirect_uri {unquoted} is not this deployment's callback. The callback URI every \
             provider registers is {fallback} (built from APP_URL). Omit \
             redirect_uri to use it."
        ));
    }
    Ok(fallback)
}

/// Undo the escapes a JSON string adds to a URL.
///
/// `serde_json` escapes `/` as `\/`, `\` as `\\` and `"` as `\"`,
/// so a value that has been through `to_string` is not the URL that was
/// put in. This reverses exactly those three and nothing else, because a
/// URL contains nothing else that JSON would change.
///
/// The caller decides whether to run this (see `resolve_redirect_uri`):
/// only a value that was actually quoted is unescaped, so a URL that
/// contains a real backslash is never rewritten by a path it did not
/// take.
fn unescape_json_slashes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('/') => {
                chars.next();
                out.push('/');
            }
            Some('\\') => {
                chars.next();
                out.push('\\');
            }
            Some('"') => {
                chars.next();
                out.push('"');
            }
            // An unrecognised escape is left as it was, including the
            // backslash: this is not a JSON decoder, and guessing at the
            // rest of the grammar would change values it should not.
            _ => out.push('\\'),
        }
    }
    out
}

/// Is `candidate` the same callback the operator configured?
///
/// Compared on SCHEME + HOST + PORT + PATH, each normalised, because the
/// string comparison refuses values that are the same URI spelled
/// differently - a trailing slash on APP_URL, an uppercase host, an
/// explicit default port. None of those reach the provider as a
/// different URI, so none of them is a defect worth failing the connect
/// over. A query string or a fragment IS a difference: it would be sent
/// verbatim and land on a URI no provider registered.
fn same_callback(candidate: &url::Url, fallback: &str) -> bool {
    let Ok(expected) = url::Url::parse(fallback) else {
        return false;
    };
    if candidate.query().is_some() || candidate.fragment().is_some() {
        return false;
    }
    candidate.scheme() == expected.scheme()
        && candidate
            .host_str()
            .map(|h| h.eq_ignore_ascii_case(expected.host_str().unwrap_or("")))
            == Some(true)
        && candidate.port_or_known_default() == expected.port_or_known_default()
        && candidate.path() == expected.path()
}

/// Does this string contain HTML the stripper should remove?
///
/// The app sends PLAIN TEXT (the editor speaks plain text at its
/// boundary, the same contract the server and the providers use), so
/// the common case is a body that is text and must be left alone - a
/// post reading "Use <div> for layout" keeps its `<div>`.
///
/// But the server is a trust boundary: HTML can arrive from the MCP
/// tool, from a webhook, or from an API client that was written against
/// an older contract. So the sanitizer must still remove markup when
/// markup is really there.
// ─── Token endpoint responses ─────────────────────────────
/// The two HTTP clients this crate uses do NOT give the same response
/// type - `reqwest` and `wreq` each return their own - so a helper that
/// reads a token endpoint has to accept both or be written twice.
///
/// The seam is what the helper actually needs and nothing else: the
/// status, and the body as text. Everything above that is the caller's.
///
/// `text` returns a boxed future rather than being an `async fn` in a
/// trait, because an `async fn` in a trait hides its `Send`ness and the
/// scheduler spawns these onto a JoinSet that requires it. An explicit
/// bound says it, and a caller that loses a `Send` future is a compile
/// error rather than a deadlock.
pub trait TokenResponse {
    fn status(&self) -> StatusLike;
    fn text(self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>;
}

/// A status without dragging in either client's version of the type.
#[derive(Clone, Copy)]
pub struct StatusLike {
    code: u16,
    is_success: bool,
}

impl StatusLike {
    pub fn as_u16(&self) -> u16 {
        self.code
    }
    pub fn is_success(&self) -> bool {
        self.is_success
    }
}

impl TokenResponse for reqwest::Response {
    fn status(&self) -> StatusLike {
        let s = self.status();
        StatusLike {
            code: s.as_u16(),
            is_success: s.is_success(),
        }
    }
    fn text(self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        Box::pin(async move {
            self.text()
                .await
                .map_err(|e| format!("<body unreadable: {e}>"))
        })
    }
}

impl TokenResponse for wreq::Response {
    fn status(&self) -> StatusLike {
        let s = self.status();
        StatusLike {
            code: s.as_u16(),
            is_success: s.is_success(),
        }
    }
    fn text(self) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>> {
        Box::pin(async move {
            self.text()
                .await
                .map_err(|e| format!("<body unreadable: {e}>"))
        })
    }
}

/// Read an access token out of a token-endpoint response WITHOUT
/// discarding the provider's error body.
///
/// The original shape of this code, repeated in every provider, was:
///
/// ```text
/// let json: Value = resp.json().await?;
/// let access_token = json["access_token"]
///     .as_str()
///     .ok_or_else(|| ProviderError::Auth("Missing access_token".into()))?;
/// ```
///
/// `reqwest`'s `error_for_status` was never called, and the body was
/// never read, so an OAuth failure and a redirect_uri mismatch and an
/// expired code and a wrong client secret all collapsed into the SAME
/// sentence: "Missing access_token". The one thing the operator needs -
/// WHICH of those it was - is the thing the code threw away.
///
/// `resp` is moved and converted to a status plus a body string, so
/// the body survives the status check instead of being dropped by it.
/// Returns `(access_token, full_json)` on success - the body is handed
/// back because the call sites read `refresh_token` and `expires_in` out
/// of it immediately after, and forcing them to re-parse would just move
/// the same problem. Errors name the provider, the status, and the
/// provider's own words - which is what the founder can actually act on.
pub async fn extract_access_token<R>(
    resp: R,
    provider: &str,
) -> Result<(String, serde_json::Value), String>
where
    R: TokenResponse,
{
    let status = resp.status();
    // Take the body FIRST: `status()` borrows, `text()` consumes, and a
    // failure needs the body to be worth reporting.
    let body = resp
        .text()
        .await
        .unwrap_or_else(|e| format!("<body unreadable: {e}>"));
    if !status.is_success() {
        // Quote the provider, but not so much of it that a login page
        // full of markup becomes the error message.
        let quoted = if body.chars().count() > 400 {
            let clipped: String = body.chars().take(400).collect();
            format!("{clipped}...")
        } else {
            body
        };
        return Err(format!(
            "{provider} token exchange failed (HTTP {}) {quoted}",
            status.as_u16()
        ));
    }
    // A 2xx with no token is still a failure, and an empty token is the
    // shape that silently "succeeds" everywhere else - see discord.rs.
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            // A char-safe clip: a byte index can slice into the middle of a
            // multi-byte character and panic.
            let clipped: String = body.chars().take(200).collect();
            let why = if looks_like_html(&body) {
                "an HTML page (check the redirect_uri and client_id)"
            } else {
                clipped.as_str()
            };
            return Err(format!(
                "{provider} token exchange returned non-JSON body: {why}"
            ));
        }
    };
    json["access_token"]
        .as_str()
        .filter(|t| !t.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{provider} token exchange returned no access_token: {body}"))
        .map(|t| (t, json))
}

/// The evidence of markup is a CLOSING tag, or a tag with an
/// attribute, or a `<br>`:
///   - `</p>` - a closing tag. A sentence never has one.
///   - `<p class="x">` - an attribute. A sentence never has one.
///   - `<br>` - a bare void tag.
///
/// A bare `<div>` is NOT evidence: it is a word in angle brackets, and
/// a post about HTML is full of them. This is the line between "the
/// founder typed `<div>`" and "this is markup", and it is the line
/// that keeps a post's text intact.
pub fn looks_like_html(s: &str) -> bool {
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '<' {
            i += 1;
            continue;
        }
        let next = bytes.get(i + 1).copied();
        // A closing tag.
        if next == Some('/') {
            return true;
        }
        // A comment, a doctype or a processing instruction.
        if next == Some('!') || next == Some('?') {
            return true;
        }
        // `<br>`, `<hr>`, `<img ...>` - a void tag, or a tag with an
        // attribute.
        let rest: String = bytes[i + 1..]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        for tag in VOID_TAGS {
            if let Some(after_tag) = rest.strip_prefix(tag) {
                let after = after_tag.chars().next();
                if matches!(after, Some('>') | Some(' ') | Some('/')) {
                    return true;
                }
            }
        }
        // A tag with an attribute: `<p class="x">`.
        for tag in KNOWN_TAGS {
            if let Some(after_tag) = rest.strip_prefix(tag) {
                let after = after_tag.chars().next();
                if matches!(after, Some(' ') | Some('\t') | Some('\n')) {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// The void tags - a tag that never has a closing form.
const VOID_TAGS: &[&str] = &["br", "hr", "img", "input", "meta", "link", "wbr", "source"];

/// The tag names the stripper recognises.
const KNOWN_TAGS: &[&str] = &[
    "p",
    "div",
    "span",
    "br",
    "hr",
    "b",
    "i",
    "u",
    "strong",
    "em",
    "s",
    "strike",
    "a",
    "ul",
    "ol",
    "li",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "code",
    "section",
    "article",
    "header",
    "footer",
    "main",
    "aside",
    "nav",
    "figure",
    "figcaption",
    "table",
    "thead",
    "tbody",
    "tr",
    "td",
    "th",
    "img",
    "picture",
    "source",
    "video",
    "audio",
    "iframe",
    "script",
    "style",
    "small",
    "sub",
    "sup",
    "mark",
    "del",
    "ins",
    "abbr",
    "cite",
    "q",
    "kbd",
    "samp",
    "var",
    "wbr",
];

/// Convert HTML to the plain text a social provider expects.
///
/// The providers publish plain text, so the rich-text editor's HTML has
/// to come back down to text on the way in.
///
/// The previous implementation was a character-level state machine: it
/// set a flag on `<` and cleared it on `>`, and dropped everything in
/// between. That silently DESTROYED content:
///
/// ```text
/// "Ship it! 5 < 10 and 10 > 5"     -> "Ship it! 5  5"
/// ```
///
/// A post that compares two numbers lost the words between the angle
/// brackets, with no error and no way for the founder to know.
///
/// This is a real HTML-to-text conversion:
///   - a tag is a `<` followed by a tag name, `/`, `!` or `?`, and it
///     ends at the matching `>`. A `<` that is not the start of a tag
///     is literal text.
///   - a BLOCK element ends a line. A `<br>` ends a line.
///   - `&amp;` and the other common entities are decoded.
///   - `<script>` and `<style>` bodies are dropped entirely (their
///     contents are code, not prose).
pub fn strip_html_tags(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0usize;
    // The tag whose content is dropped entirely (script/style).
    let mut skipping: Option<&'static str> = None;

    while i < chars.len() {
        let ch = chars[i];

        if let Some(name) = skipping {
            // Look for the closing tag.
            let close = format!("</{}", name);
            if chars[i..].len() >= close.len()
                && chars[i..]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase()
                    .starts_with(&close)
            {
                skipping = None;
                i += close.len();
                // Skip to the '>' of the closing tag.
                while i < chars.len() && chars[i] != '>' {
                    i += 1;
                }
                i += 1;
                continue;
            }
            i += 1;
            continue;
        }

        if ch == '<' {
            // Is this the start of a tag, or a literal '<'?
            let next = chars.get(i + 1).copied();
            let is_tag = matches!(next, Some(c) if c.is_ascii_alphabetic() || c == '/' || c == '!' || c == '?');
            if !is_tag {
                out.push('<');
                i += 1;
                continue;
            }

            // Find the end of the tag.
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '>' {
                j += 1;
            }
            if j >= chars.len() {
                // An unterminated '<' is literal text.
                out.push('<');
                i += 1;
                continue;
            }
            let tag: String = chars[i + 1..j].iter().collect();
            let lower = tag.to_ascii_lowercase();
            let name = lower
                .trim_start_matches('/')
                .trim_start_matches('!')
                .trim_start_matches('?')
                .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .next()
                .unwrap_or("")
                .to_string();

            if name == "script" || name == "style" {
                skipping = Some(if name == "script" { "script" } else { "style" });
            } else if name == "br" {
                out.push('\n');
            } else if is_block_element(&name) {
                // A block ends the line - but do not add a blank line if
                // the text is already at one.
                if !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }
            }
            i = j + 1;
            continue;
        }

        if ch == '&' {
            if let Some((decoded, len)) = decode_entity(&chars, i) {
                out.push_str(&decoded);
                i += len;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }

    // Collapse the runs of blank lines the block handling can leave.
    let mut result = String::with_capacity(out.len());
    let mut blank = 0usize;
    for c in out.chars() {
        if c == '\n' {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        result.push(c);
    }
    result.trim().to_string()
}

/// The elements that end a line when their tag is removed.
fn is_block_element(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "br"
            | "li"
            | "ul"
            | "ol"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "blockquote"
            | "pre"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "tr"
            | "table"
    )
}

/// Decode the HTML entities a text editor can produce.
fn decode_entity(chars: &[char], at: usize) -> Option<(String, usize)> {
    // Find the ';' within a sane distance.
    let mut j = at + 1;
    while j < chars.len() && j - at <= 10 && chars[j] != ';' {
        j += 1;
    }
    if j >= chars.len() || chars[j] != ';' {
        return None;
    }
    let name: String = chars[at + 1..j].iter().collect();
    // The named entities, and the numeric ones (&#123; / &#x1F600;).
    let decoded: Option<String> = match name.as_str() {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" => Some("'".to_string()),
        "nbsp" => Some(" ".to_string()),
        "hellip" => Some("\u{2026}".to_string()),
        "mdash" => Some("\u{2014}".to_string()),
        "ndash" => Some("\u{2013}".to_string()),
        _ => {
            if let Some(rest) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                u32::from_str_radix(rest, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
            } else {
                name.strip_prefix('#')
                    .and_then(|rest| rest.parse::<u32>().ok())
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
            }
        }
    };
    let decoded = decoded?;
    Some((decoded, j - at + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- resolve_redirect_uri -------------------------------------------------
    //
    // The handler used to take `query.redirect_uri` verbatim into the
    // authorize URL. A JSON-quoted value reached Meta with literal quotes
    // (the Threads error the operator reported) and an arbitrary host was
    // honoured too (an open redirect inside the OAuth flow).

    const APP: &str = "https://switchpost.example.com";
    const CALLBACK: &str = "https://switchpost.example.com/api/auth/callback";

    #[test]
    fn omitting_redirect_uri_uses_the_deployed_callback() {
        assert_eq!(resolve_redirect_uri(None, APP).unwrap(), CALLBACK);
    }

    #[test]
    fn empty_redirect_uri_falls_back_to_the_deployed_callback() {
        assert_eq!(resolve_redirect_uri(Some("   "), APP).unwrap(), CALLBACK);
    }

    #[test]
    fn the_deployments_own_callback_is_accepted() {
        assert_eq!(resolve_redirect_uri(Some(CALLBACK), APP).unwrap(), CALLBACK);
    }

    #[test]
    fn a_json_quoted_callback_is_unwrapped_not_rejected() {
        // The reported Threads shape: a caller that round-tripped the value
        // through serde_json sent `"https://..."` with the quotes in place.
        // The fix must accept it AND return the unquoted form, because the
        // quoted form is not a working callback.
        let quoted = format!("\"{CALLBACK}\"");
        let resolved = resolve_redirect_uri(Some(&quoted), APP).unwrap();
        assert_eq!(resolved, CALLBACK);
        assert!(!resolved.contains('"'));
    }

    #[test]
    fn the_serde_json_escaped_slash_form_is_unwrapped_not_rejected() {
        // The EXACT value from the operator's Threads consent page:
        //
        //   params[redirect_uri]=%22https%3A%5C%2F%5C%2Fswitchpost...%22
        //
        // `%22` is the quote, `%5C%2F` is a BACKSLASH then a slash. That is
        // what `serde_json::to_string` emits for `/`, so the value that
        // reached the handler was
        //
        //   "https:\/\/switchpost.example.com\/api\/auth\/callback"
        //
        // Stripping the quotes alone leaves the backslashes, the string
        // is neither the callback nor a parseable URL, and the caller is
        // refused - so the escape has to be undone in the same pass.
        let escaped = "\"https:\\/\\/switchpost.example.com\\/api\\/auth\\/callback\"";
        let resolved = resolve_redirect_uri(Some(escaped), APP).unwrap();
        assert_eq!(
            resolved, CALLBACK,
            "the JSON-escaped form must resolve to the plain callback, not a \
             string with backslashes in it. Got: {resolved}"
        );
        assert!(
            !resolved.contains('\\'),
            "the resolved value must not carry backslashes into the authorize \
             URL. Got: {resolved}"
        );
    }

    #[test]
    fn an_escaped_slash_foreign_host_is_still_refused() {
        // Unescaping must not become a bypass: a quoted-and-escaped URL
        // pointing at another host is still an open redirect and must be
        // refused, which means the unescape runs BEFORE the host check and
        // not instead of it.
        let escaped = "\"https:\\/\\/evil.test\\/cb\"";
        let err = resolve_redirect_uri(Some(escaped), APP).unwrap_err();
        assert!(
            err.contains("not this deployment's callback"),
            "an escaped foreign host was accepted: {err}"
        );
    }

    #[test]
    fn a_double_quoted_callback_is_refused() {
        // Two layers of quoting: stripping one leaves the inner quotes, so
        // the value is not the configured callback and is refused with a
        // message that names it.
        let double_quoted = format!("\"\"{CALLBACK}\"\"");
        let err = resolve_redirect_uri(Some(&double_quoted), APP).unwrap_err();
        assert!(
            err.contains(CALLBACK),
            "the refusal must name the correct callback so the caller can act: {err}"
        );
    }

    #[test]
    fn a_foreign_host_is_refused() {
        let err = resolve_redirect_uri(Some("https://evil.test/cb"), APP).unwrap_err();
        assert!(err.contains("not this deployment's callback"), "{err}");
        assert!(err.contains(CALLBACK));
    }

    #[test]
    fn a_relative_path_other_than_the_callback_is_refused() {
        // `/api/auth/close` is a real path in this app, not the callback: it
        // is refused (it is not a URL, so it cannot be compared to one) and
        // the message still points the caller at the right URI.
        let err = resolve_redirect_uri(Some("/api/auth/close"), APP).unwrap_err();
        assert!(err.contains(CALLBACK), "{err}");
    }

    #[test]
    fn the_bare_callback_path_is_accepted() {
        assert_eq!(
            resolve_redirect_uri(Some("/api/auth/callback"), APP).unwrap(),
            CALLBACK
        );
    }

    #[test]
    fn the_callback_with_a_query_string_is_refused() {
        // A query on the callback is a different URI as far as the provider
        // is concerned, and it would be sent verbatim.
        let with_query = format!("{CALLBACK}?next=/drain");
        let err = resolve_redirect_uri(Some(&with_query), APP).unwrap_err();
        assert!(err.contains("not this deployment's callback"), "{err}");
    }

    #[test]
    fn a_non_url_redirect_uri_is_refused_without_panicking() {
        let err = resolve_redirect_uri(Some("not a url at all"), APP).unwrap_err();
        assert!(err.contains("not an absolute URL"), "{err}");
        assert!(err.contains(CALLBACK), "{err}");
    }

    #[test]
    fn the_same_callback_spelled_differently_is_still_accepted() {
        // An APP_URL with a trailing slash, an uppercase host, or an explicit
        // default port describe the same URI. Refusing those would fail a
        // working connect over a spelling difference (see same_callback).
        for (app_url, provided) in [
            ("https://switchpost.example.com/", CALLBACK.to_string()),
            (
                APP,
                "HTTPS://SWITCHPOST.EXAMPLE.COM/api/auth/callback".to_string(),
            ),
            (APP, CALLBACK.to_string()),
            (
                APP,
                "https://switchpost.example.com:443/api/auth/callback".to_string(),
            ),
        ] {
            assert_eq!(
                resolve_redirect_uri(Some(&provided), app_url).unwrap(),
                format!("{}/api/auth/callback", app_url.trim_end_matches('/')),
                "{provided} against APP_URL {app_url} is the same callback"
            );
        }
    }

    #[test]
    fn a_scheme_downgrade_is_refused() {
        // http on the same host is a different URI and a downgrade: the
        // provider would be told to return the code over plaintext.
        let http = "http://switchpost.example.com/api/auth/callback";
        let err = resolve_redirect_uri(Some(http), APP).unwrap_err();
        assert!(err.contains("not this deployment's callback"), "{err}");
    }

    #[test]
    fn test_generate_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 128, "verifier should be 128 chars");
    }

    #[test]
    fn test_generate_code_verifier_charset() {
        let verifier = generate_code_verifier();
        // All chars should be from the allowed set
        for c in verifier.chars() {
            assert!(
                (c.is_ascii_alphanumeric()) || c == '-' || c == '.' || c == '_' || c == '~',
                "illegal char '{}' in verifier",
                c
            );
        }
    }

    #[test]
    fn test_generate_code_verifier_unique() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(
            v1, v2,
            "two consecutive verifiers should differ (random salt)"
        );
    }

    #[test]
    fn test_generate_code_challenge_deterministic() {
        let verifier = "test-verifier-string-1234567890abcdefghijklmnopqrstuvwxyz";
        let c1 = generate_code_challenge(verifier);
        let c2 = generate_code_challenge(verifier);
        assert_eq!(c1, c2, "same verifier should produce same challenge");
    }

    #[test]
    fn test_generate_code_challenge_length() {
        let verifier = "short";
        let challenge = generate_code_challenge(verifier);
        // SHA256 base64url = 43 chars (no padding)
        assert_eq!(
            challenge.len(),
            43,
            "SHA256 base64url challenge should be 43 chars"
        );
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        // 32 bytes hex = 64 chars
        assert_eq!(state.len(), 64, "state should be 64 hex chars");
    }

    #[test]
    fn test_generate_state_hex_chars() {
        let state = generate_state();
        for c in state.chars() {
            assert!(c.is_ascii_hexdigit(), "state char '{}' is not hex", c);
        }
    }

    #[test]
    fn test_generate_state_unique() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2, "states should be unique");
    }
}
