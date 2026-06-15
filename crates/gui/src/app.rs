use crate::panes::{
    download::{DownloadPane, DownloadPaneMessage},
    manager::ManagerPaneMessage,
    settings::{SettingsPane, SettingsPaneMessage},
    transcoder::{TranscoderPane, TranscoderPaneMessage},
};
use crate::theme;
use iced::{
    widget::{button, column, container, horizontal_rule, row, svg, text, vertical_rule, Space},
    Alignment, Application, Command, Element, Length, Subscription, Theme,
};
use std::any::TypeId;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;
use ytdlp_core::{AppConfig, AppEvent, AppUpdateInfo, ThemeChoice};

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pane {
    Download,
    Transcoder,
    Settings,
}

// ── Top-level message ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    PaneSelected(Pane),
    Download(DownloadPaneMessage),
    Transcoder(TranscoderPaneMessage),
    Settings(SettingsPaneMessage),
    AppEvent(AppEvent),
    MenuEvent(String),
    FfmpegDetected(Option<PathBuf>),
    AppUpdateChecked(Option<AppUpdateInfo>),
    OpenReleaseUrl(String),
    #[cfg(target_os = "macos")]
    InitMacosMenu,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub struct App {
    active_pane: Pane,
    config: AppConfig,
    event_tx: Arc<broadcast::Sender<AppEvent>>,
    download_pane: DownloadPane,
    transcoder_pane: TranscoderPane,
    settings_pane: SettingsPane,
    update_available: Option<AppUpdateInfo>,
    #[cfg(target_os = "macos")]
    _menu: Option<muda::Menu>,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let config = AppConfig::load().unwrap_or_default();

        let (event_tx, _) = broadcast::channel::<AppEvent>(512);
        let event_tx = Arc::new(event_tx);

        let download_pane = DownloadPane::new(&config, Arc::clone(&event_tx));
        let transcoder_pane = TranscoderPane::new(&config, Arc::clone(&event_tx));
        let settings_pane = SettingsPane::new(&config, Arc::clone(&event_tx));

        let ffmpeg_custom = config.ffmpeg_path.clone();
        let detect_cmd = Command::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    transcoder::detect_ffmpeg(ffmpeg_custom.as_deref())
                })
                .await
                .ok()
                .flatten()
            },
            Message::FfmpegDetected,
        );

        let update_cmd = Command::perform(
            yt_dlp_backend::check_for_app_update(),
            Message::AppUpdateChecked,
        );

        #[cfg(target_os = "macos")]
        let menu_init_cmd = Command::perform(async { () }, |_| Message::InitMacosMenu);

        let app = Self {
            active_pane: Pane::Download,
            config,
            event_tx,
            download_pane,
            transcoder_pane,
            settings_pane,
            update_available: None,
            #[cfg(target_os = "macos")]
            _menu: None,
        };

        #[cfg(target_os = "macos")]
        return (app, Command::batch([detect_cmd, update_cmd, menu_init_cmd]));

        #[cfg(not(target_os = "macos"))]
        (app, Command::batch([detect_cmd, update_cmd]))
    }

    fn title(&self) -> String {
        "ClipKit".to_string()
    }

    fn theme(&self) -> Theme {
        match self.config.theme {
            ThemeChoice::Dark => Theme::Dark,
            ThemeChoice::Light => Theme::Light,
            ThemeChoice::System => match dark_light::detect() {
                dark_light::Mode::Light => Theme::Light,
                _ => Theme::Dark,
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let event_tx = Arc::clone(&self.event_tx);
        let app_events =
            iced::subscription::channel(TypeId::of::<AppEvent>(), 512, move |mut output| {
                let event_tx = Arc::clone(&event_tx);
                async move {
                    let mut rx = event_tx.subscribe();
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                use iced::futures::SinkExt;
                                let _ = output.send(Message::AppEvent(event)).await;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                eprintln!("Event subscription lagged by {n} events");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                }
            });

        #[cfg(target_os = "macos")]
        let menu_events = iced::subscription::channel(
            TypeId::of::<muda::MenuEvent>(),
            100,
            |mut output| async move {
                let receiver = muda::MenuEvent::receiver();
                loop {
                    // Poll for menu events more frequently
                    while let Ok(event) = receiver.try_recv() {
                        use iced::futures::SinkExt;
                        let _ = output.send(Message::MenuEvent(event.id.0.clone())).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(16)).await;
                }
            },
        );

        #[cfg(target_os = "macos")]
        return Subscription::batch([app_events, menu_events]);

        #[cfg(not(target_os = "macos"))]
        return app_events;
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PaneSelected(pane) => {
                self.active_pane = pane;
                Command::none()
            }

            Message::AppUpdateChecked(info) => {
                self.update_available = info;
                Command::none()
            }

            Message::OpenReleaseUrl(url) => {
                let _ = open::that(&url);
                Command::none()
            }

            Message::FfmpegDetected(path) => {
                match path {
                    Some(p) => {
                        self.transcoder_pane.set_ffmpeg_detected(Some(p.clone()));
                        self.settings_pane.set_ffmpeg_detected(Some(p.clone()));
                        if self.config.ffmpeg_path.is_none() {
                            self.config.ffmpeg_path = Some(p.clone());
                        }
                        Command::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    transcoder::detect_ffmpeg_version(&p)
                                })
                                .await
                                .ok()
                                .flatten()
                            },
                            |v| {
                                Message::Settings(SettingsPaneMessage::Manager(
                                    ManagerPaneMessage::FfmpegVersionDetected(v),
                                ))
                            },
                        )
                    }
                    None => {
                        // Check if we have a previously downloaded managed binary
                        let managed = self.settings_pane.managed_ffmpeg_path();
                        if let Some(p) = managed {
                            self.transcoder_pane.set_ffmpeg_detected(Some(p.clone()));
                            self.settings_pane.set_ffmpeg_detected(Some(p.clone()));
                            self.config.ffmpeg_path = Some(p.clone());
                            Command::perform(
                                async move {
                                    tokio::task::spawn_blocking(move || {
                                        transcoder::detect_ffmpeg_version(&p)
                                    })
                                    .await
                                    .ok()
                                    .flatten()
                                },
                                |v| {
                                    Message::Settings(SettingsPaneMessage::Manager(
                                        ManagerPaneMessage::FfmpegVersionDetected(v),
                                    ))
                                },
                            )
                        } else {
                            // Auto-download ffmpeg in background
                            Command::perform(async { () }, |_| {
                                Message::Settings(SettingsPaneMessage::Manager(
                                    ManagerPaneMessage::AutoDownloadFfmpeg,
                                ))
                            })
                        }
                    }
                }
            }

            Message::Download(msg) => self.download_pane.update(msg).map(Message::Download),

            Message::Transcoder(msg) => self.transcoder_pane.update(msg).map(Message::Transcoder),

            Message::Settings(msg) => {
                let (cmd, new_config) = self.settings_pane.update(msg, &mut self.config);
                if let Some(cfg) = new_config {
                    self.config = cfg.clone();
                    self.download_pane.set_ytdlp_path(cfg.active_ytdlp_path());
                    self.transcoder_pane
                        .set_ffmpeg_path(cfg.ffmpeg_path.clone());
                    let _ = self.config.save();
                }
                cmd.map(Message::Settings)
            }

            Message::MenuEvent(id) => {
                match id.as_str() {
                    "menu-item-about" => {
                        self.active_pane = Pane::Settings;
                        let _ = self.settings_pane.update(
                            SettingsPaneMessage::TabSelected(
                                crate::panes::settings::SettingsTab::About,
                            ),
                            &mut self.config,
                        );
                    }
                    "menu-pane-download" | "menu-nav-download" => {
                        self.active_pane = Pane::Download;
                    }
                    "menu-pane-transcode" => {
                        self.active_pane = Pane::Transcoder;
                    }
                    "menu-pane-settings" => {
                        self.active_pane = Pane::Settings;
                    }
                    _ => {}
                }
                Command::none()
            }

            #[cfg(target_os = "macos")]
            Message::InitMacosMenu => {
                self._menu = Some(init_macos_menu());
                Command::none()
            }

            Message::AppEvent(ref event) => {
                // Handle app-level events before routing to panes
                match event {
                    AppEvent::YtDlpBinaryDownloaded { version, path } => {
                        self.config.active_ytdlp_version = Some(version.clone());
                        self.config.use_system_ytdlp = false;
                        self.download_pane.set_ytdlp_path(Some(path.clone()));
                        let _ = self.config.save();
                    }
                    AppEvent::FfmpegDownloaded { path, version } => {
                        self.config.ffmpeg_path = Some(path.clone());
                        self.config.managed_ffmpeg_version = Some(version.clone());
                        self.transcoder_pane.set_ffmpeg_detected(Some(path.clone()));
                        let _ = self.config.save();
                    }
                    _ => {}
                }

                Command::batch([
                    self.download_pane
                        .handle_event(event)
                        .map(Message::Download),
                    self.settings_pane
                        .handle_event(event)
                        .map(Message::Settings),
                    self.transcoder_pane
                        .handle_event(event)
                        .map(Message::Transcoder),
                ])
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        const SIDEBAR_WIDTH: u16 = 268;

        let is_dark = matches!(self.theme(), Theme::Dark);

        let sidebar = container(self.sidebar_nav())
            .style(theme::sidebar)
            .width(SIDEBAR_WIDTH)
            .height(Length::Fill);

        let content_pane: Element<Message> = match self.active_pane {
            Pane::Download => self.download_pane.view(is_dark).map(Message::Download),
            Pane::Transcoder => self.transcoder_pane.view(is_dark).map(Message::Transcoder),
            Pane::Settings => self.settings_pane.view(is_dark).map(Message::Settings),
        };

        let content = container(content_pane)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([32, 40, 24, 40]);

        let top_accent = container(Space::with_height(2))
            .width(Length::Fill)
            .height(2)
            .style(theme::top_accent);

        container(
            column![
                top_accent,
                row![sidebar, vertical_rule(0.5), content]
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .style(theme::main_background)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl App {

    fn sidebar_nav(&self) -> Element<'_, Message> {
        let nav_item =
            |icon: theme::NavIcon, label: &'static str, pane: Pane| -> Element<'_, Message> {
                let active = self.active_pane == pane;

                let content = row![
                    svg(theme::nav_icon(icon, active)).width(20).height(20),
                    Space::with_width(14),
                    text(label)
                        .size(theme::size::SUBHEAD)
                        .style(if active {
                            iced::theme::Text::Color(iced::Color::WHITE)
                        } else {
                            iced::theme::Text::Color(theme::color::secondary())
                        })
                        .width(Length::Fill),
                ]
                .align_items(Alignment::Center);

                button(content)
                    .width(Length::Fill)
                    .padding([14, 16])
                    .style(theme::button::nav(active))
                    .on_press(Message::PaneSelected(pane))
                    .into()
            };

        let logo = container(
            row![
                svg(theme::brand_mark_svg()).width(34).height(34),
                Space::with_width(12),
                text("clipkit")
                    .size(22)
                    .style(iced::theme::Text::Default),
            ]
            .spacing(0)
            .align_items(Alignment::Center),
        )
        .padding([26, 20, 22, 20]);

        // Update-available badge
        let update_badge: Element<Message> = if let Some(info) = &self.update_available {
            let url = info.release_url.clone();
            container(
                button(
                    row![
                        text("↑")
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::accent())),
                        Space::with_width(6),
                        text(format!("{} available", info.version))
                            .size(theme::size::CAPTION)
                            .style(iced::theme::Text::Color(theme::color::accent())),
                    ]
                    .align_items(Alignment::Center),
                )
                .on_press(Message::OpenReleaseUrl(url))
                .style(theme::button::ghost())
                .padding([6, 14]),
            )
            .into()
        } else {
            Space::with_height(0).into()
        };

        column![
            logo,
            container(
                column![
                    nav_item(theme::NavIcon::Download, "Download", Pane::Download),
                    Space::with_height(4),
                    nav_item(theme::NavIcon::Transcoder, "Transcoder", Pane::Transcoder),
                ]
                .spacing(0),
            )
            .padding([0, 12]),
            Space::with_height(Length::Fill),
            update_badge,
            container(horizontal_rule(1).style(theme::divider))
                .width(Length::Fill)
                .padding([0, 20, 12, 20]),
            container(nav_item(theme::NavIcon::Settings, "Settings", Pane::Settings))
                .padding([0, 12]),
            container(
                text(concat!("v", env!("CARGO_PKG_VERSION")))
                    .size(theme::size::CAPTION)
                    .style(iced::theme::Text::Color(theme::color::tertiary())),
            )
            .padding([10, 20, 16, 20]),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Start)
        .into()
    }
}

