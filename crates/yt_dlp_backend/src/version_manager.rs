use chrono::{DateTime, Utc};
use ytdlp_core::{AppEvent, YtDlpVersion};
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::broadcast;

const CACHE_TTL: Duration = Duration::from_secs(6 * 3600);

#[derive(Debug, Error)]
pub enum VersionManagerError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
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

fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp"
    }
}

/// Runs `yt-dlp --version` and returns the version string (e.g. "2024.10.22").
pub fn detect_ytdlp_version(path: &Path) -> Option<String> {
    let mut cmd = std::process::Command::new(path);
    cmd.arg("--version");
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
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

/// Manages yt-dlp binary versions: fetches from GitHub, downloads, and tracks the active one.
pub struct VersionManager {
    bin_dir: PathBuf,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    client: Client,
    /// In-memory cache: (fetched_at, versions)
    cache: Arc<Mutex<Option<(Instant, Vec<YtDlpVersion>)>>>,
}

impl VersionManager {
    pub fn new(bin_dir: PathBuf, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let client = Client::builder()
            .user_agent("clipkit/0.1")
            .build()
            .expect("Failed to build HTTP client");
        Self { bin_dir, event_tx, client, cache: Arc::new(Mutex::new(None)) }
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

    /// Downloads the given version binary and emits progress/completion events.
    pub async fn download_version(&self, version: YtDlpVersion) {
        let tag = version.tag_name.clone();
        let result = self.do_download(&version).await;
        match result {
            Ok(path) => {
                let _ = self.event_tx.send(AppEvent::YtDlpBinaryDownloaded {
                    version: tag,
                    path,
                });
            }
            Err(e) => {
                let _ = self.event_tx.send(AppEvent::YtDlpBinaryDownloadFailed {
                    version: tag,
                    error: e.to_string(),
                });
            }
        }
    }

    async fn do_download(&self, version: &YtDlpVersion) -> Result<PathBuf, VersionManagerError> {
        use tokio::io::AsyncWriteExt;

        let version_dir = self.bin_dir.join(&version.tag_name);
        tokio::fs::create_dir_all(&version_dir).await?;

        let bin_name = platform_asset_name();
        let dest = version_dir.join(bin_name);

        if dest.exists() {
            return Ok(dest);
        }

        let mut response = self
            .client
            .get(&version.download_url)
            .send()
            .await?
            .error_for_status()?;

        let total = response.content_length().unwrap_or(0);
        let mut file = tokio::fs::File::create(&dest).await?;
        let mut downloaded: u64 = 0;
        let tag = version.tag_name.clone();
        let throttle = Duration::from_millis(250);
        let mut last_emit = Instant::now()
            .checked_sub(throttle)
            .unwrap_or_else(Instant::now);

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
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

        // Make executable on Unix
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
        let bin = if cfg!(target_os = "windows") { "yt-dlp.exe" } else { "yt-dlp" };
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
        self.bin_dir.join(tag).join(platform_asset_name())
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
