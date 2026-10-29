use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use reqwest::{redirect::Policy, Client, Url};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

const REPOSITORY: &str = "ishan-parihar/switchpost";
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Check release metadata without downloading or changing files
    #[arg(long)]
    check: bool,
    /// Release channel to inspect
    #[arg(long, value_enum, default_value_t = Channel::Stable)]
    channel: Channel,
    /// Install a specific release tag or version
    #[arg(long)]
    version: Option<String>,
    /// Permit installing a version older than the current binary
    #[arg(long)]
    allow_downgrade: bool,
    /// Restart the active Switchpost systemd service after replacing the binary
    #[arg(long)]
    restart: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Channel {
    Stable,
    Preview,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct InstallOrigin {
    application: String,
    source: String,
}

#[derive(Clone, Copy)]
enum ServiceManager {
    User,
    System,
}

struct InstallLock(PathBuf);

impl Drop for InstallLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub async fn run(options: UpdateArgs, json_mode: bool) -> Result<()> {
    let executable = std::env::current_exe().context("resolve current executable")?;
    let parent = executable
        .parent()
        .context("executable has no parent directory")?;
    let client = client()?;
    let release = selected_release(&client, options.channel, options.version.as_deref()).await?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).context("parse installed version")?;
    let available = release_version(&release.tag_name)?;
    if !options.allow_downgrade && available < current {
        bail!(
            "refusing downgrade from {current} to {available}; pass --allow-downgrade to permit it"
        );
    }

    if available == current {
        report(
            &options,
            json_mode,
            &current,
            &available,
            "current",
            "up_to_date",
            false,
        );
        return Ok(());
    }
    if options.check {
        report(
            &options,
            json_mode,
            &current,
            &available,
            "check",
            "update_available",
            false,
        );
        return Ok(());
    }
    ensure_owned_install(parent)?;
    let lock_path = parent.join(".switchpost-update.lock");
    let _lock = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map(|_| InstallLock(lock_path))
        .context("another update may be running, or the install directory is not writable")?;
    if cfg!(windows) {
        bail!("Switchpost self-update is supported on Linux and macOS; reinstall the matching release asset manually");
    }

    let service = if options.restart {
        active_service()
    } else {
        None
    };

    let artifact_name = platform_asset()?;
    let artifact = release
        .assets
        .iter()
        .find(|asset| asset.name == artifact_name)
        .with_context(|| format!("release {} has no {artifact_name} asset", release.tag_name))?;
    let sums = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
        .context("release has no SHA256SUMS; refusing an unverifiable update")?;
    let expected = checksum(&client, &sums.browser_download_url, artifact_name).await?;
    let temporary = parent.join(format!(".switchpost-update-{}.new", std::process::id()));
    let backup = parent.join(format!(".switchpost-update-{}.old", std::process::id()));
    download_verified(
        &client,
        &artifact.browser_download_url,
        &temporary,
        &expected,
    )
    .await?;
    preserve_mode(&executable, &temporary)?;
    verify_candidate(&temporary, &available)?;
    replace(&executable, &temporary, &backup)?;
    if let Err(error) = verify_candidate(&executable, &available) {
        let _ = fs::rename(&backup, &executable);
        let _ = sync_parent(parent);
        return Err(error)
            .context("new binary failed its post-install version check; prior binary restored");
    }
    if let Some(service_manager) = service {
        if let Err(error) = restart_service(service_manager) {
            let _ = fs::rename(&backup, &executable);
            let _ = sync_parent(parent);
            let _ = restart_service(service_manager);
            return Err(error)
                .context("service restart failed; prior binary restored and restart retried");
        }
    }
    fs::remove_file(&backup).context("remove successful-update rollback copy")?;
    sync_parent(parent)?;
    report(
        &options,
        json_mode,
        &current,
        &available,
        "updated",
        "success",
        service.is_some(),
    );
    Ok(())
}

fn ensure_owned_install(parent: &Path) -> Result<()> {
    let receipt = parent.join("switchpost.install-origin.json");
    let bytes = fs::read(&receipt).with_context(|| {
        format!("{} has no installer ownership receipt; reinstall with scripts/install.sh before using self-update", parent.display())
    })?;
    let origin: InstallOrigin =
        serde_json::from_slice(&bytes).context("invalid install-origin receipt")?;
    if origin.application != "switchpost" || origin.source != "release-binary" {
        bail!("this installation is not a Switchpost release binary; update it through its owning installer or package manager");
    }
    Ok(())
}

fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("switchpost/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .redirect(Policy::custom(|attempt| {
            let host = attempt.url().host_str().unwrap_or_default();
            if attempt.url().scheme() == "https"
                && matches!(
                    host,
                    "api.github.com"
                        | "github.com"
                        | "objects.githubusercontent.com"
                        | "release-assets.githubusercontent.com"
                )
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()?)
}

async fn selected_release(
    client: &Client,
    channel: Channel,
    version: Option<&str>,
) -> Result<Release> {
    let endpoint = if let Some(version) = version {
        let tag = if version.starts_with('v') {
            version.to_owned()
        } else {
            format!("v{version}")
        };
        release_version(&tag)?;
        format!("https://api.github.com/repos/{REPOSITORY}/releases/tags/{tag}")
    } else if matches!(channel, Channel::Stable) {
        format!("https://api.github.com/repos/{REPOSITORY}/releases/latest")
    } else {
        format!("https://api.github.com/repos/{REPOSITORY}/releases?per_page=20")
    };
    let response = client.get(endpoint).send().await?.error_for_status()?;
    if matches!(channel, Channel::Preview) && version.is_none() {
        let releases: Vec<Release> = response.json().await?;
        return releases
            .into_iter()
            .find(|release| release.prerelease)
            .context("no preview release is published");
    }
    let release: Release = response.json().await?;
    if matches!(channel, Channel::Stable) && release.prerelease {
        bail!("requested stable channel resolved to a prerelease");
    }
    Ok(release)
}

