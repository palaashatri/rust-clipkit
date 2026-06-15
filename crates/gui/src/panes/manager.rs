use crate::theme;
use iced::{
    widget::{
        button, column, container, horizontal_rule, progress_bar, row, scrollable, text, Space,
    },
    Alignment, Command, Element, Length,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::AbortHandle;
use transcoder::{auto_download_supported, FfmpegManager};
use yt_dlp_backend::VersionManager;
use ytdlp_core::{AppConfig, AppEvent, YtDlpVersion};

/// Actions that the Manager pane needs the app shell to carry out.
pub enum ManagerAction {
    UseSystemYtDlp(PathBuf),
    ActivateVersion { version: String, _path: PathBuf },
}

#[derive(Debug, Clone)]
pub enum ManagerPaneMessage {
    // yt-dlp
    FetchVersions,
    DownloadVersion(YtDlpVersion),
    ActivateVersion(YtDlpVersion),
    CancelYtDlpDownload,
    UseSystemYtDlp,
    OpenBinDir,
    YtDlpVersionDetected(Option<String>),
    // ffmpeg
    AutoDownloadFfmpeg,
    DownloadLatestFfmpeg,
    CancelFfmpegDownload,
    OpenFfmpegDir,
    FfmpegVersionDetected(Option<String>),
}

pub struct ManagerPane {
    // ── yt-dlp state ─────────────────────────────────────────────────────────
    versions: Vec<YtDlpVersion>,
    loading_ytdlp: bool,
    active_version: Option<String>,
    active_path: Option<PathBuf>,
    system_path: Option<PathBuf>,
    downloading_version: Option<String>,
    ytdlp_download_progress: f32,
    ytdlp_error: Option<String>,
    bin_dir: PathBuf,
    detected_ytdlp_version: Option<String>,
    version_manager: Arc<VersionManager>,
    ytdlp_abort: Option<AbortHandle>,

    // ── ffmpeg state ──────────────────────────────────────────────────────────
    ffmpeg_path: Option<PathBuf>,
    detected_ffmpeg_version: Option<String>,
    downloading_ffmpeg: bool,
    ffmpeg_download_progress: f32,
    ffmpeg_error: Option<String>,
    ffmpeg_bin_dir: PathBuf,
    ffmpeg_manager: Arc<FfmpegManager>,
    ffmpeg_abort: Option<AbortHandle>,
}

impl ManagerPane {
    pub fn new(config: &AppConfig, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let version_manager = Arc::new(VersionManager::new(
            config.ytdlp_bin_dir.clone(),
            Arc::clone(&event_tx),
        ));
        let ffmpeg_manager = Arc::new(FfmpegManager::new(
            config.ffmpeg_bin_dir.clone(),
            Arc::clone(&event_tx),
        ));
        Self {
            versions: Vec::new(),
            loading_ytdlp: false,
            active_version: config.active_ytdlp_version.clone(),
            active_path: config.active_ytdlp_path(),
            system_path: VersionManager::detect_system_ytdlp(),
            downloading_version: None,
            ytdlp_download_progress: 0.0,
            ytdlp_error: None,
            bin_dir: config.ytdlp_bin_dir.clone(),
            detected_ytdlp_version: None,
            version_manager,
            ytdlp_abort: None,

            ffmpeg_path: config.ffmpeg_path.clone(),
            detected_ffmpeg_version: None,
            downloading_ffmpeg: false,
            ffmpeg_download_progress: 0.0,
            ffmpeg_error: None,
            ffmpeg_bin_dir: config.ffmpeg_bin_dir.clone(),
            ffmpeg_manager,
            ffmpeg_abort: None,
        }
    }

    /// Called by app.rs when ffmpeg is detected or changes.
    pub fn set_ffmpeg_detected(&mut self, path: Option<PathBuf>) {
        self.ffmpeg_path = path;
    }

    /// Returns the path to the managed ffmpeg binary if it exists on disk.
    pub fn managed_ffmpeg_path(&self) -> Option<PathBuf> {
        self.ffmpeg_manager.managed_path()
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> Command<ManagerPaneMessage> {
        match event {
            AppEvent::YtDlpVersionsFetched(result) => {
                self.loading_ytdlp = false;
                match result {
                    Ok(versions) => {
                        self.versions = versions.clone();
                        self.ytdlp_error = None;
                    }
                    Err(e) => self.ytdlp_error = Some(e.clone()),
                }
            }
            AppEvent::YtDlpBinaryDownloadProgress { version, percent } => {
                if self.downloading_version.as_deref() == Some(version.as_str()) {
                    self.ytdlp_download_progress = *percent;
                }
            }
            AppEvent::YtDlpBinaryDownloaded { version, path } => {
                self.downloading_version = None;
                self.ytdlp_download_progress = 0.0;
                self.ytdlp_abort = None;
                self.active_version = Some(version.clone());
                self.active_path = Some(path.clone());
                self.ytdlp_error = None;
                let p = path.clone();
                return Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            yt_dlp_backend::detect_ytdlp_version(&p)
                        })
                        .await
                        .ok()
                        .flatten()
                    },
                    ManagerPaneMessage::YtDlpVersionDetected,
                );
            }
            AppEvent::YtDlpBinaryDownloadFailed { error, .. } => {
                self.downloading_version = None;
                self.ytdlp_download_progress = 0.0;
                self.ytdlp_abort = None;
                self.ytdlp_error = Some(error.clone());
            }
            AppEvent::FfmpegDownloadProgress { percent } => {
                self.ffmpeg_download_progress = *percent;
            }
            AppEvent::FfmpegDownloaded { path, version } => {
                self.downloading_ffmpeg = false;
                self.ffmpeg_download_progress = 0.0;
                self.ffmpeg_abort = None;
                self.ffmpeg_path = Some(path.clone());
                self.detected_ffmpeg_version = Some(version.clone());
                self.ffmpeg_error = None;
            }
            AppEvent::FfmpegDownloadFailed { error } => {
                self.downloading_ffmpeg = false;
                self.ffmpeg_download_progress = 0.0;
                self.ffmpeg_abort = None;
                self.ffmpeg_error = Some(error.clone());
            }
            _ => {}
        }
        Command::none()
    }

    pub fn update(
        &mut self,
        msg: ManagerPaneMessage,
    ) -> (Command<ManagerPaneMessage>, Option<ManagerAction>) {
        match msg {
            // ── yt-dlp ────────────────────────────────────────────────────────
            ManagerPaneMessage::FetchVersions => {
                self.loading_ytdlp = true;
                self.ytdlp_error = None;
                let vm = Arc::clone(&self.version_manager);
                return (
                    Command::perform(
                        async move { vm.fetch_releases().await },
                        |_| ManagerPaneMessage::YtDlpVersionDetected(None), // dummy to satisfy command, event comes via broadcast
                    ),
                    None,
                );
            }

            ManagerPaneMessage::DownloadVersion(version) => {
                // Abort any in-flight yt-dlp download before starting a new one.
                if let Some(h) = self.ytdlp_abort.take() {
                    h.abort();
                }
                self.downloading_version = Some(version.tag_name.clone());
                self.ytdlp_download_progress = 0.0;
                let vm = Arc::clone(&self.version_manager);
                let handle = tokio::spawn(async move { vm.download_version(version).await });
                self.ytdlp_abort = Some(handle.abort_handle());
                // Drop the JoinHandle — task keeps running; events come via broadcast.
                drop(handle);
            }

            ManagerPaneMessage::ActivateVersion(version) => {
                let path = self.version_manager.version_path(&version.tag_name);
                self.active_version = Some(version.tag_name.clone());
                self.active_path = Some(path.clone());
                let p = path.clone();
                let ver = version.tag_name.clone();
                return (
                    Command::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                yt_dlp_backend::detect_ytdlp_version(&p)
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        ManagerPaneMessage::YtDlpVersionDetected,
                    ),
                    Some(ManagerAction::ActivateVersion {
                        version: ver,
                        _path: path,
                    }),
                );
            }

            ManagerPaneMessage::CancelYtDlpDownload => {
                if let Some(h) = self.ytdlp_abort.take() {
                    h.abort();
                }
                self.downloading_version = None;
                self.ytdlp_download_progress = 0.0;
            }

            ManagerPaneMessage::UseSystemYtDlp => {
                if let Some(path) = self.system_path.clone() {
                    self.active_path = Some(path.clone());
                    self.active_version = Some("system".to_string());
                    return (Command::none(), Some(ManagerAction::UseSystemYtDlp(path)));
                }
            }
            ManagerPaneMessage::OpenBinDir => {
                let _ = open::that(&self.bin_dir);
            }
            ManagerPaneMessage::YtDlpVersionDetected(v) => {
                self.detected_ytdlp_version = v;
            }

            // ── ffmpeg ────────────────────────────────────────────────────────
            ManagerPaneMessage::AutoDownloadFfmpeg | ManagerPaneMessage::DownloadLatestFfmpeg => {
                if !auto_download_supported() {
                    self.ffmpeg_error = Some(
                        "Auto-download is not available on this platform. \
                         Please install ffmpeg manually."
                            .to_string(),
                    );
                    return (Command::none(), None);
                }
                // Abort any in-flight ffmpeg download before starting a new one.
                if let Some(h) = self.ffmpeg_abort.take() {
                    h.abort();
                }
                self.downloading_ffmpeg = true;
                self.ffmpeg_download_progress = 0.0;
                self.ffmpeg_error = None;
                let fm = Arc::clone(&self.ffmpeg_manager);
                let handle = tokio::spawn(async move { fm.download_latest().await });
                self.ffmpeg_abort = Some(handle.abort_handle());
                drop(handle);
            }

            ManagerPaneMessage::CancelFfmpegDownload => {
                if let Some(h) = self.ffmpeg_abort.take() {
                    h.abort();
                }
                self.downloading_ffmpeg = false;
                self.ffmpeg_download_progress = 0.0;
            }

            ManagerPaneMessage::OpenFfmpegDir => {
                let _ = open::that(&self.ffmpeg_bin_dir);
            }
            ManagerPaneMessage::FfmpegVersionDetected(v) => {
                self.detected_ffmpeg_version = v;
            }
        }
        (Command::none(), None)
    }

    pub fn view(&self) -> Element<'_, ManagerPaneMessage> {
        let header = column![
            text("Dependency Manager").size(theme::size::TITLE),
            text("Manage yt-dlp and ffmpeg versions")
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::secondary())),
        ]
        .spacing(4);

        let ytdlp_section = self.view_ytdlp_section();
        let ffmpeg_section = self.view_ffmpeg_section();

        scrollable(
            column![
                header,
                Space::with_height(14),
                horizontal_rule(1).style(theme::divider),
                Space::with_height(20),
                ytdlp_section,
                Space::with_height(24),
                ffmpeg_section,
            ]
            .spacing(0)
            .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    fn view_ytdlp_section(&self) -> Element<'_, ManagerPaneMessage> {
        let (version_str, version_color) = match &self.active_version {
            Some(v) => (v.clone(), theme::color::success()),
            None => (
                "No version configured".to_string(),
                theme::color::secondary(),
            ),
        };
        let path_str = self
            .active_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "—".to_string());

        let detected_str = self
            .detected_ytdlp_version
            .as_deref()
            .unwrap_or("")
            .to_string();

        let status_dot: Element<ManagerPaneMessage> = container(Space::with_width(0))
            .width(8)
            .height(8)
            .style(if self.active_version.is_some() {
                theme::status_dot_ok
            } else {
                theme::status_dot_muted
            })
            .into();

        // Show path always; show "Reported: X" on the next line only when available.
        let reported_line: Element<ManagerPaneMessage> = if !detected_str.is_empty() {
            text(format!("Reported: {}", detected_str))
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::tertiary()))
                .into()
        } else {
            Space::with_height(0).into()
        };

        let active_card = container(
            column![
                row![
                    status_dot,
                    Space::with_width(8),
                    text("Active version")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary())),
                ]
                .align_items(Alignment::Center),
                Space::with_height(8),
                text(&version_str)
                    .size(theme::size::HEADLINE)
                    .style(iced::theme::Text::Color(version_color)),
                Space::with_height(2),
                text(&path_str)
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
                reported_line,
            ]
            .spacing(0),
        )
        .style(theme::elevated)
        .padding([16, 18])
        .width(Length::Fill);

        // Actions row
        let fetch_label = if self.loading_ytdlp {
            "Fetching…"
        } else {
            "Check for versions"
        };
        let mut fetch_btn = button(text(fetch_label).size(theme::size::FOOTNOTE))
            .padding([9, 16])
            .style(theme::button::primary());
        if !self.loading_ytdlp {
            fetch_btn = fetch_btn.on_press(ManagerPaneMessage::FetchVersions);
        }

        let sys_btn: Element<ManagerPaneMessage> = if let Some(p) = &self.system_path {
            button(text(format!("Use system engine ({})", p.display())).size(theme::size::FOOTNOTE))
                .on_press(ManagerPaneMessage::UseSystemYtDlp)
                .padding([9, 16])
                .style(theme::button::secondary())
                .into()
        } else {
            Space::with_width(0).into()
        };

        let bin_btn = button(
            text("Open bin folder")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::accent())),
        )
        .on_press(ManagerPaneMessage::OpenBinDir)
        .style(theme::button::ghost())
        .padding([9, 0]);

        let actions = row![fetch_btn, sys_btn, bin_btn]
            .spacing(10)
            .align_items(Alignment::Center);

        // Error
        let error_elem: Element<ManagerPaneMessage> = match &self.ytdlp_error {
            Some(err) => container(
                row![
                    text("⚠  ")
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::danger())),
                    text(err)
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::danger())),
                ]
                .align_items(Alignment::Center),
            )
            .style(theme::danger_banner)
            .padding([10, 14])
            .width(Length::Fill)
            .into(),
            None => Space::with_height(0).into(),
        };

        // Download progress
        let dl_progress: Element<ManagerPaneMessage> = match &self.downloading_version {
            Some(ver) => container(
                column![
                    row![
                        text(format!("Downloading {}", ver))
                            .size(theme::size::FOOTNOTE)
                            .width(Length::Fill),
                        button(text("Cancel").size(theme::size::CAPTION))
                            .on_press(ManagerPaneMessage::CancelYtDlpDownload)
                            .style(theme::button::danger())
                            .padding([4, 10]),
                        Space::with_width(10),
                        text(format!("{:.0}%", self.ytdlp_download_progress))
                            .size(theme::size::FOOTNOTE)
                            .style(iced::theme::Text::Color(theme::color::accent())),
                    ]
                    .align_items(Alignment::Center),
                    Space::with_height(8),
                    progress_bar(0.0..=100.0, self.ytdlp_download_progress)
                        .style(theme::progress())
                        .height(4)
                        .width(Length::Fill),
                ]
                .spacing(0),
            )
            .style(theme::card)
            .padding([14, 18])
            .width(Length::Fill)
            .into(),
            None => Space::with_height(0).into(),
        };

        // Releases list
        let releases_header = row![
            text("Engine Releases").size(theme::size::HEADLINE),
            Space::with_width(8),
            if !self.versions.is_empty() {
                text(format!("{} releases", self.versions.len()))
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::secondary()))
            } else {
                text("").size(theme::size::CAPTION)
            },
        ]
        .align_items(Alignment::Center);

        let releases: Element<ManagerPaneMessage> = if self.versions.is_empty() {
            let (msg, color) = if self.loading_ytdlp {
                ("Fetching releases from GitHub…", theme::color::accent())
            } else {
                (
                    "Click \"Check for versions\" to load available releases.",
                    theme::color::secondary(),
                )
            };
            container(
                text(msg)
                    .size(theme::size::BODY)
                    .style(iced::theme::Text::Color(color)),
            )
            .padding([16, 0])
            .into()
        } else {
            let col_header: Element<ManagerPaneMessage> = container(
                row![
                    text("Version")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                        .width(Length::FillPortion(3)),
                    text("Released")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                        .width(Length::FillPortion(3)),
                    text("Type")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                        .width(Length::FillPortion(2)),
                    Space::with_width(Length::FillPortion(3)),
                ]
                .spacing(8),
            )
            .padding([6, 4])
            .into();

            let mut rows: Vec<Element<ManagerPaneMessage>> = vec![col_header];
            for v in &self.versions {
                let is_active = self.active_version.as_deref() == Some(v.tag_name.as_str());
                let is_downloading =
                    self.downloading_version.as_deref() == Some(v.tag_name.as_str());
                let is_installed = self.version_manager.is_version_installed(&v.tag_name);
                rows.push(ytdlp_version_row(
                    v.tag_name.clone(),
                    v.published_at.format("%Y-%m-%d").to_string(),
                    v.is_prerelease,
                    v.clone(),
                    is_active,
                    is_downloading,
                    is_installed,
                ));
            }
            column(rows).spacing(4).width(Length::Fill).into()
        };

        column![
            text("Download Engine")
                .size(theme::size::SUBHEAD)
                .style(iced::theme::Text::Color(theme::color::secondary())),
            Space::with_height(10),
            active_card,
            Space::with_height(12),
            actions,
            Space::with_height(if self.ytdlp_error.is_some() {
                8u16
            } else {
                0u16
            }),
            error_elem,
            Space::with_height(if self.downloading_version.is_some() {
                8u16
            } else {
                0u16
            }),
            dl_progress,
            Space::with_height(16),
            releases_header,
            Space::with_height(8),
            releases,
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    }

    fn view_ffmpeg_section(&self) -> Element<'_, ManagerPaneMessage> {
        let has_ffmpeg = self.ffmpeg_path.is_some();
        let (version_label, version_color) = if has_ffmpeg {
            let v = self
                .detected_ffmpeg_version
                .as_deref()
                .unwrap_or("detected")
                .to_string();
            (v, theme::color::success())
        } else {
            ("Not found".to_string(), theme::color::secondary())
        };
        let status_dot: Element<ManagerPaneMessage> = container(Space::with_width(0))
            .width(8)
            .height(8)
            .style(if has_ffmpeg {
                theme::status_dot_ok
            } else {
                theme::status_dot_muted
            })
            .into();

        let path_str = self
            .ffmpeg_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "—".to_string());

        let ffmpeg_card = container(
            column![
                row![
                    status_dot,
                    Space::with_width(8),
                    text("ffmpeg")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary())),
                ]
                .align_items(Alignment::Center),
                Space::with_height(8),
                text(&version_label)
                    .size(theme::size::HEADLINE)
                    .style(iced::theme::Text::Color(version_color)),
                Space::with_height(4),
                text(path_str)
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
            ]
            .spacing(0),
        )
        .style(theme::elevated)
        .padding([16, 18])
        .width(Length::Fill);

        // Action buttons
        let dl_label = if self.ffmpeg_path.is_some() {
            "Update to latest ffmpeg"
        } else {
            "Download latest ffmpeg"
        };

        let dl_btn_elem: Element<ManagerPaneMessage> = if auto_download_supported() {
            if self.downloading_ffmpeg {
                button(text("Downloading…").size(theme::size::FOOTNOTE))
                    .padding([9, 16])
                    .style(theme::button::primary())
                    .into()
            } else {
                button(text(dl_label).size(theme::size::FOOTNOTE))
                    .on_press(ManagerPaneMessage::DownloadLatestFfmpeg)
                    .padding([9, 16])
                    .style(theme::button::primary())
                    .into()
            }
        } else {
            text("Install via: brew install ffmpeg  (macOS) / sudo apt install ffmpeg  (Linux)")
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::secondary()))
                .into()
        };

        let open_btn = button(
            text("Open folder")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::accent())),
        )
        .on_press(ManagerPaneMessage::OpenFfmpegDir)
        .style(theme::button::ghost())
        .padding([9, 0]);

        let actions = row![dl_btn_elem, open_btn]
            .spacing(10)
            .align_items(Alignment::Center);

        // Download progress
        let dl_progress: Element<ManagerPaneMessage> = if self.downloading_ffmpeg {
            container(
                column![
                    row![
                        text("Downloading ffmpeg…")
                            .size(theme::size::FOOTNOTE)
                            .width(Length::Fill),
                        button(text("Cancel").size(theme::size::CAPTION))
                            .on_press(ManagerPaneMessage::CancelFfmpegDownload)
                            .style(theme::button::danger())
                            .padding([4, 10]),
                        Space::with_width(10),
                        text(format!("{:.0}%", self.ffmpeg_download_progress))
                            .size(theme::size::FOOTNOTE)
                            .style(iced::theme::Text::Color(theme::color::accent())),
                    ]
                    .align_items(Alignment::Center),
                    Space::with_height(8),
                    progress_bar(0.0..=100.0, self.ffmpeg_download_progress)
                        .style(theme::progress())
                        .height(4)
                        .width(Length::Fill),
                ]
                .spacing(0),
            )
            .style(theme::card)
            .padding([14, 18])
            .width(Length::Fill)
            .into()
        } else {
            Space::with_height(0).into()
        };

        // Error
        let error_elem: Element<ManagerPaneMessage> = match &self.ffmpeg_error {
            Some(err) => container(
                row![
                    text("⚠  ")
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::danger())),
                    text(err)
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::danger())),
                ]
                .align_items(Alignment::Center),
            )
            .style(theme::danger_banner)
            .padding([10, 14])
            .width(Length::Fill)
            .into(),
            None => Space::with_height(0).into(),
        };

        column![
            text("ffmpeg")
                .size(theme::size::SUBHEAD)
                .style(iced::theme::Text::Color(theme::color::secondary())),
            Space::with_height(10),
            ffmpeg_card,
            Space::with_height(12),
            actions,
            Space::with_height(if self.downloading_ffmpeg { 8u16 } else { 0u16 }),
            dl_progress,
            Space::with_height(if self.ffmpeg_error.is_some() {
                8u16
            } else {
                0u16
            }),
            error_elem,
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    }
}

