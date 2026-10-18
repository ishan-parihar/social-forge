// ─── Content Splitter ───────────────────────────────────────
// Splits post content based on platform character limits.
// Handles thread creation for X/Twitter, Bluesky, etc.

/// Whether this platform supports threads (multi-part posts)
pub fn supports_threads(provider: &str) -> bool {
    matches!(provider, "x" | "bluesky" | "mastodon" | "threads")
}

/// A split segment of content
#[derive(Debug, Clone)]
pub struct SplitSegment {
    pub content: String,
    pub sequence: usize,
    pub total: usize,
}

/// Split content for a platform whose hard limit is `limit` bytes — pass the
/// provider's `max_content_length()`, the same value `validate_post_rules`
/// enforces, so a split can never leave a segment the validator rejects.
/// Returns one or more segments. For thread-capable platforms, adds numbering.
/// A `limit` of 0 means the platform has no post surface: content comes back
/// whole and the validator rejects it.
pub fn split_content(content: &str, provider: &str, limit: usize) -> Vec<SplitSegment> {
    // If content fits (or there is no limit to split against), return as-is
    if limit == 0 || content.len() <= limit {
        return vec![SplitSegment {
            content: content.to_string(),
            sequence: 1,
            total: 1,
        }];
    }

    // Account for thread numbering overhead when splitting
    // Format: "(N/M)\n" adds ~7 chars for single-digit, ~9 for double-digit
    let overhead = if supports_threads(provider) { 10 } else { 0 };
    let effective_limit = if overhead > 0 {
        limit.saturating_sub(overhead)
    } else {
        limit
    };

    let chunks = smart_split(content, effective_limit);
    let total = chunks.len();

    chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut segment = chunk;
            // Add thread numbering for thread-capable platforms
            if supports_threads(provider) && total > 1 {
                segment = format!("({}/{})\n{}", i + 1, total, segment);
            }
            SplitSegment {
                content: segment,
                sequence: i + 1,
                total,
            }
        })
        .collect()
}

/// Smart split: try paragraph boundaries first, then sentence boundaries, then word boundaries.
fn smart_split(content: &str, max_len: usize) -> Vec<String> {
    if content.len() <= max_len {
        return vec![content.to_string()];
    }

    let mut result = Vec::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            result.push(remaining.to_string());
            break;
        }

        // Try to find a good break point within the limit
        let break_at = find_break_point(remaining, max_len);
        let chunk = remaining[..break_at].to_string();
        result.push(chunk);
        remaining = remaining[break_at..].trim_start();
    }

    result
}

/// Find the best place to split text within max_len.
/// Priority: paragraph > sentence > comma > word > hard cut
fn find_break_point(text: &str, max_len: usize) -> usize {
    // Snap the byte limit back to a char boundary. A limit can land inside a
    // multibyte character (emoji, CJK), and slicing there panics.
    let mut end = max_len.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let search_zone = &text[..end];

    // Try paragraph break
    if let Some(pos) = search_zone.rfind("\n\n") {
        return pos + 2;
    }

    // Try sentence break
    if let Some(pos) = search_zone.rfind(". ") {
        return pos + 2;
    }
    if let Some(pos) = search_zone.rfind("! ") {
        return pos + 2;
    }
    if let Some(pos) = search_zone.rfind("? ") {
        return pos + 2;
    }

    // Try comma break
    if let Some(pos) = search_zone.rfind(", ") {
        return pos + 2;
    }

    // Try word break
    if let Some(pos) = search_zone.rfind(' ') {
        return pos + 1;
    }

    // Hard cut at limit. Never return 0: smart_split would then loop forever.
    // That happens when the limit is smaller than the next character.
    if end == 0 {
        return text.chars().next().map_or(1, char::len_utf8);
    }
    end
}

/// Check if content needs splitting against a platform's hard limit
pub fn needs_splitting(content: &str, limit: usize) -> bool {
    limit > 0 && content.len() > limit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_split_needed() {
        let content = "Short post";
        let segments = split_content(content, "x", 4000);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].content, "Short post");
    }

    #[test]
    fn test_split_long_content() {
        let content = "a".repeat(5000);
        let segments = split_content(&content, "x", 4000);
        assert!(segments.len() > 1);
        for seg in &segments {
            assert!(seg.content.len() <= 4000);
        }
    }

    #[test]
    fn test_bluesky_thread() {
        let content = "a".repeat(500);
        let segments = split_content(&content, "bluesky", 300);
        assert!(segments.len() > 1);
        assert!(segments[0].content.starts_with("(1/"));
    }

    /// WhatsApp's validator limit is 4096. A splitter that used a looser
    /// number let 5000-char messages through unsplit and then failed publish.
    #[test]
    fn segments_never_exceed_the_provider_limit() {
        let content = "word ".repeat(1200);
        let segments = split_content(&content, "whatsapp", 4096);
        assert!(segments.len() > 1);
        for seg in &segments {
            assert!(seg.content.len() <= 4096, "segment {} bytes", seg.content.len());
        }
    }

    #[test]
    fn split_never_slices_inside_a_multibyte_character() {
        // Each emoji is 4 bytes, so byte 10 falls inside one of them.
        // linkedin has no thread numbering, so the limit is used as-is.
        let text = "🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀🚀";
        let segments = split_content(text, "linkedin", 10);
        assert!(segments.len() > 1, "expected a split");
        for seg in &segments {
            assert!(seg.content.len() <= 10, "segment over limit: {}", seg.content.len());
        }
        let rejoined: String = segments.iter().map(|s| s.content.as_str()).collect();
        assert_eq!(rejoined, text, "no character may be lost or cut");
    }

    #[test]
    fn split_terminates_when_limit_is_below_one_character() {
        // A 2-byte limit cannot hold a 4-byte emoji. The splitter must still
        // advance one whole character at a time instead of looping.
        let text = "🚀🚀🚀";
        let segments = split_content(text, "linkedin", 2);
        let rejoined: String = segments.iter().map(|s| s.content.as_str()).collect();
        assert_eq!(rejoined, text);
    }

    #[test]
    fn thread_platform_with_tiny_limit_terminates() {
        // limit 10 minus the 10-byte numbering overhead leaves 0 for content.
        let segments = split_content("abc def ghi", "x", 10);
        assert!(!segments.is_empty());
    }

    #[test]
    fn zero_limit_returns_content_whole() {
        let segments = split_content("anything", "dead", 0);
        assert_eq!(segments.len(), 1);
        assert!(!needs_splitting("anything", 0));
    }
}
