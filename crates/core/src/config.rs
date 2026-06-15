use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    #[default]
    Light,
    Dark,
    System,
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeChoice::Light => write!(f, "Light"),
            ThemeChoice::Dark => write!(f, "Dark"),
            ThemeChoice::System => write!(f, "System"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to determine config directory")]
    NoConfigDir,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

/// Platform config directory: `%APPDATA%\clipkit` on Windows, `~/.config/clipkit` on Unix.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(base.join("clipkit"))
}

/// Platform data directory used for storing managed binaries.
pub fn data_dir() -> Result<PathBuf, ConfigError> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or(ConfigError::NoConfigDir)?;
    Ok(base.join("clipkit"))
}

/// Subdirectory inside data_dir where versioned yt-dlp binaries live.
pub fn bin_dir() -> Result<PathBuf, ConfigError> {
    Ok(data_dir()?.join("bin"))
}

/// Subdirectory where the managed ffmpeg binary lives.
pub fn ffmpeg_bin_dir() -> Result<PathBuf, ConfigError> {
    Ok(data_dir()?.join("ffmpeg"))
}

pub fn default_download_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ytdlp_bin_dir: PathBuf,
    pub ffmpeg_bin_dir: PathBuf,
    pub active_ytdlp_version: Option<String>,
    pub use_system_ytdlp: bool,
    pub system_ytdlp_path: Option<PathBuf>,
    pub ffmpeg_path: Option<PathBuf>,
    pub managed_ffmpeg_version: Option<String>,
    pub default_download_dir: PathBuf,
    pub max_concurrent_downloads: usize,
    pub max_concurrent_transcodes: usize,
    pub theme: ThemeChoice,
    pub use_gpu: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let ytdlp_bin_dir = bin_dir().unwrap_or_else(|_| PathBuf::from("bin"));
        let ffmpeg_bin_dir = ffmpeg_bin_dir().unwrap_or_else(|_| PathBuf::from("ffmpeg"));
        AppConfig {
            ytdlp_bin_dir,
            ffmpeg_bin_dir,
            active_ytdlp_version: None,
            use_system_ytdlp: false,
            system_ytdlp_path: None,
            ffmpeg_path: None,
            managed_ffmpeg_version: None,
            default_download_dir: default_download_dir(),
            max_concurrent_downloads: 3,
            max_concurrent_transcodes: 2,
            theme: ThemeChoice::Light,
            use_gpu: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_dir()?.join("config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let dir = config_dir()?;
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(dir.join("config.toml"), content)?;
        Ok(())
    }

    /// Returns the path to the active yt-dlp binary, checking managed binaries first.
    pub fn active_ytdlp_path(&self) -> Option<PathBuf> {
        if self.use_system_ytdlp {
            return self.system_ytdlp_path.clone();
        }
        self.active_ytdlp_version.as_ref().map(|v| {
            let bin_name = if cfg!(target_os = "windows") {
                "yt-dlp.exe"
            } else if cfg!(target_os = "macos") {
                "yt-dlp_macos"
            } else {
                "yt-dlp"
            };
            self.ytdlp_bin_dir.join(v).join(bin_name)
        })
    }
}
