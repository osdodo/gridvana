use super::super::Gridvana;
use super::super::ui::{
    ACCENT, APP_BACKGROUND, BORDER_STRONG, BORDER_SUBTLE, CONTROL_BACKGROUND, PANEL_BACKGROUND,
    SURFACE_BACKGROUND, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, compact_action_button_style,
};
use crate::branding;
use crate::icons::{Icon, icon_button};
use crate::types::Message;
use iced::{Border, Color, Element, Length, Shadow, Size, Theme, Vector, widget};

const CARD_BREAKPOINT: f32 = 560.0;

impl Gridvana {
    pub(super) fn about_overlay(&self) -> Option<Element<'_, Message>> {
        if !self.about_dialog_open {
            return None;
        }

        Some(widget::opaque(
            widget::container(widget::responsive(move |size| {
                widget::container(self.about_card(size))
                    .center(Length::Fill)
                    .into()
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24)
            .style(about_overlay_style()),
        ))
    }

    fn about_card(&self, size: Size) -> Element<'_, Message> {
        let compact = size.width < CARD_BREAKPOINT;
        let logo_side = if compact { 76.0 } else { 94.0 };
        let logo = widget::container(
            widget::image(branding::logo_handle())
                .width(Length::Fixed(logo_side))
                .height(Length::Fixed(logo_side)),
        )
        .width(Length::Fixed(logo_side + 24.0))
        .height(Length::Fixed(logo_side + 24.0))
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center)
        .style(logo_tile_style());

        let version = widget::container(
            widget::text(format!("VERSION {}", env!("CARGO_PKG_VERSION")))
                .size(9)
                .color(ACCENT),
        )
        .padding([5, 8])
        .style(version_badge_style());

        let identity = widget::column![
            widget::text("GRIDVANA / CREATIVE WORKSPACE")
                .size(10)
                .color(TEXT_MUTED),
            widget::text("Gridvana")
                .size(if compact { 31 } else { 38 })
                .color(TEXT_PRIMARY),
            widget::text("像素艺术与逐帧动画工作台")
                .size(14)
                .color(TEXT_SECONDARY),
            version,
        ]
        .spacing(7)
        .align_x(if compact {
            iced::Alignment::Center
        } else {
            iced::Alignment::Start
        });

        let hero: Element<'_, Message> = if compact {
            widget::column![logo, identity]
                .spacing(16)
                .align_x(iced::Alignment::Center)
                .width(Length::Fill)
                .into()
        } else {
            widget::row![logo, identity]
                .spacing(24)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .into()
        };

        let statement = widget::container(
            widget::text(
                "为像素创作者打造的专注型工作空间：从网格绘制、图层与时间轴，到可交付的动画资产和 AI 辅助工作流。",
            )
            .size(13)
            .line_height(1.55)
            .color(TEXT_SECONDARY)
            .wrapping(widget::text::Wrapping::WordOrGlyph),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(statement_style());

        let capabilities: Element<'_, Message> = if compact {
            widget::column![
                capability("01", "PIXEL STUDIO", "精确的网格绘制、选区与图层控制"),
                capability("02", "ANIMATION", "时间轴、洋葱皮与专业资产导出"),
                capability("03", "AGENT WORKFLOW", "内置终端与 MCP 画布协作能力"),
            ]
            .spacing(8)
            .into()
        } else {
            widget::row![
                capability("01", "PIXEL STUDIO", "精确的网格绘制、选区与图层控制"),
                capability("02", "ANIMATION", "时间轴、洋葱皮与专业资产导出"),
                capability("03", "AGENT WORKFLOW", "内置终端与 MCP 画布协作能力"),
            ]
            .spacing(8)
            .into()
        };

        let footer: Element<'_, Message> = if compact {
            widget::column![
                widget::column![
                    widget::text("RUST NATIVE · .GVN SCHEMA V6 · RGBA")
                        .size(9)
                        .color(TEXT_MUTED),
                    widget::text("© 2026 Gridvana. All rights reserved.")
                        .size(9)
                        .color(TEXT_MUTED),
                ]
                .spacing(4)
                .align_x(iced::Alignment::Center),
                done_button(),
            ]
            .spacing(14)
            .align_x(iced::Alignment::Center)
            .width(Length::Fill)
            .into()
        } else {
            widget::row![
                widget::column![
                    widget::text("RUST NATIVE · .GVN SCHEMA V6 · RGBA")
                        .size(9)
                        .color(TEXT_MUTED),
                    widget::text("© 2026 Gridvana. All rights reserved.")
                        .size(9)
                        .color(TEXT_MUTED),
                ]
                .spacing(4)
                .width(Length::Fill),
                done_button(),
            ]
            .spacing(18)
            .align_y(iced::Alignment::Center)
            .into()
        };

        let card = widget::column![
            widget::row![
                widget::text("ABOUT GRIDVANA").size(9).color(TEXT_MUTED),
                widget::Space::new().width(Length::Fill),
                icon_button(Icon::CloseCircle, 14.0, 28.0, false, true)
                    .on_press(Message::CloseAbout),
            ]
            .align_y(iced::Alignment::Center),
            hero,
            statement,
            capabilities,
            widget::rule::horizontal(1),
            footer,
        ]
        .spacing(if compact { 18 } else { 22 });

        let content = widget::container(card)
            .padding(if compact { 20 } else { 28 })
            .width(Length::Fill);
        let body: Element<'_, Message> = if compact {
            widget::scrollable(content).height(Length::Fill).into()
        } else {
            content.into()
        };
        let shell = widget::container(body)
            .width(Length::Fill)
            .max_width(720.0)
            .style(about_card_style());

        if compact {
            shell.height(Length::Fill).into()
        } else {
            shell.into()
        }
    }
}

