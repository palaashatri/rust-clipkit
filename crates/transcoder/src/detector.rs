use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Locates ffmpeg on the system.
///
/// Checks `custom_path` first (user override from settings), then the system PATH.
/// Returns `None` if ffmpeg cannot be found or fails a version check.
pub fn detect_ffmpeg(custom_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = custom_path {
        if path.exists() && probe_ffmpeg(path) {
            return Some(path.to_path_buf());
        }
    }

    let bin = if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let mut cmd = std::process::Command::new(bin);
    cmd.arg("-version");

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let found = cmd.output().ok().filter(|o| o.status.success()).is_some();

    if found {
        find_on_path(bin)
    } else {
        None
    }
}

fn probe_ffmpeg(path: &Path) -> bool {
    let mut cmd = std::process::Command::new(path);
    cmd.arg("-version");

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    cmd.output().map(|o| o.status.success()).unwrap_or(false)
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
