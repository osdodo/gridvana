#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod branding;
mod canvas;
mod cli_terminal;
mod color_picker;
mod i18n;
mod icons;
mod mcp;
mod types;
mod web_terminal;
#[cfg(windows)]
mod windows_integration;

pub use types::{Message, Tool};

pub fn main() -> iced::Result {
    #[cfg(windows)]
    windows_integration::initialize();

    let mut window_settings = iced::window::Settings {
        maximized: true,
        min_size: Some(iced::Size::new(640.0, 480.0)),
        icon: Some(branding::window_icon()),
        ..Default::default()
    };
    #[cfg(windows)]
    {
        window_settings.decorations = false;
        window_settings.platform_specific.undecorated_shadow = true;
    }

    iced::application(
        app::Gridvana::new,
        app::Gridvana::update,
        app::Gridvana::view,
    )
    .window(window_settings)
    .subscription(app::Gridvana::subscription)
    .title("Gridvana")
    .theme(app::Gridvana::theme)
    .run()
}
