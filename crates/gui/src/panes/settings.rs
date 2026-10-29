use crate::panes::manager::{ManagerPane, ManagerPaneMessage};
use crate::theme;
use iced::{
    widget::{
        button, checkbox, column, container, horizontal_rule, pick_list, row, scrollable, text,
        text_input, Space,
    },
    Alignment, Command, Element, Length,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use ytdlp_core::{AppConfig, AppEvent, ThemeChoice};

#[derive(Debug, Clone)]
pub enum SettingsPaneMessage {
    FfmpegPathChanged(String),
    BrowseFfmpegPath,
    FfmpegPathPicked(Option<PathBuf>),
    DownloadDirChanged(String),
    BrowseDownloadDir,
    DownloadDirPicked(Option<PathBuf>),
    MaxDownloadsChanged(String),
    MaxTranscodesChanged(String),
    ThemeChanged(ThemeChoice),
    UseGpuToggled(bool),
    Manager(ManagerPaneMessage),
    TabSelected(SettingsTab),
    Save,
    OpenConfigDir,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Engine,
    About,
}

pub struct SettingsPane {
    ffmpeg_path: String,
    download_dir: String,
    max_downloads: String,
    max_transcodes: String,
    theme: ThemeChoice,
    use_gpu: bool,
    saved_notice: bool,
    active_tab: SettingsTab,
    manager_pane: ManagerPane,
}

impl SettingsPane {
    pub fn new(config: &AppConfig, event_tx: Arc<broadcast::Sender<AppEvent>>) -> Self {
        Self {
            ffmpeg_path: config
                .ffmpeg_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            download_dir: config.default_download_dir.display().to_string(),
            max_downloads: config.max_concurrent_downloads.to_string(),
            max_transcodes: config.max_concurrent_transcodes.to_string(),
            theme: config.theme.clone(),
            use_gpu: config.use_gpu,
            saved_notice: false,
            active_tab: SettingsTab::General,
            manager_pane: ManagerPane::new(config, event_tx),
        }
    }

    pub fn handle_event(&mut self, event: &AppEvent) -> Command<SettingsPaneMessage> {
        self.manager_pane
            .handle_event(event)
            .map(SettingsPaneMessage::Manager)
    }

    pub fn set_ffmpeg_detected(&mut self, path: Option<PathBuf>) {
        self.manager_pane.set_ffmpeg_detected(path);
    }

    pub fn managed_ffmpeg_path(&self) -> Option<PathBuf> {
        self.manager_pane.managed_ffmpeg_path()
    }

    #[allow(dead_code)]
    pub fn set_active_tab(&mut self, tab: SettingsTab) {
        self.active_tab = tab;
    }

    /// Returns `(Command, Option<updated config>)`.
    pub fn update(
        &mut self,
        msg: SettingsPaneMessage,
        config: &mut AppConfig,
    ) -> (Command<SettingsPaneMessage>, Option<AppConfig>) {
        self.saved_notice = false;
        match msg {
            SettingsPaneMessage::FfmpegPathChanged(s) => self.ffmpeg_path = s,
            SettingsPaneMessage::DownloadDirChanged(s) => self.download_dir = s,
            SettingsPaneMessage::MaxDownloadsChanged(s) => self.max_downloads = s,
            SettingsPaneMessage::MaxTranscodesChanged(s) => self.max_transcodes = s,
            SettingsPaneMessage::ThemeChanged(t) => {
                self.theme = t.clone();
                config.theme = t;
                return (Command::none(), Some(config.clone()));
            }
            SettingsPaneMessage::UseGpuToggled(b) => self.use_gpu = b,

            SettingsPaneMessage::Manager(msg) => {
                let (cmd, action) = self.manager_pane.update(msg);
                if let Some(action) = action {
                    match action {
                        crate::panes::manager::ManagerAction::UseSystemYtDlp(path) => {
                            config.use_system_ytdlp = true;
                            config.system_ytdlp_path = Some(path);
                        }
                        crate::panes::manager::ManagerAction::ActivateVersion {
                            version, ..
                        } => {
                            config.active_ytdlp_version = Some(version);
                            config.use_system_ytdlp = false;
                        }
                    }
                    return (cmd.map(SettingsPaneMessage::Manager), Some(config.clone()));
                }
                return (cmd.map(SettingsPaneMessage::Manager), None);
            }
            SettingsPaneMessage::TabSelected(tab) => {
                self.active_tab = tab;
            }

            SettingsPaneMessage::BrowseFfmpegPath => {
                return (
                    Command::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_title("Locate ffmpeg executable")
                                .pick_file()
                                .await
                                .map(|h| h.path().to_path_buf())
                        },
                        SettingsPaneMessage::FfmpegPathPicked,
                    ),
                    None,
                );
            }
            SettingsPaneMessage::FfmpegPathPicked(p) => {
                if let Some(path) = p {
                    self.ffmpeg_path = path.display().to_string();
                }
            }

            SettingsPaneMessage::BrowseDownloadDir => {
                let current = self.download_dir.clone();
                return (
                    Command::perform(
                        async move {
                            rfd::AsyncFileDialog::new()
                                .set_title("Choose default download folder")
                                .set_directory(&current)
                                .pick_folder()
                                .await
                                .map(|h| h.path().to_path_buf())
                        },
                        SettingsPaneMessage::DownloadDirPicked,
                    ),
                    None,
                );
            }
            SettingsPaneMessage::DownloadDirPicked(p) => {
                if let Some(path) = p {
                    self.download_dir = path.display().to_string();
                }
            }

            SettingsPaneMessage::Save => {
                config.ffmpeg_path = if self.ffmpeg_path.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(&self.ffmpeg_path))
                };
                config.default_download_dir = PathBuf::from(&self.download_dir);
                config.max_concurrent_downloads = self.max_downloads.parse().unwrap_or(3).max(1);
                config.max_concurrent_transcodes = self.max_transcodes.parse().unwrap_or(2).max(1);
                config.theme = self.theme.clone();
                config.use_gpu = self.use_gpu;
                self.saved_notice = true;
                return (Command::none(), Some(config.clone()));
            }

            SettingsPaneMessage::OpenConfigDir => {
                if let Ok(dir) = ytdlp_core::config_dir() {
                    let _ = open::that(dir);
                }
            }
        }
        (Command::none(), None)
    }

    pub fn view(&self, _is_dark: bool) -> Element<'_, SettingsPaneMessage> {
        let tabs = row![
            tab_button("General", SettingsTab::General, &self.active_tab),
            tab_button("Engine Management", SettingsTab::Engine, &self.active_tab),
            tab_button("About", SettingsTab::About, &self.active_tab),
        ]
        .spacing(8);

        let content = match self.active_tab {
            SettingsTab::General => self.view_general(),
            SettingsTab::Engine => self.manager_pane.view().map(SettingsPaneMessage::Manager),
            SettingsTab::About => self.view_about(),
        };

        column![
            text("Settings").size(32),
            Space::with_height(6),
            text("Configure ClipKit's behavior, engines, and appearance")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::secondary())),
            Space::with_height(20),
            horizontal_rule(1).style(theme::divider),
            Space::with_height(22),
            tabs,
            Space::with_height(22),
            content,
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_general(&self) -> Element<'_, SettingsPaneMessage> {
        // ── GENERAL section ───────────────────────────────────────────────────
        let themes = vec![ThemeChoice::Dark, ThemeChoice::Light, ThemeChoice::System];
        let theme_row = form_row(
            "Theme",
            column![
                pick_list(
                    themes,
                    Some(self.theme.clone()),
                    SettingsPaneMessage::ThemeChanged
                )
                .text_size(theme::size::FOOTNOTE)
                .padding([8, 10])
                .width(200),
                Space::with_height(4),
                text("\"System\" follows your OS dark/light mode preference")
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
            ]
            .spacing(0)
            .into(),
        );

        let gpu_row = form_row(
            "Hardware acceleration",
            checkbox(
                "Enable GPU acceleration (VideoToolbox, NVENC, etc.)",
                self.use_gpu,
            )
            .on_toggle(SettingsPaneMessage::UseGpuToggled)
            .text_size(theme::size::FOOTNOTE)
            .into(),
        );

        let general_card = section_card(
            "GENERAL",
            column![theme_row, Space::with_height(12), gpu_row]
                .spacing(0)
                .into(),
        );

        // ── PATHS section ─────────────────────────────────────────────────────
        let ffmpeg_row = form_row(
            "ffmpeg path",
            row![
                text_input("Detected automatically, or leave blank", &self.ffmpeg_path)
                    .on_input(SettingsPaneMessage::FfmpegPathChanged)
                    .style(theme::input())
                    .padding([8, 10])
                    .size(theme::size::FOOTNOTE)
                    .width(Length::Fill),
                Space::with_width(8),
                button(text("Browse").size(theme::size::CAPTION))
                    .on_press(SettingsPaneMessage::BrowseFfmpegPath)
                    .padding([6, 12])
                    .style(theme::button::secondary()),
            ]
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into(),
        );

        let dl_dir_row = form_row(
            "Download folder",
            row![
                text_input("Default download folder", &self.download_dir)
                    .on_input(SettingsPaneMessage::DownloadDirChanged)
                    .style(theme::input())
                    .padding([8, 10])
                    .size(theme::size::FOOTNOTE)
                    .width(Length::Fill),
                Space::with_width(8),
                button(text("Browse").size(theme::size::CAPTION))
                    .on_press(SettingsPaneMessage::BrowseDownloadDir)
                    .padding([6, 12])
                    .style(theme::button::secondary()),
            ]
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into(),
        );

        let cfg_dir_row = row![
            Space::with_width(180),
            button(
                row![
                    text("●")
                        .size(9)
                        .style(iced::theme::Text::Color(theme::color::accent())),
                    Space::with_width(7),
                    text("Open config folder")
                        .size(theme::size::CAPTION)
                        .style(iced::theme::Text::Color(theme::color::accent())),
                ]
                .align_items(Alignment::Center),
            )
            .on_press(SettingsPaneMessage::OpenConfigDir)
            .style(theme::button::ghost())
            .padding([6, 0]),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let paths_card = section_card(
            "PATHS",
            column![
                ffmpeg_row,
                Space::with_height(10),
                dl_dir_row,
                Space::with_height(8),
                cfg_dir_row
            ]
            .spacing(0)
            .into(),
        );

        // ── PERFORMANCE section ───────────────────────────────────────────────
        let max_dl_row = form_row(
            "Max concurrent downloads",
            row![
                text_input("3", &self.max_downloads)
                    .on_input(SettingsPaneMessage::MaxDownloadsChanged)
                    .style(theme::input())
                    .padding([8, 10])
                    .size(theme::size::FOOTNOTE)
                    .width(80),
                Space::with_width(8),
                text("(1–20)")
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
            ]
            .align_items(Alignment::Center)
            .into(),
        );

        let max_tc_row = form_row(
            "Max concurrent transcodes",
            row![
                text_input("2", &self.max_transcodes)
                    .on_input(SettingsPaneMessage::MaxTranscodesChanged)
                    .style(theme::input())
                    .padding([8, 10])
                    .size(theme::size::FOOTNOTE)
                    .width(80),
                Space::with_width(8),
                text("(1–8)")
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
            ]
            .align_items(Alignment::Center)
            .into(),
        );

        let perf_card = section_card(
            "PERFORMANCE",
            column![max_dl_row, Space::with_height(10), max_tc_row]
                .spacing(0)
                .into(),
        );

        // ── Save row ──────────────────────────────────────────────────────────
        let saved_notice: Element<SettingsPaneMessage> = if self.saved_notice {
            row![
                text("✓")
                    .size(theme::size::FOOTNOTE)
                    .style(iced::theme::Text::Color(theme::color::success())),
                Space::with_width(6),
                text("Settings saved")
                    .size(theme::size::FOOTNOTE)
                    .style(iced::theme::Text::Color(theme::color::success())),
            ]
            .align_items(Alignment::Center)
            .into()
        } else {
            Space::with_width(0).into()
        };

        let save_row = row![
            button(text("Save Settings").size(theme::size::FOOTNOTE))
                .on_press(SettingsPaneMessage::Save)
                .padding([11, 24])
                .style(theme::button::primary()),
            Space::with_width(12),
            saved_notice,
        ]
        .align_items(Alignment::Center);

        let inner = column![
            general_card,
            Space::with_height(12),
            paths_card,
            Space::with_height(12),
            perf_card,
            Space::with_height(24),
            save_row,
        ]
        .spacing(0)
        .width(Length::Fill);

        scrollable(inner).height(Length::Fill).into()
    }

    fn view_about(&self) -> Element<'_, SettingsPaneMessage> {
        column![
            row![
                iced::widget::image(theme::brand_icon_handle()).width(64).height(64),
                Space::with_width(18),
                column![
                    text("ClipKit").size(theme::size::LARGE_TITLE),
                    Space::with_height(2),
                    text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .size(theme::size::FOOTNOTE)
                        .style(iced::theme::Text::Color(theme::color::accent())),
                ]
                .spacing(0),
            ]
            .align_items(Alignment::Center),
            Space::with_height(20),
            text("A high-performance, native GUI media toolkit for downloads and FFmpeg transcoding.")
                .size(theme::size::BODY),
            Space::with_height(6),
            text("Crafted with Rust and Iced following Apple Human Interface Guidelines.")
                .size(theme::size::FOOTNOTE)
                .style(iced::theme::Text::Color(theme::color::tertiary())),
            Space::with_height(24),
            text("MIT License").size(theme::size::HEADLINE),
            Space::with_height(2),
            text("Copyright (c) 2026 Palaash Atri")
                .size(theme::size::CAPTION)
                .style(iced::theme::Text::Color(theme::color::tertiary())),
            Space::with_height(12),
            container(
                scrollable(
                    text(include_str!("../../../../LICENSE"))
                        .size(11)
                        .style(iced::theme::Text::Color(theme::color::secondary()))
                )
                .height(200),
            )
            .style(theme::code_block)
            .padding(14)
            .width(Length::Fill),
        ]
        .spacing(0)
        .into()
    }
}

