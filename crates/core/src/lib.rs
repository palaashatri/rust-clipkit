pub mod config;
pub mod events;
pub mod models;
pub mod presets;

pub use config::{AppConfig, ConfigError, ThemeChoice, bin_dir, config_dir, data_dir, default_download_dir, ffmpeg_bin_dir};
pub use events::AppEvent;
pub use models::{
    AppUpdateInfo, DownloadJob, DownloadProgress, DownloadResult, DownloadStage, DownloadStatus,
    FfmpegRelease, TranscodeJob, TranscodeProgress, TranscodeResult, TranscodeStatus, YtDlpVersion,
};
pub use presets::{
    AudioCodec, AudioFormat, ContainerFormat, ContentType, DownloadPreset, EncodingPreset,
    HdrPolicy, Resolution, SponsorBlockOptions, SubtitleOptions, TranscodePreset, VideoCodec,
    VideoContainer, VideoQuality,
};
