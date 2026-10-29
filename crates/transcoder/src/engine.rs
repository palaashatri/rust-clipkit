use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;
use ytdlp_core::{
    AppEvent, AudioCodec, ContainerFormat, HdrPolicy, Resolution, TranscodePreset,
    TranscodeProgress, TranscodeResult, VideoCodec,
};

static TIME_RE: OnceLock<Regex> = OnceLock::new();
static FPS_RE: OnceLock<Regex> = OnceLock::new();

fn time_re() -> &'static Regex {
    TIME_RE.get_or_init(|| Regex::new(r"out_time_ms=(\d+)").unwrap())
}

fn fps_re() -> &'static Regex {
    FPS_RE.get_or_init(|| Regex::new(r"fps=([\d.]+)").unwrap())
}

const PROGRESS_THROTTLE: Duration = Duration::from_millis(250);

pub struct TranscodeRequest {
    pub job_id: Uuid,
    pub input: PathBuf,
    pub output: PathBuf,
    pub preset: TranscodePreset,
}

/// Manages concurrent ffmpeg transcoding jobs.
pub struct TranscodeManager {
    ffmpeg_path: Arc<Mutex<Option<PathBuf>>>,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    active_handles: Arc<Mutex<HashMap<Uuid, JoinHandle<()>>>>,
    semaphore: Arc<Semaphore>,
}

impl TranscodeManager {
    pub fn new(
        ffmpeg_path: Option<PathBuf>,
        event_tx: Arc<broadcast::Sender<AppEvent>>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            ffmpeg_path: Arc::new(Mutex::new(ffmpeg_path)),
            event_tx,
            active_handles: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    pub fn set_ffmpeg_path(&self, path: Option<PathBuf>) {
        *self.ffmpeg_path.lock().unwrap() = path;
    }

    pub fn start_transcode(&self, request: TranscodeRequest) {
        let job_id = request.job_id;
        let ffmpeg_path = self.ffmpeg_path.lock().unwrap().clone();
        let event_tx = Arc::clone(&self.event_tx);
        let semaphore = Arc::clone(&self.semaphore);

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            run_transcode(request, ffmpeg_path, event_tx).await;
        });

        self.active_handles.lock().unwrap().insert(job_id, handle);
    }

    pub fn cancel_transcode(&self, job_id: Uuid) {
        let handle = self.active_handles.lock().unwrap().remove(&job_id);
        if let Some(h) = handle {
            h.abort();
        }
        let _ = self.event_tx.send(AppEvent::TranscodeCancelled { job_id });
    }
}

