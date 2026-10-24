// ─── Deckmill autopost (`switchpost autopost`) ──────────────────
//
// One command that renders a carousel with Deckmill, uploads the PNGs
// to switchpost's media store, and stages (or publishes) the post.
//
// The pipeline is the canonical one from the Deckmill publishing skill,
// with the flags it ACTUALLY needs (verified against the binary):
//
//   deckmill generate-slide <type> --params '<json>' --output <file>
//   deckmill render-carousel deck.json --platform <p> --aspect-ratio 4:5
//       --brand-name <n> --brand-handle <h> --topic <t>
//       --url <u> --hashtags <csv> --output carousel.html
//       (--url and --hashtags are REQUIRED — they render overlays)
//   deckmill export carousel.html --slides N --output-dir out/
//       --preset instagram_portrait
//
// Then, in-process:
//   upload each PNG -> media rows -> `/api/media/<uuid>` URLs
//   stage_post(...) with those URLs as the media array
//
// Everything after the render goes through switchpost's OWN services, so
// the readiness checks, the provider settings and the publish pipeline
// are the same ones the dashboard uses.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::AppState;

/// The Deckmill binary. Overridable so a checkout can be tested against a
/// local build.
fn deckmill_bin() -> String {
    std::env::var("DECKMILL_BIN").unwrap_or_else(|_| "deckmill".to_string())
}

/// One rendered slide, ready to upload.
pub struct RenderedSlide {
    pub path: std::path::PathBuf,
    pub mime_type: String,
}

/// What the render produced.
pub struct RenderResult {
    pub slides: Vec<RenderedSlide>,
    pub html: std::path::PathBuf,
}

/// Render a carousel with Deckmill.
///
/// `slides` is one spec per slide: `{"type": "hero", "params": {...}}`.
/// The params are Deckmill's OWN contract (`slide-info <type>` prints it),
/// so this module does not translate them.
pub async fn render_carousel(
    specs: &[Value],
    platform: &str,
    aspect_ratio: &str,
    primary_color: &str,
    brand_name: &str,
    brand_handle: &str,
    topic: &str,
    url: &str,
    hashtags: &str,
    work_dir: &std::path::Path,
) -> Result<RenderResult> {
    if specs.len() < 2 {
        return Err(anyhow!(
            "A carousel needs at least 2 slides (got {})",
            specs.len()
        ));
    }
    let bin = deckmill_bin();
    tokio::fs::create_dir_all(work_dir)
        .await
        .with_context(|| format!("Failed to create {}", work_dir.display()))?;

    // 1. One slide file per spec.
    let mut slide_files: Vec<std::path::PathBuf> = Vec::with_capacity(specs.len());
    for (index, spec) in specs.iter().enumerate() {
        let slide_type = spec
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("slide {index} has no `type`"))?;
        let params = spec.get("params").cloned().unwrap_or_else(|| json!({}));
        let file = work_dir.join(format!("slide_{index}.json"));
        run_deckmill(
            &bin,
            &[
                "generate-slide",
                slide_type,
                "--primary-color",
                primary_color,
                "--params",
                &params.to_string(),
                "--output",
                &file.to_string_lossy(),
            ],
        )
        .await?;
        slide_files.push(file);
    }

    // 2. The deck: a JSON array of the slide objects.
    let mut deck = Vec::with_capacity(slide_files.len());
    for file in &slide_files {
        let bytes = tokio::fs::read(file)
            .await
            .with_context(|| format!("Failed to read {}", file.display()))?;
        let value: Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("{} is not valid JSON", file.display()))?;
        deck.push(value);
    }
    let deck_path = work_dir.join("deck.json");
    tokio::fs::write(&deck_path, serde_json::to_vec_pretty(&deck)?)
        .await
        .with_context(|| format!("Failed to write {}", deck_path.display()))?;

    // 3. Render the carousel HTML.
    let html_path = work_dir.join("carousel.html");
    run_deckmill(
        &bin,
        &[
            "render-carousel",
            &deck_path.to_string_lossy(),
            "--platform",
            platform,
            "--aspect-ratio",
            aspect_ratio,
            "--brand-name",
            brand_name,
            "--brand-handle",
            brand_handle,
            "--topic",
            topic,
            "--url",
            url,
            "--hashtags",
            hashtags,
            "--output",
            &html_path.to_string_lossy(),
        ],
    )
    .await?;

    // 4. Export the PNGs.
    let out_dir = work_dir.join("slides");
    run_deckmill(
        &bin,
        &[
            "export",
            &html_path.to_string_lossy(),
            "--slides",
            &specs.len().to_string(),
            "--output-dir",
            &out_dir.to_string_lossy(),
            "--preset",
            platform,
        ],
    )
    .await?;

    // 5. Collect the exported slides in order.
    let mut slides = Vec::with_capacity(specs.len());
    for index in 0..specs.len() {
        let path = out_dir.join(format!("slide_{}.png", index + 1));
        if !path.exists() {
            return Err(anyhow!(
                "Deckmill exported no image for slide {} (expected {})",
                index + 1,
                path.display()
            ));
        }
        slides.push(RenderedSlide {
            path,
            mime_type: "image/png".to_string(),
        });
    }

    Ok(RenderResult {
        slides,
        html: html_path,
    })
}

