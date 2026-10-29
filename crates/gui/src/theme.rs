use iced::widget::{
    button as iced_button, container, progress_bar as iced_progress, rule as iced_rule,
    svg as iced_svg, text_input as iced_text_input,
};
use iced::{Background, Border, Color, Shadow, Vector};

/// Typography scale.
#[allow(dead_code)]
pub mod size {
    pub const LARGE_TITLE: u16 = 32;
    pub const TITLE: u16 = 28;
    pub const HEADLINE: u16 = 18;
    pub const SUBHEAD: u16 = 15;
    pub const BODY: u16 = 14;
    pub const FOOTNOTE: u16 = 13;
    pub const CAPTION: u16 = 12;
}

/// Semantic color palette — synchronized with `assets/brand/palette.json`.
pub mod color {
    use iced::Color;

    /// Brand coral — synchronized with `assets/brand/palette.json` (#FF7750).
    pub fn accent() -> Color {
        Color::from_rgb8(255, 119, 80)
    }
    /// High-contrast accent color for text labels in light mode (WCAG AA ≥ 4.5:1, #A03219).
    pub fn accent_text(is_dark: bool) -> Color {
        if is_dark {
            Color::from_rgb8(255, 119, 80)
        } else {
            Color::from_rgb8(160, 50, 25)
        }
    }
    pub fn accent_hover() -> Color {
        Color::from_rgb8(255, 140, 105)
    }
    pub fn accent_soft() -> Color {
        Color::from_rgba(1.0, 0.467, 0.314, 0.12)
    }
    pub fn accent_ring() -> Color {
        Color::from_rgba(1.0, 0.467, 0.314, 0.40)
    }
    pub fn accent_glow() -> Color {
        Color::from_rgba(1.0, 0.467, 0.314, 0.25)
    }
    /// Brand alias kept for backwards compatibility with existing call sites.
    #[allow(dead_code)]
    pub fn brand() -> Color {
        accent()
    }
    pub fn success() -> Color {
        Color::from_rgb8(65, 203, 129)
    }
    pub fn warning() -> Color {
        Color::from_rgb8(238, 184, 82)
    }
    pub fn danger() -> Color {
        Color::from_rgb8(239, 93, 101)
    }
    /// Secondary text color: slate-400 in dark mode (#94A3B8), slate-600 in light mode (#5B6577).
    pub fn secondary_text(is_dark: bool) -> Color {
        if is_dark {
            Color::from_rgb8(148, 163, 184)
        } else {
            Color::from_rgb8(91, 101, 119)
        }
    }
    /// Default fallback secondary color ensuring ≥ 4.7:1 contrast on light surfaces (#64748B).
    pub fn secondary() -> Color {
        Color::from_rgb8(100, 116, 139)
    }
    pub fn tertiary() -> Color {
        Color::from_rgb8(100, 116, 139)
    }
}

fn is_light(theme: &iced::Theme) -> bool {
    matches!(theme, iced::Theme::Light)
}

fn surface(theme: &iced::Theme) -> Color {
    if is_light(theme) {
        Color::from_rgb8(255, 255, 255)
    } else {
        Color::from_rgb8(22, 31, 44)
    }
}

fn surface_raised(theme: &iced::Theme) -> Color {
    if is_light(theme) {
        Color::from_rgb8(241, 245, 249)
    } else {
        Color::from_rgb8(30, 41, 59)
    }
}

fn text_color(theme: &iced::Theme) -> Color {
    if is_light(theme) {
        Color::from_rgb8(15, 23, 42)
    } else {
        Color::from_rgb8(244, 247, 252)
    }
}

fn subtle_border(theme: &iced::Theme) -> Color {
    if is_light(theme) {
        Color::from_rgba(0.059, 0.090, 0.165, 0.08)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
    }
}