/// Builds the ffmpeg argument list for a transcode request.
pub fn build_ffmpeg_args(request: &TranscodeRequest) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // Overwrite without prompt; progress to stderr
    args.extend(["-y".into(), "-progress".into(), "pipe:2".into()]);

    // Input
    args.extend(["-i".into(), request.input.to_string_lossy().into_owned()]);

    let preset = &request.preset;
    let is_audio_only = matches!(
        preset.container,
        ContainerFormat::Mp3
            | ContainerFormat::Flac
            | ContainerFormat::Aac
            | ContainerFormat::Ogg
            | ContainerFormat::Wav
            | ContainerFormat::Wma
            | ContainerFormat::M4a
    );

    if is_audio_only {
        // Audio-only transcode
        args.push("-vn".into());
        match &preset.audio_codec {
            AudioCodec::Mp3 => {
                args.extend(["-c:a".into(), "libmp3lame".into()]);
            }
            AudioCodec::Flac => {
                args.extend(["-c:a".into(), "flac".into()]);
            }
            AudioCodec::Aac => {
                args.extend(["-c:a".into(), "aac".into()]);
            }
            AudioCodec::Opus => {
                args.extend(["-c:a".into(), "libopus".into()]);
            }
            AudioCodec::Vorbis => {
                args.extend(["-c:a".into(), "libvorbis".into()]);
            }
            AudioCodec::Copy => {
                args.extend(["-c:a".into(), "copy".into()]);
            }
        }
        if let Some(br) = preset.audio_bitrate_kbps {
            args.extend(["-b:a".into(), format!("{}k", br)]);
        }
    } else {
        // Video transcode
        match &preset.video_codec {
            VideoCodec::Copy => {
                args.extend(["-c:v".into(), "copy".into()]);
            }
            VideoCodec::H264 => {
                if preset.use_gpu {
                    // Detect and use hardware encoder
                    if cfg!(target_os = "macos") {
                        args.extend(["-c:v".into(), "h264_videotoolbox".into()]);
                        args.extend(["-realtime".into(), "true".into()]);
                    } else if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
                        // Assuming NVENC as high-probability; could probe in future
                        args.extend(["-c:v".into(), "h264_nvenc".into()]);
                        args.extend(["-preset".into(), "p4".into()]); // nvenc uses p1-p7
                    } else {
                        args.extend(["-c:v".into(), "libx264".into()]);
                        args.extend(["-preset".into(), preset.speed.as_str().into()]);
                    }
                } else {
                    args.extend(["-c:v".into(), "libx264".into()]);
                    args.extend(["-preset".into(), preset.speed.as_str().into()]);
                }
                args.extend(["-crf".into(), preset.crf.to_string()]);
            }
            VideoCodec::H265 => {
                if preset.use_gpu {
                    if cfg!(target_os = "macos") {
                        args.extend(["-c:v".into(), "hevc_videotoolbox".into()]);
                        args.extend(["-realtime".into(), "true".into()]);
                    } else if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
                        args.extend(["-c:v".into(), "hevc_nvenc".into()]);
                    } else {
                        args.extend(["-c:v".into(), "libx265".into()]);
                        args.extend(["-preset".into(), preset.speed.as_str().into()]);
                    }
                } else {
                    args.extend(["-c:v".into(), "libx265".into()]);
                    args.extend(["-preset".into(), preset.speed.as_str().into()]);
                }
                args.extend(["-crf".into(), preset.crf.to_string()]);
            }
            VideoCodec::Av1 => {
                if preset.use_gpu {
                    // Only very recent GPUs support AV1 encoding
                    if cfg!(target_os = "macos") {
                        args.extend(["-c:v".into(), "av1_videotoolbox".into()]);
                    } else {
                        args.extend(["-c:v".into(), "av1_nvenc".into()]);
                    }
                } else {
                    args.extend(["-c:v".into(), "libsvtav1".into()]);
                    args.extend(["-preset".into(), "8".into()]);
                }
                args.extend(["-crf".into(), preset.crf.to_string()]);
            }
            VideoCodec::ProRes => {
                if preset.use_gpu && cfg!(target_os = "macos") {
                    args.extend(["-c:v".into(), "prores_videotoolbox".into()]);
                } else {
                    args.extend(["-c:v".into(), "prores_ks".into()]);
                    args.extend(["-profile:v".into(), "3".into()]); // ProRes 422 (HQ)
                    args.extend(["-vendor".into(), "apl0".into()]);
                    args.extend(["-bits_per_mb".into(), "8000".into()]);
                }
            }
            VideoCodec::Vp9 => {
                args.extend(["-c:v".into(), "libvpx-vp9".into()]);
                args.extend(["-crf".into(), preset.crf.to_string()]);
                args.extend(["-b:v".into(), "0".into()]);
            }
            VideoCodec::Vp8 => {
                args.extend(["-c:v".into(), "libvpx".into()]);
                args.extend(["-crf".into(), preset.crf.to_string()]);
                args.extend(["-b:v".into(), "1M".into()]);
            }
            VideoCodec::Mpeg4 => {
                args.extend(["-c:v".into(), "mpeg4".into()]);
                args.extend(["-vtag".into(), "xvid".into()]);
                args.extend(["-qscale:v".into(), preset.crf.to_string()]);
            }
            VideoCodec::Mpeg2 => {
                args.extend(["-c:v".into(), "mpeg2video".into()]);
                args.extend(["-qscale:v".into(), preset.crf.to_string()]);
            }
            VideoCodec::Mpeg1 => {
                args.extend(["-c:v".into(), "mpeg1video".into()]);
                args.extend(["-qscale:v".into(), preset.crf.to_string()]);
            }
            VideoCodec::Theora => {
                args.extend(["-c:v".into(), "libtheora".into()]);
                args.extend(["-qscale:v".into(), preset.crf.to_string()]);
            }
            VideoCodec::Mjpeg => {
                args.extend(["-c:v".into(), "mjpeg".into()]);
                args.extend(["-qscale:v".into(), preset.crf.to_string()]);
            }
        }

        // Resolution scaling
        let scale_filter = match &preset.resolution {
            Resolution::Source => None,
            Resolution::Res8K => Some("scale=-2:4320"),
            Resolution::Res4K => Some("scale=-2:2160"),
            Resolution::Res1440p => Some("scale=-2:1440"),
            Resolution::Res1080p => Some("scale=-2:1080"),
            Resolution::Res720p => Some("scale=-2:720"),
            Resolution::Res480p => Some("scale=-2:480"),
            Resolution::Res360p => Some("scale=-2:360"),
            Resolution::Res240p => Some("scale=-2:240"),
        };

        // HDR handling
        // Tonemap fallback: zscale is often missing. Using standard filters.
        let hdr_filter = match &preset.hdr_policy {
            HdrPolicy::Preserve => None,
            HdrPolicy::ToneMapToSdr => Some("tonemap=hable,format=yuv420p".to_string()),
        };

        // Build video filter chain (only when re-encoding; streamcopy cannot use filters)
        if !matches!(preset.video_codec, VideoCodec::Copy) {
            let mut filters: Vec<String> = Vec::new();
            if let Some(s) = scale_filter {
                filters.push(s.to_string());
            }
            if let Some(h) = hdr_filter {
                filters.push(h);
            }
            if !filters.is_empty() {
                args.extend(["-vf".into(), filters.join(",")]);
            }
        }

        // Preserve HDR color metadata when not tone-mapping
        if matches!(preset.hdr_policy, HdrPolicy::Preserve)
            && !matches!(preset.video_codec, VideoCodec::Copy)
        {
            args.extend([
                "-color_primaries".into(),
                "bt2020".into(),
                "-color_trc".into(),
                "smpte2084".into(),
                "-colorspace".into(),
                "bt2020nc".into(),
            ]);
        }

        // Audio codec
        match &preset.audio_codec {
            AudioCodec::Aac => {
                args.extend(["-c:a".into(), "aac".into()]);
            }
            AudioCodec::Mp3 => {
                args.extend(["-c:a".into(), "libmp3lame".into()]);
            }
            AudioCodec::Opus => {
                args.extend(["-c:a".into(), "libopus".into()]);
            }
            AudioCodec::Flac => {
                args.extend(["-c:a".into(), "flac".into()]);
            }
            AudioCodec::Vorbis => {
                args.extend(["-c:a".into(), "libvorbis".into()]);
            }
            AudioCodec::Copy => {
                args.extend(["-c:a".into(), "copy".into()]);
            }
        }
        if let Some(br) = preset.audio_bitrate_kbps {
            args.extend(["-b:a".into(), format!("{}k", br)]);
        }
    }

    // Output file
    args.push(request.output.to_string_lossy().into_owned());
    args
}

