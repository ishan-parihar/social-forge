// ─── Common OAuth2 Helpers ────────────────────────────────────
// PKCE code generation, state generation, and token exchange.

use rand::{Rng, RngCore};
use sha2::{Digest, Sha256};

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
///
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