fn soft_shadow(theme: &iced::Theme) -> Shadow {
    Shadow {
        color: if is_light(theme) {
            Color::from_rgba(0.059, 0.090, 0.165, 0.06)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.28)
        },
        offset: Vector::new(0.0, 6.0),
        blur_radius: 20.0,
    }
}

fn strong_shadow(theme: &iced::Theme) -> Shadow {
    Shadow {
        color: if is_light(theme) {
            Color::from_rgba(0.059, 0.090, 0.165, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.38)
        },
        offset: Vector::new(0.0, 10.0),
        blur_radius: 28.0,
    }
}

/// Standard content card. Apple HIG styled with delicate borders and rounded corners.
pub fn card(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(surface(theme))),
        border: Border {
            radius: 12.0.into(),
            width: 0.5,
            color: subtle_border(theme),
        },
        text_color: None,
        shadow: soft_shadow(theme),
    }
}

/// Elevated card: for active/focused items.
pub fn elevated(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(surface_raised(theme))),
        border: Border {
            radius: 12.0.into(),
            width: 0.5,
            color: subtle_border(theme),
        },
        text_color: None,
        shadow: strong_shadow(theme),
    }
}

/// Tinted panel — soft coral background wash for highlighted sections.
pub fn tinted_panel(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(if is_light(theme) {
            Color::from_rgba(1.0, 0.467, 0.314, 0.06)
        } else {
            Color::from_rgba(1.0, 0.467, 0.314, 0.09)
        })),
        border: Border {
            radius: 12.0.into(),
            width: 0.5,
            color: Color::from_rgba(1.0, 0.467, 0.314, 0.18),
        },
        text_color: None,
        shadow: Shadow::default(),
    }
}

pub fn empty_state(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: None,
        border: Border::default(),
        text_color: None,
        shadow: Shadow::default(),
    }
}

#[allow(dead_code)]
pub fn topbar(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(if is_light(theme) {
            Color::from_rgb8(255, 255, 255)
        } else {
            Color::from_rgb8(26, 29, 36)
        })),
        border: Border::default(),
        text_color: Some(text_color(theme)),
        shadow: Shadow::default(),
    }
}

/// 2px solid pink bar at the very top of the window.
pub fn top_accent(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(color::accent())),
        border: Border::default(),
        text_color: None,
        shadow: Shadow::default(),
    }
}

/// Pink-tinted logo container — kept for backwards compatibility.
/// Prefer `brand_mark_svg()` for new code.
#[allow(dead_code)]
pub fn logo_mark(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(color::accent())),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        text_color: Some(Color::WHITE),
        shadow: Shadow {
            color: color::accent_glow(),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NavIcon {
    Download,
    Transcoder,
    Settings,
}

pub fn nav_icon(icon: NavIcon, active: bool) -> iced_svg::Handle {
    let color = if active { "#FF7750" } else { "#8b95a6" };
    let body = match icon {
        NavIcon::Download => format!(
            r#"<path d="M12 3v10m0 0 4-4m-4 4-4-4" fill="none" stroke="{color}" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/><path d="M5 14v3.5A2.5 2.5 0 0 0 7.5 20h9a2.5 2.5 0 0 0 2.5-2.5V14" fill="none" stroke="{color}" stroke-width="1.75" stroke-linecap="round"/>"#
        ),
        NavIcon::Transcoder => format!(
            r#"<rect x="3" y="6" width="18" height="12" rx="2" fill="none" stroke="{color}" stroke-width="1.75"/><path d="M7 6v12M17 6v12M3 10h4M3 14h4M17 10h4M17 14h4" fill="none" stroke="{color}" stroke-width="1.75" stroke-linecap="round"/>"#
        ),
        NavIcon::Settings => format!(
            r#"<circle cx="12" cy="12" r="3.2" fill="none" stroke="{color}" stroke-width="1.75"/><path d="M19.4 13.3a7.6 7.6 0 0 0 0-2.6l2-1.6-2-3.4-2.4 1a7.6 7.6 0 0 0-2.2-1.3L14.4 3h-4l-.4 2.4a7.6 7.6 0 0 0-2.2 1.3l-2.4-1-2 3.4 2 1.6a7.6 7.6 0 0 0 0 2.6l-2 1.6 2 3.4 2.4-1a7.6 7.6 0 0 0 2.2 1.3l.4 2.4h4l.4-2.4a7.6 7.6 0 0 0 2.2-1.3l2.4 1 2-3.4-2-1.6Z" fill="none" stroke="{color}" stroke-width="1.75" stroke-linejoin="round"/>"#
        ),
    };
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">{body}</svg>"#
    );
    iced_svg::Handle::from_memory(svg.into_bytes())
}

