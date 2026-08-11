use super::super::Gridvana;
use super::super::ui::{
    BORDER_SUBTLE, CONTROL_BACKGROUND, PANEL_BACKGROUND, SUCCESS, SURFACE_BACKGROUND, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY, compact_action_button_style, panel_style, pick_list_menu_style,
    pick_list_style,
};
use crate::types::{
    Message, SpriteFrameRangeChoice, SpriteLayerRangeChoice, SpriteLayoutChoice,
    SpriteMetadataChoice, SpriteTrimChoice,
};
use gridvana_core::model::TagDirection;
use iced::{Element, Length, Theme, widget};

const LABEL_WIDTH: f32 = 52.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScaleChoice(u8);

impl ScaleChoice {
    const ALL: [Self; 5] = [Self(1), Self(2), Self(4), Self(8), Self(16)];
}

impl std::fmt::Display for ScaleChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}×", self.0)
    }
}

impl Gridvana {
    pub(super) fn editor_export_inspector(&self) -> Element<'_, Message> {
        let form = self.sprite_sheet_export_form;
        let tabs = self.editor_inspector_tabs();
        let active_tag = self
            .project
            .active_tag_id
            .and_then(|id| self.project.tag(id));
        let active_tag_name = active_tag.map_or("无活动标签", |tag| tag.name.as_str());

        let path_label = self
            .pending_sprite_sheet_export_path
            .as_ref()
            .and_then(|path| path.parent())
            .map(|directory| directory.display().to_string())
            .unwrap_or_else(|| "选择输出目录…".to_string());
        let target_path = widget::button(
            widget::container(
                widget::text(path_label)
                    .size(10)
                    .color(if self.pending_sprite_sheet_export_path.is_some() {
                        TEXT_PRIMARY
                    } else {
                        TEXT_MUTED
                    })
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            )
            .width(Length::Fill)
            .align_x(iced::Alignment::Start),
        )
        .on_press(Message::ExportSpriteSheet)
        .padding([5, 7])
        .width(Length::Fill)
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, false));

        let mut settings = widget::column![
            field_row("目录", target_path.into()),
            choice_field(
                "帧范围",
                SpriteFrameRangeChoice::ALL,
                form.frame_range,
                Message::SelectSpriteFrameRange,
            ),
            choice_field(
                "图层",
                SpriteLayerRangeChoice::ALL,
                form.layer_range,
                Message::SelectSpriteLayerRange,
            ),
            choice_field(
                "布局",
                SpriteLayoutChoice::ALL,
                form.layout,
                Message::SelectSpriteLayout,
            ),
            choice_field(
                "倍率",
                ScaleChoice::ALL,
                ScaleChoice(form.scale),
                |choice: ScaleChoice| Message::SetSpriteScale(choice.0),
            ),
            choice_field(
                "元数据",
                SpriteMetadataChoice::ALL,
                form.metadata,
                Message::SelectSpriteMetadata,
            ),
        ]
        .spacing(6);

        if matches!(
            form.layout,
            SpriteLayoutChoice::FixedColumns | SpriteLayoutChoice::FixedRows
        ) {
            settings = settings.push(field_row(
                if form.layout == SpriteLayoutChoice::FixedColumns {
                    "列数"
                } else {
                    "行数"
                },
                widget::row![
                    widget::slider(1..=16, form.fixed_count, Message::SetSpriteFixedCount)
                        .step(1u8)
                        .width(Length::Fill),
                    widget::text(form.fixed_count.to_string())
                        .size(10)
                        .color(TEXT_SECONDARY)
                        .width(Length::Fixed(16.0))
                        .align_x(iced::Alignment::End),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .into(),
            ));
        }

        let frame_positions = export_preview_frame_positions(self);
        let frame_count = frame_positions.len();
        let thumbnails = export_thumbnails(self, &frame_positions);
        let preview_strip = widget::container(widget::row(thumbnails).spacing(1))
            .width(Length::Fill)
            .height(Length::Fixed(58.0))
            .style(|_| {
                widget::container::Style::default()
                    .background(BORDER_SUBTLE)
                    .border(iced::Border {
                        color: BORDER_SUBTLE,
                        width: 1.0,
                        radius: 2.0.into(),
                    })
            });

        let (png_name, json_name) = export_file_names(self);
        let estimate = match &self.sprite_sheet_export_estimate {
            Ok((width, height)) => format!("{frame_count} 帧 · {width} × {height} px"),
            Err(error) => format!("无法预览：{error}"),
        };
        let direction = active_tag.map_or("正向", |tag| match tag.direction {
            TagDirection::Forward => "正向",
            TagDirection::Reverse => "反向",
            TagDirection::PingPong => "Ping-Pong",
        });
        let preview_metadata = widget::container(
            widget::column![
                widget::text(format!("{png_name} · {json_name}"))
                    .size(9)
                    .color(TEXT_SECONDARY),
                widget::text(format!("{estimate} · 标签 {active_tag_name} · {direction}"))
                    .size(9)
                    .color(TEXT_MUTED),
            ]
            .spacing(3),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(|_| {
            widget::container::Style::default()
                .background(CONTROL_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 2.0.into(),
                })
        });

        let trim_enabled = form.trim == SpriteTrimChoice::PerFrame;
        let preview_body = widget::column![
            widget::row![
                widget::checkbox(trim_enabled)
                    .label("裁切透明边缘")
                    .size(13)
                    .text_size(10)
                    .on_toggle(Message::SetSpriteTrimPerFrame),
                widget::checkbox(form.extrude > 0)
                    .label("Extrude 1 px")
                    .size(13)
                    .text_size(10)
                    .on_toggle(Message::SetSpriteExtrudeEnabled),
            ]
            .spacing(14),
            preview_strip,
            preview_metadata,
        ]
        .spacing(7);

        let can_export = self.pending_sprite_sheet_export_path.is_some()
            && self.sprite_sheet_export_estimate.is_ok();
        let export_message = if can_export {
            Some(Message::ConfirmSpriteSheetExport)
        } else if self.pending_sprite_sheet_export_path.is_none() {
            Some(Message::ExportSpriteSheet)
        } else {
            None
        };
        let export_button = widget::button(
            widget::container(
                widget::text(if can_export {
                    "导出 PNG + JSON"
                } else {
                    "选择输出目录"
                })
                .size(11),
            )
            .width(Length::Fill)
            .align_x(iced::Alignment::Center),
        )
        .on_press_maybe(export_message)
        .padding([8, 12])
        .width(Length::Fill)
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, true));

        let gif_button = widget::button(
            widget::container(widget::text("导出 GIF 动画").size(10))
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press_maybe(self.has_canvas.then_some(Message::ExportGif))
        .padding([7, 12])
        .width(Length::FillPortion(1))
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, false));

        let png_sequence_button = widget::button(
            widget::container(widget::text("导出逐帧 PNG").size(10))
                .width(Length::Fill)
                .align_x(iced::Alignment::Center),
        )
        .on_press_maybe(self.has_canvas.then_some(Message::ExportPngSequence))
        .padding([7, 12])
        .width(Length::FillPortion(1))
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, false));

        let export_feedback: Element<'_, Message> = match &self.last_export_summary {
            Some(summary) => {
                widget::container(widget::text(format!("✓ {summary}")).size(9).color(SUCCESS))
                    .width(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .into()
            }
            None => widget::Space::new().into(),
        };

        let content = widget::column![
            tabs,
            widget::rule::horizontal(1),
            section_heading("Sprite Sheet", widget::Space::new().into()),
            widget::container(settings).padding(iced::Padding::new(10.0).top(0)),
            widget::rule::horizontal(1),
            section_heading(
                "预览",
                widget::text(format!("{frame_count} 帧"))
                    .size(9)
                    .color(TEXT_MUTED)
                    .into(),
            ),
            widget::container(preview_body).padding(iced::Padding::new(10.0).top(0)),
            widget::container(
                widget::column![
                    export_button,
                    widget::row![png_sequence_button, gif_button].spacing(6),
                    export_feedback
                ]
                .spacing(6)
            )
            .padding(iced::Padding::new(10.0).top(0)),
        ]
        .spacing(0);

        widget::container(widget::scrollable(content).height(Length::Fill))
            .width(Length::Fixed(self.inspector_width))
            .height(Length::Fill)
            .style(panel_style)
            .into()
    }
}

