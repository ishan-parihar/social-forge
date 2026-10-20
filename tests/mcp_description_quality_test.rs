// ─── An MCP tool description must be a complete sentence ───────
//
// Commit 7c93166 ("MCP refactor + schema optimizer") did a "token
// optimization" pass over the tool descriptions and CUT THEM MID-SENTENCE:
//
//   "Get OAuth URL to connect a social media channel"
//     -> "Get OAuth URL to connect a social"
//   "List posts with optional state filter (draft|queued|published|error)"
//     -> "List posts with optional state filter"
//   "Publish a post immediately. Accepts content, media, targets"
//     -> "Publish a post immediately. Accepts"
//
// 245 of the 389 descriptions were cut. An LLM agent reads these to
// CHOOSE a tool, so a description cut off at "connect a social" tells
// it nothing about what the tool does - on the app's primary AI
// interface.
//
// The optimisation saved a few hundred tokens and cost the agent its
// ability to pick the right tool.
//
// This test holds the property: every description ends on a word that
// can end a sentence, and none is cut off at a dangling preposition,
// article, conjunction or possessive.
//
// Run: cargo test --test mcp_description_quality_test
// Uses no database.

use std::path::PathBuf;

/// A description that ends on one of these is cut off mid-sentence.
///
/// The list is the English function words and the dangling
/// determiners. A description that ends on a NOUN ("Get a single post
/// by ID") is complete, and the test does not flag it.
const DANGLING_ENDINGS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "to", "for", "with", "on", "in", "by", "from", "their",
    "its", "his", "her", "your", "our", "this", "that", "these", "those", "at", "as", "is", "are",
    "was", "were", "be", "been", "being", "into", "over", "per", "via", "than", "when", "where",
    "while", "if", "but", "so", "up", "out", "about", "after", "before", "during", "under",
    "above", "between", "through", "without", "within", "against", "among", "across", "behind",
    "beyond", "since", "until", "upon", "what", "which", "who", "whom", "whose", "how", "why",
    "not", "no", "any", "each", "some", "most", "much", "many", "few", "more", "less", "very",
    "also", "only", "just", "even", "still", "already", "yet", "soon", "now", "then", "onto",
    "off", "down", "away", "back",
];

fn mod_rs() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/mcp/mod.rs");
    std::fs::read_to_string(&path).expect("read src/mcp/mod.rs")
}

/// Every (tool name, description) pair in the MCP surface.
fn tools() -> Vec<(String, String)> {
    let src = mod_rs();
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(rel) = src[idx..].find("#[tool(") {
        let at = idx + rel;
        // The attribute's balanced parens.
        //
        // The paren counter has to SKIP STRING LITERALS: a description
        // like "Get a single post (by ID)" has a paren in it, and
        // counting it as code swallowed the rest of the file - which is
        // why the first version of this parser found 86 tools instead
        // of 389.
        let i = at + "#[tool".len();
        let mut depth = 0usize;
        let mut j = i;
        let mut in_string = false;
        let mut escaped = false;
        while j < bytes.len() {
            let c = bytes[j];
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == b'\\' {
                    escaped = true;
                } else if c == b'"' {
                    in_string = false;
                }
            } else {
                match c {
                    b'"' => in_string = true,
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            j += 1;
        }
        let attr = &src[i..j.min(src.len())];
        // The description, with its escaped quotes.
        let mut desc = String::new();
        if let Some(dpos) = attr.find("description") {
            let rest = &attr[dpos..];
            if let Some(q1) = rest.find('"') {
                let after = &rest[q1 + 1..];
                let b = after.as_bytes();
                let mut k = 0usize;
                while k < b.len() {
                    if b[k] == b'\\' && k + 1 < b.len() {
                        desc.push(b[k + 1] as char);
                        k += 2;
                        continue;
                    }
                    if b[k] == b'"' {
                        break;
                    }
                    desc.push(b[k] as char);
                    k += 1;
                }
            }
        }
        // The tool's name, from the fn that follows.
        let tail = &src[j..];
        let name = tail
            .find("fn ")
            .map(|p| {
                tail[p + 3..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .unwrap_or_default();
        if !name.is_empty() {
            out.push((name, desc));
        }
        idx = j;
    }
    out
}

#[test]
fn every_tool_has_a_description() {
    let tools = tools();
    assert!(
        tools.len() > 380,
        "the whole tool surface must be parsed, got {}",
        tools.len()
    );
    let empty: Vec<&String> = tools
        .iter()
        .filter(|(_, d)| d.trim().is_empty())
        .map(|(n, _)| n)
        .collect();
    assert!(
        empty.is_empty(),
        "every tool needs a description: {empty:?}"
    );
}

#[test]
fn no_description_ends_mid_sentence() {
    let tools = tools();
    let offenders: Vec<String> = tools
        .iter()
        .filter(|(_, d)| {
            // A description that ends with terminal punctuation is a
            // finished sentence, whatever its last word is: "List all
            // WhatsApp groups the user participates in." is complete.
            //
            // The truncation is the case with NO terminal punctuation
            // AND a last word that cannot end a sentence - a
            // preposition, an article, a conjunction, a possessive
            // determiner.
            if d.ends_with('.') || d.ends_with('?') || d.ends_with('!') {
                return false;
            }
            let last = d
                .rsplit(|c: char| c.is_whitespace() || c == '(' || c == ',')
                .find(|c| !c.trim().is_empty())
                .unwrap_or("");
            let last = last
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            DANGLING_ENDINGS.contains(&last.as_str())
        })
        .map(|(n, d)| format!("{n}: {d:?}"))
        .collect();
    assert!(
        offenders.is_empty(),
        "a description that ends on a dangling word is cut off mid-sentence \
         - an agent cannot tell what the tool does:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn no_description_is_empty_or_tiny() {
    let tools = tools();
    let tiny: Vec<String> = tools
        .iter()
        .filter(|(_, d)| d.len() < 12)
        .map(|(n, d)| format!("{n}: {d:?}"))
        .collect();
    assert!(
        tiny.is_empty(),
        "a description under 12 characters says nothing: {tiny:?}"
    );
}
