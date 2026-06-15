use crate::option_map::build_ytdlp_args;
use ytdlp_core::{AppEvent, DownloadPreset, DownloadProgress, DownloadResult, DownloadStage};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, Semaphore};
use tokio::task::JoinHandle;
use uuid::Uuid;

static PROGRESS_RE: OnceLock<Regex> = OnceLock::new();
static DEST_RE: OnceLock<Regex> = OnceLock::new();
static MERGE_RE: OnceLock<Regex> = OnceLock::new();

fn progress_re() -> &'static Regex {
    PROGRESS_RE.get_or_init(|| {
        Regex::new(
            r"\[download\]\s+([\d.]+)%\s+of\s+~?\s*[\d.]+\S+\s+at\s+([\S]+)\s+ETA\s+([\d:]+)",
        )
        .unwrap()
    })
}

fn dest_re() -> &'static Regex {
    DEST_RE.get_or_init(|| {
        Regex::new(r"\[(?:download|ExtractAudio|ffmpeg)\]\s+Destination:\s+(.+)").unwrap()
    })
}

/// Matches "[Merger] Merging formats into '/path/file.ext'" (single or double quotes).
fn merge_re() -> &'static Regex {
    MERGE_RE.get_or_init(|| {
        Regex::new(r#"\[(?:Merger|ffmpeg)\]\s+Merging formats into\s+(?:"([^"]+)"|'([^']+)')"#)
            .unwrap()
    })
}

/// Returns true if `path` looks like a yt-dlp intermediate format file (e.g. `title.f248.webm`).
fn is_format_file(path: &std::path::Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|stem| {
            stem.rfind('.').map_or(false, |pos| {
                let suffix = &stem[pos + 1..];
                suffix.starts_with('f')
                    && suffix.len() > 1
                    && suffix[1..].chars().all(|c| c.is_ascii_digit())
            })
        })
        .unwrap_or(false)
}

const PROGRESS_THROTTLE: Duration = Duration::from_millis(250);

pub struct DownloadRequest {
    pub job_id: Uuid,
    pub url: String,
    pub output_dir: PathBuf,
    pub preset: DownloadPreset,
}

/// Manages multiple concurrent yt-dlp downloads.
pub struct DownloadManager {
    ytdlp_path: Arc<Mutex<Option<PathBuf>>>,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    active_handles: Arc<Mutex<HashMap<Uuid, JoinHandle<()>>>>,
    semaphore: Arc<Semaphore>,
}

impl DownloadManager {
    pub fn new(
        ytdlp_path: Option<PathBuf>,
        event_tx: Arc<broadcast::Sender<AppEvent>>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            ytdlp_path: Arc::new(Mutex::new(ytdlp_path)),
            event_tx,
            active_handles: Arc::new(Mutex::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    /// Updates the path to the active yt-dlp binary.
    pub fn set_ytdlp_path(&self, path: Option<PathBuf>) {
        *self.ytdlp_path.lock().unwrap() = path;
    }

    /// Starts a download. Returns immediately; progress comes through broadcast events.
    /// Respects the max-concurrent limit set at construction.
    pub fn start_download(&self, request: DownloadRequest) {
        let job_id = request.job_id;
        let ytdlp_path = self.ytdlp_path.lock().unwrap().clone();
        let event_tx = Arc::clone(&self.event_tx);
        let semaphore = Arc::clone(&self.semaphore);

        let handle = tokio::spawn(async move {
            // Wait for a concurrency slot before starting the subprocess.
            let _permit = semaphore.acquire().await.unwrap();
            run_download(request, ytdlp_path, event_tx).await;
        });

        self.active_handles.lock().unwrap().insert(job_id, handle);
    }

    /// Cancels a running download by aborting its task.
    pub fn cancel_download(&self, job_id: Uuid) {
        let handle = self.active_handles.lock().unwrap().remove(&job_id);
        if let Some(h) = handle {
            h.abort();
        }
        let _ = self.event_tx.send(AppEvent::DownloadCancelled { job_id });
    }
}

async fn run_download(
    request: DownloadRequest,
    ytdlp_path: Option<PathBuf>,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
) {
    let job_id = request.job_id;

    let ytdlp = match ytdlp_path {
        Some(p) if p.exists() => p,
        Some(p) => {
            let _ = event_tx.send(AppEvent::DownloadFailed {
                job_id,
                error: format!(
                    "yt-dlp not found at '{}'. Go to the Manager pane to download a version.",
                    p.display()
                ),
            });
            return;
        }
        None => {
            let _ = event_tx.send(AppEvent::DownloadFailed {
                job_id,
                error: "yt-dlp is not configured. Go to the Manager pane to download a version."
                    .to_string(),
            });
            return;
        }
    };

    tokio::fs::create_dir_all(&request.output_dir)
        .await
        .ok();

    let args = build_ytdlp_args(&request.url, &request.preset, &request.output_dir);

    let mut cmd = tokio::process::Command::new(&ytdlp);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = event_tx.send(AppEvent::DownloadFailed {
                job_id,
                error: format!("Failed to launch yt-dlp: {}", e),
            });
            return;
        }
    };

