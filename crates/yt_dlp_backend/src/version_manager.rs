use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;
use ytdlp_core::{AppEvent, YtDlpVersion};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);

type ReleaseCache = Arc<Mutex<Option<(Instant, Vec<YtDlpVersion>)>>>;

#[derive(Debug, Error)]
pub enum VersionManagerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Checksum mismatch for {filename}: expected {expected}, actual {actual}")]
    ChecksumMismatch {
        filename: String,
        expected: String,
        actual: String,
    },
    #[error("Checksum for {filename} not found in SHA2-256SUMS")]
    ChecksumNotFound { filename: String },
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    published_at: DateTime<Utc>,
    prerelease: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

pub fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp"
    }
}

/// Sanitizes a release tag to prevent path traversal.
pub fn sanitize_tag(tag: &str) -> String {
    let mut clean: String = tag
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect();
    while clean.contains("..") {
        clean = clean.replace("..", "");
    }
    clean = clean.trim_matches('.').to_string();
    if clean.is_empty() {
        "unknown".to_string()
    } else {
        clean
    }
}

/// Parses the expected SHA-256 checksum for `target_filename` from `SHA2-256SUMS` text.
/// Supports standard formats: `<hash>  <filename>` and `<hash> *<filename>`.
pub fn parse_sha256sums(sums_content: &str, target_filename: &str) -> Option<String> {
    for line in sums_content.lines() {
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

/// Runs `yt-dlp --version` and returns the version string (e.g. "2024.10.22").
pub fn detect_ytdlp_version(path: &Path) -> Option<String> {
    let mut cmd = std::process::Command::new(path);
    cmd.arg("--version");

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Manages yt-dlp binary versions: fetches from GitHub, downloads with checksum verification, and tracks the active one.
pub struct VersionManager {
    bin_dir: PathBuf,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    client: Client,
    /// In-memory cache: (fetched_at, versions)
    cache: ReleaseCache,
}

impl VersionManager {
    pub fn new(bin_dir: PathBuf, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("clipkit/0.1")
            .build()
            .expect("Failed to build HTTP client");
        Self {
            bin_dir,
            event_tx,
            client,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Fetches releases from the GitHub API (cached for 6 h) and emits `YtDlpVersionsFetched`.
    pub async fn fetch_releases(&self) {
        let result = self.do_fetch_releases().await;
        let event = AppEvent::YtDlpVersionsFetched(result.map_err(|e| e.to_string()));
        let _ = self.event_tx.send(event);
    }

    /// Bypasses cache and forces a fresh fetch.
    pub async fn force_fetch_releases(&self) {
        {
            let mut cache = self.cache.lock().unwrap();
            *cache = None;
        }
        self.fetch_releases().await;
    }

    async fn do_fetch_releases(&self) -> Result<Vec<YtDlpVersion>, VersionManagerError> {
        // Check in-memory cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some((ts, ref versions)) = *cache {
                if ts.elapsed() < CACHE_TTL {
                    return Ok(versions.clone());
                }
            }
        }

        let url = "https://api.github.com/repos/yt-dlp/yt-dlp/releases?per_page=30";
        let releases: Vec<GhRelease> = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let asset_name = platform_asset_name();
        let versions: Vec<YtDlpVersion> = releases
            .into_iter()
            .filter_map(|r| {
                let asset = r.assets.iter().find(|a| a.name == asset_name)?;
                Some(YtDlpVersion {
                    tag_name: r.tag_name,
                    published_at: r.published_at,
                    is_prerelease: r.prerelease,
                    download_url: asset.browser_download_url.clone(),
                })
            })
            .collect();

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            *cache = Some((Instant::now(), versions.clone()));
        }

        Ok(versions)
    }

    /// Downloads the given version binary, verifies SHA-256 against release checksums, and emits progress/completion events.
    pub async fn download_version(&self, version: YtDlpVersion) {
        let tag = version.tag_name.clone();
        let result = self.do_download(&version).await;
        match result {
            Ok(path) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::YtDlpBinaryDownloaded { version: tag, path });
            }
            Err(e) => {
                let _ = self.event_tx.send(AppEvent::YtDlpBinaryDownloadFailed {
                    version: tag,
                    error: e.to_string(),
                });
            }
        }
    }

    async fn fetch_release_checksums(&self, tag: &str) -> Result<String, VersionManagerError> {
        let sums_url = format!(
            "https://github.com/yt-dlp/yt-dlp/releases/download/{}/SHA2-256SUMS",
            tag
        );
        let text = self
            .client
            .get(&sums_url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }

    async fn do_download(&self, version: &YtDlpVersion) -> Result<PathBuf, VersionManagerError> {
        use tokio::io::AsyncWriteExt;

        let clean_tag = sanitize_tag(&version.tag_name);
        let version_dir = self.bin_dir.join(&clean_tag);
        tokio::fs::create_dir_all(&version_dir).await?;

        let bin_name = platform_asset_name();
        let dest = version_dir.join(bin_name);

        // Fetch official checksums file first
        let checksums_text = self.fetch_release_checksums(&clean_tag).await?;
        let expected_sha256 = parse_sha256sums(&checksums_text, bin_name).ok_or_else(|| {
            VersionManagerError::ChecksumNotFound {
                filename: bin_name.to_string(),
            }
        })?;

        // If dest already exists, verify its checksum
        if dest.exists() {
            if let Ok(actual_sha256) = compute_file_sha256(&dest) {
                if actual_sha256 == expected_sha256 {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ =
                            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
                    }
                    return Ok(dest);
                }
            }
            // File is stale or corrupted; remove and re-download
            let _ = tokio::fs::remove_file(&dest).await;
        }

        let temp_dest = version_dir.join(format!("{}.download", bin_name));
        if temp_dest.exists() {
            let _ = tokio::fs::remove_file(&temp_dest).await;
        }

        let mut response = self
            .client
            .get(&version.download_url)
            .send()
            .await?
            .error_for_status()?;

        let total = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(&temp_dest).await?;
        let mut hasher = Sha256::new();
        let mut downloaded: u64 = 0;
        let tag = version.tag_name.clone();
        let throttle = Duration::from_millis(250);
        let mut last_emit = Instant::now()
            .checked_sub(throttle)
            .unwrap_or_else(Instant::now);

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;
            if total > 0 {
                let now = Instant::now();
                if now.duration_since(last_emit) >= throttle {
                    let percent = (downloaded as f32 / total as f32) * 100.0;
                    let _ = self.event_tx.send(AppEvent::YtDlpBinaryDownloadProgress {
                        version: tag.clone(),
                        percent,
                    });
                    last_emit = now;
                }
            }
        }
        file.flush().await?;
        drop(file);

        let actual_sha256 = format!("{:x}", hasher.finalize());
        if actual_sha256 != expected_sha256 {
            let _ = tokio::fs::remove_file(&temp_dest).await;
            return Err(VersionManagerError::ChecksumMismatch {
                filename: bin_name.to_string(),
                expected: expected_sha256,
                actual: actual_sha256,
            });
        }

        // Atomically rename verified binary to destination
        tokio::fs::rename(&temp_dest, &dest).await?;

        // Make executable on Unix only after checksum verification succeeds
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest, perms)?;
        }

        Ok(dest)
    }

    /// Detects yt-dlp on the system PATH and returns its absolute path if found.
    pub fn detect_system_ytdlp() -> Option<PathBuf> {
        let bin = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };
        let ok = std::process::Command::new(bin)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            return None;
        }
        find_on_path(bin)
    }

    /// Returns the stored path for a managed version.
    pub fn version_path(&self, tag: &str) -> PathBuf {
        let clean_tag = sanitize_tag(tag);
        self.bin_dir.join(clean_tag).join(platform_asset_name())
    }

    pub fn is_version_installed(&self, tag: &str) -> bool {
        self.version_path(tag).exists()
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SHA256SUMS: &str = r#"
6cf237b6070a7b45db72079fbe54c5e8c142b918a598cfa7051b43aa6a4aeb65  yt-dlp
f9611db18f6709dd8cf56f08170c8a6663ceec4ba2748da33633e2184d081f9b *yt-dlp.exe
e02319f390099516ca4d7dfae54302c0b4ebf5139fb2b58872b7a9fbb00bc8e5  yt-dlp_macos
"#;

    #[test]
    fn test_parse_sha256sums_all_platforms() {
        assert_eq!(
            parse_sha256sums(SAMPLE_SHA256SUMS, "yt-dlp").as_deref(),
            Some("6cf237b6070a7b45db72079fbe54c5e8c142b918a598cfa7051b43aa6a4aeb65")
        );
        assert_eq!(
            parse_sha256sums(SAMPLE_SHA256SUMS, "yt-dlp.exe").as_deref(),
            Some("f9611db18f6709dd8cf56f08170c8a6663ceec4ba2748da33633e2184d081f9b")
        );
        assert_eq!(
            parse_sha256sums(SAMPLE_SHA256SUMS, "yt-dlp_macos").as_deref(),
            Some("e02319f390099516ca4d7dfae54302c0b4ebf5139fb2b58872b7a9fbb00bc8e5")
        );
    }

    #[test]
    fn test_parse_sha256sums_missing_target() {
        assert_eq!(parse_sha256sums(SAMPLE_SHA256SUMS, "nonexistent"), None);
    }

    #[test]
    fn test_parse_sha256sums_empty_or_comment() {
        let content = "# Comment\n\n   \n# Another comment\n";
        assert_eq!(parse_sha256sums(content, "yt-dlp"), None);
    }

    #[test]
    fn test_compute_file_sha256() {
        let tmp_dir = std::env::temp_dir();
        let test_file = tmp_dir.join("test_sha256_clipkit.txt");
        std::fs::write(&test_file, b"clipkit test content").unwrap();

        let hash = compute_file_sha256(&test_file).unwrap();
        // SHA256 of "clipkit test content"
        let mut hasher = Sha256::new();
        hasher.update(b"clipkit test content");
        let expected = format!("{:x}", hasher.finalize());
        assert_eq!(hash, expected);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_sanitize_tag() {
        assert_eq!(sanitize_tag("2024.10.22"), "2024.10.22");
        assert_eq!(sanitize_tag("v1.0.0-beta.1"), "v1.0.0-beta.1");
        assert_eq!(sanitize_tag("../../etc/cron.d"), "etccron.d");
        assert_eq!(sanitize_tag("/absolute/path"), "absolutepath");
        assert_eq!(sanitize_tag(".."), "unknown");
    }
}