/// The 3D sculpted translucent coral ClipKit app icon from `assets/brand/icon-128.png`.
pub fn brand_icon_handle() -> iced::widget::image::Handle {
    iced::widget::image::Handle::from_memory(
        include_bytes!("../../../assets/brand/icon-128.png").to_vec(),
    )
}

/// The official ClipKit brand wordmark from `assets/brand/wordmark.png`.
#[allow(dead_code)]
pub fn brand_wordmark_handle() -> iced::widget::image::Handle {
    iced::widget::image::Handle::from_memory(
        include_bytes!("../../../assets/brand/wordmark.png").to_vec(),
    )
}

/// The ClipKit brand mark vector fallback.
#[allow(dead_code)]
pub fn brand_mark_svg() -> iced_svg::Handle {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#FF8C69"/>
      <stop offset="1" stop-color="#FF7750"/>
    </linearGradient>
  </defs>
  <rect x="4" y="4" width="40" height="40" rx="10" fill="url(#g)"/>
  <polygon points="19,14 34,24 19,34" fill="#ffffff"/>
</svg>"##;
    iced_svg::Handle::from_memory(svg.as_bytes().to_vec())
}

pub fn divider(theme: &iced::Theme) -> iced_rule::Appearance {
    iced_rule::Appearance {
        color: subtle_border(theme),
        width: 1,
        radius: 0.0.into(),
        fill_mode: iced_rule::FillMode::Full,
    }
}

/// Empty-state illustration — adapts to light or dark backgrounds.
pub fn empty_download_svg_for(theme: &iced::Theme) -> iced_svg::Handle {
    let svg = if is_light(theme) {
        EMPTY_DOWNLOAD_LIGHT
    } else {
        EMPTY_DOWNLOAD_DARK
    };
    iced_svg::Handle::from_memory(svg.as_bytes().to_vec())
}

pub fn empty_transcoder_svg_for(theme: &iced::Theme) -> iced_svg::Handle {
    let svg = if is_light(theme) {
        EMPTY_TRANSCODER_LIGHT
    } else {
        EMPTY_TRANSCODER_DARK
    };
    iced_svg::Handle::from_memory(svg.as_bytes().to_vec())
}

const EMPTY_DOWNLOAD_LIGHT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="140" viewBox="0 0 220 140">
  <ellipse cx="110" cy="120" rx="68" ry="6" fill="#eef0f5"/>
  <path d="M58 100c-3-24 18-42 40-36 9-22 44-29 60-7 20 0 35 16 35 37v12H58z" fill="#dde4f1" opacity=".85"/>
  <ellipse cx="110" cy="104" rx="55" ry="18" fill="#3a4360"/>
  <ellipse cx="110" cy="100" rx="48" ry="12" fill="#f7f9fd"/>
  <ellipse cx="110" cy="102" rx="40" ry="7" fill="#2d3550"/>
  <path d="M92 24h54l25 25v62H92z" fill="#ffffff" stroke="#c8ced9" stroke-width="1.4"/>
  <path d="M146 24v25h25z" fill="#f4f6fa" stroke="#c8ced9" stroke-width="1.4"/>
  <path d="M115 50a4 4 0 0 1 4-4h13a4 4 0 0 1 4 4v32l13-13a4 4 0 0 1 6 0l7 7a4 4 0 0 1 0 6l-33 33a4 4 0 0 1-6 0l-33-33a4 4 0 0 1 0-6l7-7a4 4 0 0 1 6 0l12 13z" fill="#566283"/>
