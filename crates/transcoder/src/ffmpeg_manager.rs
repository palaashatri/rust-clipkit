use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;
use ytdlp_core::{AppEvent, FfmpegRelease};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Error)]
pub enum FfmpegManagerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Archive extraction failed: {0}")]
    Extraction(String),
    #[error("Platform not supported for auto-download")]
    UnsupportedPlatform,
    #[error("ffmpeg binary not found in archive")]
    BinaryNotFound,
    #[error("Checksum mismatch for {filename}: expected {expected}, actual {actual}")]
    ChecksumMismatch {
        filename: String,
        expected: String,
        actual: String,
    },
    #[error("Checksum for {filename} not found in checksums.sha256")]
    ChecksumNotFound { filename: String },
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    published_at: DateTime<Utc>,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Returns the platform-specific archive name, or None if auto-download is unsupported.
fn platform_asset_name() -> Option<&'static str> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("ffmpeg-master-latest-win64-gpl.zip")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("ffmpeg-master-latest-linux64-gpl.tar.xz")
    } else {
        None
    }
}

fn ffmpeg_bin_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

fn ffprobe_bin_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    }
}

/// Runs `ffmpeg -version` and extracts the version token (e.g. "7.0.2").
pub fn detect_ffmpeg_version(path: &Path) -> Option<String> {
    let mut cmd = std::process::Command::new(path);
    cmd.arg("-version");

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // First line: "ffmpeg version N.N.N Copyright (c)..."
    let first_line = stdout.lines().next()?;
    first_line.split_whitespace().nth(2).map(|v| v.to_string())
}

/// Returns true if auto-download is supported on the current platform.
pub fn auto_download_supported() -> bool {
    platform_asset_name().is_some()
}

/// Computes the lowercase hex SHA-256 hash of a file on disk.
pub fn compute_file_sha256(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Parses the expected SHA-256 checksum for `target_filename` from `checksums.sha256` text.
pub fn parse_checksums(content: &str, target_filename: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0];
            let filename = parts[1].trim_start_matches('*').trim_start_matches("./");
            if filename == target_filename
                && hash.len() == 64
                && hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Some(hash.to_lowercase());
            }
        }
    }
    None
}

/// Manages the ffmpeg binary: fetches the latest build from yt-dlp/FFmpeg-Builds,
/// downloads, and extracts to the configured bin directory.
pub struct FfmpegManager {
    bin_dir: PathBuf,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    client: Client,
}

impl FfmpegManager {
    pub fn new(bin_dir: PathBuf, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("clipkit/0.1")
            .build()
            .expect("Failed to build HTTP client");
        Self {
            bin_dir,
            event_tx,
            client,
        }
    }