fn ytdlp_version_row(
    tag: String,
    date: String,
    is_prerelease: bool,
    version: YtDlpVersion,
    is_active: bool,
    is_downloading: bool,
    is_installed: bool,
) -> Element<'static, ManagerPaneMessage> {
    let type_elem: Element<ManagerPaneMessage> = if is_prerelease {
        text("pre-release")
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::warning()))
            .into()
    } else {
        text("stable")
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::success()))
            .into()
    };

    let action: Element<ManagerPaneMessage> = if is_active {
        row![
            text("✓")
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::success())),
            Space::with_width(4),
            text("Active")
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::success())),
        ]
        .align_items(Alignment::Center)
        .into()
    } else if is_downloading {
        text("Downloading…")
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::accent()))
            .into()
    } else if is_installed {
        // Already on disk — just switch the active pointer, no re-download needed.
        button(text("Activate").size(theme::size::CAPTION))
            .on_press(ManagerPaneMessage::ActivateVersion(version))
            .padding([5, 10])
            .style(theme::button::secondary())
            .into()
    } else {
        button(text("Download & activate").size(theme::size::CAPTION))
            .on_press(ManagerPaneMessage::DownloadVersion(version))
            .padding([5, 10])
            .style(theme::button::secondary())
            .into()
    };

    // Subtle "on disk" indicator
    let on_disk: Element<ManagerPaneMessage> = if is_installed && !is_active {
        text("↓")
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::tertiary()))
            .into()
    } else {
        Space::with_width(12).into()
    };

    container(
        row![
            row![
                on_disk,
                Space::with_width(4),
                text(tag).size(theme::size::FOOTNOTE),
            ]
            .align_items(Alignment::Center)
            .width(Length::FillPortion(3)),
            text(date)
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::secondary()))
                .width(Length::FillPortion(3)),
            container(type_elem).width(Length::FillPortion(2)),
            container(action).width(Length::FillPortion(3)),
        ]
        .spacing(8)
        .align_items(Alignment::Center),
    )
    .style(theme::card)
    .padding([10, 12])
    .width(Length::Fill)
    .into()
}
