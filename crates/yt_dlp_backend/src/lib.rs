pub mod download_engine;
pub mod option_map;
pub mod self_updater;
pub mod version_manager;

pub use download_engine::{DownloadManager, DownloadRequest};
pub use self_updater::check_for_app_update;
pub use version_manager::{detect_ytdlp_version, VersionManager};