fn section_heading<'a>(
    label: &'static str,
    trailing: Element<'a, Message>,
) -> Element<'a, Message> {
    widget::container(
        widget::row![
            widget::text(label).size(10).color(TEXT_SECONDARY),
            widget::Space::new().width(Length::Fill),
            trailing,
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([4, 10])
    .height(Length::Fixed(26.0))
    .width(Length::Fill)
    .into()
}

fn field_row<'a>(label: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    widget::row![
        widget::text(label)
            .size(10)
            .color(TEXT_SECONDARY)
            .width(Length::Fixed(LABEL_WIDTH)),
        control,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn choice_field<'a, T, L>(
    label: &'static str,
    options: L,
    selected: T,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
{
    field_row(
        label,
        widget::pick_list(options, Some(selected), on_select)
            .text_size(10)
            .padding([5, 7])
            .width(Length::Fill)
            .style(pick_list_style)
            .menu_style(pick_list_menu_style)
            .into(),
    )
}

fn export_preview_frame_positions(app: &Gridvana) -> Vec<usize> {
    let frame_ids = if app.sprite_sheet_export_form.frame_range == SpriteFrameRangeChoice::ActiveTag
    {
        app.project
            .active_tag_id
            .and_then(|tag_id| app.project.frame_ids_for_tag(tag_id).ok())
            .unwrap_or_default()
    } else {
        app.project.frames.iter().map(|frame| frame.id).collect()
    };

    frame_ids
        .into_iter()
        .filter_map(|frame_id| {
            app.project
                .frames
                .iter()
                .position(|frame| frame.id == frame_id)
        })
        .collect()
}

fn export_thumbnails<'a>(app: &Gridvana, positions: &[usize]) -> Vec<Element<'a, Message>> {
    let mut thumbnails = positions
        .iter()
        .take(4)
        .filter_map(|position| {
            let (width, height, rgba) =
                gridvana_core::io::render_frame_rgba(&app.project, *position).ok()?;
            let image = widget::image(widget::image::Handle::from_rgba(width, height, rgba))
                .content_fit(iced::ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill);
            Some(
                widget::container(image)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill)
                    .padding(5)
                    .style(|_| widget::container::Style::default().background(SURFACE_BACKGROUND))
                    .into(),
            )
        })
        .collect::<Vec<Element<'a, Message>>>();
    while thumbnails.len() < 4 {
        thumbnails.push(
            widget::container(widget::Space::new())
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .style(|_| widget::container::Style::default().background(PANEL_BACKGROUND))
                .into(),
        );
    }
    thumbnails
}

fn export_file_names(app: &Gridvana) -> (String, String) {
    let png_name = app
        .pending_sprite_sheet_export_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "spritesheet.png".to_string());
    let mut json_name = std::path::PathBuf::from(&png_name);
    json_name.set_extension("json");
    (png_name, json_name.to_string_lossy().into_owned())
}
