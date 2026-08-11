use super::super::ui::{
    ACCENT, APP_BACKGROUND, BORDER_STRONG, BORDER_SUBTLE, CONTROL_BACKGROUND, SURFACE_BACKGROUND,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, pick_list_menu_style, pick_list_style,
};
use super::super::{Gridvana, PendingRecovery};
use crate::i18n::tr;
use crate::types::{
    InspectorPanel, Message, SpriteEmptyChoice, SpriteFrameRangeChoice, SpriteLayerRangeChoice,
    SpriteLayoutChoice, SpriteMetadataChoice, SpriteTrimChoice,
};
use iced::{Background, Border, Color, Element, Length, Shadow, Size, Theme, Vector, widget};

impl Gridvana {
    pub(super) fn recovery_overlay(&self) -> Option<Element<'_, Message>> {
        let pending = self.pending_recovery.as_ref()?;
        let (title, description, can_recover) = match pending {
            PendingRecovery::Available(recovery) => {
                let source = recovery
                    .project_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| tr("Untitled project", "未命名项目").to_string());
                (
                    tr("Unsaved project found", "发现未保存的项目"),
                    format!("{}: {source}", tr("Recovery source", "恢复来源")),
                    true,
                )
            }
            PendingRecovery::Damaged(error) => (
                tr("Recovery file is damaged", "恢复文件已损坏"),
                format!(
                    "{}: {error}",
                    tr("Could not read autosave recovery", "无法读取自动恢复内容")
                ),
                false,
            ),
        };

        let discard = widget::button(
            widget::container(widget::text(if can_recover {
                tr("Discard recovery file", "丢弃恢复文件")
            } else {
                tr("Discard damaged file", "丢弃损坏文件")
            }))
            .width(Length::Fill)
            .align_x(iced::Alignment::Center),
        )
        .on_press(Message::DiscardAutosave)
        .padding([9, 12])
        .width(Length::FillPortion(1))
        .style(start_action_button_style(false));

        let actions: Element<'_, Message> = if can_recover {
            let recover = widget::button(
                widget::container(widget::text(tr("Recover project", "恢复项目")))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .on_press(Message::RecoverAutosave)
            .padding([9, 12])
            .width(Length::FillPortion(1))
            .style(start_action_button_style(true));
            widget::row![recover, discard].spacing(10).into()
        } else {
            discard.into()
        };

        let card = widget::container(
            widget::column![
                widget::text(title).size(20).color(TEXT_PRIMARY),
                widget::text(description)
                    .size(11)
                    .color(TEXT_SECONDARY)
                    .width(Length::Fill),
                actions,
            ]
            .spacing(16),
        )
        .padding(20)
        .width(Length::Fill)
        .max_width(440.0)
        .style(start_card_style());

        Some(widget::opaque(
            widget::container(widget::center(card))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(28)
                .style(start_overlay_style()),
        ))
    }

    pub(super) fn sprite_sheet_export_overlay(&self) -> Option<Element<'_, Message>> {
        if self.inspector_panel == InspectorPanel::Export {
            return None;
        }
        let png_path = self.pending_sprite_sheet_export_path.as_ref()?;
        let mut json_path = png_path.clone();
        json_path.set_extension("json");
        let form = self.sprite_sheet_export_form;
        let estimate_valid = self.sprite_sheet_export_estimate.is_ok();
        let estimate = match &self.sprite_sheet_export_estimate {
            Ok((width, height)) => format!(
                "{}: {width} × {height} px",
                tr("Estimated size", "预计尺寸")
            ),
            Err(error) => format!("{}: {error}", tr("Cannot export", "无法导出")),
        };
        let first_row = widget::row![
            export_select_field(
                tr("Frame range", "帧范围"),
                widget::pick_list(
                    SpriteFrameRangeChoice::ALL,
                    Some(form.frame_range),
                    Message::SelectSpriteFrameRange,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
            export_select_field(
                tr("Layer range", "图层范围"),
                widget::pick_list(
                    SpriteLayerRangeChoice::ALL,
                    Some(form.layer_range),
                    Message::SelectSpriteLayerRange,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
            export_select_field(
                tr("Layout", "布局"),
                widget::pick_list(
                    SpriteLayoutChoice::ALL,
                    Some(form.layout),
                    Message::SelectSpriteLayout,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
        ]
        .spacing(10);
        let second_row = widget::row![
            export_select_field(
                tr("Trim", "裁切"),
                widget::pick_list(
                    SpriteTrimChoice::ALL,
                    Some(form.trim),
                    Message::SelectSpriteTrim,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
            export_select_field(
                tr("Empty frames", "空帧"),
                widget::pick_list(
                    SpriteEmptyChoice::ALL,
                    Some(form.empty),
                    Message::SelectSpriteEmpty,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
            export_select_field(
                tr("Metadata", "元数据"),
                widget::pick_list(
                    SpriteMetadataChoice::ALL,
                    Some(form.metadata),
                    Message::SelectSpriteMetadata,
                )
                .width(Length::Fill)
                .style(pick_list_style)
                .menu_style(pick_list_menu_style)
                .into(),
            ),
        ]
        .spacing(10);
        let slider = |label: String,
                      range: std::ops::RangeInclusive<u8>,
                      value: u8,
                      on_change: fn(u8) -> Message| {
            widget::column![
                widget::text(label).size(10).color(TEXT_SECONDARY),
                widget::slider(range, value, on_change).width(Length::Fill),
            ]
            .spacing(4)
            .width(Length::Fill)
        };
        let numeric_row = widget::row![
            slider(
                format!("{} {}×", tr("Scale", "倍率"), form.scale),
                1..=16,
                form.scale,
                Message::SetSpriteScale,
            ),
            slider(
                format!("{} {}", tr("Rows/columns", "行/列数"), form.fixed_count),
                1..=16,
                form.fixed_count,
                Message::SetSpriteFixedCount,
            ),
            slider(
                format!("Padding {}", form.padding),
                0..=32,
                form.padding,
                Message::SetSpritePadding,
            ),
        ]
        .spacing(10);
        let spacing_row = widget::row![
            slider(
                format!("Spacing {}", form.spacing),
                0..=32,
                form.spacing,
                Message::SetSpriteSpacing,
            ),
            slider(
                format!("Border {}", form.border),
                0..=32,
                form.border,
                Message::SetSpriteBorder,
            ),
            slider(
                format!("Extrude {}", form.extrude),
                0..=form.padding,
                form.extrude,
                Message::SetSpriteExtrude,
            ),
        ]
        .spacing(10);
        let confirm = widget::button(
            widget::container(widget::text(tr("Export PNG + JSON", "导出 PNG + JSON")).size(12))
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .padding([9, 12])
        .width(Length::FillPortion(1))
        .style(start_action_button_style(true));
        let confirm: Element<'_, Message> = if estimate_valid {
            confirm.on_press(Message::ConfirmSpriteSheetExport).into()
        } else {
            confirm.into()
        };
        let cancel = widget::button(
            widget::container(widget::text(tr("Cancel", "取消")).size(12))
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press(Message::CancelSpriteSheetExport)
        .padding([9, 12])
        .width(Length::FillPortion(1))
        .style(start_action_button_style(false));
        let card = widget::container(
            widget::column![
                widget::text(tr("Advanced Sprite Sheet export", "高级 Sprite Sheet 导出"))
                    .size(20)
                    .color(TEXT_PRIMARY),
                widget::text(format!("PNG：{}", png_path.display()))
                    .size(10)
                    .color(TEXT_MUTED),
                widget::text(format!("JSON：{}", json_path.display()))
                    .size(10)
                    .color(TEXT_MUTED),
                first_row,
                second_row,
                numeric_row,
                spacing_row,
                widget::text(estimate).size(11).color(if estimate_valid {
                    TEXT_SECONDARY
                } else {
                    Color::from_rgb(1.0, 0.4, 0.35)
                }),
                widget::row![confirm, cancel].spacing(10),
            ]
            .spacing(13),
        )
        .padding(20)
        .width(Length::Fill)
        .max_width(680.0)
        .style(start_card_style());

        Some(widget::opaque(
            widget::container(widget::center(card))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(28)
                .style(start_overlay_style()),
        ))
    }

    pub(super) fn new_project_overlay(&self) -> Option<Element<'_, Message>> {
        if !self.new_project_dialog_open {
            return None;
        }

        Some(widget::opaque(
            widget::container(widget::responsive(move |size| {
                let stacked_inputs = size.width < 560.0;

                widget::container(self.new_project_form_card(size, stacked_inputs))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .padding(28)
                    .into()
            }))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(start_overlay_style()),
        ))
    }

    fn new_project_form_card(&self, size: Size, stacked_inputs: bool) -> Element<'_, Message> {
        let (width_valid, height_valid, project_ready) = self.new_project_form_state();
        let stacked_actions = size.width < 420.0;

        let width_field = start_dimension_field(
            tr("Width", "宽度"),
            &self.new_project_width,
            width_valid,
            Message::UpdateCanvasWidth,
        );
        let height_field = start_dimension_field(
            tr("Height", "高度"),
            &self.new_project_height,
            height_valid,
            Message::UpdateCanvasHeight,
        );

        let size_fields: Element<'_, Message> = if stacked_inputs {
            widget::column![width_field, height_field]
                .spacing(10)
                .into()
        } else {
            widget::row![width_field, height_field]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into()
        };

        let create_button: Element<'_, Message> = if project_ready {
            widget::button(
                widget::container(widget::text(tr("Create canvas", "创建画布")).size(12))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .padding([9, 12])
            .width(Length::FillPortion(1))
            .style(start_action_button_style(true))
            .on_press(Message::CreateNewProject)
            .into()
        } else {
            widget::button(
                widget::container(widget::text(tr("Create canvas", "创建画布")).size(12))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Center),
            )
            .padding([9, 12])
            .width(Length::FillPortion(1))
            .style(start_action_button_style(true))
            .into()
        };

        let secondary_button = widget::button(
            widget::container(widget::text(tr("Cancel", "取消")).size(12))
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press(Message::CloseNewProjectDialog)
        .padding([9, 12])
        .width(Length::FillPortion(1))
        .style(start_action_button_style(false));

        let mut form = widget::column![
            widget::column![
                widget::text(tr("Create canvas", "创建画布"))
                    .size(20)
                    .color(TEXT_PRIMARY),
                widget::text(tr("Set the canvas size", "设置画布尺寸"))
                    .size(11)
                    .color(TEXT_MUTED),
            ]
            .spacing(4),
            size_fields,
        ]
        .spacing(16)
        .align_x(iced::Alignment::Start);

        if !project_ready {
            form = form.push(
                widget::text(tr("Size must be between 1 and 4096", "尺寸范围 1 - 4096"))
                    .size(11)
                    .color(Color::from_rgb8(225, 125, 133)),
            );
        }

        let action_row: Element<'_, Message> = if stacked_actions {
            widget::column![create_button, secondary_button]
                .spacing(10)
                .into()
        } else {
            widget::row![create_button, secondary_button]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .into()
        };

        form = form.push(action_row);

        widget::container(form)
            .padding(20)
            .width(Length::Fill)
            .max_width(360.0)
            .style(start_card_style())
            .into()
    }

    fn new_project_form_state(&self) -> (bool, bool, bool) {
        let width_valid = self
            .new_project_width
            .trim()
            .parse::<u32>()
            .map(|value| (1..=4096).contains(&value))
            .unwrap_or(false);
        let height_valid = self
            .new_project_height
            .trim()
            .parse::<u32>()
            .map(|value| (1..=4096).contains(&value))
            .unwrap_or(false);

        (width_valid, height_valid, width_valid && height_valid)
    }
}

fn start_dimension_field<'a>(
    label: &'a str,
    value: &'a str,
    valid: bool,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    widget::column![
        widget::text(label).size(11).color(TEXT_SECONDARY),
        widget::text_input("1 - 4096", value)
            .on_input(on_input)
            .padding([9, 11])
            .size(14)
            .style(start_input_style(valid))
            .width(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

fn export_select_field<'a>(
    label: &'static str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    widget::column![widget::text(label).size(10).color(TEXT_SECONDARY), control,]
        .spacing(4)
        .width(Length::Fill)
        .into()
}

fn start_overlay_style() -> impl Fn(&Theme) -> widget::container::Style {
    move |_theme: &Theme| {
        widget::container::Style::default().background(Color::from_rgba(
            APP_BACKGROUND.r,
            APP_BACKGROUND.g,
            APP_BACKGROUND.b,
            0.80,
        ))
    }
}

fn start_card_style() -> impl Fn(&Theme) -> widget::container::Style {
    move |_theme: &Theme| widget::container::Style {
        background: Some(SURFACE_BACKGROUND.into()),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: iced::border::Radius::from(12.0),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.22),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn start_input_style(
    valid: bool,
) -> impl Fn(&Theme, widget::text_input::Status) -> widget::text_input::Style {
    move |theme: &Theme, status| {
        let mut style = widget::text_input::default(theme, status);
        style.background = Background::Color(CONTROL_BACKGROUND);
        style.border.radius = iced::border::Radius::from(8.0);
        style.border.width = 1.0;
        style.border.color = match status {
            widget::text_input::Status::Focused { .. } => {
                Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.82)
            }
            widget::text_input::Status::Hovered => BORDER_STRONG,
            widget::text_input::Status::Disabled => Color::TRANSPARENT,
            _ if valid => BORDER_SUBTLE,
            _ => Color::from_rgba8(225, 125, 133, 0.72),
        };
        style.value = TEXT_PRIMARY;
        style.placeholder = TEXT_MUTED;
        style.selection = Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.34);
        style
    }
}

fn start_action_button_style(
    is_primary: bool,
) -> impl Fn(&Theme, widget::button::Status) -> widget::button::Style {
    move |theme: &Theme, status| {
        let (base, border, text) = if is_primary {
            (
                Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.92),
                Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.96),
                TEXT_PRIMARY,
            )
        } else {
            (CONTROL_BACKGROUND, BORDER_SUBTLE, TEXT_PRIMARY)
        };

        let (background, border_color) = match status {
            widget::button::Status::Hovered => (
                Color::from_rgba(base.r, base.g, base.b, (base.a + 0.05).min(1.0)),
                Color::from_rgba(border.r, border.g, border.b, (border.a + 0.08).min(1.0)),
            ),
            widget::button::Status::Pressed => (
                Color::from_rgba(base.r, base.g, base.b, (base.a + 0.10).min(1.0)),
                Color::from_rgba(border.r, border.g, border.b, (border.a + 0.12).min(1.0)),
            ),
            widget::button::Status::Disabled => (
                Color::from_rgba(base.r, base.g, base.b, 0.28),
                Color::from_rgba(border.r, border.g, border.b, 0.12),
            ),
            _ => (base, border),
        };

        let mut style = widget::button::text(theme, status);
        style.text_color = text;
        style.background = Some(background.into());
        style.border = Border {
            color: border_color,
            width: 1.0,
            radius: iced::border::Radius::from(8.0),
        };
        style.shadow = Shadow::default();
        style
    }
}
