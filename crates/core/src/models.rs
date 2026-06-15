use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;
use crate::presets::TranscodePreset;

// ── Download ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: Uuid,
    pub url: String,
    pub title: Option<String>,
    pub status: DownloadStatus,
    pub progress: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub output_path: Option<PathBuf>,
    pub error_detail: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DownloadJob {
    pub fn new(url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            title: None,
            status: DownloadStatus::Pending,
            progress: 0.0,
            speed: None,
            eta: None,
            output_path: None,
            error_detail: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStatus::Pending => write!(f, "Pending"),
            DownloadStatus::Downloading => write!(f, "Downloading"),
            DownloadStatus::Processing => write!(f, "Processing"),
            DownloadStatus::Completed => write!(f, "Completed"),
            DownloadStatus::Failed => write!(f, "Failed"),
            DownloadStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub percent: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub stage: DownloadStage,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadStage {
    Downloading,
    Processing,
    Merging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub output_path: PathBuf,
    pub title: Option<String>,
}

// ── Transcode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeJob {
    pub id: Uuid,
    pub input: PathBuf,
    pub output: PathBuf,
    pub status: TranscodeStatus,
    pub progress: f32,
    pub fps: Option<f32>,
    pub eta: Option<Duration>,
    pub error_detail: Option<String>,
    pub preset: Option<TranscodePreset>,
    pub created_at: DateTime<Utc>,
}

impl TranscodeJob {
    pub fn new(input: PathBuf, output: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            input,
            output,
            status: TranscodeStatus::Pending,
            progress: 0.0,
            fps: None,
            eta: None,
            error_detail: None,
            preset: None,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TranscodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TranscodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscodeStatus::Pending => write!(f, "Pending"),
            TranscodeStatus::Running => write!(f, "Running"),
            TranscodeStatus::Completed => write!(f, "Completed"),
            TranscodeStatus::Failed => write!(f, "Failed"),
            TranscodeStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeProgress {
    pub percent: f32,
    pub fps: Option<f32>,
    pub eta: Option<Duration>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeResult {
    pub output_path: PathBuf,
}

// ── yt-dlp version ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtDlpVersion {
    pub tag_name: String,
    pub published_at: DateTime<Utc>,
    pub is_prerelease: bool,
    pub download_url: String,
}

impl std::fmt::Display for YtDlpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag_name)
    }
}

// ── ffmpeg release ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfmpegRelease {
    pub tag_name: String,
    pub published_at: DateTime<Utc>,
    pub download_url: String,
}

// ── app update info ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppUpdateInfo {
    pub version: String,
    pub release_url: String,
}
