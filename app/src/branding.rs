pub const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");

use std::sync::LazyLock;

static LOGO_HANDLE: LazyLock<iced::widget::image::Handle> =
    LazyLock::new(|| iced::widget::image::Handle::from_bytes(LOGO_PNG));

pub fn logo_handle() -> iced::widget::image::Handle {
    LOGO_HANDLE.clone()
}

pub fn window_icon() -> iced::window::Icon {
    iced::window::icon::from_file_data(LOGO_PNG, None)
        .expect("the embedded Gridvana logo must be a valid image")
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_logo_decodes_as_a_window_icon() {
        let _ = super::window_icon();
    }
}
