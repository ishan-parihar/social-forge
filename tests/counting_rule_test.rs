// ─── The composer's count and the server's count must agree ───
//
// The composer shows a character count and the server enforces one.
// They are two implementations of the same rule, in two languages, and
// they drifted:
//
//   - the plain rule was `String.length` in the browser (UTF-16 code
//     units) and `String::len()` on the server (UTF-8 bytes). A
//     200-emoji Bluesky post read "200/300" in the composer and came
//     back from the server as "Content is 800 chars, max 300 for
//     Bluesky" - the user was told the post was 800 characters when
//     the composer had said 200.
//
//   - X's rule is the WEIGHTED one (twitter-text v3: an emoji or a CJK
//     glyph costs 2), which the composer implemented and the server
//     did not. The server measured bytes for X too.
//
// These tests pin the server's rule. The composer's `countFor` is the
// mirror image of it, and the two functions are compared here against
// the same inputs.
//
// Run: cargo test --test counting_rule_test

use social_forge::social::{weighted_length, CountingRule};

/// The composer's `countFor` for the plain rule, transcribed.
///
/// It is UTF-8 byte length - `new TextEncoder().encode(text).length`
/// in the browser.
fn plain_count(text: &str) -> usize {
    text.len()
}

#[test]
fn the_plain_rule_is_bytes() {
    // ASCII: bytes == characters.
    assert_eq!(plain_count("hello"), 5);
    assert_eq!(CountingRule::Bytes.measure("hello"), 5);

    // An emoji is 4 UTF-8 bytes and 1 character.
    let emoji = "\u{1F600}";
    assert_eq!(emoji.chars().count(), 1);
    assert_eq!(plain_count(emoji), 4);
    assert_eq!(CountingRule::Bytes.measure(emoji), 4);

    // A CJK glyph is 3 UTF-8 bytes.
    let cjk = "\u{4E00}";
    assert_eq!(cjk.chars().count(), 1);
    assert_eq!(plain_count(cjk), 3);
    assert_eq!(CountingRule::Bytes.measure(cjk), 3);
}

#[test]
fn the_weighted_rule_costs_two_for_emoji_and_cjk() {
    // A plain ASCII character costs 1.
    assert_eq!(weighted_length("a"), 1);
    assert_eq!(weighted_length("hello"), 5);

    // An emoji costs 2.
    assert_eq!(weighted_length("\u{1F600}"), 2);
    // Two emoji: 4.
    assert_eq!(weighted_length("\u{1F600}\u{1F600}"), 4);

    // A CJK ideograph costs 2.
    assert_eq!(weighted_length("\u{4E00}"), 2);
    // Hiragana costs 2.
    assert_eq!(weighted_length("\u{3042}"), 2);
    // Hangul costs 2.
    assert_eq!(weighted_length("\u{AC00}"), 2);
}

#[test]
fn the_weighted_rule_matches_the_composers_rule() {
    // The same inputs the composer counts. The composer's
    // `weightedLength` iterates code points and charges 2 for the
    // emoji and CJK ranges; this is the server's copy of that rule.
    let cases = [
        "Hello, world!",
        "Hello \u{1F600} world",
        "\u{4E00}\u{4E00}\u{4E00}",
        "caf\u{00E9} \u{1F389} tail",
        "",
    ];
    for case in cases {
        assert_eq!(
            weighted_length(case),
            // The composer's rule, transcribed: code points, 2 for the
            // heavy ranges.
            case.chars()
                .map(|c| {
                    let cp = c as u32;
                    let heavy = (0x3040..=0x30ff).contains(&cp)
                        || (0x3400..=0x4dbf).contains(&cp)
                        || (0x4e00..=0x9fff).contains(&cp)
                        || (0xf900..=0xfaff).contains(&cp)
                        || (0xac00..=0xd7af).contains(&cp)
                        || (0x1f300..=0x1f9ff).contains(&cp)
                        || (0x2600..=0x27bf).contains(&cp);
                    if heavy { 2 } else { 1 }
                })
                .sum::<usize>(),
            "weighted length must match the composer's rule for {case:?}"
        );
    }
}

#[test]
fn a_200_emoji_post_is_800_bytes_not_200() {
    // The exact case from the defect: the composer said 200, the
    // server said 800.
    let post = "\u{1F600}".repeat(200);
    assert_eq!(post.chars().count(), 200);
    assert_eq!(CountingRule::Bytes.measure(&post), 800);
    // The X rule charges 2 per emoji: 400.
    assert_eq!(CountingRule::Weighted.measure(&post), 400);
}
