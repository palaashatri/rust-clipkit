use crate::theme;
use iced::{
    widget::{
        button, checkbox, column, container, horizontal_rule, pick_list, progress_bar, row,
        scrollable, svg, text, text_input, Space,
    },
    Alignment, Command, Element, Length,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use yt_dlp_backend::{DownloadManager, DownloadRequest};
use ytdlp_core::{
    AppConfig, AppEvent, AudioFormat, ContentType, DownloadJob, DownloadPreset, DownloadStatus,
    VideoContainer, VideoQuality,
};

#[derive(Debug, Clone)]
pub enum DownloadPaneMessage {
    UrlChanged(String),
    ContentTypeSelected(ContentType),
    QualitySelected(VideoQuality),
    ContainerSelected(VideoContainer),
    AudioFormatSelected(AudioFormat),
    AdvancedToggled,
    EmbedMetadataToggled(bool),
    EmbedThumbnailToggled(bool),
    SponsorBlockToggled(bool),
    OutputTemplateChanged(String),
    SubtitleDownloadToggled(bool),
    SubtitleEmbedToggled(bool),
    SubtitleLanguagesChanged(String),
    SubtitleAutoGenToggled(bool),
    PickOutputDir,
    OutputDirPicked(Option<PathBuf>),
    PreferHdrToggled(bool),
    StartDownload,
    CancelDownload(Uuid),
    OpenOutputFolder(Uuid),
    OpenOutputFile(Uuid),
}

pub struct DownloadPane {
    url: String,
    content_type: ContentType,
    quality: VideoQuality,
    container: VideoContainer,
    audio_format: AudioFormat,
    output_dir: PathBuf,
    advanced_open: bool,
    embed_metadata: bool,
    embed_thumbnail: bool,
    sponsorblock: bool,
    output_template: String,
    subtitle_download: bool,
    subtitle_embed: bool,
    subtitle_languages: String,
    subtitle_auto_gen: bool,
    prefer_hdr: bool,
    jobs: Vec<DownloadJob>,
    manager: Arc<DownloadManager>,
}

impl DownloadPane {
    pub fn new(config: &AppConfig, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let manager = Arc::new(DownloadManager::new(
            config.active_ytdlp_path(),
            event_tx,
            config.max_concurrent_downloads,
        ));
        Self {
            url: String::new(),
            content_type: ContentType::VideoAudio,
            quality: VideoQuality::Best,
            container: VideoContainer::Mp4,
            audio_format: AudioFormat::Best,
            output_dir: config.default_download_dir.clone(),
            advanced_open: false,
            embed_metadata: true,
            embed_thumbnail: false,
            sponsorblock: false,
            output_template: "%(title)s.%(ext)s".to_string(),
            subtitle_download: false,
            subtitle_embed: false,
            subtitle_languages: "en".to_string(),
            subtitle_auto_gen: false,
            prefer_hdr: true,
            jobs: Vec::new(),
            manager,
        }
    }

    pub fn set_ytdlp_path(&self, path: Option<PathBuf>) {
        self.manager.set_ytdlp_path(path);
    }

    #[allow(dead_code)]
    pub fn set_advanced_open(&mut self, open: bool) {
        self.advanced_open = open;
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> Command<DownloadPaneMessage> {
        match event {
            AppEvent::DownloadProgress { job_id, progress } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = DownloadStatus::Downloading;
                    job.progress = progress.percent;
                    job.speed = progress.speed.clone();
                    job.eta = progress.eta.clone();
                }
            }
            AppEvent::DownloadTitle { job_id, title } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.title = Some(title.clone());
                }
            }
            AppEvent::DownloadOutputPath { job_id, path } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.output_path = Some(path.clone());
                }
            }
            AppEvent::DownloadCompleted { job_id, .. } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = DownloadStatus::Completed;
                    job.progress = 100.0;
                }
            }
            AppEvent::DownloadFailed { job_id, error } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = DownloadStatus::Failed;
                    job.error_detail = Some(error.clone());
                }
            }
            AppEvent::DownloadCancelled { job_id } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = DownloadStatus::Cancelled;
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn update(&mut self, msg: DownloadPaneMessage) -> Command<DownloadPaneMessage> {
        match msg {
            DownloadPaneMessage::UrlChanged(s) => self.url = s,
            DownloadPaneMessage::ContentTypeSelected(ct) => self.content_type = ct,
            DownloadPaneMessage::QualitySelected(q) => self.quality = q,
            DownloadPaneMessage::ContainerSelected(c) => self.container = c,
            DownloadPaneMessage::AudioFormatSelected(f) => self.audio_format = f,
            DownloadPaneMessage::AdvancedToggled => self.advanced_open = !self.advanced_open,
            DownloadPaneMessage::EmbedMetadataToggled(b) => self.embed_metadata = b,
            DownloadPaneMessage::EmbedThumbnailToggled(b) => self.embed_thumbnail = b,
            DownloadPaneMessage::SponsorBlockToggled(b) => self.sponsorblock = b,
            DownloadPaneMessage::OutputTemplateChanged(s) => self.output_template = s,
            DownloadPaneMessage::SubtitleDownloadToggled(b) => self.subtitle_download = b,
            DownloadPaneMessage::SubtitleEmbedToggled(b) => self.subtitle_embed = b,
            DownloadPaneMessage::SubtitleLanguagesChanged(s) => self.subtitle_languages = s,
            DownloadPaneMessage::SubtitleAutoGenToggled(b) => self.subtitle_auto_gen = b,
            DownloadPaneMessage::PreferHdrToggled(b) => self.prefer_hdr = b,

            DownloadPaneMessage::PickOutputDir => {
                let current = self.output_dir.clone();
                return Command::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Choose download folder")
                            .set_directory(&current)
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    DownloadPaneMessage::OutputDirPicked,
                );
            }
            DownloadPaneMessage::OutputDirPicked(path) => {
                if let Some(p) = path {
                    self.output_dir = p;
                }
            }

            DownloadPaneMessage::StartDownload => {
                let url = self.url.trim().to_string();
                if url.is_empty() {
                    return Command::none();
                }
                let preset = DownloadPreset {
                    content_type: self.content_type.clone(),
                    video_quality: self.quality.clone(),
                    video_container: self.container.clone(),
                    audio_format: self.audio_format.clone(),
                    embed_metadata: self.embed_metadata,
                    embed_thumbnail: self.embed_thumbnail,
                    sponsorblock: ytdlp_core::SponsorBlockOptions {
                        enabled: self.sponsorblock,
                        ..Default::default()
                    },
                    output_template: self.output_template.clone(),
                    subtitles: ytdlp_core::SubtitleOptions {
                        download: self.subtitle_download,
                        embed: self.subtitle_embed,
                        auto_generated: self.subtitle_auto_gen,
                        languages: self
                            .subtitle_languages
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    },
                    prefer_hdr: self.prefer_hdr,
                    ..Default::default()
                };

                let job = DownloadJob::new(url.clone());
                let job_id = job.id;

                // Prune oldest terminal jobs when at capacity to bound memory usage.
                const MAX_JOBS: usize = 200;
                if self.jobs.len() >= MAX_JOBS {
                    let excess = self.jobs.len() - MAX_JOBS + 1;
                    let mut pruned = 0usize;
                    self.jobs.retain(|j| {
                        if pruned < excess
                            && matches!(
                                j.status,
                                DownloadStatus::Completed
                                    | DownloadStatus::Failed
                                    | DownloadStatus::Cancelled
                            )
                        {
                            pruned += 1;
                            false
                        } else {
                            true
                        }
                    });
                }

                self.jobs.push(job);

                self.manager.start_download(DownloadRequest {
                    job_id,
                    url,
                    output_dir: self.output_dir.clone(),
                    preset,
                });
                self.url.clear();
            }

            DownloadPaneMessage::CancelDownload(id) => {
                self.manager.cancel_download(id);
            }

            DownloadPaneMessage::OpenOutputFolder(id) => {
                if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                    let dir = job
                        .output_path
                        .as_ref()
                        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                        .unwrap_or_else(|| self.output_dir.clone());
                    let _ = open::that(dir);
                }
            }

            DownloadPaneMessage::OpenOutputFile(id) => {
                if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                    if let Some(path) = &job.output_path {
                        let _ = open::that(path);
                    }
                }
            }
        }
        Command::none()
    }

    pub fn view(&self, is_dark: bool) -> Element<'_, DownloadPaneMessage> {
        // ── Page header ──────────────────────────────────────────────────────
        let header = column![
            text("Download").size(32),
            Space::with_height(6),
            text("Paste a URL to download video, audio, or subtitles")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::secondary())),
        ]
        .spacing(0);

        // ── URL input + Download button ───────────────────────────────────────
        let url_bar = row![
            text_input("https://video-site.com/watch?v=…", &self.url)
                .on_input(DownloadPaneMessage::UrlChanged)
                .on_submit(DownloadPaneMessage::StartDownload)
                .style(theme::input())
                .padding([15, 18])
                .size(theme::size::BODY)
                .width(Length::Fill),
            button(text("Download").size(theme::size::SUBHEAD))
                .on_press(DownloadPaneMessage::StartDownload)
                .padding([15, 30])
                .style(theme::button::primary()),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        // ── Quick options row ─────────────────────────────────────────────────
        let content_types = vec![
            ContentType::VideoAudio,
            ContentType::AudioOnly,
            ContentType::SubtitlesOnly,
        ];
        let dir_display = truncate_path(&self.output_dir.display().to_string(), 42);

        let label = |s: &'static str| {
            text(s)
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::secondary()))
        };

        let options_row = row![
            label("Save as:"),
            Space::with_width(4),
            pick_list(
                content_types,
                Some(self.content_type.clone()),
                DownloadPaneMessage::ContentTypeSelected,
            )
            .text_size(theme::size::FOOTNOTE)
            .padding([10, 14]),
            Space::with_width(28),
            label("To:"),
            Space::with_width(4),
            text(dir_display)
                .size(theme::size::FOOTNOTE)
                .width(Length::Fill),
            button(text("Change").size(theme::size::FOOTNOTE))
                .on_press(DownloadPaneMessage::PickOutputDir)
                .padding([10, 18])
                .style(theme::button::secondary()),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        // ── Advanced toggle (pink bullet + WHITE text per reference) ──────────
        let adv_label = if self.advanced_open {
            "Hide advanced options"
        } else {
            "Advanced options"
        };
        let advanced_toggle = button(
            row![
                text("●")
                    .size(10)
                    .style(iced::theme::Text::Color(theme::color::accent())),
                Space::with_width(9),
                text(adv_label).size(theme::size::FOOTNOTE),
            ]
            .align_items(Alignment::Center),
        )
        .on_press(DownloadPaneMessage::AdvancedToggled)
        .style(theme::button::ghost())
        .padding([4, 0]);

        // ── Advanced panel ────────────────────────────────────────────────────
        let advanced_panel: Element<DownloadPaneMessage> = if self.advanced_open {
            let qualities = VideoQuality::all_variants();
            let containers = VideoContainer::all_variants();
            let audio_formats = AudioFormat::all_variants();

            // Audio format row — only shown for AudioOnly mode
            let audio_format_row: Element<DownloadPaneMessage> =
                if self.content_type == ContentType::AudioOnly {
                    row![
                        text("Audio format:").size(theme::size::FOOTNOTE).width(110),
                        pick_list(
                            audio_formats,
                            Some(self.audio_format.clone()),
                            DownloadPaneMessage::AudioFormatSelected,
                        )
                        .text_size(theme::size::FOOTNOTE)
                        .padding([7, 10]),
                    ]
                    .spacing(8)
                    .align_items(Alignment::Center)
                    .into()
                } else {
                    Space::with_height(0).into()
                };

            let hdr_toggle: Element<DownloadPaneMessage> = if self.container.supports_hdr() {
                checkbox("Prefer HDR", self.prefer_hdr)
                    .on_toggle(DownloadPaneMessage::PreferHdrToggled)
                    .text_size(theme::size::FOOTNOTE)
                    .into()
            } else {
                Space::with_width(0).into()
            };

            let inner = column![
                row![
                    text("Quality:").size(theme::size::FOOTNOTE).width(90),
                    pick_list(
                        qualities,
                        Some(self.quality.clone()),
                        DownloadPaneMessage::QualitySelected,
                    )
                    .text_size(theme::size::FOOTNOTE)
                    .padding([7, 10]),
                    Space::with_width(20),
                    text("Container:").size(theme::size::FOOTNOTE).width(80),
                    pick_list(
                        containers,
                        Some(self.container.clone()),
                        DownloadPaneMessage::ContainerSelected,
                    )
                    .text_size(theme::size::FOOTNOTE)
                    .padding([7, 10]),
                    Space::with_width(20),
                    hdr_toggle,
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                Space::with_height(10),
                audio_format_row,
                Space::with_height(if self.content_type == ContentType::AudioOnly {
                    10u16
                } else {
                    0u16
                }),
                row![
                    text("Output template:")
                        .size(theme::size::FOOTNOTE)
                        .width(130),
                    text_input("%(title)s.%(ext)s", &self.output_template)
                        .on_input(DownloadPaneMessage::OutputTemplateChanged)
                        .style(theme::input())
                        .padding([7, 10])
                        .size(theme::size::FOOTNOTE)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
                Space::with_height(10),
                row![
                    checkbox("Embed metadata", self.embed_metadata)
                        .on_toggle(DownloadPaneMessage::EmbedMetadataToggled)
                        .text_size(theme::size::FOOTNOTE),
                    Space::with_width(20),
                    checkbox("Embed thumbnail", self.embed_thumbnail)
                        .on_toggle(DownloadPaneMessage::EmbedThumbnailToggled)
                        .text_size(theme::size::FOOTNOTE),
                    Space::with_width(20),
                    checkbox("SponsorBlock", self.sponsorblock)
                        .on_toggle(DownloadPaneMessage::SponsorBlockToggled)
                        .text_size(theme::size::FOOTNOTE),
                ]
                .spacing(0)
                .align_items(Alignment::Center),
                Space::with_height(12),
                // ── Subtitles section ──────────────────────────────────────
                text("Subtitles")
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::secondary())),
                Space::with_height(8),
                row![
                    checkbox("Download subtitles", self.subtitle_download)
                        .on_toggle(DownloadPaneMessage::SubtitleDownloadToggled)
                        .text_size(theme::size::FOOTNOTE),
                    Space::with_width(20),
                    checkbox("Embed in file", self.subtitle_embed)
                        .on_toggle(DownloadPaneMessage::SubtitleEmbedToggled)
                        .text_size(theme::size::FOOTNOTE),
                    Space::with_width(20),
                    checkbox("Include auto-generated", self.subtitle_auto_gen)
                        .on_toggle(DownloadPaneMessage::SubtitleAutoGenToggled)
                        .text_size(theme::size::FOOTNOTE),
                ]
                .spacing(0)
                .align_items(Alignment::Center),
                Space::with_height(8),
                row![
                    text("Languages (comma-separated):")
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                        .width(220),
                    text_input("en, fr, de", &self.subtitle_languages)
                        .on_input(DownloadPaneMessage::SubtitleLanguagesChanged)
                        .style(theme::input())
                        .padding([7, 10])
                        .size(theme::size::FOOTNOTE)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_items(Alignment::Center),
            ]
            .spacing(0);

            container(inner)
                .style(theme::tinted_panel)
                .padding([16, 18])
                .width(Length::Fill)
                .into()
        } else {
            Space::with_height(0).into()
        };

        // ── Queue section ─────────────────────────────────────────────────────
        let active_count = self
            .jobs
            .iter()
            .filter(|j| {
                matches!(
                    j.status,
                    DownloadStatus::Downloading | DownloadStatus::Pending
                )
            })
            .count();

        let count_str = if active_count > 0 {
            format!("{} active", active_count)
        } else {
            format!("{} items", self.jobs.len())
        };
        let queue_header = row![
            text("Queue").size(theme::size::HEADLINE),
            text(format!("  ·  {}", count_str))
                .size(theme::size::SUBHEAD)
                .style(iced::theme::Text::Color(theme::color::secondary())),
        ]
        .align_items(Alignment::Center);

        let queue: Element<DownloadPaneMessage> = if self.jobs.is_empty() {
            let theme_for_svg = if is_dark {
                iced::Theme::Dark
            } else {
                iced::Theme::Light
            };
            container(
                column![
                    Space::with_height(Length::FillPortion(1)),
                    svg(theme::empty_download_svg_for(&theme_for_svg))
                        .width(220)
                        .height(140),
                    Space::with_height(18),
                    text("No downloads yet").size(24),
                    Space::with_height(8),
                    text("Paste a URL above to start downloading.")
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::tertiary())),
                    Space::with_height(Length::FillPortion(1)),
                ]
                .spacing(0)
                .width(Length::Fill)
                .align_items(Alignment::Center),
            )
            .center_x()
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([24, 24])
            .style(theme::empty_state)
            .into()
        } else {
            let rows: Vec<Element<DownloadPaneMessage>> =
                self.jobs.iter().rev().map(job_row).collect();
            scrollable(column(rows).spacing(8).width(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        column![
            header,
            Space::with_height(20),
            horizontal_rule(1).style(theme::divider),
            Space::with_height(22),
            url_bar,
            Space::with_height(16),
            options_row,
            Space::with_height(18),
            advanced_toggle,
            Space::with_height(if self.advanced_open { 12u16 } else { 0u16 }),
            advanced_panel,
            Space::with_height(18),
            horizontal_rule(1).style(theme::divider),
            Space::with_height(20),
            queue_header,
            Space::with_height(12),
            queue,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn job_row(job: &DownloadJob) -> Element<'_, DownloadPaneMessage> {
    let title = job
        .title
        .clone()
        .unwrap_or_else(|| truncate_url(&job.url, 70));

    let (status_label, chip_kind) = match job.status {
        DownloadStatus::Completed => ("Completed", theme::ChipKind::Success),
        DownloadStatus::Failed => ("Failed", theme::ChipKind::Danger),
        DownloadStatus::Cancelled => ("Cancelled", theme::ChipKind::Neutral),
        DownloadStatus::Downloading => ("Downloading", theme::ChipKind::Accent),
        DownloadStatus::Processing => ("Processing", theme::ChipKind::Accent),
        DownloadStatus::Pending => ("Pending", theme::ChipKind::Neutral),
    };

    let speed_eta = match (&job.speed, &job.eta) {
        (Some(sp), Some(eta)) => format!("{} · ETA {}", sp, eta),
        (Some(sp), None) => sp.clone(),
        _ => String::new(),
    };

    let progress_bar_elem: Element<DownloadPaneMessage> = if matches!(
        job.status,
        DownloadStatus::Downloading | DownloadStatus::Processing
    ) {
        progress_bar(0.0..=100.0, job.progress)
            .style(theme::progress())
            .height(4)
            .width(Length::Fill)
            .into()
    } else {
        Space::with_height(4).into()
    };

    let action: Element<DownloadPaneMessage> = match job.status {
        DownloadStatus::Completed => {
            let has_file = job.output_path.is_some();
            row![
                if has_file {
                    button(text("Open file").size(theme::size::CAPTION))
                        .on_press(DownloadPaneMessage::OpenOutputFile(job.id))
                        .style(theme::button::primary())
                        .padding([5, 10])
                } else {
                    button(text("Open file").size(theme::size::CAPTION))
                        .style(theme::button::primary())
                        .padding([5, 10])
                },
                Space::with_width(6),
                button(text("Open folder").size(theme::size::CAPTION))
                    .on_press(DownloadPaneMessage::OpenOutputFolder(job.id))
                    .style(theme::button::secondary())
                    .padding([5, 10]),
            ]
            .align_items(Alignment::Center)
            .into()
        }
        DownloadStatus::Downloading | DownloadStatus::Pending => {
            button(text("Cancel").size(theme::size::CAPTION))
                .on_press(DownloadPaneMessage::CancelDownload(job.id))
                .style(theme::button::danger())
                .padding([5, 10])
                .into()
        }
        _ => Space::with_width(0).into(),
    };

    let error_elem: Element<DownloadPaneMessage> = match &job.error_detail {
        Some(err) => text(format!("Error: {}", err))
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::danger()))
            .into(),
        None => Space::with_height(0).into(),
    };

    let url_display = truncate_url(&job.url, 80);

    let status_chip = container(text(status_label).size(theme::size::CAPTION))
        .style(theme::chip(chip_kind))
        .padding([3, 9]);

    container(
        column![
            row![
                column![
                    text(title).size(theme::size::FOOTNOTE).width(Length::Fill),
                    Space::with_height(4),
                    text(url_display)
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::tertiary()))
                        .width(Length::Fill),
                ]
                .spacing(0)
                .width(Length::Fill),
                Space::with_width(10),
                column![status_chip, Space::with_height(6), action,].align_items(Alignment::End),
            ]
            .align_items(Alignment::Center),
            Space::with_height(8),
            progress_bar_elem,
            Space::with_height(4),
            row![
                text(speed_eta)
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::secondary()))
                    .width(Length::Fill),
                text(format!("{:.0}%", job.progress))
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::secondary())),
            ],
            error_elem,
        ]
        .spacing(0)
        .width(Length::Fill),
    )
    .style(theme::card)
    .padding([14, 18])
    .width(Length::Fill)
    .into()
}

fn truncate_url(url: &str, max: usize) -> String {
    if url.chars().count() <= max {
        url.to_string()
    } else {
        let s: String = url.chars().take(max).collect();
        format!("{}…", s)
    }
}

fn truncate_path(path: &str, max: usize) -> String {
    let char_count = path.chars().count();
    if char_count <= max {
        path.to_string()
    } else {
        let skip = char_count - max;
        let byte_offset = path
            .char_indices()
            .nth(skip)
            .map(|(i, _)| i)
            .unwrap_or(path.len());
        format!("…{}", &path[byte_offset..])
    }
}
