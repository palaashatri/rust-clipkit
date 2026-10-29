use crate::theme;
use iced::{
    widget::{
        button, checkbox, column, container, horizontal_rule, pick_list, progress_bar, row,
        scrollable, slider, svg, text, Space,
    },
    Alignment, Command, Element, Length,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use transcoder::{TranscodeManager, TranscodeRequest};
use uuid::Uuid;
use ytdlp_core::{
    AppConfig, AppEvent, AudioCodec, ContainerFormat, EncodingPreset, HdrPolicy, Resolution,
    TranscodeJob, TranscodePreset, TranscodeStatus, VideoCodec,
};

#[derive(Debug, Clone)]
pub enum TranscoderPaneMessage {
    PickInputFile,
    InputFilePicked(Option<PathBuf>),
    PickOutputDir,
    OutputDirPicked(Option<PathBuf>),
    PresetSelected(TranscodePreset),
    ContainerSelected(ContainerFormat),
    VideoCodecSelected(VideoCodec),
    AudioCodecSelected(ytdlp_core::AudioCodec),
    ResolutionSelected(Resolution),
    CrfChanged(u32),
    SpeedSelected(ytdlp_core::EncodingPreset),
    HdrPolicyToggled(bool),
    CommandPreviewToggled,
    AdvancedSettingsToggled,
    StartTranscode,
    CancelTranscode(Uuid),
    OpenOutputFile(Uuid),
}

pub struct TranscoderPane {
    input_file: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    selected_preset: TranscodePreset,
    presets: Vec<TranscodePreset>,
    jobs: Vec<TranscodeJob>,
    ffmpeg_available: bool,
    ffmpeg_path: Option<PathBuf>,
    command_preview_open: bool,
    advanced_open: bool,
    manager: Arc<TranscodeManager>,
}

impl TranscoderPane {
    pub fn new(config: &AppConfig, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        let manager = Arc::new(TranscodeManager::new(
            config.ffmpeg_path.clone(),
            event_tx,
            config.max_concurrent_transcodes,
        ));
        let presets = TranscodePreset::builtin_presets();
        let selected_preset = presets[0].clone();
        Self {
            input_file: None,
            output_dir: None,
            selected_preset,
            presets,
            jobs: Vec::new(),
            ffmpeg_available: false,
            ffmpeg_path: config.ffmpeg_path.clone(),
            command_preview_open: false,
            advanced_open: false,
            manager,
        }
    }

    pub fn set_ffmpeg_path(&self, path: Option<PathBuf>) {
        self.manager.set_ffmpeg_path(path.clone());
    }

    pub fn set_ffmpeg_detected(&mut self, path: Option<PathBuf>) {
        self.ffmpeg_available = path.is_some();
        self.ffmpeg_path = path.clone();
        self.manager.set_ffmpeg_path(path);
    }

    #[allow(dead_code)]
    pub fn set_advanced_open(&mut self, open: bool) {
        self.advanced_open = open;
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> Command<TranscoderPaneMessage> {
        match event {
            AppEvent::TranscodeProgress { job_id, progress } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = TranscodeStatus::Running;
                    job.progress = progress.percent;
                    job.fps = progress.fps;
                    job.eta = progress.eta;
                }
            }
            AppEvent::TranscodeCompleted { job_id, result } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = TranscodeStatus::Completed;
                    job.progress = 100.0;
                    job.output = result.output_path.clone();
                }
            }
            AppEvent::TranscodeFailed { job_id, error } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = TranscodeStatus::Failed;
                    job.error_detail = Some(error.clone());
                }
            }
            AppEvent::TranscodeCancelled { job_id } => {
                if let Some(job) = self.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = TranscodeStatus::Cancelled;
                }
            }
            _ => {}
        }
        Command::none()
    }

    pub fn update(&mut self, msg: TranscoderPaneMessage) -> Command<TranscoderPaneMessage> {
        match msg {
            TranscoderPaneMessage::PickInputFile => {
                let current = self
                    .input_file
                    .as_ref()
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()));
                return Command::perform(
                    async move {
                        let mut dlg =
                            rfd::AsyncFileDialog::new().set_title("Select video or audio file");
                        if let Some(dir) = current {
                            dlg = dlg.set_directory(dir);
                        }
                        dlg.add_filter(
                            "Video/Audio",
                            &[
                                "mp4", "mkv", "webm", "mov", "avi", "mp3", "flac", "aac", "ogg",
                            ],
                        )
                        .pick_file()
                        .await
                        .map(|h| h.path().to_path_buf())
                    },
                    TranscoderPaneMessage::InputFilePicked,
                );
            }
            TranscoderPaneMessage::InputFilePicked(path) => {
                self.input_file = path;
            }

            TranscoderPaneMessage::PickOutputDir => {
                let current = self.output_dir.clone().or_else(|| {
                    self.input_file
                        .as_ref()
                        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                });
                return Command::perform(
                    async move {
                        let mut dlg = rfd::AsyncFileDialog::new().set_title("Choose output folder");
                        if let Some(dir) = current {
                            dlg = dlg.set_directory(dir);
                        }
                        dlg.pick_folder().await.map(|h| h.path().to_path_buf())
                    },
                    TranscoderPaneMessage::OutputDirPicked,
                );
            }
            TranscoderPaneMessage::OutputDirPicked(path) => {
                self.output_dir = path;
            }

            TranscoderPaneMessage::PresetSelected(preset) => {
                self.selected_preset = preset;
            }
            TranscoderPaneMessage::ContainerSelected(c) => {
                self.selected_preset.container = c;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::VideoCodecSelected(v) => {
                if matches!(v, VideoCodec::Copy) {
                    self.selected_preset.resolution = Resolution::Source;
                    self.selected_preset.hdr_policy = HdrPolicy::Preserve;
                }
                self.selected_preset.video_codec = v;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::AudioCodecSelected(a) => {
                self.selected_preset.audio_codec = a;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::ResolutionSelected(r) => {
                if !matches!(r, Resolution::Source)
                    && matches!(self.selected_preset.video_codec, VideoCodec::Copy)
                {
                    self.selected_preset.video_codec = VideoCodec::H264;
                }
                self.selected_preset.resolution = r;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::CrfChanged(val) => {
                self.selected_preset.crf = val;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::SpeedSelected(s) => {
                self.selected_preset.speed = s;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::HdrPolicyToggled(b) => {
                let policy = if b {
                    HdrPolicy::Preserve
                } else {
                    HdrPolicy::ToneMapToSdr
                };
                if matches!(policy, HdrPolicy::ToneMapToSdr)
                    && matches!(self.selected_preset.video_codec, VideoCodec::Copy)
                {
                    self.selected_preset.video_codec = VideoCodec::H264;
                }
                self.selected_preset.hdr_policy = policy;
                self.selected_preset.name = "Custom".to_string();
            }
            TranscoderPaneMessage::CommandPreviewToggled => {
                self.command_preview_open = !self.command_preview_open;
            }
            TranscoderPaneMessage::AdvancedSettingsToggled => {
                self.advanced_open = !self.advanced_open;
            }
            TranscoderPaneMessage::StartTranscode => {
                let input = match &self.input_file {
                    Some(p) => p.clone(),
                    None => return Command::none(),
                };

                let ext = match &self.selected_preset.container {
                    ContainerFormat::Mp4 => "mp4",
                    ContainerFormat::Mkv => "mkv",
                    ContainerFormat::WebM => "webm",
                    ContainerFormat::Mov => "mov",
                    ContainerFormat::Avi => "avi",
                    ContainerFormat::Flv => "flv",
                    ContainerFormat::Mp3 => "mp3",
                    ContainerFormat::Aac => "aac",
                    ContainerFormat::Flac => "flac",
                    ContainerFormat::Ogg => "ogg",
                    ContainerFormat::Wav => "wav",
                    ContainerFormat::Wma => "wma",
                    ContainerFormat::M4a => "m4a",
                    ContainerFormat::ThreeGp => "3gp",
                    ContainerFormat::Ts => "ts",
                    ContainerFormat::Vob => "vob",
                    ContainerFormat::Gif => "gif",
                };

                let stem = input
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "output".to_string());

                let base_dir = self.output_dir.clone().unwrap_or_else(|| {
                    input
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."))
                });
                let mut output = base_dir.join(format!("{}_transcoded.{}", stem, ext));
                if output == input {
                    output = base_dir.join(format!("{}_transcoded_out.{}", stem, ext));
                }

                // Prune oldest terminal jobs when at capacity.
                const MAX_JOBS: usize = 200;
                if self.jobs.len() >= MAX_JOBS {
                    let excess = self.jobs.len() - MAX_JOBS + 1;
                    let mut pruned = 0usize;
                    self.jobs.retain(|j| {
                        if pruned < excess
                            && matches!(
                                j.status,
                                TranscodeStatus::Completed
                                    | TranscodeStatus::Failed
                                    | TranscodeStatus::Cancelled
                            )
                        {
                            pruned += 1;
                            false
                        } else {
                            true
                        }
                    });
                }

                let mut job = TranscodeJob::new(input.clone(), output.clone());
                job.preset = Some(self.selected_preset.clone());
                let job_id = job.id;
                self.jobs.push(job);

                self.manager.start_transcode(TranscodeRequest {
                    job_id,
                    input,
                    output,
                    preset: self.selected_preset.clone(),
                });
            }
            TranscoderPaneMessage::CancelTranscode(id) => {
                self.manager.cancel_transcode(id);
            }
            TranscoderPaneMessage::OpenOutputFile(id) => {
                if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                    let _ = open::that(&job.output);
                }
            }
        }
        Command::none()
    }

    pub fn view(&self, is_dark: bool) -> Element<'_, TranscoderPaneMessage> {
        // ── Page header ───────────────────────────────────────────────────────
        let header = column![
            text("Transcoder").size(32),
            Space::with_height(6),
            text("Convert video and audio files with HDR-aware presets")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::secondary())),
        ]
        .spacing(0);

        // ── ffmpeg warning ────────────────────────────────────────────────────
        let ffmpeg_banner: Element<TranscoderPaneMessage> = if !self.ffmpeg_available {
            container(
                row![
                    text("⚠")
                        .size(theme::size::BODY)
                        .style(iced::theme::Text::Color(theme::color::warning())),
                    Space::with_width(10),
                    column![
                        text("ffmpeg not found")
                            .size(theme::size::FOOTNOTE)
                            .style(iced::theme::Text::Color(theme::color::warning())),
                        text("Transcoding requires ffmpeg. Go to Settings → Engine Management to download or locate it.")
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::secondary())),
                    ]
                    .spacing(2),
                ]
                .align_items(Alignment::Center),
            )
            .style(theme::warning_banner)
            .padding([12, 16])
            .width(Length::Fill)
            .into()
        } else {
            Space::with_height(0).into()
        };

        // ── Input file card ───────────────────────────────────────────────────
        let file_name = self
            .input_file
            .as_ref()
            .map(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| "No file selected".to_string());

        let file_color = if self.input_file.is_some() {
            iced::theme::Text::Default
        } else {
            iced::theme::Text::Color(theme::color::secondary())
        };

        let out_dir_label = self
            .output_dir
            .as_ref()
            .map(|p| truncate_path(&p.display().to_string(), 38))
            .unwrap_or_else(|| "Same as input file".to_string());

        let input_card = container(
            column![
                row![
                    column![
                        text("Input file")
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::secondary())),
                        Space::with_height(4),
                        text(file_name)
                            .size(theme::size::FOOTNOTE)
                            .style(file_color)
                            .width(Length::Fill),
                    ]
                    .spacing(0)
                    .width(Length::Fill),
                    Space::with_width(12),
                    button(text("Browse…").size(theme::size::FOOTNOTE))
                        .on_press(TranscoderPaneMessage::PickInputFile)
                        .padding([9, 18])
                        .style(theme::button::secondary()),
                ]
                .align_items(Alignment::Center),
                Space::with_height(10),
                row![
                    column![
                        text("Output folder")
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::secondary())),
                        Space::with_height(4),
                        text(out_dir_label)
                            .size(theme::size::FOOTNOTE)
                            .style(iced::theme::Text::Color(theme::color::secondary()))
                            .width(Length::Fill),
                    ]
                    .spacing(0)
                    .width(Length::Fill),
                    Space::with_width(12),
                    button(text("Change…").size(theme::size::FOOTNOTE))
                        .on_press(TranscoderPaneMessage::PickOutputDir)
                        .padding([9, 18])
                        .style(theme::button::secondary()),
                ]
                .align_items(Alignment::Center),
            ]
            .spacing(0),
        )
        .style(theme::card)
        .padding([14, 16])
        .width(Length::Fill);

        // ── Preset card ───────────────────────────────────────────────────────
        let hdr_toggle: Element<TranscoderPaneMessage> =
            if self.selected_preset.container.supports_hdr() {
                checkbox(
                    "Preserve HDR",
                    matches!(self.selected_preset.hdr_policy, HdrPolicy::Preserve),
                )
                .on_toggle(TranscoderPaneMessage::HdrPolicyToggled)
                .text_size(theme::size::FOOTNOTE)
                .into()
            } else {
                Space::with_width(0).into()
            };

        let command_toggle_label = if self.command_preview_open {
            "Hide generated command"
        } else {
            "Show generated command"
        };
        let command_toggle_btn = button(
            row![
                text("●")
                    .size(9)
                    .style(iced::theme::Text::Color(theme::color::accent())),
                Space::with_width(7),
                text(command_toggle_label)
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::accent())),
            ]
            .align_items(Alignment::Center),
        )
        .on_press(TranscoderPaneMessage::CommandPreviewToggled)
        .style(theme::button::ghost())
        .padding([4, 0]);

        let cmd_preview: Element<TranscoderPaneMessage> = if self.command_preview_open {
            let request = TranscodeRequest {
                job_id: Uuid::nil(),
                input: self
                    .input_file
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("input.mp4")),
                output: PathBuf::from("output.ext"),
                preset: self.selected_preset.clone(),
            };
            let args = transcoder::build_ffmpeg_args(&request);
            let cmd = format!("ffmpeg {}", args.join(" "));

            container(
                scrollable(
                    text(cmd)
                        .size(theme::size::CAPTION)
                        .font(iced::Font::MONOSPACE)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                        .width(Length::Fill),
                )
                .height(48),
            )
            .style(theme::code_block)
            .padding(12)
            .width(Length::Fill)
            .into()
        } else {
            Space::with_height(0).into()
        };

        let codec_note: Element<TranscoderPaneMessage> = if self.command_preview_open {
            text(
                "Dolby Vision: stream-copy attempted when video codec is \"Copy\"; \
                 falls back to HDR10 if unsupported.",
            )
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::tertiary()))
            .into()
        } else {
            Space::with_height(0).into()
        };

        // Advanced settings toggle (chevron-style bullet)
        let adv_label = if self.advanced_open {
            "Hide advanced settings"
        } else {
            "Advanced settings"
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
        .on_press(TranscoderPaneMessage::AdvancedSettingsToggled)
        .style(theme::button::ghost())
        .padding([4, 0]);

        // Advanced panel — codec/resolution + CRF/speed, only when expanded
        let advanced_panel: Element<TranscoderPaneMessage> = if self.advanced_open {
            container(
                column![
                    row![
                        text("Codec & format")
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::secondary())),
                        Space::with_width(Length::Fill),
                        hdr_toggle,
                    ]
                    .align_items(Alignment::Center),
                    Space::with_height(10),
                    row![
                        pick_list(
                            ContainerFormat::all_variants(),
                            Some(self.selected_preset.container.clone()),
                            TranscoderPaneMessage::ContainerSelected,
                        )
                        .text_size(theme::size::FOOTNOTE)
                        .padding([8, 12])
                        .width(Length::FillPortion(1)),
                        Space::with_width(12),
                        pick_list(
                            VideoCodec::all_variants(),
                            Some(self.selected_preset.video_codec.clone()),
                            TranscoderPaneMessage::VideoCodecSelected
                        )
                        .text_size(theme::size::FOOTNOTE)
                        .padding([8, 12])
                        .width(Length::FillPortion(1)),
                        Space::with_width(12),
                        pick_list(
                            AudioCodec::all_variants(),
                            Some(self.selected_preset.audio_codec.clone()),
                            TranscoderPaneMessage::AudioCodecSelected
                        )
                        .text_size(theme::size::FOOTNOTE)
                        .padding([8, 12])
                        .width(Length::FillPortion(1)),
                        Space::with_width(12),
                        pick_list(
                            Resolution::all_variants(),
                            Some(self.selected_preset.resolution.clone()),
                            TranscoderPaneMessage::ResolutionSelected
                        )
                        .text_size(theme::size::FOOTNOTE)
                        .padding([8, 12])
                        .width(Length::FillPortion(1)),
                    ]
                    .align_items(Alignment::Center),
                    Space::with_height(18),
                    row![
                        column![
                            row![
                                text("Quality (CRF)")
                                    .size(theme::size::CAPTION)
                                    .style(iced::theme::Text::Color(theme::color::secondary())),
                                Space::with_width(Length::Fill),
                                text(self.selected_preset.crf.to_string())
                                    .size(theme::size::CAPTION),
                            ],
                            Space::with_height(8),
                            slider(
                                0..=51,
                                self.selected_preset.crf,
                                TranscoderPaneMessage::CrfChanged
                            ),
                        ]
                        .spacing(0)
                        .width(Length::Fill),
                        Space::with_width(24),
                        column![
                            text("Speed (Preset)")
                                .size(theme::size::CAPTION)
                                .style(iced::theme::Text::Color(theme::color::secondary())),
                            Space::with_height(4),
                            pick_list(
                                EncodingPreset::all_variants(),
                                Some(self.selected_preset.speed.clone()),
                                TranscoderPaneMessage::SpeedSelected
                            )
                            .text_size(theme::size::FOOTNOTE)
                            .padding([8, 12])
                            .width(Length::Fill),
                        ]
                        .spacing(0)
                        .width(Length::FillPortion(1)),
                    ]
                    .align_items(Alignment::Center),
                    Space::with_height(if self.command_preview_open {
                        10u16
                    } else {
                        0u16
                    }),
                    codec_note,
                ]
                .spacing(0),
            )
            .style(theme::tinted_panel)
            .padding([16, 18])
            .width(Length::Fill)
            .into()
        } else {
            Space::with_height(0).into()
        };

        let preset_card = container(
            column![
                row![
                    text("Preset")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::secondary())),
                    Space::with_width(Length::Fill),
                    command_toggle_btn,
                ],
                Space::with_height(8),
                pick_list(
                    self.presets.clone(),
                    if self.selected_preset.name == "Custom" {
                        None
                    } else {
                        Some(self.selected_preset.clone())
                    },
                    TranscoderPaneMessage::PresetSelected,
                )
                .placeholder("Custom Settings")
                .text_size(theme::size::FOOTNOTE)
                .padding([10, 14])
                .width(Length::Fill),
                Space::with_height(if self.command_preview_open {
                    10u16
                } else {
                    0u16
                }),
                cmd_preview,
                Space::with_height(16),
                advanced_toggle,
                Space::with_height(if self.advanced_open { 12u16 } else { 0u16 }),
                advanced_panel,
            ]
            .spacing(0),
        )
        .style(theme::card)
        .padding([14, 16])
        .width(Length::Fill);

        // ── Start button ──────────────────────────────────────────────────────
        let can_start = self.input_file.is_some() && self.ffmpeg_available;
        let start_btn = {
            let b = button(text("Start Transcode").size(theme::size::FOOTNOTE))
                .style(theme::button::primary())
                .padding([11, 24]);
            if can_start {
                b.on_press(TranscoderPaneMessage::StartTranscode)
            } else {
                b
            }
        };

        // ── Queue section ─────────────────────────────────────────────────────
        let active_count = self
            .jobs
            .iter()
            .filter(|j| {
                matches!(
                    j.status,
                    TranscodeStatus::Running | TranscodeStatus::Pending
                )
            })
            .count();

        let t_count_str = if active_count > 0 {
            format!("{} active", active_count)
        } else {
            format!("{} items", self.jobs.len())
        };
        let queue_header = row![
            text("Queue").size(theme::size::HEADLINE),
            text(format!("  ·  {}", t_count_str))
                .size(theme::size::SUBHEAD)
                .style(iced::theme::Text::Color(theme::color::secondary())),
        ]
        .align_items(Alignment::Center);

        let queue: Element<TranscoderPaneMessage> = if self.jobs.is_empty() {
            let theme_for_svg = if is_dark {
                iced::Theme::Dark
            } else {
                iced::Theme::Light
            };
            container(
                column![
                    Space::with_height(Length::FillPortion(1)),
                    svg(theme::empty_transcoder_svg_for(&theme_for_svg))
                        .width(220)
                        .height(140),
                    Space::with_height(18),
                    text("No transcodes yet").size(24),
                    Space::with_height(8),
                    text("Select a file above and press Start Transcode.")
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
            let rows: Vec<Element<TranscoderPaneMessage>> =
                self.jobs.iter().rev().map(transcode_job_row).collect();
            scrollable(column(rows).spacing(8).width(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        column![
            header,
            Space::with_height(20),
            horizontal_rule(1).style(theme::divider),
            Space::with_height(22),
            ffmpeg_banner,
            Space::with_height(if !self.ffmpeg_available { 14u16 } else { 0u16 }),
            input_card,
            Space::with_height(12),
            preset_card,
            Space::with_height(16),
            start_btn,
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

fn transcode_job_row(job: &TranscodeJob) -> Element<'static, TranscoderPaneMessage> {
    let input_name = job
        .input
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let _output_name = job
        .output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (status_label, chip_kind) = match job.status {
        TranscodeStatus::Completed => ("Completed", theme::ChipKind::Success),
        TranscodeStatus::Failed => ("Failed", theme::ChipKind::Danger),
        TranscodeStatus::Cancelled => ("Cancelled", theme::ChipKind::Neutral),
        TranscodeStatus::Running => ("Running", theme::ChipKind::Accent),
        TranscodeStatus::Pending => ("Pending", theme::ChipKind::Neutral),
    };

    let progress_elem: Element<TranscoderPaneMessage> =
        if matches!(job.status, TranscodeStatus::Running) {
            progress_bar(0.0..=100.0, job.progress)
                .style(theme::progress())
                .height(4)
                .width(Length::Fill)
                .into()
        } else {
            Space::with_height(4).into()
        };

    let fps_text = job
        .fps
        .map(|fps| format!("{:.0} fps", fps))
        .unwrap_or_default();

    let action: Element<TranscoderPaneMessage> = match job.status {
        TranscodeStatus::Completed => button(text("Open file").size(theme::size::CAPTION))
            .on_press(TranscoderPaneMessage::OpenOutputFile(job.id))
            .style(theme::button::primary())
            .padding([5, 10])
            .into(),
        TranscodeStatus::Running | TranscodeStatus::Pending => {
            button(text("Cancel").size(theme::size::CAPTION))
                .on_press(TranscoderPaneMessage::CancelTranscode(job.id))
                .style(theme::button::danger())
                .padding([5, 10])
                .into()
        }
        _ => Space::with_width(0).into(),
    };

    let error_elem: Element<TranscoderPaneMessage> = if let Some(err) = &job.error_detail {
        text(format!("Error: {}", err))
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::danger()))
            .into()
    } else {
        Space::with_height(0).into()
    };

    let title = job
        .input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| input_name.clone());

    let preset_label = job
        .preset
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "(preset)".to_string());

    let status_chip = container(text(status_label).size(theme::size::CAPTION))
        .style(theme::chip(chip_kind))
        .padding([3, 9]);

    container(
        column![
            row![
                column![
                    text(title).size(theme::size::FOOTNOTE).width(Length::Fill),
                    Space::with_height(4),
                    text(preset_label)
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
            progress_elem,
            Space::with_height(4),
            row![
                text(fps_text)
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