</svg>"##;

const EMPTY_DOWNLOAD_DARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="140" viewBox="0 0 220 140">
  <ellipse cx="110" cy="120" rx="68" ry="6" fill="#1c2030"/>
  <path d="M58 100c-3-24 18-42 40-36 9-22 44-29 60-7 20 0 35 16 35 37v12H58z" fill="#2a3349" opacity=".75"/>
  <ellipse cx="110" cy="104" rx="55" ry="18" fill="#10131a"/>
  <ellipse cx="110" cy="100" rx="48" ry="12" fill="#2c3346"/>
  <ellipse cx="110" cy="102" rx="40" ry="7" fill="#0c0f17"/>
  <path d="M92 24h54l25 25v62H92z" fill="#2a3144" stroke="#3a4258" stroke-width="1.4"/>
  <path d="M146 24v25h25z" fill="#363c52" stroke="#3a4258" stroke-width="1.4"/>
  <path d="M115 50a4 4 0 0 1 4-4h13a4 4 0 0 1 4 4v32l13-13a4 4 0 0 1 6 0l7 7a4 4 0 0 1 0 6l-33 33a4 4 0 0 1-6 0l-33-33a4 4 0 0 1 0-6l7-7a4 4 0 0 1 6 0l12 13z" fill="#9aa3bd"/>
</svg>"##;

const EMPTY_TRANSCODER_LIGHT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="140" viewBox="0 0 220 140">
  <ellipse cx="110" cy="122" rx="64" ry="6" fill="#eef0f5"/>
  <rect x="48" y="36" width="124" height="68" rx="8" fill="#ffffff" stroke="#c8ced9" stroke-width="1.4"/>
  <rect x="48" y="36" width="124" height="14" rx="8" fill="#f4f6fa"/>
  <circle cx="56" cy="43" r="2.4" fill="#dde4f1"/>
  <circle cx="64" cy="43" r="2.4" fill="#dde4f1"/>
  <circle cx="72" cy="43" r="2.4" fill="#dde4f1"/>
  <path d="M94 62l28 14-28 14z" fill="#566283"/>
  <path d="M58 110h36M126 110h36" stroke="#c8ced9" stroke-width="2" stroke-linecap="round"/>
</svg>"##;

const EMPTY_TRANSCODER_DARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="220" height="140" viewBox="0 0 220 140">
  <ellipse cx="110" cy="122" rx="64" ry="6" fill="#1c2030"/>
  <rect x="48" y="36" width="124" height="68" rx="8" fill="#2a3144" stroke="#3a4258" stroke-width="1.4"/>
  <rect x="48" y="36" width="124" height="14" rx="8" fill="#363c52"/>
  <circle cx="56" cy="43" r="2.4" fill="#5d6679"/>
  <circle cx="64" cy="43" r="2.4" fill="#5d6679"/>
  <circle cx="72" cy="43" r="2.4" fill="#5d6679"/>
  <path d="M94 62l28 14-28 14z" fill="#9aa3bd"/>
  <path d="M58 110h36M126 110h36" stroke="#3a4258" stroke-width="2" stroke-linecap="round"/>
</svg>"##;

/// Warning banner — amber tint for missing-dependency alerts.
pub fn warning_banner(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(Color::from_rgba(1.0, 0.8, 0.0, 0.09))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 0.8, 0.0, 0.28),
        },
        text_color: None,
        shadow: Default::default(),
    }
}

