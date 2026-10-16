// ─── Media Transform Service (E1, audit §8.1) ────────────────────────
// Aspect-ratio crop/blur-pad + resize + trim + video re-encode for the
// platforms with hard reject rules — the #1 real-world publish-failure
// cause per the audit.
//
// Everything shells out to the optional `ffmpeg` binary: no image/video
// crates, one transform engine for both media kinds, and an operator who
// lacks ffmpeg just gets a warning + the original file (their platform
// may still reject it, but nothing breaks locally).
//
// Opt-in per post via settings (same convention as utm_campaign):
//   transform_media: bool          — master switch; no silent rewriting
//                                    of the user's bytes otherwise
//   transform_mode: "crop"|"blur"  — v2: blur pads to the target with a
//                                    blurred background (the full frame
//                                    stays visible); crop is the default
//   trim_start / trim_end: number   — v2: seconds, video only

use crate::social::MediaAttachment;
use std::path::Path;

/// How to fit the source into the target rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    /// Scale to fill, crop the overflow (v1 behaviour, default).
    #[default]
    Crop,
    /// v2: scale-to-fill blurred background with the scale-to-fit
    /// foreground centered on top — nothing is cut off.
    BlurPad,
}

/// Per-post transform settings parsed once from `post.settings`.
#[derive(Debug, Clone, Default)]
pub struct TransformSettings {
    pub enabled: bool,
    pub mode: TransformMode,
    /// Video trim window start, seconds from the clip start.
    pub trim_start: Option<f64>,
    /// Absolute end in seconds; normalized away when <= trim_start.
    pub trim_end: Option<f64>,
}

impl TransformSettings {
    /// Parse from the post's settings JSON. Invalid values normalize to
    /// defaults (never fail the publish): negatives drop, an end at or
    /// before the start drops, an unknown mode means crop.
    pub fn from_settings(settings: &serde_json::Value) -> Self {
        let mode = match settings.get("transform_mode").and_then(|v| v.as_str()) {
            Some("blur") | Some("blur-pad") | Some("blurpad") => TransformMode::BlurPad,
            _ => TransformMode::Crop,
        };
        let trim_start = settings
            .get("trim_start")
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0);
        let trim_end = settings
            .get("trim_end")
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0 && *v > trim_start.unwrap_or(0.0));
        Self {
            enabled: settings
                .get("transform_media")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            mode,
            trim_start,
            trim_end,
        }
    }
}

/// Target render for a provider's media, in pixels. ffmpeg does the real
/// math (`force_original_aspect_ratio` + `crop`/`overlay`), so the source
/// dimensions never need probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
}

/// The platforms that REJECT out-of-spec media (the audit's list + v2
/// rows). A provider absent from here keeps its original files — e.g. X
/// tolerates a wide band, and a regular YouTube feed upload letterboxes
/// itself (the youtube row is the 9:16 Shorts target — only applied when
/// the poster opts into transform_media).
pub fn render_target(provider: &str, is_video: bool) -> Option<RenderTarget> {
    match (provider, is_video) {
        // IG feed: 4:5 portrait band is the safe intersection of the
        // allowed 4:5–1.91:1 range (outside it, the API rejects).
        ("instagram", false) | ("instagram", true) => Some(RenderTarget { width: 1080, height: 1350 }),
        ("instagram-standalone", _) => Some(RenderTarget { width: 1080, height: 1350 }),
        // Reels-style verticals.
        ("tiktok", _) => Some(RenderTarget { width: 1080, height: 1920 }),
        // v2: Facebook feed 4:5 — the widest portrait band FB accepts
        // without cropping in the feed UI.
        ("facebook", _) => Some(RenderTarget { width: 1080, height: 1350 }),
        // v2: YouTube Shorts 9:16 (video only).
        ("youtube", true) => Some(RenderTarget { width: 1080, height: 1920 }),
        _ => None,
    }
}

