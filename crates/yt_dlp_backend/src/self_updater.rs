use reqwest::Client;
use serde::Deserialize;
use ytdlp_core::AppUpdateInfo;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASES_API: &str = "https://api.github.com/repos/palaashatri/clipkit/releases/latest";

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
}

/// Checks GitHub for a newer ClipKit release.
/// Returns `Some(AppUpdateInfo)` if a newer version is available, `None` otherwise.
pub async fn check_for_app_update() -> Option<AppUpdateInfo> {
    let client = Client::builder()
        .user_agent("clipkit/0.1")
        .build()
        .ok()?;

    let resp = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;

    let release: GhRelease = resp.json().await.ok()?;
    let latest = release.tag_name.trim_start_matches('v');

    if is_newer(APP_VERSION, latest) {
        Some(AppUpdateInfo {
            version: release.tag_name,
            release_url: release.html_url,
        })
    } else {
        None
    }
}

/// Returns true if `latest` is a higher semver than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

fn parse_semver(v: &str) -> (u64, u64, u64) {
    let v = v.trim_start_matches('v');
    let mut parts = v.splitn(3, '.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_comparison() {
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.0", "1.0.0"));
        assert!(!is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
    }
}