    /// Path to the managed ffmpeg binary if it has been downloaded.
    pub fn managed_path(&self) -> Option<PathBuf> {
        let p = self.bin_dir.join(ffmpeg_bin_name());
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    /// Fetches the latest FFmpeg release info from GitHub.
    pub async fn fetch_latest_release(&self) -> Result<FfmpegRelease, FfmpegManagerError> {
        let url = "https://api.github.com/repos/yt-dlp/FFmpeg-Builds/releases/latest";
        let gh: GhRelease = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let asset_name = platform_asset_name().ok_or(FfmpegManagerError::UnsupportedPlatform)?;
        let asset = gh
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or(FfmpegManagerError::BinaryNotFound)?;

        Ok(FfmpegRelease {
            tag_name: gh.tag_name,
            published_at: gh.published_at,
            download_url: asset.browser_download_url.clone(),
        })
    }

    /// Downloads and installs the latest ffmpeg, emitting progress events.
    pub async fn download_latest(&self) {
        match self.do_download_latest().await {
            Ok(path) => {
                let version = detect_ffmpeg_version(&path).unwrap_or_else(|| "unknown".to_string());
                let _ = self
                    .event_tx
                    .send(AppEvent::FfmpegDownloaded { path, version });
            }
            Err(e) => {
                let _ = self.event_tx.send(AppEvent::FfmpegDownloadFailed {
                    error: e.to_string(),
                });
            }
        }
    }

    async fn do_download_latest(&self) -> Result<PathBuf, FfmpegManagerError> {
        let url = "https://api.github.com/repos/yt-dlp/FFmpeg-Builds/releases/latest";
        let gh: GhRelease = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let archive_name = platform_asset_name().ok_or(FfmpegManagerError::UnsupportedPlatform)?;
        let archive_asset = gh
            .assets
            .iter()
            .find(|a| a.name == archive_name)
            .ok_or(FfmpegManagerError::BinaryNotFound)?;

        let checksums_asset = gh
            .assets
            .iter()
            .find(|a| a.name == "checksums.sha256")
            .ok_or_else(|| FfmpegManagerError::ChecksumNotFound {
                filename: "checksums.sha256".to_string(),
            })?;

        // 1. Fetch checksums text and parse expected hash
        let checksums_text = self
            .client
            .get(&checksums_asset.browser_download_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let expected_sha256 = parse_checksums(&checksums_text, archive_name).ok_or_else(|| {
            FfmpegManagerError::ChecksumNotFound {
                filename: archive_name.to_string(),
            }
        })?;

        tokio::fs::create_dir_all(&self.bin_dir).await?;

        let archive_path = self.bin_dir.join(archive_name);
        let temp_archive = self.bin_dir.join(format!("{}.download", archive_name));
        if temp_archive.exists() {
            let _ = tokio::fs::remove_file(&temp_archive).await;
        }

        // 2. Download archive to temp file
        self.download_archive(&archive_asset.browser_download_url, &temp_archive)
            .await?;

        // 3. Verify SHA-256 hash before extraction
        let actual_sha256 = compute_file_sha256(&temp_archive)?;
        if actual_sha256.to_lowercase() != expected_sha256.to_lowercase() {
            let _ = tokio::fs::remove_file(&temp_archive).await;
            return Err(FfmpegManagerError::ChecksumMismatch {
                filename: archive_name.to_string(),
                expected: expected_sha256,
                actual: actual_sha256,
            });
        }

        // 4. Atomically rename
        tokio::fs::rename(&temp_archive, &archive_path).await?;

        // 5. Extract archive
        let ffmpeg_path = self.extract_archive(&archive_path).await?;

        // 6. Clean up archive
        tokio::fs::remove_file(&archive_path).await.ok();

        Ok(ffmpeg_path)
    }

    async fn download_archive(&self, url: &str, dest: &Path) -> Result<(), FfmpegManagerError> {
        use tokio::io::AsyncWriteExt;

        let mut resp = self.client.get(url).send().await?.error_for_status()?;
        let total = resp.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(dest).await?;
        let mut downloaded: u64 = 0;
        let throttle = Duration::from_millis(250);
        let mut last_emit = Instant::now()
            .checked_sub(throttle)
            .unwrap_or_else(Instant::now);

        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let now = Instant::now();
                if now.duration_since(last_emit) >= throttle {
                    let percent = (downloaded as f32 / total as f32) * 100.0;
                    let _ = self
                        .event_tx
                        .send(AppEvent::FfmpegDownloadProgress { percent });
                    last_emit = now;
                }
            }
        }
        file.flush().await?;
        Ok(())
    }

    async fn extract_archive(&self, archive_path: &Path) -> Result<PathBuf, FfmpegManagerError> {
        let bin_dir = self.bin_dir.clone();
        let archive_path = archive_path.to_path_buf();

        tokio::task::spawn_blocking(move || extract_archive_sync(&archive_path, &bin_dir))
            .await
            .map_err(|e| FfmpegManagerError::Extraction(e.to_string()))?
    }
}

fn extract_archive_sync(
    archive_path: &Path,
    dest_dir: &Path,
) -> Result<PathBuf, FfmpegManagerError> {
    if archive_path
        .extension()
        .map(|e| e == "zip")
        .unwrap_or(false)
    {
        extract_zip(archive_path, dest_dir)
    } else {
        extract_tar(archive_path, dest_dir)
    }
}

/// ZIP extraction (Windows). Uses the `zip` crate.
#[cfg(windows)]
fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<PathBuf, FfmpegManagerError> {
    use std::io;

    let file = std::fs::File::open(archive_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| FfmpegManagerError::Extraction(e.to_string()))?;

    let ffmpeg_name = ffmpeg_bin_name();
    let ffprobe_name = ffprobe_bin_name();
    let mut ffmpeg_dest: Option<PathBuf> = None;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| FfmpegManagerError::Extraction(e.to_string()))?;
        if !entry.is_file() {
            continue;
        }
        let entry_name = entry.name().replace('\\', "/");
        let file_name = entry_name.split('/').next_back().unwrap_or("");

        if file_name == ffmpeg_name || file_name == ffprobe_name {
            let dest = dest_dir.join(file_name);
            let mut out = std::fs::File::create(&dest)?;
            io::copy(&mut entry, &mut out)?;
            if file_name == ffmpeg_name {
                ffmpeg_dest = Some(dest);
            }
        }
    }

    ffmpeg_dest.ok_or(FfmpegManagerError::BinaryNotFound)
}

