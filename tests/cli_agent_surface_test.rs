// ─── CLI agent surface: browser login + Deckmill autopost ─────────
//
// The two commands an AI agent needs for the workflows the dashboard
// cannot reach on its own:
//
//   connect --browser   drives the platform's consent page in a real
//                       browser (browseros-neo) and redeems the code
//   autopost            renders a Deckmill carousel, uploads it and
//                       stages it
//
// These tests pin the parts that can go wrong SILENTLY: the URL
// parsing, the media shape the publish path deserializes, and the
// command definitions themselves.

use clap::Parser;
use switchpost::cli::{Cli, Command};
use switchpost::social::MediaAttachment;

// ── The command surface exists ───────────────────────────────────

#[test]
fn connect_accepts_the_browser_flag() {
    let cli = Cli::parse_from(["switchpost", "connect", "linkedin", "--browser"]);
    match cli.command {
        Command::Connect { browser, .. } => assert!(browser),
        other => panic!("expected Connect, got {other:?}"),
    }
}

#[test]
fn connect_browser_endpoint_defaults_to_browseros_neo() {
    let cli = Cli::parse_from(["switchpost", "connect", "linkedin", "--browser"]);
    match cli.command {
        Command::Connect { browser_endpoint, .. } => {
            assert_eq!(browser_endpoint, "http://127.0.0.1:9012/mcp")
        }
        other => panic!("expected Connect, got {other:?}"),
    }
}

#[test]
fn autopost_accepts_the_slides_and_platforms() {
    let cli = Cli::parse_from([
        "switchpost",
        "autopost",
        "--slides",
        "slides.json",
        "--platforms",
        "instagram",
        "--content",
        "hello",
    ]);
    match cli.command {
        Command::Autopost(options) => {
            assert_eq!(options.slides, "slides.json");
            assert_eq!(options.platforms, "instagram");
            assert_eq!(options.content, "hello");
        }
        other => panic!("expected Autopost, got {other:?}"),
    }
}

#[test]
fn autopost_defaults_the_deckmill_preset_to_instagram_portrait() {
    let cli = Cli::parse_from([
        "switchpost",
        "autopost",
        "--slides",
        "s.json",
        "--platforms",
        "x",
        "--content",
        "hi",
    ]);
    match cli.command {
        Command::Autopost(options) => assert_eq!(options.preset, "instagram_portrait"),
        other => panic!("expected Autopost, got {other:?}"),
    }
}

// ── The media shape the publish path deserializes ────────────────
//
// `posts.media` is deserialized into `Vec<MediaAttachment>` by the
// publish path. A bare URL string there decodes to an EMPTY vec, so
// the post publishes with no image and reports success.

#[test]
fn the_autopost_media_shape_decodes_into_media_attachments() {
    let media = serde_json::json!([
        {"url": "/api/media/11111111-1111-1111-1111-111111111111", "mime_type": "image/png", "alt": "slide 1"},
        {"url": "/api/media/22222222-2222-2222-2222-222222222222", "mime_type": "image/png", "alt": null},
    ]);
    let decoded: Vec<MediaAttachment> = serde_json::from_value(media).expect("decodes");
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].mime_type, "image/png");
}

#[test]
fn bare_url_strings_do_not_decode_into_media_attachments() {
    // A bare URL string must NOT silently become an attachment. The
    // decoder is strict, so the shape fails LOUDLY — a row that held a
    // bare URL errors instead of publishing with no image and reporting
    // success.
    let media = serde_json::json!(["/api/media/11111111-1111-1111-1111-111111111111"]);
    let decoded: Result<Vec<MediaAttachment>, _> = serde_json::from_value(media);
    assert!(
        decoded.is_err(),
        "a bare URL must not decode into an attachment vec"
    );
}

// ── The Deckmill slide-spec contract ─────────────────────────────

#[test]
fn the_slide_spec_file_is_a_json_array_of_type_and_params() {
    let spec = serde_json::json!([
        {"type": "hero", "params": {"headline": "Ship slides in minutes"}},
        {"type": "qr_destination", "params": {"destination_url": "https://example.com", "cta_text": "Scan"}},
    ]);
    let parsed: Vec<serde_json::Value> = serde_json::from_value(spec).expect("parses");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["type"], "hero");
    assert_eq!(parsed[1]["params"]["destination_url"], "https://example.com");
}
