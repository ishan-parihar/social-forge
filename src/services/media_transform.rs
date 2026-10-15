// ─── Media Transform Service (E1, audit §8.1) ────────────────────────
// Aspect-ratio crop + resize + video re-encode for the platforms with hard
// reject rules — the #1 real-world publish-failure cause per the audit.
//
// Everything shells out to the optional `ffmpeg` binary: no image/video
// crates, one transform engine for both media kinds, and an operator who
// lacks ffmpeg just gets a warning + the original file (their platform
// may still reject it, but nothing breaks locally).
//
// Opt-in per post: settings.transform_media = true (same convention as
// utm_campaign). No silent rewriting of the user's bytes otherwise.

use crate::social::MediaAttachment;
use std::path::Path;

/// Target render for a provider's media, in pixels. ffmpeg does the real
/// math (`force_original_aspect_ratio=increase` + `crop`), so the source
/// dimensions never need probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
}

/// The platforms that REJECT out-of-spec media (the audit's list). A
/// provider absent from here keeps its original files — e.g. X tolerates a
/// wide band and YouTube letterboxes itself.
pub fn render_target(provider: &str, is_video: bool) -> Option<RenderTarget> {
    match (provider, is_video) {
        // IG feed: 4:5 portrait band is the safe intersection of the
        // allowed 4:5–1.91:1 range (outside it, the API rejects).
        ("instagram", false) | ("instagram", true) => Some(RenderTarget { width: 1080, height: 1350 }),
        ("instagram-standalone", _) => Some(RenderTarget { width: 1080, height: 1350 }),
        // Reels-style verticals.
        ("tiktok", _) => Some(RenderTarget { width: 1080, height: 1920 }),
        _ => None,
    }
}

/// Build the ffmpeg argument list for one transform. Pure + unit-tested —
/// the process spawn is the only untestable part.
///
/// Images get `-frames:v 1` (single frame, any common image decoder);
/// videos get a fast h264/aac re-encode with `+faststart` so platforms
/// can start streaming before the whole file lands.
pub fn ffmpeg_args(
    input: &Path,
    output: &Path,
    target: RenderTarget,
    is_video: bool,
) -> Vec<String> {
    let vf = format!(
        "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
        target.width, target.height, target.width, target.height
    );
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        input.display().to_string(),
        "-vf".into(),
        vf,
    ];
    if is_video {
        args.extend([
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-c:a".into(),
            "aac".into(),
            "-movflags".into(),
            "+faststart".into(),
        ]);
    } else {
        args.extend(["-frames:v".into(), "1".into()]);
    }
    args.push(output.display().to_string());
    args
}

/// Transform `attachments` for `provider` when opted in
/// (settings.transform_media). Returns the attachments to publish — the
/// originals when ffmpeg is missing (with a warning logged) or when the
/// provider has no hard spec.
///
/// `local_path_of` maps a media URL to a local file path; attachments
/// that don't live locally (remote http URLs) are skipped — ffmpeg will
/// not fetch, and the upload dir is where staged media lives anyway.
pub async fn transform_attachments(
    provider: &str,
    attachments: Vec<MediaAttachment>,
    opted_in: bool,
    local_path_of: impl Fn(&str) -> Option<std::path::PathBuf>,
    out_dir: &Path,
) -> Vec<MediaAttachment> {
    if !opted_in {
        return attachments;
    }
    if attachments.is_empty() {
        return attachments;
    }
    if !ffmpeg_available().await {
        tracing::warn!(
            "transform_media is set but ffmpeg is not on PATH — publishing the original files; \
             {provider} may reject them for aspect ratio"
        );
        return attachments;
    }
    let _ = tokio::fs::create_dir_all(out_dir).await;
    if !out_dir.is_dir() {
        tracing::warn!(
            "cannot create the transform dir {} — publishing originals",
            out_dir.display()
        );
        return attachments;
    }

    let mut out = Vec::with_capacity(attachments.len());
    for mut att in attachments {
        let is_video = att.mime_type.starts_with("video/");
        let Some(target) = render_target(provider, is_video) else {
            out.push(att);
            continue;
        };
        let Some(local) = local_path_of(&att.url) else {
            out.push(att);
            continue;
        };
        let ext = local
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or(if is_video { "mp4" } else { "jpg" });
        let out_path = out_dir.join(format!("tf-{}.{ext}", uuid::Uuid::new_v4()));
        let args = ffmpeg_args(&local, &out_path, target, is_video);

        let status = tokio::process::Command::new("ffmpeg")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        match status {
            Ok(s) if s.success() => {
                tracing::info!(
                    "transformed {} -> {} for {provider} ({}x{})",
                    local.display(),
                    out_path.display(),
                    target.width,
                    target.height
                );
                // The publish path rewrites URLs against APP_URL; keep the
                // same URL shape with the transform dir swapped in.
                att.url = url_for(&att.url, &out_path);
                out.push(att);
            }
            Ok(s) => {
                tracing::warn!(
                    "ffmpeg transform failed for {} (exit {}): {s:?} — publishing the original",
                    local.display(),
                    s.code().unwrap_or(-1)
                );
                out.push(att);
            }
            Err(e) => {
                tracing::warn!("ffmpeg could not run for {}: {e}", local.display());
                out.push(att);
            }
        }
    }
    out
}