#[cfg(not(windows))]
fn extract_zip(_archive_path: &Path, _dest_dir: &Path) -> Result<PathBuf, FfmpegManagerError> {
    Err(FfmpegManagerError::UnsupportedPlatform)
}

/// tar.xz extraction (Linux/macOS). Uses the system `tar` command.
#[cfg(not(windows))]
fn extract_tar(archive_path: &Path, dest_dir: &Path) -> Result<PathBuf, FfmpegManagerError> {
    use std::process::Command;

    let tmp_dir = dest_dir.join(".ffmpeg_extract");
    // Remove any stale directory left by a previously interrupted extraction.
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)?;
    }
    std::fs::create_dir_all(&tmp_dir)?;

    let status = Command::new("tar")
        .arg("-xJf")
        .arg(archive_path)
        .arg("-C")
        .arg(&tmp_dir)
        .status()?;

    if !status.success() {
        return Err(FfmpegManagerError::Extraction(
            "tar -xJf failed".to_string(),
        ));
    }

    // Walk tmp_dir looking for the ffmpeg binary inside a bin/ subdirectory.
    let ffmpeg_name = ffmpeg_bin_name();
    let ffprobe_name = ffprobe_bin_name();
    let mut ffmpeg_src: Option<PathBuf> = None;
    let mut ffprobe_src: Option<PathBuf> = None;

    for entry in walkdir_two_levels(&tmp_dir) {
        if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
            if name == ffmpeg_name {
                ffmpeg_src = Some(entry.clone());
            } else if name == ffprobe_name {
                ffprobe_src = Some(entry.clone());
            }
        }
    }

    let src = ffmpeg_src.ok_or(FfmpegManagerError::BinaryNotFound)?;
    let dest = dest_dir.join(ffmpeg_name);
    std::fs::copy(&src, &dest)?;

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }

    if let Some(ffprobe_src) = ffprobe_src {
        let ffprobe_dest = dest_dir.join(ffprobe_name);
        std::fs::copy(&ffprobe_src, &ffprobe_dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&ffprobe_dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&ffprobe_dest, perms)?;
        }
    }

    // Cleanup temp extraction dir
    std::fs::remove_dir_all(&tmp_dir).ok();

    Ok(dest)
}

#[cfg(windows)]
fn extract_tar(archive_path: &Path, dest_dir: &Path) -> Result<PathBuf, FfmpegManagerError> {
    let _ = (archive_path, dest_dir);
    Err(FfmpegManagerError::UnsupportedPlatform)
}

/// Simple two-level directory walker (archive always has ≤3 levels deep).
#[cfg(not(windows))]
fn walkdir_two_levels(root: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                results.push(p);
            } else if p.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&p) {
                    for sub_entry in sub.flatten() {
                        let sp = sub_entry.path();
                        if sp.is_file() {
                            results.push(sp);
                        } else if sp.is_dir() {
                            if let Ok(sub2) = std::fs::read_dir(&sp) {
                                for e2 in sub2.flatten() {
                                    if e2.path().is_file() {
                                        results.push(e2.path());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CHECKSUMS: &str = r#"
0862210a5e0d32b3378403bfa479caf53bac294630ee2746904dd8828211db65  ffmpeg-master-latest-linux64-gpl.tar.xz
64bd76725aaf89def5d78ec79ea42b713324bde7a4634e115cd958f0e8ca02d2  ffmpeg-master-latest-win64-gpl.zip
"#;

    #[test]
    fn test_parse_checksums_linux_and_win() {
        assert_eq!(
            parse_checksums(SAMPLE_CHECKSUMS, "ffmpeg-master-latest-linux64-gpl.tar.xz").as_deref(),
            Some("0862210a5e0d32b3378403bfa479caf53bac294630ee2746904dd8828211db65")
        );
        assert_eq!(
            parse_checksums(SAMPLE_CHECKSUMS, "ffmpeg-master-latest-win64-gpl.zip").as_deref(),
            Some("64bd76725aaf89def5d78ec79ea42b713324bde7a4634e115cd958f0e8ca02d2")
        );
        assert_eq!(
            parse_checksums(SAMPLE_CHECKSUMS, "nonexistent-archive.zip"),
            None
        );
    }
}