/// Upload the rendered slides and return the media attachment array
/// `posts.media` expects.
///
/// The shape is the provider-facing one (`MediaAttachment`), NOT bare URL
/// strings: the publish path deserializes `posts.media` into
/// `Vec<MediaAttachment>` and the providers read `mime_type`.
pub async fn upload_slides(state: &AppState, slides: &[RenderedSlide]) -> Result<Value> {
    let mut media = Vec::with_capacity(slides.len());
    for slide in slides {
        let uploaded = crate::mcp::tools_media::upload_from_path(
            state,
            &slide.path.to_string_lossy(),
        )
        .await
        .map_err(|e| anyhow!("Failed to upload {}: {e}", slide.path.display()))?;

        media.push(json!({
            "url": uploaded.0.url,
            "mime_type": uploaded.0.mime_type,
            "alt": uploaded.0.alt,
        }));
    }
    Ok(Value::Array(media))
}

/// Stage the post on the given integrations.
pub async fn stage(
    state: &AppState,
    content: &str,
    media: Value,
    integration_ids: Vec<Uuid>,
    scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    settings: Value,
    overrides: Value,
) -> Result<crate::services::staging::StagingResult> {
    let user_id = crate::cli::run::resolve_user(state).await?;
    let request = crate::services::staging::StagingRequest {
        content: content.to_string(),
        media,
        integration_ids,
        settings,
        scheduled_at,
        next_slot: false,
        first_comment: None,
        overrides,
        dry_run: false,
        force: false,
        group_id: None,
    };
    crate::services::staging::validate_staging_request(&request).map_err(|e| anyhow!(e))?;
    crate::services::staging::stage_post(
        &state.db,
        &state.providers,
        &state.config.app_url,
        user_id,
        request,
    )
    .await
    .map_err(|e| anyhow!("Staging failed: {e}"))
}

/// Run the Deckmill binary and fail loudly on a non-zero exit.
async fn run_deckmill(bin: &str, args: &[&str]) -> Result<()> {
    let output = tokio::process::Command::new(bin)
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to run '{bin}' — is it on PATH?"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "deckmill {} failed ({}): {}{}",
            args.first().copied().unwrap_or(""),
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_carousel_needs_at_least_two_slides() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt.block_on(render_carousel(
            &[json!({"type": "hero", "params": {}})],
            "instagram_portrait",
            "4:5",
            "#2D5BFF",
            "Acme",
            "@acme",
            "Topic",
            "https://example.com",
            "ai",
            std::path::Path::new("/tmp/switchpost-autopost-test"),
        ));
        assert!(err.is_err());
    }
}
