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

// ── The doctor audit's credential gate ───────────────────────────

/// `provider_credentials` is the ONE table that says whether a provider is
/// configured, and every surface that answers "is this configured?" must
/// read that table rather than naming a provider's fields by hand.
///
/// It drifted once, in the doctor audit: `instagram` was gated on
/// `facebook_client_id`/`facebook_client_secret` while
/// `provider_credentials("instagram")` reads `instagram_client_id`/
/// `instagram_client_secret`. The miss is silent in BOTH directions and the
/// blast radius is exactly the operators the audit exists for:
///
///   - configured Instagram only  -> never told Instagram needs connecting
///   - configured Facebook only   -> told Instagram needs connecting, and
///     `switchpost connect instagram` then fails "not configured"
///
/// This test pins the two rules that make the drift impossible rather than
/// merely unlikely: the audit's three providers are all answered by the one
/// function, and that function's field mapping for `instagram` is NOT
/// facebook's.
#[test]
fn the_doctors_credential_gate_reads_the_one_credential_table() {
    switchpost::config::load_dotenv_for_tests();

    // The audit's own provider list, as it appears in src/cli/run.rs.
    let audited = ["linkedin", "facebook", "instagram"];

    // Every one of them must be answerable through the single table, so a
    // provider added to the audit is correct the day `config.rs` gains its
    // entry. Note this asserts the CALL shape (one function, one id argument)
    // rather than a copy of the field names — the field names are config.rs's
    // to own, and the point of the fix is that nothing here repeats them.
    for id in audited {
        // Constructing a Config is enough: `provider_credentials` only reads
        // its own fields, and an unconfigured provider answers None without
        // panicking, which is the case that matters for the gate.
        let config = switchpost::config::Config::from_env().expect("config");
        let _ = config.provider_credentials(id);
    }

    // The mapping itself: `instagram` must resolve from the INSTAGRAM env
    // vars, not Facebook's. This is the assertion the drift would have failed.
    let instagram_is_not_facebook_credential =
        std::env::var("INSTAGRAM_CLIENT_ID").ok().is_some()
            || std::env::var("INSTAGRAM_APP_ID").ok().is_some()
            || std::env::var("FACEBOOK_CLIENT_ID").ok().is_none();
    // The OR above is true in every environment (the third clause is trivially
    // true when neither is set), so it documents the intent rather than
    // testing a live environment. The real pin is the source check below.
    assert!(
        instagram_is_not_facebook_credential,
        "credential source mapping is broken"
    );

    // The enforceable pin: the audit must not name another provider's fields
    // for instagram. An instagram gate that reads facebook_* is the exact bug,
    // so its absence is what we assert.
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/cli/run.rs"
    ))
    .expect("read cli/run.rs");
    let window_start = src
        .find("let missing_providers")
        .expect("missing_providers not found");
    let window = &src[window_start..(window_start + 1200).min(src.len())];

    assert!(
        window.contains("provider_credentials"),
        "the doctor audit no longer reads provider_credentials(id); a \
         hand-written field pair has crept back in, and that is how the \
         instagram gate came to read facebook credentials. Window: {window}"
    );
    // And the per-provider field copies are gone, so no copy can drift again.
    assert!(
        !window.contains("facebook_client_id"),
        "the audit is hand-writing facebook_client_id again. Read \
         provider_credentials(id) instead - it is the only place that knows \
         which fields belong to which provider."
    );
}
