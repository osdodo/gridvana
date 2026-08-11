use gridvana_core::model::Rgba;
use iced::{Background, Color, Shadow, Theme, Vector, widget};
use std::path::PathBuf;
use std::sync::LazyLock;

pub(crate) const APP_BACKGROUND: Color = Color::from_rgb8(14, 16, 21);
pub(crate) const PANEL_BACKGROUND: Color = Color::from_rgb8(19, 22, 28);
pub(crate) const SURFACE_BACKGROUND: Color = Color::from_rgb8(28, 32, 40);
pub(crate) const CONTROL_BACKGROUND: Color = Color::from_rgb8(16, 19, 24);
pub(crate) const OVERLAY_BACKGROUND: Color = Color::from_rgba8(15, 18, 23, 0.96);
pub(crate) const BORDER_SUBTLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub(crate) const BORDER_STRONG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.14);
pub(crate) const TEXT_PRIMARY: Color = Color::from_rgb8(235, 237, 242);
pub(crate) const TEXT_SECONDARY: Color = Color::from_rgba8(196, 199, 210, 0.78);
pub(crate) const TEXT_MUTED: Color = Color::from_rgba8(177, 181, 194, 0.58);
pub(crate) const ACCENT: Color = Color::from_rgb8(213, 170, 91);
pub(crate) const SUCCESS: Color = Color::from_rgb8(91, 173, 125);

static APP_THEME: LazyLock<Theme> = LazyLock::new(|| {
    Theme::custom(
        "Gridvana",
        iced::theme::Palette {
            background: PANEL_BACKGROUND,
            text: TEXT_PRIMARY,
            primary: ACCENT,
            success: Color::from_rgb8(91, 173, 125),
            warning: Color::from_rgb8(213, 164, 82),
            danger: Color::from_rgb8(216, 103, 113),
        },
    )
});

pub(super) fn app_theme() -> Theme {
    APP_THEME.clone()
}

pub(crate) fn panel_style(_theme: &Theme) -> widget::container::Style {
    widget::container::Style::default().background(PANEL_BACKGROUND)
}

pub(crate) fn text_input_style(
    theme: &Theme,
    status: widget::text_input::Status,
) -> widget::text_input::Style {
    let mut style = widget::text_input::default(theme, status);
    style.background = Background::Color(CONTROL_BACKGROUND);
    style.border = iced::Border {
        color: match status {
            widget::text_input::Status::Focused { .. } => {
                Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.82)
            }
            widget::text_input::Status::Hovered => BORDER_STRONG,
            widget::text_input::Status::Disabled => Color::TRANSPARENT,
            widget::text_input::Status::Active => BORDER_SUBTLE,
        },
        width: 1.0,
        radius: 2.0.into(),
    };
    style.value = TEXT_PRIMARY;
    style.placeholder = TEXT_MUTED;
    style.selection = Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.34);
    style
}

pub(crate) fn pick_list_style(
    _theme: &Theme,
    status: widget::pick_list::Status,
) -> widget::pick_list::Style {
    let border_color = match status {
        widget::pick_list::Status::Hovered | widget::pick_list::Status::Opened { .. } => {
            BORDER_STRONG
        }
        widget::pick_list::Status::Active => BORDER_SUBTLE,
    };

    widget::pick_list::Style {
        text_color: TEXT_PRIMARY,
        placeholder_color: TEXT_MUTED,
        handle_color: TEXT_SECONDARY,
        background: Background::Color(CONTROL_BACKGROUND),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: 2.0.into(),
        },
    }
}

pub(crate) fn pick_list_menu_style(_theme: &Theme) -> widget::overlay::menu::Style {
    widget::overlay::menu::Style {
        background: Background::Color(OVERLAY_BACKGROUND),
        border: iced::Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            // Iced reuses the menu border radius for hovered rows. Keeping it
            // square avoids a pill-shaped hover background.
            radius: 0.0.into(),
        },
        text_color: TEXT_PRIMARY,
        selected_text_color: TEXT_PRIMARY,
        selected_background: Background::Color(Color::from_rgba(
            ACCENT.r, ACCENT.g, ACCENT.b, 0.22,
        )),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
    }
}