fn capability<'a>(
    number: &'static str,
    title: &'static str,
    body: &'static str,
) -> Element<'a, Message> {
    widget::container(
        widget::column![
            widget::row![
                widget::text(number).size(10).color(ACCENT),
                widget::Space::new().width(Length::Fill),
                widget::container(widget::Space::new())
                    .width(Length::Fixed(18.0))
                    .height(Length::Fixed(2.0))
                    .style(|_| widget::container::Style::default().background(ACCENT)),
            ]
            .align_y(iced::Alignment::Center),
            widget::text(title).size(11).color(TEXT_PRIMARY),
            widget::text(body)
                .size(10)
                .line_height(1.45)
                .color(TEXT_MUTED)
                .wrapping(widget::text::Wrapping::WordOrGlyph),
        ]
        .spacing(8),
    )
    .padding(13)
    .width(Length::Fill)
    .style(capability_style())
    .into()
}

fn done_button<'a>() -> Element<'a, Message> {
    widget::button(widget::text("完成").size(11))
        .on_press(Message::CloseAbout)
        .padding([8, 22])
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, true))
        .into()
}

fn about_overlay_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| {
        widget::container::Style::default().background(Color::from_rgba(
            APP_BACKGROUND.r,
            APP_BACKGROUND.g,
            APP_BACKGROUND.b,
            0.88,
        ))
    }
}

fn about_card_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| widget::container::Style {
        background: Some(PANEL_BACKGROUND.into()),
        border: Border {
            color: BORDER_STRONG,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.42),
            offset: Vector::new(0.0, 14.0),
            blur_radius: 42.0,
        },
        ..Default::default()
    }
}

fn logo_tile_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| {
        widget::container::Style::default()
            .background(CONTROL_BACKGROUND)
            .border(Border {
                color: Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.42),
                width: 1.0,
                radius: 18.0.into(),
            })
    }
}

fn version_badge_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| {
        widget::container::Style::default()
            .background(Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.09))
            .border(Border {
                color: Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.28),
                width: 1.0,
                radius: 4.0.into(),
            })
    }
}

fn statement_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| {
        widget::container::Style::default()
            .background(CONTROL_BACKGROUND)
            .border(Border {
                color: BORDER_SUBTLE,
                width: 1.0,
                radius: 8.0.into(),
            })
    }
}

fn capability_style() -> impl Fn(&Theme) -> widget::container::Style {
    |_| {
        widget::container::Style::default()
            .background(SURFACE_BACKGROUND)
            .border(Border {
                color: BORDER_SUBTLE,
                width: 1.0,
                radius: 7.0.into(),
            })
    }
}
