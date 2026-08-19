use super::super::Gridvana;
use super::super::ui::{
    BORDER_SUBTLE, OVERLAY_BACKGROUND, PANEL_BACKGROUND, SURFACE_BACKGROUND, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::branding;
use crate::i18n::tr;
use crate::types::Message;
use iced::{Background, Element, Length, Theme, mouse, widget};

const TITLE_BAR_HEIGHT: f32 = 40.0;
const MENU_LEFT: f32 = 104.0;
const MENU_WIDTH: f32 = 210.0;

impl Gridvana {
    pub(super) fn window_title_bar(&self) -> Element<'_, Message> {
        let brand = widget::mouse_area(
            widget::row![
                widget::image(branding::logo_handle())
                    .width(Length::Fixed(19.0))
                    .height(Length::Fixed(19.0)),
                widget::text("Gridvana").size(13).color(TEXT_PRIMARY),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::BeginWindowDrag)
        .on_double_click(Message::ToggleWindowMaximized)
        .interaction(mouse::Interaction::Grab);

        let menu_toggle = widget::button(
            widget::container(widget::text("☰").size(15).color(TEXT_MUTED))
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill),
        )
        .on_press(Message::ToggleAppMenu)
        .width(Length::Fixed(30.0))
        .height(Length::Fill)
        .padding(0)
        .style(title_button_style(false, self.app_menu_open));

        let drag_region = widget::mouse_area(widget::Space::new().width(Length::Fill))
            .on_press(Message::BeginWindowDrag)
            .on_double_click(Message::ToggleWindowMaximized)
            .interaction(mouse::Interaction::Grab);

        let minimize = window_control("−", Message::MinimizeWindow, false);
        let maximize = window_control("□", Message::ToggleWindowMaximized, false);
        let close = window_control("×", Message::ExitApplication, true);

        widget::container(
            widget::row![brand, menu_toggle, drag_region, minimize, maximize, close]
                .height(Length::Fill)
                .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 20.0,
        })
        .style(|_| {
            widget::container::Style::default()
                .background(PANEL_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 0.0,
                    radius: 0.0.into(),
                })
        })
        .into()
    }

    pub(super) fn app_menu_overlay(&self) -> Option<Element<'_, Message>> {
        if !self.app_menu_open {
            return None;
        }

        let menu = widget::container(widget::column![
            menu_item(tr("New Canvas", "创建画布"), Message::OpenNewProjectDialog),
            menu_item(tr("Open...", "打开..."), Message::OpenProject),
            menu_item(tr("Save", "保存"), Message::SaveProject),
            menu_separator(),
            menu_item(tr("Settings...", "设置..."), Message::OpenCliSettings),
            menu_item(tr("About", "关于"), Message::OpenAbout),
            menu_separator(),
            menu_item(tr("Exit", "退出"), Message::ExitApplication),
        ])
        .width(Length::Fixed(MENU_WIDTH))
        .padding(4)
        .style(|_| {
            widget::container::Style::default()
                .background(OVERLAY_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 2.0.into(),
                })
                .shadow(iced::Shadow {
                    color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.32),
                    offset: iced::Vector::new(0.0, 6.0),
                    blur_radius: 16.0,
                })
        });

        let positioned = widget::column![
            widget::Space::new().height(Length::Fixed(TITLE_BAR_HEIGHT)),
            widget::row![
                widget::Space::new().width(Length::Fixed(MENU_LEFT)),
                menu,
                widget::Space::new().width(Length::Fill),
            ],
            widget::Space::new().height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        Some(
            widget::opaque(
                widget::mouse_area(positioned)
                    .on_press(Message::CloseAppMenu)
                    .on_right_press(Message::CloseAppMenu),
            )
            .into(),
        )
    }
}

fn window_control(
    label: &'static str,
    message: Message,
    destructive: bool,
) -> Element<'static, Message> {
    widget::button(
        widget::container(widget::text(label).size(16).color(TEXT_PRIMARY))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
    )
    .on_press(message)
    .width(Length::Fixed(46.0))
    .height(Length::Fill)
    .padding(0)
    .style(title_button_style(destructive, false))
    .into()
}

fn title_button_style(
    destructive: bool,
    selected: bool,
) -> impl Fn(&Theme, widget::button::Status) -> widget::button::Style {
    move |theme, status| {
        let mut style = widget::button::text(theme, status);
        style.background = match status {
            widget::button::Status::Hovered if destructive => {
                Some(Background::Color(iced::Color::from_rgb8(196, 43, 28)))
            }
            widget::button::Status::Pressed if destructive => {
                Some(Background::Color(iced::Color::from_rgb8(161, 35, 24)))
            }
            widget::button::Status::Hovered | widget::button::Status::Pressed => {
                Some(Background::Color(SURFACE_BACKGROUND))
            }
            _ if selected => Some(Background::Color(SURFACE_BACKGROUND)),
            _ => Some(Background::Color(iced::Color::TRANSPARENT)),
        };
        style.border = iced::Border::default();
        style
    }
}

fn menu_item(label: &'static str, message: Message) -> Element<'static, Message> {
    widget::button(
        widget::container(widget::text(label).size(12).color(TEXT_PRIMARY))
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
    )
    .on_press(message)
    .padding([7, 10])
    .width(Length::Fill)
    .style(|theme: &Theme, status| {
        let mut style = widget::button::text(theme, status);
        if matches!(
            status,
            widget::button::Status::Hovered | widget::button::Status::Pressed
        ) {
            style.background = Some(Background::Color(SURFACE_BACKGROUND));
        }
        style.border = iced::Border::default();
        style
    })
    .into()
}

fn menu_separator() -> Element<'static, Message> {
    widget::container(widget::Space::new())
        .height(Length::Fixed(1.0))
        .width(Length::Fill)
        .style(|_| widget::container::Style::default().background(BORDER_SUBTLE))
        .into()
}