pub(super) fn color_channel_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(super) fn color_to_hex(color: Rgba) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color_channel_u8(color.r),
        color_channel_u8(color.g),
        color_channel_u8(color.b),
        color_channel_u8(color.a)
    )
}

pub(super) fn parse_hex_color_input(input: &str) -> Option<Rgba> {
    let hex = input.trim().trim_start_matches('#');
    let (normalized, includes_alpha) = match hex.len() {
        3 | 4 => (
            hex.chars()
                .map(|c| [c, c])
                .collect::<Vec<[char; 2]>>()
                .into_iter()
                .flatten()
                .collect::<String>(),
            hex.len() == 4,
        ),
        6 | 8 => (hex.to_string(), hex.len() == 8),
        _ => return None,
    };

    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    let value = u32::from_str_radix(&normalized, 16).ok()?;
    if includes_alpha {
        Some(Rgba::new(
            ((value >> 24) & 0xff) as f32 / 255.0,
            ((value >> 16) & 0xff) as f32 / 255.0,
            ((value >> 8) & 0xff) as f32 / 255.0,
            (value & 0xff) as f32 / 255.0,
        ))
    } else {
        Some(Rgba::hex(value))
    }
}

pub(super) fn normalize_project_save_path(mut path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.push("project.gvn");
        return path;
    }
    if path.extension().and_then(|e| e.to_str()).is_none() {
        path.set_extension("gvn");
    }
    path
}

pub(super) fn compact_action_button_style(
    theme: &Theme,
    status: widget::button::Status,
    is_primary: bool,
) -> widget::button::Style {
    let (base_color, border_color, text_color) = if is_primary {
        (
            Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.90),
            Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.92),
            APP_BACKGROUND,
        )
    } else {
        (SURFACE_BACKGROUND, BORDER_SUBTLE, TEXT_PRIMARY)
    };

    let (bg, border) = match status {
        widget::button::Status::Hovered => (
            iced::Color::from_rgba(
                base_color.r,
                base_color.g,
                base_color.b,
                (base_color.a + 0.06).min(1.0),
            ),
            iced::Color::from_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                (border_color.a + 0.08).min(1.0),
            ),
        ),
        widget::button::Status::Pressed => (
            iced::Color::from_rgba(
                base_color.r,
                base_color.g,
                base_color.b,
                (base_color.a + 0.12).min(1.0),
            ),
            iced::Color::from_rgba(
                border_color.r,
                border_color.g,
                border_color.b,
                (border_color.a + 0.12).min(1.0),
            ),
        ),
        _ => (base_color, border_color),
    };

    let mut style = widget::button::text(theme, status);
    style.text_color = text_color;
    style.background = Some(bg.into());
    style.border = iced::Border {
        color: border,
        width: 1.0,
        radius: iced::border::Radius::from(2.0),
    };
    style
}

#[cfg(test)]
mod tests {
    use super::{color_to_hex, parse_hex_color_input};
    use gridvana_core::model::Rgba;

    #[test]
    fn rgba_hex_input_round_trips_alpha_and_keeps_rgb_compatibility() {
        let color = Rgba::new(
            0x12 as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x56 as f32 / 255.0,
            0.5,
        );
        assert_eq!(color_to_hex(color), "#12345680");
        assert_eq!(
            parse_hex_color_input("#12345680"),
            Some(Rgba::new(
                0x12 as f32 / 255.0,
                0x34 as f32 / 255.0,
                0x56 as f32 / 255.0,
                0x80 as f32 / 255.0,
            ))
        );
        assert_eq!(parse_hex_color_input("#abc"), Some(Rgba::hex(0xaabbcc)));
        assert_eq!(
            parse_hex_color_input("#abcd"),
            Some(Rgba::new(
                0xaa as f32 / 255.0,
                0xbb as f32 / 255.0,
                0xcc as f32 / 255.0,
                0xdd as f32 / 255.0,
            ))
        );
    }
}