/// Build the ffmpeg argument list for one transform. Pure + unit-tested —
/// the process spawn is the only untestable part.
///
/// Crop mode scales to fill then crops the overflow (v1). Blur-pad mode
/// builds a blurred scale-to-fill background and overlays the full
/// scale-to-fit frame centered on top, so nothing is cut off.
///
/// Trim (video only): input-side `-ss` for a fast seek, `-t` for the clip
/// duration (end − start; just the end when no start was given).
pub fn ffmpeg_args(
    input: &Path,
    output: &Path,
    target: RenderTarget,
    is_video: bool,
    settings: &TransformSettings,
) -> Vec<String> {
    let (w, h) = (target.width, target.height);
    let mut args: Vec<String> = vec!["-y".into()];

    // -ss before -i seeks on the input without decoding the skipped span.
    if is_video {
        if let Some(start) = settings.trim_start {
            args.extend(["-ss".into(), format!("{start}")]);
        }
    }
    args.extend(["-i".into(), input.display().to_string()]);

    match settings.mode {
        TransformMode::Crop => {
            let vf =
                format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}");
            args.extend(["-vf".into(), vf]);
        }
        TransformMode::BlurPad => {
            let fc = format!(
                "split[a][b];\
                 [a]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},gblur=sigma=20[bg];\
                 [b]scale={w}:{h}:force_original_aspect_ratio=decrease[fg];\
                 [bg][fg]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2[out]"
            );
            args.extend(["-filter_complex".into(), fc, "-map".into(), "[out]".into()]);
        }
    }

    if is_video {
        match (settings.trim_start, settings.trim_end) {
            (Some(start), Some(end)) => {
                args.extend(["-t".into(), format!("{}", (end - start).max(0.0))]);
            }
            (None, Some(end)) => {
                args.extend(["-t".into(), format!("{end}")]);
            }
            _ => {}
        }
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

/// Transform `attachments` for `provider` per the post's
/// [`TransformSettings`]. Returns the attachments to publish — the
/// originals when not opted in, when ffmpeg is missing (with a warning
/// logged), or when the provider has no hard spec.
///
/// `local_path_of` maps a media URL to a local file path; attachments
/// that don't live locally (remote http URLs) are skipped — ffmpeg will
/// not fetch, and the upload dir is where staged media lives anyway.
pub async fn transform_attachments(
    provider: &str,
    attachments: Vec<MediaAttachment>,
    settings: &TransformSettings,
    local_path_of: impl Fn(&str) -> Option<std::path::PathBuf>,
    out_dir: &Path,
) -> Vec<MediaAttachment> {
    if !settings.enabled {
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
        let args = ffmpeg_args(&local, &out_path, target, is_video, settings);

        let status = tokio::process::Command::new("ffmpeg")
            .args(&args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
        match status {
            Ok(s) if s.success() => {
                tracing::info!(
                    "transformed {} -> {} for {provider} ({}x{}, {:?})",
                    local.display(),
                    out_path.display(),
                    target.width,
                    target.height,
                    settings.mode
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

    fn settings(enabled: bool, mode: TransformMode) -> TransformSettings {
        TransformSettings {
            enabled,
            mode,
            ..Default::default()
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
        // v2 rows: Facebook 4:5, YouTube Shorts 9:16 (video only).
        assert_eq!(
            render_target("facebook", false),
            Some(RenderTarget { width: 1080, height: 1350 })
        );
        assert_eq!(
            render_target("youtube", true),
            Some(RenderTarget { width: 1080, height: 1920 })
        );
        assert_eq!(render_target("youtube", false), None);
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
            &settings(true, TransformMode::Crop),
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
    fn ffmpeg_args_blur_pad_builds_background_and_overlay() {
        let args = ffmpeg_args(
            Path::new("/u/a.jpg"),
            Path::new("/t/b.jpg"),
            RenderTarget { width: 1080, height: 1350 },
            false,
            &settings(true, TransformMode::BlurPad),
        );
        let joined = args.join(" ");
        assert!(joined.contains("gblur=sigma=20"), "blurred bg: {joined}");
        assert!(
            joined.contains("force_original_aspect_ratio=decrease"),
            "fit-not-fill foreground: {joined}"
        );
        assert!(joined.contains("overlay="), "centered overlay: {joined}");
        assert!(joined.contains("-filter_complex"), "multi-output graph: {joined}");
        assert!(joined.contains("-map [out]"), "map the graph output: {joined}");
        // The [b]scale= foreground sub-chain must fit, not crop: between
        // [b]scale= and [fg] there is decrease and no crop.
        let fg_chain = joined
            .split("[b]scale=")
            .nth(1)
            .and_then(|s| s.split("[fg]").next())
            .unwrap_or_default();
        assert!(
            fg_chain.contains("force_original_aspect_ratio=decrease") && !fg_chain.contains("crop"),
            "foreground fits uncropped: {fg_chain}"
        );
        assert!(!joined.contains("-vf"), "blur mode does not use -vf: {joined}");
    }

    #[test]
    fn ffmpeg_args_reencode_videos_with_faststart() {
        let args = ffmpeg_args(
            Path::new("/u/a.mp4"),
            Path::new("/t/b.mp4"),
            RenderTarget { width: 1080, height: 1920 },
            true,
            &settings(true, TransformMode::Crop),
        );
        let joined = args.join(" ");
        assert!(joined.contains("libx264"), "got: {joined}");
        assert!(joined.contains("aac"), "got: {joined}");
        assert!(joined.contains("+faststart"), "got: {joined}");
        assert!(!joined.contains("frames:v"), "video is not single-frame: {joined}");
    }

    #[test]
    fn ffmpeg_args_trim_emits_ss_before_input_and_t_duration() {
        let s = TransformSettings {
            enabled: true,
            mode: TransformMode::Crop,
            trim_start: Some(1.5),
            trim_end: Some(4.0),
        };
        let args = ffmpeg_args(
            Path::new("/u/a.mp4"),
            Path::new("/t/b.mp4"),
            RenderTarget { width: 1080, height: 1920 },
            true,
            &s,
        );
        // -ss precedes -i (input-side seek).
        let ss = args.iter().position(|a| a == "-ss").unwrap();
        let input = args.iter().position(|a| a == "-i").unwrap();
        assert!(ss < input, "-ss before -i: {args:?}");
        assert_eq!(args[ss + 1], "1.5");
        // duration = end - start
        let t = args.iter().position(|a| a == "-t").unwrap();
        assert_eq!(args[t + 1], "2.5");
    }

    #[test]
    fn ffmpeg_args_trim_end_only_counts_from_zero() {
        let s = TransformSettings {
            enabled: true,
            mode: TransformMode::Crop,
            trim_start: None,
            trim_end: Some(3.0),
        };
        let args = ffmpeg_args(
            Path::new("/u/a.mp4"),
            Path::new("/t/b.mp4"),
            RenderTarget { width: 1080, height: 1920 },
            true,
            &s,
        );
        assert!(!args.contains(&"-ss".to_string()));
        let t = args.iter().position(|a| a == "-t").unwrap();
        assert_eq!(args[t + 1], "3");
    }

    #[test]
    fn trim_is_ignored_for_images() {
        let s = TransformSettings {
            enabled: true,
            mode: TransformMode::BlurPad,
            trim_start: Some(1.0),
            trim_end: Some(2.0),
        };
        let args = ffmpeg_args(
            Path::new("/u/a.jpg"),
            Path::new("/t/b.jpg"),
            RenderTarget { width: 1080, height: 1350 },
            false,
            &s,
        );
        assert!(!args.contains(&"-ss".to_string()), "no seek on images");
        assert!(!args.contains(&"-t".to_string()), "no duration on images");
    }

    #[test]
    fn settings_parse_mode_and_normalized_trim() {
        let v = serde_json::json!({
            "transform_media": true,
            "transform_mode": "blur",
            "trim_start": 2,
            "trim_end": 1, // before the start — must drop, not produce a negative -t
        });
        let s = TransformSettings::from_settings(&v);
        assert!(s.enabled);
        assert_eq!(s.mode, TransformMode::BlurPad);
        assert_eq!(s.trim_start, Some(2.0));
        assert_eq!(s.trim_end, None);

        let s = TransformSettings::from_settings(&serde_json::json!({
            "transform_media": true,
            "transform_mode": "nonsense",
            "trim_start": -5,
        }));
        assert_eq!(s.mode, TransformMode::Crop);
        assert_eq!(s.trim_start, None);

        let s = TransformSettings::from_settings(&serde_json::json!({}));
        assert!(!s.enabled);
        assert_eq!(s.mode, TransformMode::Crop);
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
            &settings(false, TransformMode::Crop),
            |_| None,
            Path::new("/tmp/t"),
        ));
        assert_eq!(out[0].url, a[0].url);
    }

    /// E1 v2 acceptance: real ffmpeg on a wide sample through blur-pad
    /// must produce exactly the target dimensions. Self-skipping when
    /// ffmpeg is missing (CI boxes) — the arg tests above carry the
    /// logic; this proves the filter graph itself is well-formed.
    #[test]
    fn live_blur_pad_produces_target_dimensions() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if !ffmpeg_available().await {
                eprintln!("⏭ skipped: ffmpeg not on PATH");
                return;
            }
            let dir = std::env::temp_dir().join(format!("sf-e1v2-{}", uuid::Uuid::new_v4()));
            tokio::fs::create_dir_all(&dir).await.unwrap();

            // A 1600x900 (16:9) wide sample — badly out of spec for 4:5.
            let wide = dir.join("wide.mp4");
            let ok = tokio::process::Command::new("ffmpeg")
                .args([
                    "-y", "-f", "lavfi", "-i", "testsrc=size=1600x900:duration=2:rate=10",
                    "-c:v", "libx264", "-preset", "ultrafast",
                ])
                .arg(&wide)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .unwrap();
            assert!(ok.success(), "fixture generation failed");

            let target = render_target("instagram", true).unwrap();
            let s = TransformSettings {
                enabled: true,
                mode: TransformMode::BlurPad,
                trim_start: Some(0.5),
                trim_end: Some(1.5),
            };
            let out_file = dir.join("out.mp4");
            let args = ffmpeg_args(&wide, &out_file, target, true, &s);
            let status = tokio::process::Command::new("ffmpeg")
                .args(&args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .unwrap();
            assert!(status.success(), "blur-pad transform failed");

            // ffprobe the produced dimensions.
            let probe = tokio::process::Command::new("ffprobe")
                .args([
                    "-v", "error", "-select_streams", "v:0",
                    "-show_entries", "stream=width,height",
                    "-of", "csv=p=0",
                ])
                .arg(&out_file)
                .output()
                .await
                .unwrap();
            assert!(probe.status.success(), "ffprobe failed");
            let dims = String::from_utf8_lossy(&probe.stdout);
            assert_eq!(
                dims.trim(),
                format!("{},{}", target.width, target.height),
                "blur-pad output must be exactly the target"
            );
            let _ = tokio::fs::remove_dir_all(&dir).await;
        });
    }

    /// The engine runs tokio; tests for the async fn need an executor. Tiny
    /// helper instead of a dev-dependency.
    fn futures_executor_block<F: Future>(fut: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(fut)
    }
}