fn release_version(tag: &str) -> Result<Version> {
    Version::parse(tag.trim_start_matches('v'))
        .with_context(|| format!("invalid release version: {tag}"))
}

fn platform_asset() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("switchpost-linux-x64"),
        ("linux", "aarch64") => Ok("switchpost-linux-arm64"),
        ("macos", "x86_64") => Ok("switchpost-macos-x64"),
        ("macos", "aarch64") => Ok("switchpost-macos-arm64"),
        (os, arch) => bail!("no Switchpost release binary for {os}/{arch}; install a supported release from https://github.com/{REPOSITORY}/releases"),
    }
}

async fn checksum(client: &Client, url: &str, name: &str) -> Result<String> {
    let sums = client
        .get(checked_asset_url(url)?)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    for line in sums.lines() {
        let mut fields = line.split_whitespace();
        let Some(digest) = fields.next() else {
            continue;
        };
        let Some(file) = fields.next() else { continue };
        if file.trim_start_matches('*') == name {
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                bail!("SHA256SUMS contains an invalid digest for {name}");
            }
            return Ok(digest.to_ascii_lowercase());
        }
    }
    bail!("SHA256SUMS has no entry for {name}")
}

async fn download_verified(client: &Client, url: &str, path: &Path, expected: &str) -> Result<()> {
    let mut response = client
        .get(checked_asset_url(url)?)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        bail!("release asset exceeds the 512 MiB update limit");
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let result = async {
        while let Some(chunk) = response.chunk().await? {
            total = total
                .checked_add(chunk.len() as u64)
                .context("asset size overflow")?;
            if total > MAX_DOWNLOAD_BYTES {
                bail!("release asset exceeds the 512 MiB update limit");
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        file.sync_all().await?;
        let actual = format!("{:x}", hasher.finalize());
        if actual != expected {
            bail!("SHA-256 mismatch: expected {expected}, received {actual}");
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(path).await;
    }
    result
}

fn checked_asset_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).context("invalid release asset URL")?;
    let host = url.host_str().unwrap_or_default();
    if url.scheme() != "https"
        || !matches!(
            host,
            "api.github.com"
                | "github.com"
                | "objects.githubusercontent.com"
                | "release-assets.githubusercontent.com"
        )
    {
        bail!("release asset URL is outside the trusted HTTPS host allowlist");
    }
    Ok(url)
}

fn preserve_mode(current: &Path, candidate: &Path) -> Result<()> {
    fs::set_permissions(candidate, fs::metadata(current)?.permissions())?;
    Ok(())
}

fn verify_candidate(path: &Path, expected: &Version) -> Result<()> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .context("execute candidate --version")?;
    let expected = expected.to_string();
    let reports_expected = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .any(|token| token == expected);
    if !output.status.success() || !reports_expected {
        bail!("candidate binary does not report version {expected}");
    }
    Ok(())
}

fn replace(current: &Path, candidate: &Path, backup: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = (current, candidate, backup);
        bail!("Switchpost release updates are available for Linux and macOS only");
    }
    #[cfg(not(windows))]
    {
        fs::rename(current, backup).context("stage prior executable for rollback")?;
        if let Err(error) = fs::rename(candidate, current) {
            let _ = fs::rename(backup, current);
            return Err(error).context("atomically install verified binary");
        }
        Ok(())
    }
}

fn sync_parent(parent: &Path) -> Result<()> {
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn active_service() -> Option<ServiceManager> {
    if systemctl_status(&["--user", "is-active", "--quiet", "switchpost.service"]) {
        Some(ServiceManager::User)
    } else if systemctl_status(&["is-active", "--quiet", "switchpost.service"]) {
        Some(ServiceManager::System)
    } else {
        None
    }
}

fn systemctl_status(args: &[&str]) -> bool {
    Command::new("systemctl")
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

fn restart_service(manager: ServiceManager) -> Result<()> {
    let args = match manager {
        ServiceManager::User => vec!["--user", "restart", "switchpost.service"],
        ServiceManager::System => vec!["restart", "switchpost.service"],
    };
    let status = Command::new("systemctl")
        .args(&args)
        .status()
        .context("run systemctl restart")?;
    if !status.success() {
        bail!("systemctl {} exited with {status}", args.join(" "));
    }
    let check_args = match manager {
        ServiceManager::User => vec!["--user", "is-active", "--quiet", "switchpost.service"],
        ServiceManager::System => vec!["is-active", "--quiet", "switchpost.service"],
    };
    if !systemctl_status(&check_args) {
        bail!("switchpost.service did not become active after restart");
    }
    Ok(())
}

fn report(
    options: &UpdateArgs,
    json_mode: bool,
    current: &Version,
    available: &Version,
    action: &str,
    result: &str,
    service_restarted: bool,
) {
    if json_mode {
        println!(
            "{}",
            serde_json::json!({
                "application": "switchpost", "installed_version": current.to_string(),
                "available_version": available.to_string(),
                "channel": if matches!(options.channel, Channel::Stable) { "stable" } else { "preview" },
                "action": action, "result": result, "service_restarted": service_restarted
            })
        );
    } else if result == "up_to_date" {
        println!("switchpost {current} is current");
    } else if action == "check" {
        println!("switchpost {available} is available (installed: {current}); run `switchpost update` to install it");
    } else {
        let service_message = if service_restarted {
            "the active service was restarted"
        } else {
            "running processes still use the old executable image"
        };
        println!("switchpost updated from {current} to {available}; {service_message}");
    }
}