/// Danger banner — red tint for error states.
pub fn danger_banner(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(Color::from_rgba(1.0, 0.271, 0.227, 0.08))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 0.271, 0.227, 0.28),
        },
        text_color: None,
        shadow: Default::default(),
    }
}

/// Sidebar surface.
pub fn sidebar(theme: &iced::Theme) -> container::Appearance {
    let bg = if is_light(theme) {
        Color::from_rgb8(243, 245, 249)
    } else {
        Color::from_rgb8(12, 17, 24)
    };
    container::Appearance {
        background: Some(Background::Color(bg)),
        border: Border::default(),
        text_color: None,
        shadow: Default::default(),
    }
}

/// Main background.
pub fn main_background(theme: &iced::Theme) -> container::Appearance {
    let bg = if is_light(theme) {
        Color::from_rgb8(248, 250, 252)
    } else {
        Color::from_rgb8(16, 23, 32)
    };
    container::Appearance {
        background: Some(Background::Color(bg)),
        border: Border::default(),
        text_color: Some(text_color(theme)),
        shadow: Default::default(),
    }
}

/// Code-block surface — used for the generated ffmpeg command preview.
pub fn code_block(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(if is_light(theme) {
            Color::from_rgb8(241, 245, 249)
        } else {
            Color::from_rgb8(12, 17, 24)
        })),
        border: Border {
            radius: 8.0.into(),
            width: 0.5,
            color: subtle_border(theme),
        },
        text_color: None,
        shadow: Shadow::default(),
    }
}

/// Small colored status dot — used in Manager for binary status.
pub fn status_dot_ok(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(color::success())),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: None,
        shadow: Shadow::default(),
    }
}

#[allow(dead_code)]
pub fn status_dot_warn(_theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(color::warning())),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: None,
        shadow: Shadow::default(),
    }
}

pub fn status_dot_muted(theme: &iced::Theme) -> container::Appearance {
    container::Appearance {
        background: Some(Background::Color(if is_light(theme) {
            Color::from_rgb8(200, 206, 217)
        } else {
            Color::from_rgb8(80, 88, 104)
        })),
        border: Border {
            radius: 5.0.into(),
            ..Border::default()
        },
        text_color: None,
        shadow: Shadow::default(),
    }
}

/// Status chip — colored pill for download/transcode status cells.
#[derive(Debug, Clone, Copy)]
pub enum ChipKind {
    Accent,
    Success,
    Danger,
    Neutral,
}

pub fn chip(kind: ChipKind) -> impl Fn(&iced::Theme) -> container::Appearance {
    move |theme: &iced::Theme| {
        let (bg, fg) = match kind {
            ChipKind::Accent => (color::accent_soft(), color::accent()),
            ChipKind::Success => (
                Color::from_rgba(0.255, 0.796, 0.506, 0.14),
                color::success(),
            ),
            ChipKind::Danger => (Color::from_rgba(0.937, 0.365, 0.396, 0.14), color::danger()),
            ChipKind::Neutral => (
                if is_light(theme) {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.05)
                } else {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.07)
                },
                color::secondary(),
            ),
        };
        container::Appearance {
            background: Some(Background::Color(bg)),
            border: Border {
                radius: 6.0.into(),
                ..Border::default()
            },
            text_color: Some(fg),
            shadow: Shadow::default(),
        }
    }
}

pub mod button {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    enum Kind {
        Primary,
        Secondary,
        Ghost,
        Nav(bool),
        Pill(bool),
        Danger,
    }

    #[derive(Debug, Clone, Copy)]
    struct Style {
        kind: Kind,
    }

    pub fn primary() -> iced::theme::Button {
        iced::theme::Button::custom(Style {
            kind: Kind::Primary,
        })
    }

    pub fn secondary() -> iced::theme::Button {
        iced::theme::Button::custom(Style {
            kind: Kind::Secondary,
        })
    }

    pub fn ghost() -> iced::theme::Button {
        iced::theme::Button::custom(Style { kind: Kind::Ghost })
    }

