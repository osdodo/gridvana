use super::super::Gridvana;
use super::super::ui::{
    APP_BACKGROUND, BORDER_STRONG, BORDER_SUBTLE, OVERLAY_BACKGROUND, SURFACE_BACKGROUND,
    TEXT_MUTED, TEXT_PRIMARY,
};
use crate::canvas;
use crate::i18n::tr;
use crate::types::Message;
use gridvana_core::grid::GridIndex;
use iced::{Element, Length, Theme, widget};

const PREVIEW_PANEL_SIDE: f32 = 168.0;
const PREVIEW_PANEL_MARGIN: f32 = 10.0;
const CONTEXT_TOOLBAR_MARGIN: f32 = 10.0;

impl Gridvana {
    pub(super) fn editor_empty_canvas(&self) -> Element<'_, Message> {
        let empty_stage = widget::container(
            widget::button(widget::text(tr("Create canvas", "创建画布")).size(11))
                .on_press(Message::OpenNewProjectDialog)
                .padding([7, 14]),
        )
        .center(Length::Fill);

        widget::stack(vec![
            empty_stage.into(),
            self.editor_context_toolbar_layer(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn editor_canvas_with_preview(
        &self,
        selection_display_indices: Vec<GridIndex>,
    ) -> Element<'_, Message> {
        let displayed_project = self.displayed_project();
        let brush_preview_indices = self.brush_preview_indices();
        let eraser_preview_indices = self.eraser_preview_indices();
        let preview_color = if let Some(shape) = self.current_shape {
            Some(shape.color)
        } else if !brush_preview_indices.is_empty() {
            Some(self.project.foreground_color)
        } else {
            None
        };
        let preview_indices = if self.current_shape.is_some() {
            self.shape_preview_indices.clone()
        } else {
            brush_preview_indices
        };
        let canvas_area = canvas::view(
            displayed_project,
            canvas::ViewOptions {
                input_enabled: !self.cli_settings_open
                    && self.selection_context_menu.is_none()
                    && self.canvas_context_menu.is_none()
                    && !self.canvas_size_popover_open
                    && self.cel_context_menu.is_none(),
                preview_indices,
                preview_color,
                eraser_preview_indices,
                selection_indices: selection_display_indices,
                move_mode_active: self.move_mode_active,
                global_left_button_down: self.global_left_button_down,
                size_modifier_pressed: self.shift_pressed,
                zoom_modifier_pressed: self.zoom_modifier_pressed,
                pan_modifier_pressed: self.space_pressed,
                current_tool: self.current_tool,
                brush_size: self.brush_size,
                eraser_size: self.eraser_size,
                onion_skin_enabled: self.onion_skin_enabled,
                onion_skin_settings: self.onion_skin_settings,
                transform_targets: self.pixel_transform_targets(),
                floating_pixels: self.floating_selection_display_pixels(),
            },
        );

        let preview_image_panel: Element<'_, Message> = match gridvana_core::io::render_frame_rgba(
            displayed_project,
            displayed_project.active_frame_position().unwrap_or(0),
        ) {
            Ok((preview_width, preview_height, rgba)) => {
                let preview_image = widget::image(widget::image::Handle::from_rgba(
                    preview_width,
                    preview_height,
                    rgba,
                ))
                .content_fit(iced::ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill);

                widget::container(preview_image)
                    .width(Length::Fixed(PREVIEW_PANEL_SIDE))
                    .height(Length::Fixed(PREVIEW_PANEL_SIDE))
                    .padding(6)
                    .style(|_| {
                        widget::container::Style::default()
                            .background(OVERLAY_BACKGROUND)
                            .border(iced::Border {
                                color: BORDER_SUBTLE,
                                width: 1.0,
                                radius: 2.0.into(),
                            })
                    })
                    .into()
            }
            Err(_) => widget::container(
                widget::text(tr("Preview render failed", "预览渲染失败")).size(10),
            )
            .width(Length::Fixed(PREVIEW_PANEL_SIDE))
            .height(Length::Fixed(PREVIEW_PANEL_SIDE))
            .center(Length::Fill)
            .style(|_| {
                widget::container::Style::default()
                    .background(SURFACE_BACKGROUND)
                    .border(iced::Border {
                        color: BORDER_SUBTLE,
                        width: 1.0,
                        radius: 2.0.into(),
                    })
            })
            .into(),
        };
        let preview_title = widget::container(
            widget::text(format!(
                "{} · {} {}/{}",
                tr("Preview", "预览"),
                tr("Frame", "帧"),
                displayed_project.active_frame_position().unwrap_or(0) + 1,
                displayed_project.frames.len(),
            ))
            .size(10)
            .color(TEXT_PRIMARY),
        )
        .padding([4, 6])
        .style(|_| {
            widget::container::Style::default()
                .background(OVERLAY_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 1.0.into(),
                })
        });
        let floating_preview: Element<'_, Message> = widget::stack(vec![
            preview_image_panel,
            widget::container(
                widget::column![
                    widget::Space::new().height(Length::Fixed(6.0)),
                    widget::row![
                        widget::Space::new().width(Length::Fixed(6.0)),
                        preview_title,
                        widget::Space::new().width(Length::Fill),
                    ],
                    widget::Space::new().height(Length::Fill),
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        ])
        .width(Length::Fixed(PREVIEW_PANEL_SIDE))
        .height(Length::Fixed(PREVIEW_PANEL_SIDE))
        .into();
        let preview_layer: Element<'_, Message> = widget::container(
            widget::column![
                widget::Space::new().height(Length::Fixed(PREVIEW_PANEL_MARGIN)),
                widget::row![
                    widget::Space::new().width(Length::Fill),
                    floating_preview,
                    widget::Space::new().width(Length::Fixed(PREVIEW_PANEL_MARGIN)),
                ],
                widget::Space::new().height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        let coordinate_label = self.hovered_grid_index.map_or_else(
            || "X — · Y —".to_string(),
            |index| format!("X {} · Y {}", index.x, index.y),
        );
        let coordinate_chip =
            widget::container(widget::text(coordinate_label).size(10).color(TEXT_MUTED))
                .padding([4, 7])
                .style(|_| {
                    widget::container::Style::default()
                        .background(OVERLAY_BACKGROUND)
                        .border(iced::Border {
                            color: BORDER_SUBTLE,
                            width: 1.0,
                            radius: 1.0.into(),
                        })
                });
        let coordinate_layer: Element<'_, Message> = widget::container(
            widget::column![
                widget::Space::new().height(Length::Fill),
                widget::row![
                    widget::Space::new().width(Length::Fill),
                    coordinate_chip,
                    widget::Space::new().width(Length::Fixed(10.0)),
                ],
                widget::Space::new().height(Length::Fixed(9.0)),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        let canvas_workspace = widget::container(canvas_area)
            .width(Length::Fill)
            .height(Length::Fill);
        let stage = widget::stack(vec![
            canvas_workspace.into(),
            preview_layer,
            coordinate_layer,
            self.editor_context_toolbar_layer(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true);

        stage.into()
    }

    fn editor_context_toolbar_layer(&self) -> Element<'_, Message> {
        widget::container(
            widget::column![
                widget::Space::new().height(Length::Fixed(CONTEXT_TOOLBAR_MARGIN)),
                widget::row![
                    widget::Space::new().width(Length::Fixed(CONTEXT_TOOLBAR_MARGIN)),
                    self.editor_context_toolbar(),
                    widget::Space::new().width(Length::Fill),
                ],
                widget::Space::new().height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn editor_main_workspace<'a>(
        &self,
        tool_rail: Element<'a, Message>,
        canvas_with_preview: Element<'a, Message>,
        inspector: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let divider_color = if self.inspector_resize.is_some() {
            BORDER_STRONG
        } else {
            BORDER_SUBTLE
        };
        let resize_handle = widget::mouse_area(
            widget::container(
                widget::container(widget::Space::new())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fill)
                    .style(move |_| widget::container::Style::default().background(divider_color)),
            )
            .width(Length::Fixed(7.0))
            .height(Length::Fill)
            .align_x(iced::Alignment::End),
        )
        .on_press(Message::BeginInspectorResize)
        .on_release(Message::EndInspectorResize)
        .interaction(iced::mouse::Interaction::ResizingHorizontally);

        widget::row![
            tool_rail,
            widget::rule::vertical(1),
            canvas_with_preview,
            resize_handle,
            inspector,
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn editor_main_content<'a>(
        &self,
        main_workspace: Element<'a, Message>,
        bottom_panel: Element<'a, Message>,
        status_bar: Element<'a, Message>,
    ) -> Element<'a, Message> {
        widget::container(
            widget::column![
                main_workspace,
                widget::rule::horizontal(1),
                bottom_panel,
                widget::rule::horizontal(1),
                status_bar
            ]
            .spacing(0)
            .padding(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_t| widget::container::Style::default().background(APP_BACKGROUND))
        .into()
    }

    pub(super) fn editor_floating_hint_layer(
        &self,
        shape_preview_hint: Option<&str>,
    ) -> Element<'_, Message> {
        if let (Some(shape_hint), Some(cursor_position)) =
            (shape_preview_hint, self.cursor_position)
        {
            let shape_hint = shape_hint.to_string();
            let estimated_hint_width =
                (shape_hint.chars().count() as f32 * 8.0 + 16.0).clamp(32.0, 280.0);
            let hint_height = 28.0;

            widget::responsive(move |size| {
                let margin = 14.0;
                let max_x = (size.width - estimated_hint_width).max(0.0);
                let max_y = (size.height - hint_height).max(0.0);
                let right_x = cursor_position.x + margin;
                let bottom_y = cursor_position.y + margin;
                let left_x = cursor_position.x - estimated_hint_width - margin;
                let top_y = cursor_position.y - hint_height - margin;
                let offset_x = if right_x > max_x {
                    left_x.max(0.0)
                } else {
                    right_x.min(max_x)
                };
                let offset_y = if bottom_y > max_y {
                    top_y.max(0.0)
                } else {
                    bottom_y.min(max_y)
                };
                let hint_chip = widget::container(widget::text(shape_hint.clone()).size(11))
                    .padding([4, 8])
                    .style(|_t: &Theme| {
                        widget::container::Style::default()
                            .background(OVERLAY_BACKGROUND)
                            .border(iced::Border {
                                color: BORDER_STRONG,
                                width: 1.0,
                                radius: 2.0.into(),
                            })
                    });

                widget::container(
                    widget::column![
                        widget::Space::new().height(Length::Fixed(offset_y)),
                        widget::row![
                            widget::Space::new().width(Length::Fixed(offset_x)),
                            hint_chip,
                            widget::Space::new().width(Length::Fill),
                        ]
                        .width(Length::Fill),
                        widget::Space::new().height(Length::Fill),
                    ]
                    .width(Length::Fill)
                    .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }
}
