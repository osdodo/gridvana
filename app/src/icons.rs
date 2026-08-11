use iced::{
    Element, Length, Theme,
    widget::{self, svg::Handle},
};

use crate::Message;
use crate::app::ui::{ACCENT, TEXT_SECONDARY};

#[derive(Debug, Clone, Copy)]
pub enum Icon {
    HandPoint,
    MagicWand,
    ColorSelect,
    Brush,
    Eraser,
    PaintBucket,
    Picker,
    Rectangle,
    RectangleHollow,
    Circle,
    CircleHollow,
    Line,
    Eye,
    EyeSlash,
    Play,
    Pause,
    Add,
    Copy,
    Remove,
    CloseCircle,
}

fn icon_handle(icon: Icon) -> Handle {
    match icon {
        Icon::HandPoint => {
            Handle::from_memory(include_bytes!("../assets/hand-point.svg").as_slice())
        }
        Icon::MagicWand => {
            Handle::from_memory(include_bytes!("../assets/magic-wand.svg").as_slice())
        }
        Icon::ColorSelect => {
            Handle::from_memory(include_bytes!("../assets/color-select.svg").as_slice())
        }
        Icon::Brush => Handle::from_memory(include_bytes!("../assets/brush.svg").as_slice()),
        Icon::Eraser => Handle::from_memory(include_bytes!("../assets/eraser.svg").as_slice()),
        Icon::PaintBucket => {
            Handle::from_memory(include_bytes!("../assets/paint-bucket.svg").as_slice())
        }
        Icon::Picker => Handle::from_memory(include_bytes!("../assets/picker-half.svg").as_slice()),
        Icon::Rectangle => {
            Handle::from_memory(include_bytes!("../assets/rectangle.svg").as_slice())
        }
        Icon::RectangleHollow => {
            Handle::from_memory(include_bytes!("../assets/rectangle-hollow.svg").as_slice())
        }
        Icon::Circle => Handle::from_memory(include_bytes!("../assets/circle.svg").as_slice()),
        Icon::CircleHollow => {
            Handle::from_memory(include_bytes!("../assets/circle-hollow.svg").as_slice())
        }
        Icon::Line => Handle::from_memory(include_bytes!("../assets/line.svg").as_slice()),
        Icon::Eye => Handle::from_memory(include_bytes!("../assets/eye.svg").as_slice()),
        Icon::EyeSlash => Handle::from_memory(include_bytes!("../assets/eye-slash.svg").as_slice()),
        Icon::Play => Handle::from_memory(include_bytes!("../assets/play-circle.svg").as_slice()),
        Icon::Pause => Handle::from_memory(include_bytes!("../assets/pause-circle.svg").as_slice()),
        Icon::Add => {
            Handle::from_memory(include_bytes!("../assets/add-plus-square.svg").as_slice())
        }
        Icon::Copy => Handle::from_memory(include_bytes!("../assets/copy.svg").as_slice()),
        Icon::Remove => {
            Handle::from_memory(include_bytes!("../assets/add-minus-square.svg").as_slice())
        }
        Icon::CloseCircle => {
            Handle::from_memory(include_bytes!("../assets/close-circle.svg").as_slice())
        }
    }
}

fn icon_svg(icon: Icon, size: f32, is_active: bool) -> Element<'static, Message> {
    let svg = widget::svg::Svg::new(icon_handle(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_theme: &Theme, _status| {
            let color = if is_active {
                iced::Color::WHITE
            } else {
                TEXT_SECONDARY
            };
            widget::svg::Style { color: Some(color) }
        });
    svg.into()
}

pub fn icon_button(
    icon: Icon,
    icon_size: f32,
    button_size: f32,
    is_active: bool,
    show_background: bool,
) -> widget::Button<'static, Message> {
    let content = widget::container(icon_svg(icon, icon_size, is_active))
        .width(Length::Fixed(button_size))
        .height(Length::Fixed(button_size))
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center)
        .style(move |_theme| {
            if show_background && is_active {
                let border = iced::Border {
                    color: iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.52),
                    width: 1.0,
                    radius: iced::border::Radius::from(2.0_f32),
                };
                widget::container::Style {
                    background: Some(
                        iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.22).into(),
                    ),
                    border,
                    ..Default::default()
                }
            } else {
                widget::container::Style::default()
            }
        });
    widget::button(content)
        .padding(0)
        .style(move |theme, status| {
            let mut style = widget::button::text(theme, status);

            if show_background {
                let bg = match status {
                    widget::button::Status::Hovered => iced::Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                    widget::button::Status::Pressed => iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    _ => iced::Color::TRANSPARENT,
                };
                style.background = Some(bg.into());
                style.border = iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: iced::border::Radius::from(2.0),
                };
            }

            style
        })
}