#[cfg(target_os = "macos")]
pub fn init_macos_menu() -> muda::Menu {
    use muda::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

    let menu = Menu::new();

    // App Menu
    let app_menu = Submenu::new("ClipKit", true);
    let _ = app_menu.append_items(&[
        &MenuItem::with_id(
            MenuId::new("menu-item-about"),
            "ClipKit Info...",
            true,
            None,
        ),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::services(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(None),
        &PredefinedMenuItem::hide_others(None),
        &PredefinedMenuItem::show_all(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ]);

    // File Menu
    let file_menu = Submenu::new("File", true);
    let _ = file_menu.append_items(&[
        &MenuItem::with_id(MenuId::new("menu-nav-download"), "New Download", true, None),
        &PredefinedMenuItem::close_window(None),
    ]);

    // Edit Menu
    let edit_menu = Submenu::new("Edit", true);
    let _ = edit_menu.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ]);

    // Navigation Menu (Custom)
    let nav_menu = Submenu::new("Navigation", true);
    let _ = nav_menu.append_items(&[
        &MenuItem::with_id(
            MenuId::new("menu-pane-download"),
            "Download Pane",
            true,
            None,
        ),
        &MenuItem::with_id(
            MenuId::new("menu-pane-transcode"),
            "Transcoder Pane",
            true,
            None,
        ),
        &MenuItem::with_id(
            MenuId::new("menu-pane-settings"),
            "Settings Pane",
            true,
            None,
        ),
    ]);

    // Window Menu
    let window_menu = Submenu::new("Window", true);
    let _ = window_menu.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::bring_all_to_front(None),
    ]);

    let _ = menu.append_items(&[&app_menu, &file_menu, &edit_menu, &nav_menu, &window_menu]);
    menu.init_for_nsapp();
    menu
}