    // Merge stdout and stderr into our line reader (yt-dlp writes progress to stdout or stderr)
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    
    // Process output asynchronously
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);
    
    let tx_out = tx.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_out.send(line).await;
        }
    });

    let tx_err = tx;
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_err.send(line).await;
        }
    });

    let mut last_progress_emit: Option<Instant> = None;

    while let Some(line) = rx.recv().await {
        // Progress line: [download]  55.3% of 123.45MiB at 2.34MiB/s ETA 00:45
        if let Some(caps) = progress_re().captures(&line) {
            let now = Instant::now();
            if last_progress_emit.map_or(true, |t| now.duration_since(t) >= PROGRESS_THROTTLE) {
                let percent: f32 = caps[1].parse().unwrap_or(0.0);
                let speed = caps.get(2).map(|m| m.as_str().to_string());
                let eta = caps.get(3).map(|m| m.as_str().to_string());
                let _ = event_tx.send(AppEvent::DownloadProgress {
                    job_id,
                    progress: DownloadProgress {
                        percent,
                        speed,
                        eta,
                        stage: DownloadStage::Downloading,
                        message: line.clone(),
                    },
                });
                last_progress_emit = Some(now);
            }
        } else if line.contains("[Merger]") || line.contains("[ffmpeg]") {
            // Capture the final merged output path from "Merging formats into '...'" lines.
            if let Some(caps) = merge_re().captures(&line) {
                let path_str = caps
                    .get(1)
                    .or_else(|| caps.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let _ = event_tx
                            .send(AppEvent::DownloadTitle { job_id, title: stem.to_string() });
                    }
                    let _ = event_tx.send(AppEvent::DownloadOutputPath { job_id, path });
                }
            } else if let Some(caps) = dest_re().captures(&line) {
                // "[ffmpeg] Destination:" lines (format conversion final output).
                let path = PathBuf::from(caps[1].trim());
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let _ = event_tx
                        .send(AppEvent::DownloadTitle { job_id, title: stem.to_string() });
                }
                let _ = event_tx.send(AppEvent::DownloadOutputPath { job_id, path });
            }
            let _ = event_tx.send(AppEvent::DownloadProgress {
                job_id,
                progress: DownloadProgress {
                    percent: 99.0,
                    speed: None,
                    eta: None,
                    stage: DownloadStage::Merging,
                    message: "Merging streams…".to_string(),
                },
            });
        } else if let Some(caps) = dest_re().captures(&line) {
            let path = PathBuf::from(caps[1].trim());
            // Only extract a title from non-intermediate-format files. yt-dlp names
            // intermediate streams "title.f248.webm"; we skip those and wait for the
            // merged output path (handled above) for multi-stream downloads.
            if !is_format_file(&path) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let _ = event_tx
                        .send(AppEvent::DownloadTitle { job_id, title: stem.to_string() });
                }
            }
            let _ = event_tx.send(AppEvent::DownloadOutputPath { job_id, path });
        }
    }

    let status = child.wait().await;
    match status {
        Ok(s) if s.success() => {
            let _ = event_tx.send(AppEvent::DownloadCompleted {
                job_id,
                result: DownloadResult {
                    output_path: request.output_dir.clone(),
                    title: None,
                },
            });
        }
        Ok(s) => {
            // Check if it was cancelled (task abort shows up as a non-zero exit)
            let _ = event_tx.send(AppEvent::DownloadFailed {
                job_id,
                error: format!("yt-dlp exited with status {}", s),
            });
        }
        Err(e) => {
            let _ = event_tx.send(AppEvent::DownloadFailed {
                job_id,
                error: format!("Error waiting for yt-dlp: {}", e),
            });
        }
    }
}