/// Map the transform output path back into a URL with the ORIGINAL URL's
/// prefix — transformed files land in the same media dir the
/// `/api/media/{id}` route already serves, so only the filename changes:
/// "https://host:6543/api/media/a.jpg" → "…/api/media/tf-<uuid>.jpg".
fn url_for(original_url: &str, out_path: &Path) -> String {
    let file = out_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match original_url.rsplit_once('/') {
        Some((prefix, _)) => format!("{prefix}/{file}"),
        None => format!("/api/media/{file}"),
    }
}

async fn ffmpeg_available() -> bool {
    tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    fn att(url: &str, mime: &str) -> MediaAttachment {
        MediaAttachment {
            url: url.into(),
            mime_type: mime.into(),
            alt: None,
            poster_url: None,
        }
    }

    #[test]
    fn spec_targets_match_the_hard_reject_platforms() {
        assert_eq!(
            render_target("instagram", false),
            Some(RenderTarget { width: 1080, height: 1350 })
        );
        assert_eq!(
            render_target("tiktok", true),
            Some(RenderTarget { width: 1080, height: 1920 })
        );
        // No hard spec → no transform, ever.
        assert_eq!(render_target("x", true), None);
        assert_eq!(render_target("wordpress", false), None);
    }

    #[test]
    fn ffmpeg_args_crop_to_target_for_images() {
        let args = ffmpeg_args(
            Path::new("/u/a.jpg"),
            Path::new("/t/b.jpg"),
            RenderTarget { width: 1080, height: 1350 },
            false,
        );
        assert_eq!(args[0], "-y");
        assert_eq!(args[1], "-i");
        assert_eq!(args[2], "/u/a.jpg");
        assert_eq!(args[3], "-vf");
        assert_eq!(
            args[4],
            "scale=1080:1350:force_original_aspect_ratio=increase,crop=1080:1350"
        );
        assert_eq!(args[5], "-frames:v");
        assert_eq!(args[6], "1");
        assert_eq!(args.last().unwrap(), "/t/b.jpg");
    }

    #[test]
    fn ffmpeg_args_reencode_videos_with_faststart() {
        let args = ffmpeg_args(
            Path::new("/u/a.mp4"),
            Path::new("/t/b.mp4"),
            RenderTarget { width: 1080, height: 1920 },
            true,
        );
        let joined = args.join(" ");
        assert!(joined.contains("libx264"), "got: {joined}");
        assert!(joined.contains("aac"), "got: {joined}");
        assert!(joined.contains("+faststart"), "got: {joined}");
        assert!(!joined.contains("frames:v"), "video is not single-frame: {joined}");
    }

    #[test]
    fn url_for_keeps_prefix_and_swaps_filename() {
        let u = url_for(
            "https://localhost:6543/api/media/pic.jpg",
            Path::new("./uploads/tf-abc.jpg"),
        );
        assert_eq!(u, "https://localhost:6543/api/media/tf-abc.jpg");
    }

    #[test]
    fn not_opted_in_returns_attachments_untouched() {
        let a = vec![att("https://x/a.jpg", "image/jpeg")];
        let out = futures_executor_block(transform_attachments(
            "instagram",
            a.clone(),
            false,
            |_| None,
            Path::new("/tmp/t"),
        ));
        assert_eq!(out[0].url, a[0].url);
    }

    /// The engine runs tokio; tests for the async fn need an executor. Tiny
    /// helper instead of a dev-dependency.
    fn futures_executor_block<F: Future>(fut: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(fut)
    }
}
