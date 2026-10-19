// ─── HTML -> plain text must not eat the post ─────────────────
//
// The providers publish plain text, so the rich-text editor's HTML has
// to come back down to text on the way in.
//
// The conversion used to be a character-level state machine: set a flag
// on `<`, clear it on `>`, drop everything in between. It silently
// DESTROYED content:
//
//   "Use <div> for layout"       -> "Use  for layout"
//   "Ship it! 5 < 10 and 10 > 5"  -> "Ship it! 5  5"
//
// A post that talks about code, or compares two numbers, lost the words
// between the angle brackets, with no error and no way for the founder
// to know.
//
// Run: cargo test --test html_to_text_test

use social_forge::services::PostService;
use social_forge::social::common::strip_html_tags;

#[test]
fn angle_brackets_that_are_not_tags_are_kept() {
    // A `<` that does not start a tag is LITERAL TEXT. The old
    // implementation dropped everything from the `<` to the next `>`,
    // so a post comparing two numbers lost the middle.
    assert_eq!(
        strip_html_tags("Ship it! 5 < 10 and 10 > 5"),
        "Ship it! 5 < 10 and 10 > 5"
    );
    assert_eq!(strip_html_tags("a < b > c"), "a < b > c");
    assert_eq!(strip_html_tags("100% < 50%?"), "100% < 50%?");
    // A `<` followed by a SPACE is never a tag.
    assert_eq!(strip_html_tags("if a < b then"), "if a < b then");
}

#[test]
fn a_real_tag_is_still_removed() {
    // A `<` followed by a letter IS a tag, and it is removed - that is
    // the point of the function. The app never sends HTML any more (the
    // editor speaks plain text, the same contract the server and the
    // providers use), so this is the safety net for HTML that arrives
    // from somewhere else.
    //
    // `<div>` is a BLOCK element, so it ends a line - which is why the
    // result has a newline where the tag was.
    assert_eq!(strip_html_tags("Use <div> for layout"), "Use \n for layout");
    assert_eq!(strip_html_tags("<p>Hello</p>"), "Hello");
    // An inline tag leaves no line break.
    assert_eq!(strip_html_tags("Hello <b>world</b>"), "Hello world");
}

#[test]
fn plain_text_is_left_alone_by_the_service() {
    // The app sends plain text, so the service must NOT touch it. A
    // post that mentions a tag keeps the tag, because the text the
    // founder typed is the text that gets published.
    let body = "Use <div> for layout";
    let clean = PostService::sanitize_content(body, 2000);
    assert_eq!(clean, body, "plain text must survive untouched");
    assert!(clean.contains("<div>"));

    // And a comparison.
    let body2 = "Ship it! 5 < 10 and 10 > 5";
    assert_eq!(PostService::sanitize_content(body2, 2000), body2);
}

#[test]
fn html_is_still_converted_by_the_service() {
    // The server is a trust boundary: HTML that arrives from the MCP
    // tool or a webhook is still converted. The evidence of markup is a
    // CLOSING tag, an attribute, or a void tag.
    let clean = PostService::sanitize_content("<p>Hello <b>world</b></p>", 2000);
    assert_eq!(clean, "Hello world");
    let clean2 = PostService::sanitize_content("<p>One</p><p>Two</p>", 2000);
    assert_eq!(clean2, "One\nTwo");
    // A void tag.
    let clean3 = PostService::sanitize_content("One<br>Two", 2000);
    assert_eq!(clean3, "One\nTwo");
    // An attribute.
    let clean4 = PostService::sanitize_content("<p class=\"lead\">Hello</p>", 2000);
    assert_eq!(clean4, "Hello");
}

#[test]
fn real_tags_are_removed() {
    assert_eq!(strip_html_tags("<p>Hello</p>"), "Hello");
    assert_eq!(strip_html_tags("<p>Hello <b>world</b></p>"), "Hello world");
    assert_eq!(strip_html_tags("<em>a</em> and <i>b</i>"), "a and b");
}

#[test]
fn block_elements_end_a_line() {
    assert_eq!(strip_html_tags("<p>One</p><p>Two</p>"), "One\nTwo");
    assert_eq!(strip_html_tags("One<br>Two"), "One\nTwo");
    assert_eq!(strip_html_tags("<ul><li>A</li><li>B</li></ul>"), "A\nB");
    assert_eq!(strip_html_tags("<h2>Title</h2><p>Body</p>"), "Title\nBody");
}

#[test]
fn entities_are_decoded() {
    assert_eq!(strip_html_tags("Tom &amp; Jerry"), "Tom & Jerry");
    assert_eq!(strip_html_tags("&lt;div&gt;"), "<div>");
    // An entity the decoder does not know is left as it is.
    assert_eq!(strip_html_tags("caf&eacute;"), "caf&eacute;");
    assert_eq!(strip_html_tags("&#65;"), "A");
    assert_eq!(strip_html_tags("&#x1F600;"), "\u{1F600}");
}

#[test]
fn script_and_style_bodies_are_dropped() {
    assert_eq!(
        strip_html_tags("<p>Keep</p><script>alert(1)</script>"),
        "Keep"
    );
    assert_eq!(
        strip_html_tags("<style>.a{color:red}</style><p>Keep</p>"),
        "Keep"
    );
}