async fn run_transcode(
    request: TranscodeRequest,
    ffmpeg_path: Option<PathBuf>,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
) {
    let job_id = request.job_id;

    if request.input == request.output {
        let _ = event_tx.send(AppEvent::TranscodeFailed {
            job_id,
            error: "Input and output file paths cannot be identical; refusing to overwrite source."
                .to_string(),
        });
        return;
    }

    let ffmpeg = match ffmpeg_path {
        Some(p) if p.exists() => p,
        _ => {
            let _ = event_tx.send(AppEvent::TranscodeFailed {
                job_id,
                error: "ffmpeg not found. Please install it and set its path in Settings."
                    .to_string(),
            });
            return;
        }
    };

    // Probe input duration for percent calculation
    let total_secs = probe_duration(&ffmpeg, &request.input).await.unwrap_or(0.0);

    let args = build_ffmpeg_args(&request);

    let mut cmd = tokio::process::Command::new(&ffmpeg);
    cmd.args(&args).stdout(Stdio::null()).stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = event_tx.send(AppEvent::TranscodeFailed {
                job_id,
                error: format!("Failed to launch ffmpeg: {}", e),
            });
            return;
        }
    };

    let stderr = child.stderr.take().expect("stderr piped");
    let mut lines = BufReader::new(stderr).lines();

    let mut current_ms: u64 = 0;
    let mut current_fps: Option<f32> = None;
    let mut last_progress_emit: Option<Instant> = None;
    let mut error_log: Vec<String> = Vec::new();

    while let Ok(Some(line)) = lines.next_line().await {
        // Keep a small buffer of the last lines for error reporting
        if error_log.len() > 20 {
            error_log.remove(0);
        }
        error_log.push(line.clone());

        if let Some(caps) = time_re().captures(&line) {
            current_ms = caps[1].parse().unwrap_or(current_ms);
        }
        if let Some(caps) = fps_re().captures(&line) {
            current_fps = caps[1].parse().ok();
        }

        if line.starts_with("progress=") {
            let now = Instant::now();
            if last_progress_emit.is_none_or(|t| now.duration_since(t) >= PROGRESS_THROTTLE) {
                let elapsed_secs = current_ms as f32 / 1_000_000.0;
                let percent = if total_secs > 0.0 {
                    (elapsed_secs / total_secs * 100.0).min(100.0)
                } else {
                    0.0
                };
                let eta = if total_secs > 0.0 && elapsed_secs < total_secs {
                    Some(Duration::from_secs_f32(total_secs - elapsed_secs))
                } else {
                    None
                };
                let _ = event_tx.send(AppEvent::TranscodeProgress {
                    job_id,
                    progress: TranscodeProgress {
                        percent,
                        fps: current_fps,
                        eta,
                        message: format!("{:.1}%", percent),
                    },
                });
                last_progress_emit = Some(now);
            }
        }
    }

    let status = child.wait().await;
    match status {
        Ok(s) if s.success() => {
            let _ = event_tx.send(AppEvent::TranscodeCompleted {
                job_id,
                result: TranscodeResult {
                    output_path: request.output.clone(),
                },
            });
        }
        Ok(s) => {
            let detail = if !error_log.is_empty() {
                format!("\nLast output:\n{}", error_log.join("\n"))
            } else {
                "".to_string()
            };
            let _ = event_tx.send(AppEvent::TranscodeFailed {
                job_id,
                error: format!("ffmpeg exited with status {}{}", s, detail),
            });
        }
        Err(e) => {
            let _ = event_tx.send(AppEvent::TranscodeFailed {
                job_id,
                error: format!("Error waiting for ffmpeg: {}", e),
            });
        }
    }
}

