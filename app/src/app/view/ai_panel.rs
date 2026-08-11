use super::super::Gridvana;
use super::super::ui::PANEL_BACKGROUND;
use crate::types::Message;
use iced::{Element, Length, widget};

impl Gridvana {
    pub(super) fn editor_ai_agent_inspector(&self) -> Element<'_, Message> {
        let tabs = self.editor_inspector_tabs();
        let terminal: Element<'_, Message> =
            if self.terminal_session.is_some() && self.terminal_webview_ready {
                iced_wry::webview(&self.terminal_webview).into()
            } else {
                widget::container(
                    widget::column![
                        widget::text(&self.cli_status)
                            .size(12)
                            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                        widget::button(widget::text("启动终端").size(12))
                            .padding([8, 12])
                            .style(widget::button::primary)
                            .on_press(Message::StartCliTerminal),
                    ]
                    .spacing(12)
                    .align_x(iced::Alignment::Center),
                )
                .center(Length::Fill)
                .into()
            };

        widget::opaque(
            widget::container(
                widget::column![tabs, widget::rule::horizontal(1), terminal].spacing(0),
            )
            .width(Length::Fixed(self.ai_inspector_width))
            .height(Length::Fill)
            .style(|_| widget::container::Style::default().background(PANEL_BACKGROUND)),
        )
    }
}
