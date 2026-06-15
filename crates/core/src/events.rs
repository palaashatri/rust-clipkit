use crate::models::{
    AppUpdateInfo, DownloadProgress, DownloadResult, FfmpegRelease, TranscodeProgress,
    TranscodeResult, YtDlpVersion,
};
use std::path::PathBuf;
use uuid::Uuid;

/// Events broadcast from backend tasks to the UI layer.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Download events
    DownloadProgress { job_id: Uuid, progress: DownloadProgress },
    DownloadTitle { job_id: Uuid, title: String },
    DownloadOutputPath { job_id: Uuid, path: PathBuf },
    DownloadCompleted { job_id: Uuid, result: DownloadResult },
    DownloadFailed { job_id: Uuid, error: String },
    DownloadCancelled { job_id: Uuid },

    // Transcode events
    TranscodeProgress { job_id: Uuid, progress: TranscodeProgress },
    TranscodeCompleted { job_id: Uuid, result: TranscodeResult },
    TranscodeFailed { job_id: Uuid, error: String },
    TranscodeCancelled { job_id: Uuid },

    // yt-dlp version manager events
    YtDlpVersionsFetched(Result<Vec<YtDlpVersion>, String>),
    YtDlpBinaryDownloadProgress { version: String, percent: f32 },
    YtDlpBinaryDownloaded { version: String, path: PathBuf },
    YtDlpBinaryDownloadFailed { version: String, error: String },

    // ffmpeg manager events
    FfmpegReleasesFetched(Result<Vec<FfmpegRelease>, String>),
    FfmpegDownloadProgress { percent: f32 },
    FfmpegDownloaded { path: PathBuf, version: String },
    FfmpegDownloadFailed { error: String },

    // App self-update
    AppUpdateAvailable(AppUpdateInfo),

    // Tool detection
    FfmpegDetected(Option<PathBuf>),
}
