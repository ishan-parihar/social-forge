// ─── Content Splitter ───────────────────────────────────────
// Splits post content based on platform character limits.
// Handles thread creation for X/Twitter, Bluesky, etc.


/// Character limits per platform.
///
/// TODO: this duplicates `SocialProvider::max_content_length()`. The
/// splitter should take `&dyn SocialProvider` instead of a `&str` so
/// the limits live on the providers themselves. For now, dead providers
/// (twitch/nostr/mewe/moltbook) have been removed.
pub fn platform_limit(provider: &str) -> usize {
    match provider {
        "x" => 4000,
        "linkedin" | "linkedin_page" => 3000,
        "bluesky" => 300,
        "threads" => 500,
        "mastodon" => 500,
        "instagram" | "instagram_standalone" => 2200,
        "reddit" => 10000,
        "facebook" => 63206,
        "youtube" => 5000,
        "tiktok" => 2200,
        "pinterest" => 500,
        "discord" => 2000,
        "slack" => 40000,
        "telegram_bot" | "telegram_user" => 4096,
        "whatsapp" => 65536,
        "vk" => 21000,
        "github" => 65536,
        "medium" => 100000,
        "devto" => 100000,
        "hashnode" => 100000,
        "wordpress" => 100000,
        "farcaster" => 1024,
        "lemmy" => 10000,
        "skool" => 100000,
        "kick" => 10000,
        "whop" => 10000,
        _ => 10000, // safe default
    }
}

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

/// Split content for a specific platform, respecting character limits.
/// Returns one or more segments. For thread-capable platforms, adds numbering.
pub fn split_content(content: &str, provider: &str, _max_media_per_post: usize) -> Vec<SplitSegment> {
    let limit = platform_limit(provider);
    
    // If content fits, return as-is
    if content.len() <= limit {
        return vec![SplitSegment {
            content: content.to_string(),
            sequence: 1,
            total: 1,
        }];
    }
    
    // Account for thread numbering overhead when splitting
    // Format: "(N/M)\n" adds ~7 chars for single-digit, ~9 for double-digit
    let overhead = if supports_threads(provider) { 10 } else { 0 };
    let effective_limit = if overhead > 0 { limit.saturating_sub(overhead) } else { limit };
    
    let chunks = smart_split(content, effective_limit);
    let total = chunks.len();
    
    chunks.into_iter().enumerate().map(|(i, chunk)| {
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
    }).collect()
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
        remaining = &remaining[break_at..].trim_start();
    }
    
    result
}

/// Find the best place to split text within max_len.
/// Priority: paragraph > sentence > comma > word > hard cut
fn find_break_point(text: &str, max_len: usize) -> usize {
    let search_zone = &text[..max_len];
    
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
    
    // Hard cut at limit
    max_len
}

/// Check if content needs splitting for a platform
pub fn needs_splitting(content: &str, provider: &str) -> bool {
    content.len() > platform_limit(provider)
}

/// Get a summary of how content will be split across platforms
pub fn preview_split(content: &str, providers: &[&str]) -> Vec<(String, usize, usize)> {
    providers.iter().map(|&p| {
        let segments = split_content(content, p, 4);
        (p.to_string(), segments.len(), platform_limit(p))
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_no_split_needed() {
        let content = "Short post";
        let segments = split_content(content, "x", 4);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].content, "Short post");
    }
    
    #[test]
    fn test_split_long_content() {
        let content = "a".repeat(5000);
        let segments = split_content(&content, "x", 4);
        assert!(segments.len() > 1);
        for seg in &segments {
            assert!(seg.content.len() <= 4000);
        }
    }
    
    #[test]
    fn test_bluesky_thread() {
        let content = "a".repeat(500);
        let segments = split_content(&content, "bluesky", 4);
        assert!(segments.len() > 1);
        assert!(segments[0].content.starts_with("(1/"));
    }
    
    #[test]
    fn test_platform_limits() {
        assert_eq!(platform_limit("x"), 4000);
        assert_eq!(platform_limit("linkedin"), 3000);
        assert_eq!(platform_limit("bluesky"), 300);
        assert_eq!(platform_limit("threads"), 500);
    }
}
