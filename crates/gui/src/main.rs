#![windows_subsystem = "windows"]

mod app;
mod panes;
mod theme;

use iced::{Application, Settings};

fn main() -> iced::Result {
    // Redirect stdout/stderr to /dev/null in release builds on macOS to avoid console noise
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    {
        if let Ok(null) = std::fs::File::create("/dev/null") {
            use std::os::unix::io::AsRawFd;
            let fd = null.as_raw_fd();
            unsafe {
                libc::dup2(fd, 1);
                libc::dup2(fd, 2);
            }
        }
    }

    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1100.0, 750.0),
            min_size: Some(iced::Size::new(900.0, 600.0)),
            transparent: false,
            #[cfg(target_os = "macos")]
            platform_specific: iced::window::settings::PlatformSpecific {
                title_hidden: true,
                titlebar_transparent: true,
                fullsize_content_view: true,
            },
            ..Default::default()
        },
        default_font: iced::Font {
            family: iced::font::Family::SansSerif,
            ..iced::Font::DEFAULT
        },
        ..Default::default()
    };
    app::App::run(settings)
}