/// Uses ffprobe to determine total duration of the input file in seconds.
async fn probe_duration(ffmpeg: &Path, input: &Path) -> Option<f32> {
    // Derive ffprobe path from ffmpeg path
    let ffprobe = ffmpeg.parent()?.join(if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });

    let mut cmd = tokio::process::Command::new(&ffprobe);
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-i",
        input.to_str()?,
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd.output().await.ok()?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let duration_str = json["format"]["duration"].as_str()?;
    duration_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use ytdlp_core::{
        AudioCodec, ContainerFormat, EncodingPreset, HdrPolicy, Resolution, TranscodePreset,
        VideoCodec,
    };

    #[test]
    fn test_build_ffmpeg_args_audio_only() {
        let preset = TranscodePreset {
            name: "MP3 Test".to_string(),
            container: ContainerFormat::Mp3,
            video_codec: VideoCodec::Copy,
            audio_codec: AudioCodec::Mp3,
            resolution: Resolution::Source,
            hdr_policy: HdrPolicy::ToneMapToSdr,
            crf: 0,
            speed: EncodingPreset::Medium,
            audio_bitrate_kbps: Some(320),
            use_gpu: false,
        };

        let req = TranscodeRequest {
            job_id: Uuid::new_v4(),
            input: PathBuf::from("/tmp/input.mkv"),
            output: PathBuf::from("/tmp/output.mp3"),
            preset,
        };

        let args = build_ffmpeg_args(&req);
        assert!(args.contains(&"-y".to_string()));
        assert!(args.contains(&"-vn".to_string()));
        let ca_idx = args.iter().position(|a| a == "-c:a").unwrap();
        assert_eq!(args[ca_idx + 1], "libmp3lame");
        let ba_idx = args.iter().position(|a| a == "-b:a").unwrap();
        assert_eq!(args[ba_idx + 1], "320k");
        assert_eq!(args.last().unwrap(), "/tmp/output.mp3");
    }

    #[test]
    fn test_build_ffmpeg_args_video_scaling_and_crf() {
        let preset = TranscodePreset {
            name: "MP4 1080p Test".to_string(),
            container: ContainerFormat::Mp4,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            resolution: Resolution::Res1080p,
            hdr_policy: HdrPolicy::ToneMapToSdr,
            crf: 20,
            speed: EncodingPreset::Fast,
            audio_bitrate_kbps: Some(192),
            use_gpu: false,
        };

        let req = TranscodeRequest {
            job_id: Uuid::new_v4(),
            input: PathBuf::from("/tmp/video.mov"),
            output: PathBuf::from("/tmp/video.mp4"),
            preset,
        };

        let args = build_ffmpeg_args(&req);
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"-crf".to_string()));
        let crf_idx = args.iter().position(|a| a == "-crf").unwrap();
        assert_eq!(args[crf_idx + 1], "20");

        // Scale filter should be in -vf
        let vf_idx = args.iter().position(|a| a == "-vf").unwrap();
        assert!(args[vf_idx + 1].contains("scale=-2:1080"));
    }

    #[test]
    fn test_build_ffmpeg_args_copy_codec() {
        let preset = TranscodePreset {
            name: "Remux Test".to_string(),
            container: ContainerFormat::Mkv,
            video_codec: VideoCodec::Copy,
            audio_codec: AudioCodec::Copy,
            resolution: Resolution::Source,
            hdr_policy: HdrPolicy::Preserve,
            crf: 0,
            speed: EncodingPreset::Medium,
            audio_bitrate_kbps: None,
            use_gpu: false,
        };

        let req = TranscodeRequest {
            job_id: Uuid::new_v4(),
            input: PathBuf::from("/tmp/input.mp4"),
            output: PathBuf::from("/tmp/output.mkv"),
            preset,
        };

        let args = build_ffmpeg_args(&req);
        let cv_idx = args.iter().position(|a| a == "-c:v").unwrap();
        assert_eq!(args[cv_idx + 1], "copy");
        let ca_idx = args.iter().position(|a| a == "-c:a").unwrap();
        assert_eq!(args[ca_idx + 1], "copy");
        assert!(!args.contains(&"-vf".to_string()));
    }

    #[test]
    fn test_build_ffmpeg_args_copy_codec_disallows_filters() {
        let preset = TranscodePreset {
            name: "Remux Test With Res".to_string(),
            container: ContainerFormat::Mp4,
            video_codec: VideoCodec::Copy,
            audio_codec: AudioCodec::Copy,
            resolution: Resolution::Res1080p, // Even if 1080p is selected
            hdr_policy: HdrPolicy::ToneMapToSdr, // Or tonemapping selected
            crf: 0,
            speed: EncodingPreset::Medium,
            audio_bitrate_kbps: None,
            use_gpu: false,
        };

        let req = TranscodeRequest {
            job_id: Uuid::new_v4(),
            input: PathBuf::from("/tmp/input.mov"),
            output: PathBuf::from("/tmp/output.mp4"),
            preset,
        };

        let args = build_ffmpeg_args(&req);
        // Must never include -vf when video_codec is Copy to prevent FFmpeg error 234
        assert!(!args.contains(&"-vf".to_string()));
    }
}