    pub fn danger() -> iced::theme::Button {
        iced::theme::Button::custom(Style { kind: Kind::Danger })
    }

    pub fn nav(active: bool) -> iced::theme::Button {
        iced::theme::Button::custom(Style {
            kind: Kind::Nav(active),
        })
    }

    pub fn pill(active: bool) -> iced::theme::Button {
        iced::theme::Button::custom(Style {
            kind: Kind::Pill(active),
        })
    }

    impl iced_button::StyleSheet for Style {
        type Style = iced::Theme;

        fn active(&self, theme: &Self::Style) -> iced_button::Appearance {
            match self.kind {
                Kind::Primary => rounded_button(
                    color::accent(),
                    Color::WHITE,
                    Color::TRANSPARENT,
                    Shadow {
                        color: color::accent_glow(),
                        offset: Vector::new(0.0, 4.0),
                        blur_radius: 14.0,
                    },
                    8.0,
                ),
                Kind::Secondary => rounded_button(
                    surface(theme),
                    text_color(theme),
                    subtle_border(theme),
                    Shadow::default(),
                    8.0,
                ),
                Kind::Ghost => iced_button::Appearance {
                    text_color: color::accent(),
                    border: Border::with_radius(8),
                    ..iced_button::Appearance::default()
                },
                Kind::Danger => rounded_button(
                    Color::from_rgba(0.94, 0.36, 0.40, 0.12),
                    color::danger(),
                    Color::from_rgba(0.94, 0.36, 0.40, 0.22),
                    Shadow::default(),
                    8.0,
                ),
                Kind::Nav(active) => {
                    if active {
                        iced_button::Appearance {
                            background: Some(Background::Color(if is_light(theme) {
                                Color::from_rgba(1.0, 0.467, 0.314, 0.12)
                            } else {
                                Color::from_rgba(1.0, 0.467, 0.314, 0.16)
                            })),
                            border: Border {
                                radius: 8.0.into(),
                                width: 1.0,
                                color: Color::from_rgba(1.0, 0.467, 0.314, 0.35),
                            },
                            text_color: if is_light(theme) {
                                Color::from_rgb8(160, 50, 25)
                            } else {
                                color::accent()
                            },
                            ..iced_button::Appearance::default()
                        }
                    } else {
                        iced_button::Appearance {
                            text_color: if is_light(theme) {
                                Color::from_rgb8(91, 101, 119)
                            } else {
                                color::secondary()
                            },
                            border: Border::with_radius(8),
                            ..iced_button::Appearance::default()
                        }
                    }
                }
                Kind::Pill(active) => {
                    if active {
                        rounded_button(
                            color::accent(),
                            Color::WHITE,
                            Color::TRANSPARENT,
                            Shadow {
                                color: color::accent_glow(),
                                offset: Vector::new(0.0, 3.0),
                                blur_radius: 10.0,
                            },
                            999.0,
                        )
                    } else {
                        iced_button::Appearance {
                            text_color: color::secondary(),
                            border: Border::with_radius(999),
                            ..iced_button::Appearance::default()
                        }
                    }
                }
            }
        }