#[test]
fn the_service_sanitizer_agrees_with_the_helper() {
    // The service and the helper must be ONE implementation, or the
    // stored content and the length that was validated differ.
    let html = "<p>Use &lt;div&gt; for layout</p><p>Second</p>";
    assert_eq!(
        PostService::sanitize_content(html, 2000),
        strip_html_tags(html)
    );
}

#[test]
fn the_service_sanitizer_does_not_eat_a_comparison() {
    // The case that motivated the fix: a post that compares two numbers
    // lost everything between the angle brackets, with no error.
    let body = "Ship it! 5 < 10 and 10 > 5";
    let clean = PostService::sanitize_content(body, 2000);
    assert_eq!(clean, body, "the post text must survive sanitisation");
    assert!(clean.contains("< 10"), "the comparison must survive");
}

#[test]
fn an_unterminated_angle_bracket_is_text() {
    assert_eq!(strip_html_tags("5 < 10"), "5 < 10");
    assert_eq!(strip_html_tags("open <br"), "open <br");
}

// ─── The composer and the server must agree ───────────────────
//
// The composer's `plainText` (frontend/src/lib/composer/platforms.ts)
// and the server's `sanitize_content` are two implementations of one
// rule, in two languages. The character count the composer shows is
// the count of THIS string, so if the two disagree the number on
// screen is the count of a different post than the one that gets
// published.
//
// They disagreed once: the browser stripped on any `<` followed by a
// letter, while the server (behind `looks_like_html`) only converted
// on EVIDENCE of markup. A post reading "Use <div> for layout"
// measured 19 in the composer and was stored as 23.
//
// The browser's implementation is transcribed here and compared against
// the server's.

/// The composer's `plainText`, transcribed from
/// frontend/src/lib/composer/platforms.ts.
fn composer_plain_text(s: &str) -> String {
    if !composer_looks_like_html(s) {
        return s.trim().to_string();
    }
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '<' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let next = chars.get(i + 1).copied();
        let is_tag = matches!(next, Some(c) if c.is_ascii_alphabetic() || c == '/' || c == '!' || c == '?');
        if !is_tag {
            out.push('<');
            i += 1;
            continue;
        }
        let rest: String = chars[i + 1..].iter().collect();
        let lower = rest.to_ascii_lowercase();
        let end = lower.find('>').map(|p| p + i + 1);
        let Some(end) = end else {
            out.push('<');
            i += 1;
            continue;
        };
        let tag: String = chars[i + 1..end].iter().collect::<String>().to_ascii_lowercase();
        let name = tag
            .trim_start_matches(['/', '!', '?'])
            .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or("")
            .to_string();
        // `<br>` is a line break, and a BLOCK element ends the line -
        // but a block only ends a line that has content, so the two
        // cases differ in the guard, not in the action.
        let is_line_break = name == "br";
        let is_block_end = composer_is_block(&name) && !out.is_empty() && !out.ends_with('\n');
        if is_line_break || is_block_end {
            out.push('\n');
        }
        i = end + 1;
    }
    // Collapse the runs of blank lines.
    let mut result = String::new();
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

fn composer_is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div" | "li" | "ul" | "ol"
            | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            | "blockquote" | "pre" | "section" | "article"
            | "header" | "footer" | "tr" | "table"
    )
}

fn composer_looks_like_html(s: &str) -> bool {
    const VOID_TAGS: &[&str] = &["br", "hr", "img", "input", "meta", "link", "wbr", "source"];
    const KNOWN_TAGS: &[&str] = &[
        "p", "div", "span", "br", "hr", "b", "i", "u", "strong", "em", "s", "strike", "a", "ul",
        "ol", "li", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "pre", "code", "section",
        "article", "header", "footer", "main", "aside", "nav", "figure", "figcaption", "table",
        "thead", "tbody", "tr", "td", "th", "img", "picture", "source", "video", "audio", "iframe",
        "script", "style", "small", "sub", "sup", "mark", "del", "ins", "abbr", "cite", "q",
        "kbd", "samp", "var", "wbr",
    ];
    let lower = s.to_ascii_lowercase();
    let bytes: Vec<char> = lower.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != '<' {
            i += 1;
            continue;
        }
        let next = bytes.get(i + 1).copied();
        if matches!(next, Some('/') | Some('!') | Some('?')) {
            return true;
        }
        let rest: String = bytes[i + 1..].iter().collect();
        for tag in VOID_TAGS {
            if let Some(after_tag) = rest.strip_prefix(tag) {
                if matches!(after_tag.chars().next(), Some('>') | Some(' ') | Some('/')) {
                    return true;
                }
            }
        }
        for tag in KNOWN_TAGS {
            if let Some(after_tag) = rest.strip_prefix(tag) {
                if matches!(after_tag.chars().next(), Some(' ') | Some('\t') | Some('\n')) {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

#[test]
fn the_composer_and_the_server_produce_the_same_text() {
    let cases = [
        "Use <div> for layout",
        "Ship it! 5 < 10 and 10 > 5",
        "<p>Hello <b>world</b></p>",
        "<p>One</p><p>Two</p>",
        "One<br>Two",
        "Plain text with no markup at all",
        "a < b > c",
        "<p class=\"lead\">Hello</p>",
        "Tom &amp; Jerry",
        "",
    ];
    for case in cases {
        assert_eq!(
            composer_plain_text(case),
            PostService::sanitize_content(case, 2000),
            "the composer and the server disagree on {case:?}"
        );
    }
}