fn tab_button<'a>(
    label: &'a str,
    tab: SettingsTab,
    active: &SettingsTab,
) -> Element<'a, SettingsPaneMessage> {
    let is_active = tab == *active;
    button(text(label).size(theme::size::FOOTNOTE))
        .on_press(SettingsPaneMessage::TabSelected(tab))
        .style(theme::button::pill(is_active))
        .padding([8, 16])
        .into()
}

/// A labeled form row: 180-wide label on the left, content on the right.
fn form_row<'a>(
    label: &'a str,
    content: Element<'a, SettingsPaneMessage>,
) -> Element<'a, SettingsPaneMessage> {
    row![text(label).size(theme::size::FOOTNOTE).width(180), content,]
        .spacing(12)
        .align_items(Alignment::Center)
        .into()
}

/// A named section with an uppercase header label and a thin underline divider.
fn section_card<'a>(
    title: &'a str,
    content: Element<'a, SettingsPaneMessage>,
) -> Element<'a, SettingsPaneMessage> {
    let section_header = column![
        text(title)
            .size(theme::size::CAPTION)
            .style(iced::theme::Text::Color(theme::color::accent())),
        Space::with_height(8),
        horizontal_rule(1).style(theme::divider),
        Space::with_height(14),
    ]
    .spacing(0);

    container(column![section_header, content].spacing(0))
        .style(theme::card)
        .padding([18, 20])
        .width(Length::Fill)
        .into()
}