        fn hovered(&self, theme: &Self::Style) -> iced_button::Appearance {
            let active = self.active(theme);
            match self.kind {
                Kind::Primary => iced_button::Appearance {
                    background: Some(Background::Color(color::accent_hover())),
                    shadow: Shadow {
                        color: color::accent_glow(),
                        offset: Vector::new(0.0, 6.0),
                        blur_radius: 18.0,
                    },
                    ..active
                },
                Kind::Secondary => iced_button::Appearance {
                    background: Some(Background::Color(surface_raised(theme))),
                    ..active
                },
                Kind::Ghost => iced_button::Appearance {
                    background: Some(Background::Color(color::accent_soft())),
                    ..active
                },
                Kind::Danger => iced_button::Appearance {
                    background: Some(Background::Color(Color::from_rgba(0.94, 0.36, 0.40, 0.18))),
                    ..active
                },
                Kind::Nav(active_state) => {
                    if active_state {
                        iced_button::Appearance {
                            background: Some(Background::Color(color::accent_hover())),
                            ..active
                        }
                    } else {
                        iced_button::Appearance {
                            background: Some(Background::Color(color::accent_soft())),
                            text_color: color::accent(),
                            ..active
                        }
                    }
                }
                Kind::Pill(active_state) => {
                    if active_state {
                        iced_button::Appearance {
                            background: Some(Background::Color(color::accent_hover())),
                            ..active
                        }
                    } else {
                        iced_button::Appearance {
                            background: Some(Background::Color(color::accent_soft())),
                            text_color: color::accent(),
                            ..active
                        }
                    }
                }
            }
        }

        fn pressed(&self, theme: &Self::Style) -> iced_button::Appearance {
            iced_button::Appearance {
                shadow: Shadow::default(),
                shadow_offset: Vector::default(),
                ..self.active(theme)
            }
        }
    }

    fn rounded_button(
        background: Color,
        text_color: Color,
        border_color: Color,
        shadow: Shadow,
        radius: f32,
    ) -> iced_button::Appearance {
        iced_button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                radius: radius.into(),
                width: if border_color.a == 0.0 { 0.0 } else { 1.0 },
                color: border_color,
            },
            shadow,
            ..iced_button::Appearance::default()
        }
    }
}

pub fn input() -> iced::theme::TextInput {
    iced::theme::TextInput::Custom(Box::new(InputStyle))
}

#[derive(Debug, Clone, Copy)]
struct InputStyle;

impl iced_text_input::StyleSheet for InputStyle {
    type Style = iced::Theme;

    fn active(&self, theme: &Self::Style) -> iced_text_input::Appearance {
        iced_text_input::Appearance {
            background: Background::Color(surface(theme)),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: subtle_border(theme),
            },
            icon_color: color::secondary(),
        }
    }

    fn focused(&self, theme: &Self::Style) -> iced_text_input::Appearance {
        iced_text_input::Appearance {
            border: Border {
                color: color::accent_ring(),
                width: 1.5,
                ..self.active(theme).border
            },
            ..self.active(theme)
        }
    }

    fn hovered(&self, theme: &Self::Style) -> iced_text_input::Appearance {
        iced_text_input::Appearance {
            background: Background::Color(surface_raised(theme)),
            ..self.active(theme)
        }
    }

    fn placeholder_color(&self, _theme: &Self::Style) -> Color {
        color::tertiary()
    }

    fn value_color(&self, theme: &Self::Style) -> Color {
        text_color(theme)
    }

    fn disabled_color(&self, _theme: &Self::Style) -> Color {
        color::tertiary()
    }

    fn selection_color(&self, _theme: &Self::Style) -> Color {
        color::accent_ring()
    }

    fn disabled(&self, theme: &Self::Style) -> iced_text_input::Appearance {
        iced_text_input::Appearance {
            background: Background::Color(surface_raised(theme)),
            ..self.active(theme)
        }
    }
}

/// Thin pink progress bar.
pub fn progress() -> iced::theme::ProgressBar {
    iced::theme::ProgressBar::Custom(Box::new(ProgressStyle))
}

#[derive(Debug, Clone, Copy)]
struct ProgressStyle;

impl iced_progress::StyleSheet for ProgressStyle {
    type Style = iced::Theme;
    fn appearance(&self, theme: &Self::Style) -> iced_progress::Appearance {
        iced_progress::Appearance {
            background: Background::Color(if is_light(theme) {
                Color::from_rgba(0.0, 0.0, 0.0, 0.06)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            }),
            bar: Background::Color(color::accent()),
            border_radius: 999.0.into(),
        }
    }
}
