pub mod detector;
pub mod engine;
pub mod ffmpeg_manager;

pub use detector::detect_ffmpeg;
pub use engine::{build_ffmpeg_args, TranscodeManager, TranscodeRequest};
pub use ffmpeg_manager::{FfmpegManager, auto_download_supported, detect_ffmpeg_version};
