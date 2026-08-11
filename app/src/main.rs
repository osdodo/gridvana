mod app;
mod branding;
mod canvas;
mod cli_terminal;
mod color_wheel;
mod i18n;
mod icons;
mod mcp;
mod types;
mod web_terminal;

pub use types::{Message, Tool};

pub fn main() -> iced::Result {
    iced::application(
        app::Gridvana::new,
        app::Gridvana::update,
        app::Gridvana::view,
    )
    .window(iced::window::Settings {
        maximized: true,
        icon: Some(branding::window_icon()),
        ..Default::default()
    })
    .subscription(app::Gridvana::subscription)
    .title("Gridvana Studio")
    .theme(app::Gridvana::theme)
    .run()
}
