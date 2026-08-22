use super::super::geometry::effective_tool_span;
use super::super::ui::{
    BORDER_STRONG, BORDER_SUBTLE, OVERLAY_BACKGROUND, PANEL_BACKGROUND, SURFACE_BACKGROUND,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, color_channel_u8, compact_action_button_style,
    panel_style, pick_list_menu_style, pick_list_style, text_input_style,
};
use super::super::{Gridvana, MAX_BRUSH_SIZE, MAX_ERASER_SIZE, MIN_BRUSH_SIZE, MIN_ERASER_SIZE};
use crate::color_picker;
use crate::i18n::tr;
use crate::icons::{Icon, icon_button};
use crate::types::{ColorSlot, InspectorPanel, Message, Tool};
use gridvana_core::model::{BlendMode, LayerId, LayerKind};
use gridvana_core::transform::PixelTransform;
use iced::{Background, Element, Length, Theme, widget};

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayerParentChoice {
    layer_id: Option<LayerId>,
    label: String,
}

impl std::fmt::Display for LayerParentChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

impl Gridvana {
    pub(super) fn active_tool_name(&self) -> &'static str {
        match self.current_tool {
            Tool::HandPoint => tr("Select", "选择"),
            Tool::MagicWand => tr("Magic wand", "魔棒选择"),
            Tool::ColorSelect => tr("Select by color", "按颜色选择"),
            Tool::Brush => tr("Brush", "画笔"),
            Tool::Eraser => tr("Eraser", "橡皮擦"),
            Tool::PaintBucket => tr("Paint bucket", "油漆桶"),
            Tool::Picker => tr("Color picker", "拾色器"),
            Tool::Rectangle => tr("Rectangle", "矩形"),
            Tool::RectangleHollow => tr("Rectangle outline", "空心矩形"),
            Tool::Circle => tr("Circle", "圆形"),
            Tool::CircleHollow => tr("Circle outline", "空心圆"),
            Tool::Line => tr("Line", "线条"),
        }
    }

    fn editor_tool_button(&self, icon: Icon, tool: Tool) -> widget::Button<'static, Message> {
        icon_button(icon, 13.0, 30.0, self.current_tool == tool, true)
            .on_press(Message::SelectTool(tool))
    }

    pub(super) fn editor_tool_rail(&self) -> Element<'_, Message> {
        let separator = || {
            widget::container(widget::Space::new())
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(1.0))
                .style(|_| widget::container::Style::default().background(BORDER_SUBTLE))
        };
        let foreground = self.project.foreground_color;
        let foreground_swatch = widget::button(
            widget::container(widget::Space::new())
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0))
                .style(move |_| {
                    widget::container::Style::default()
                        .background(iced::Color::from_rgba(
                            foreground.r,
                            foreground.g,
                            foreground.b,
                            foreground.a,
                        ))
                        .border(iced::Border {
                            color: BORDER_STRONG,
                            width: 1.0,
                            radius: 1.0.into(),
                        })
                }),
        )
        .padding(8)
        .style(widget::button::text);

        widget::container(
            widget::column![
                self.editor_tool_button(Icon::HandPoint, Tool::HandPoint),
                self.editor_tool_button(Icon::MagicWand, Tool::MagicWand),
                self.editor_tool_button(Icon::ColorSelect, Tool::ColorSelect),
                separator(),
                self.editor_tool_button(Icon::Brush, Tool::Brush),
                self.editor_tool_button(Icon::Eraser, Tool::Eraser),
                self.editor_tool_button(Icon::PaintBucket, Tool::PaintBucket),
                self.editor_tool_button(Icon::Picker, Tool::Picker),
                separator(),
                self.editor_tool_button(Icon::Line, Tool::Line),
                self.editor_tool_button(Icon::RectangleHollow, Tool::RectangleHollow),
                self.editor_tool_button(Icon::CircleHollow, Tool::CircleHollow),
                separator(),
                self.editor_tool_button(Icon::Rectangle, Tool::Rectangle),
                self.editor_tool_button(Icon::Circle, Tool::Circle),
                widget::Space::new().height(Length::Fill),
                foreground_swatch,
            ]
            .spacing(2)
            .align_x(iced::Alignment::Center),
        )
        .padding([6, 5])
        .width(Length::Fixed(48.0))
        .height(Length::Fill)
        .style(panel_style)
        .into()
    }

    pub(super) fn editor_context_toolbar(&self) -> Element<'_, Message> {
        let tool_size: Element<'_, Message> = match self.current_tool {
            Tool::Brush => widget::row![
                widget::text(self.active_tool_name())
                    .size(10)
                    .color(TEXT_MUTED),
                widget::text(format!("{} px", effective_tool_span(self.brush_size))).size(10),
                widget::slider(
                    MIN_BRUSH_SIZE..=MAX_BRUSH_SIZE,
                    self.brush_size,
                    Message::UpdateBrushSize,
                )
                .step(1u8)
                .width(Length::Fixed(96.0)),
            ]
            .spacing(7)
            .align_y(iced::Alignment::Center)
            .into(),
            Tool::Eraser => widget::row![
                widget::text(self.active_tool_name())
                    .size(10)
                    .color(TEXT_MUTED),
                widget::text(format!("{} px", effective_tool_span(self.eraser_size))).size(10),
                widget::slider(
                    MIN_ERASER_SIZE..=MAX_ERASER_SIZE,
                    self.eraser_size,
                    Message::UpdateEraserSize,
                )
                .step(1u8)
                .width(Length::Fixed(96.0)),
            ]
            .spacing(7)
            .align_y(iced::Alignment::Center)
            .into(),
            _ => widget::text(self.active_tool_name())
                .size(10)
                .color(TEXT_MUTED)
                .into(),
        };

        let preview_toggle = icon_button(
            if self.preview_visible {
                Icon::EyeSlash
            } else {
                Icon::Eye
            },
            12.0,
            24.0,
            self.preview_visible,
            true,
        )
        .on_press(Message::TogglePreview);

        widget::container(
            widget::row![
                tool_size,
                widget::Space::new().width(Length::Fixed(8.0)),
                preview_toggle
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([7, 9])
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

    pub(super) fn editor_inspector_tabs(&self) -> Element<'_, Message> {
        let tab_style = |active: bool| {
            move |theme: &Theme, status| {
                let mut style = widget::button::text(theme, status);
                style.text_color = if active { TEXT_PRIMARY } else { TEXT_MUTED };
                style.background = active.then_some(Background::Color(SURFACE_BACKGROUND));
                style.border = iced::Border {
                    color: if active {
                        TEXT_SECONDARY
                    } else {
                        BORDER_SUBTLE
                    },
                    width: if active { 1.0 } else { 0.0 },
                    radius: 0.0.into(),
                };
                style
            }
        };
        let tab_label = |label: &'static str| {
            widget::container(widget::text(label).size(10))
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
        };
        let mut tabs = widget::row![
            widget::button(tab_label(tr("Layers", "图层")))
                .on_press(Message::SetInspectorPanel(InspectorPanel::Layers))
                .width(Length::FillPortion(1))
                .height(Length::Fixed(34.0))
                .style(tab_style(self.inspector_panel == InspectorPanel::Layers)),
            widget::button(tab_label(tr("Export", "导出")))
                .on_press(Message::SetInspectorPanel(InspectorPanel::Export))
                .width(Length::FillPortion(1))
                .height(Length::Fixed(34.0))
                .style(tab_style(self.inspector_panel == InspectorPanel::Export)),
        ]
        .spacing(0);
        if self.ai_agent_panel_available() {
            tabs = tabs.push(
                widget::button(tab_label("AI Agent"))
                    .on_press(Message::SetInspectorPanel(InspectorPanel::AiAgent))
                    .width(Length::FillPortion(1))
                    .height(Length::Fixed(34.0))
                    .style(tab_style(self.inspector_panel == InspectorPanel::AiAgent)),
            );
        }
        tabs.into()
    }

    pub(super) fn editor_inspector(&self) -> Element<'_, Message> {
        if self.inspector_panel == InspectorPanel::AiAgent && self.ai_agent_panel_available() {
            return self.editor_ai_agent_inspector();
        }
        if self.inspector_panel == InspectorPanel::Export {
            return self.editor_export_inspector();
        }

        let tabs = self.editor_inspector_tabs();

        let layer_actions: Element<'_, Message> = widget::row![
            icon_button(Icon::Add, 11.0, 24.0, false, true).on_press(Message::AddLayer),
            widget::button(widget::text("▣").size(12))
                .on_press(Message::AddLayerGroup)
                .padding([4, 7])
                .style(widget::button::text),
        ]
        .spacing(2)
        .into();
        let mut layer_rows = Vec::new();
        for layer in self.project.layers.iter().rev() {
            let layer_id = layer.id;
            let active = layer_id == self.project.active_layer_id;
            let row = widget::container(
                widget::row![
                    icon_button(
                        if layer.visible {
                            Icon::Eye
                        } else {
                            Icon::EyeSlash
                        },
                        11.0,
                        24.0,
                        false,
                        false,
                    )
                    .on_press(Message::ToggleLayerVisibility(layer_id)),
                    widget::button(widget::text(if layer.locked { "◆" } else { "◇" }).size(10))
                        .on_press(Message::ToggleLayerLocked(layer_id))
                        .padding([4, 6])
                        .style(widget::button::text),
                    widget::button(widget::text(&layer.name).size(10))
                        .on_press(Message::SelectLayer(layer_id))
                        .padding([5, 2])
                        .width(Length::Fill)
                        .style(widget::button::text),
                    widget::text(layer.kind.to_string())
                        .size(9)
                        .color(TEXT_MUTED),
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center),
            )
            .padding([3, 8])
            .height(Length::Fixed(34.0))
            .width(Length::Fill)
            .style(move |_| {
                widget::container::Style::default()
                    .background(if active {
                        SURFACE_BACKGROUND
                    } else {
                        PANEL_BACKGROUND
                    })
                    .border(iced::Border {
                        color: BORDER_SUBTLE,
                        width: 1.0,
                        radius: 0.0.into(),
                    })
            });
            layer_rows.push(row.into());
        }
        let layer_section = widget::column![
            inspector_section_heading(tr("Layers", "图层").to_string(), layer_actions),
            widget::column(layer_rows).spacing(0),
        ]
        .spacing(0);

        let active_layer = self
            .project
            .current_layer()
            .expect("valid projects always have an active layer");
        let active_layer_id = active_layer.id;
        let active_opacity = (active_layer.opacity * 100.0).round() as u8;
        let descendants = self
            .project
            .descendant_layer_ids(active_layer_id)
            .unwrap_or_default();
        let mut parent_choices = vec![LayerParentChoice {
            layer_id: None,
            label: "Root".to_string(),
        }];
        parent_choices.extend(
            self.project
                .layers
                .iter()
                .filter(|layer| layer.kind == LayerKind::Group)
                .filter(|layer| layer.id != active_layer_id && !descendants.contains(&layer.id))
                .map(|layer| LayerParentChoice {
                    layer_id: Some(layer.id),
                    label: layer.name.clone(),
                }),
        );
        let selected_parent = parent_choices
            .iter()
            .find(|choice| choice.layer_id == active_layer.parent_id)
            .cloned();
        let (cel_label, cel_offset) = self.project.current_cel().map_or_else(
            || ("—".to_string(), "X 0 · Y 0".to_string()),
            |cel| {
                (
                    format!("C{} · {} px", cel.id, cel.pixels.len()),
                    format!("X {} · Y {}", cel.offset.x, cel.offset.y),
                )
            },
        );
        let property_label = |label: &'static str| {
            widget::text(label)
                .size(10)
                .color(TEXT_SECONDARY)
                .width(Length::Fixed(78.0))
        };
        let property_section = widget::column![
            inspector_section_heading(
                tr("Active layer", "活动图层").to_string(),
                widget::Space::new().into()
            ),
            widget::container(
                widget::column![
                    widget::row![
                        property_label(tr("Type", "类型")),
                        widget::pick_list(LayerKind::ALL, Some(active_layer.kind), move |kind| {
                            Message::SetLayerKind(active_layer_id, kind)
                        },)
                        .text_size(10)
                        .padding([5, 7])
                        .width(Length::Fill)
                        .style(pick_list_style)
                        .menu_style(pick_list_menu_style),
                    ]
                    .align_y(iced::Alignment::Center),
                    widget::row![
                        property_label(tr("Blend mode", "混合模式")),
                        widget::pick_list(
                            BlendMode::ALL,
                            Some(active_layer.blend_mode),
                            move |blend| Message::SetLayerBlendMode(active_layer_id, blend),
                        )
                        .text_size(10)
                        .padding([5, 7])
                        .width(Length::Fill)
                        .style(pick_list_style)
                        .menu_style(pick_list_menu_style),
                    ]
                    .align_y(iced::Alignment::Center),
                    widget::row![
                        property_label(tr("Parent group", "父组")),
                        widget::pick_list(parent_choices, selected_parent, move |choice| {
                            Message::SetLayerParent(active_layer_id, choice.layer_id)
                        },)
                        .text_size(10)
                        .padding([5, 7])
                        .width(Length::Fill)
                        .style(pick_list_style)
                        .menu_style(pick_list_menu_style),
                    ]
                    .align_y(iced::Alignment::Center),
                    widget::row![
                        property_label(tr("Opacity", "不透明度")),
                        widget::slider(0..=100, active_opacity, move |value| {
                            Message::SetLayerOpacity(active_layer_id, value)
                        })
                        .width(Length::Fill),
                        widget::text(format!("{active_opacity}%"))
                            .size(9)
                            .color(TEXT_MUTED),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                    widget::row![
                        property_label(tr("Active Cel", "活动 Cel")),
                        widget::text(cel_label).size(10).color(TEXT_SECONDARY),
                    ]
                    .align_y(iced::Alignment::Center),
                    widget::row![
                        property_label(tr("Offset", "偏移")),
                        widget::text(cel_offset).size(10).color(TEXT_SECONDARY),
                    ]
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(10),
        ]
        .spacing(0);

        let current_color = self.active_color();
        let current_alpha = color_channel_u8(current_color.a);
        let color_slot_button =
            |label: &'static str, slot: ColorSlot, color: gridvana_core::model::Rgba| {
                let active = self.active_color_slot == slot;
                widget::button(
                    widget::row![
                        widget::container(widget::Space::new())
                            .width(Length::Fixed(26.0))
                            .height(Length::Fixed(22.0))
                            .style(move |_| {
                                widget::container::Style::default()
                                    .background(iced::Color::from_rgba(
                                        color.r, color.g, color.b, color.a,
                                    ))
                                    .border(iced::Border {
                                        color: BORDER_STRONG,
                                        width: 1.0,
                                        radius: 1.0.into(),
                                    })
                            }),
                        widget::text(label).size(10),
                    ]
                    .spacing(7)
                    .align_y(iced::Alignment::Center),
                )
                .on_press(Message::SetActiveColorSlot(slot))
                .padding([4, 7])
                .width(Length::Fill)
                .style(move |theme: &Theme, status| {
                    compact_action_button_style(theme, status, active)
                })
            };
        let color_section = widget::column![
            inspector_section_heading(tr("Color", "颜色").to_string(), widget::Space::new().into()),
            widget::container(
                widget::column![
                    widget::row![
                        color_slot_button(
                            tr("Foreground", "前景"),
                            ColorSlot::Foreground,
                            self.project.foreground_color
                        ),
                        color_slot_button(
                            tr("Background", "背景"),
                            ColorSlot::Background,
                            self.project.background_color
                        ),
                    ]
                    .spacing(6),
                    widget::row![
                        widget::text_input("#RRGGBBAA", &self.current_color_hex_input)
                            .on_input(Message::UpdateColorHexInput)
                            .on_submit(Message::SubmitColorHexInput)
                            .padding([6, 8])
                            .size(10)
                            .style(text_input_style)
                            .width(Length::Fill),
                        widget::button(widget::text(tr("Swap", "交换")).size(9))
                            .on_press(Message::SwapForegroundBackground)
                            .padding([6, 8]),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                    color_picker::view(current_color),
                    widget::row![
                        widget::text("Alpha")
                            .size(10)
                            .color(TEXT_SECONDARY)
                            .width(Length::Fixed(42.0)),
                        widget::slider(0..=255, current_alpha, Message::UpdateColorAlpha)
                            .step(1u8)
                            .width(Length::Fill),
                        widget::text(format!("{current_alpha:03}"))
                            .size(9)
                            .color(TEXT_MUTED),
                    ]
                    .spacing(6)
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(9),
            )
            .padding(10),
        ]
        .spacing(0);

        let mut used_colors = self.recent_colors.clone();
        let mut usage =
            std::collections::HashMap::<u32, (gridvana_core::model::Rgba, usize)>::new();
        for color in self
            .project
            .cels
            .iter()
            .flat_map(|cel| cel.pixels.values().copied())
        {
            let key = u32::from_be_bytes([
                color_channel_u8(color.r),
                color_channel_u8(color.g),
                color_channel_u8(color.b),
                color_channel_u8(color.a),
            ]);
            let entry = usage.entry(key).or_insert((color, 0));
            entry.1 += 1;
        }
        let mut project_colors = usage.into_iter().collect::<Vec<_>>();
        project_colors.sort_by(
            |(left_key, (_, left_count)), (right_key, (_, right_count))| {
                right_count
                    .cmp(left_count)
                    .then_with(|| left_key.cmp(right_key))
            },
        );
        for (_, (color, _)) in project_colors {
            if !used_colors.contains(&color) {
                used_colors.push(color);
            }
            if used_colors.len() >= 16 {
                break;
            }
        }
        let used_color_grid: Element<'_, Message> = if used_colors.is_empty() {
            widget::text(tr("No color history", "暂无颜色记录"))
                .size(9)
                .color(TEXT_MUTED)
                .into()
        } else {
            widget::column(
                used_colors
                    .chunks(8)
                    .map(|chunk| {
                        widget::row(
                            chunk
                                .iter()
                                .copied()
                                .map(|color| color_swatch(color, 27.0, Message::SelectColor(color)))
                                .collect::<Vec<Element<Message>>>(),
                        )
                        .spacing(4)
                        .into()
                    })
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(4)
            .into()
        };
        let palette_section = widget::column![
            inspector_section_heading(
                tr("Used colors", "使用过的颜色").to_string(),
                widget::text(format!("{} / 16", used_colors.len().min(16)))
                    .size(9)
                    .color(TEXT_MUTED)
                    .into(),
            ),
            widget::container(used_color_grid).padding(10),
        ]
        .spacing(0);

        let inspector_content = widget::column![
            tabs,
            widget::rule::horizontal(1),
            layer_section,
            widget::rule::horizontal(1),
            property_section,
            widget::rule::horizontal(1),
            color_section,
            widget::rule::horizontal(1),
            palette_section,
        ]
        .spacing(0);

        widget::container(widget::scrollable(inspector_content).height(Length::Fill))
            .width(Length::Fixed(self.inspector_width))
            .height(Length::Fill)
            .style(panel_style)
            .into()
    }

    pub(super) fn selection_context_menu_overlay(&self) -> Option<Element<'_, Message>> {
        let anchor = self.selection_context_menu?;
        let square_grid = matches!(
            self.project.grid_config,
            gridvana_core::model::GridConfig::Square { .. }
        );
        let transform = |transform: PixelTransform| {
            square_grid.then(|| Message::TransformPixelSelectionSequence(vec![transform]))
        };

        // (label, shortcut hint, message).
        let entries: Vec<(&'static str, &'static str, Option<Message>)> = vec![
            (tr("Cut", "剪切"), "⌘X", Some(Message::CutPixelSelection)),
            (tr("Copy", "复制"), "⌘C", Some(Message::CopySelection)),
            (
                tr("Paste", "粘贴"),
                "⌘V",
                self.selection_clipboard
                    .is_some()
                    .then_some(Message::PasteSelection),
            ),
            (
                tr("Duplicate", "创建副本"),
                "⌘D",
                Some(Message::DuplicateSelection),
            ),
            (
                tr("Delete", "删除"),
                "⌫",
                Some(Message::DeletePixelSelection),
            ),
            (
                tr("Flip horizontal", "水平翻转"),
                "⇧H",
                transform(PixelTransform::FlipHorizontal),
            ),
            (
                tr("Flip vertical", "垂直翻转"),
                "⇧V",
                transform(PixelTransform::FlipVertical),
            ),
            (
                tr("Rotate clockwise", "顺时针旋转"),
                "⇧R",
                transform(PixelTransform::RotateClockwise),
            ),
            (
                tr("Rotate counterclockwise", "逆时针旋转"),
                "⇧L",
                transform(PixelTransform::RotateCounterClockwise),
            ),
            (
                tr("Deselect", "取消选择"),
                "Esc",
                Some(Message::ClearPixelSelection),
            ),
        ];
        let menu_width = 188.0;

        Some(context_menu_overlay(
            anchor,
            menu_width,
            entries,
            Message::CloseSelectionContextMenu,
        ))
    }

    pub(super) fn canvas_context_menu_overlay(&self) -> Option<Element<'_, Message>> {
        let anchor = self.canvas_context_menu?;
        let square_grid = matches!(
            self.project.grid_config,
            gridvana_core::model::GridConfig::Square { .. }
        );

        let entries: Vec<(&'static str, &'static str, Option<Message>)> = vec![
            (
                tr("Crop to selection", "裁到选区"),
                "",
                (square_grid && !self.selection_indices.is_empty())
                    .then_some(Message::CropCanvasToSelection),
            ),
            (
                tr("Trim transparent edges", "裁去透明边"),
                "",
                square_grid.then_some(Message::TrimCanvas),
            ),
            (
                tr("Canvas size…", "画布尺寸…"),
                "",
                Some(Message::ToggleCanvasSizePopover),
            ),
        ];

        Some(context_menu_overlay(
            anchor,
            196.0,
            entries,
            Message::CloseCanvasContextMenu,
        ))
    }

    pub(super) fn canvas_size_popover_overlay(&self) -> Option<Element<'_, Message>> {
        if !self.canvas_size_popover_open {
            return None;
        }

        let card = widget::container(
            widget::column![
                widget::text(tr("Canvas size", "画布尺寸"))
                    .size(12)
                    .color(TEXT_PRIMARY),
                widget::row![
                    widget::text_input(tr("Width", "宽"), &self.resize_canvas_width)
                        .on_input(Message::UpdateResizeCanvasWidth)
                        .on_submit(Message::ResizeCurrentCanvas)
                        .padding([6, 7])
                        .size(10)
                        .style(text_input_style)
                        .width(Length::FillPortion(1)),
                    widget::text_input(tr("Height", "高"), &self.resize_canvas_height)
                        .on_input(Message::UpdateResizeCanvasHeight)
                        .on_submit(Message::ResizeCurrentCanvas)
                        .padding([6, 7])
                        .size(10)
                        .style(text_input_style)
                        .width(Length::FillPortion(1)),
                ]
                .spacing(6),
                widget::row![
                    widget::button(widget::text(tr("Cancel", "取消")).size(10))
                        .on_press(Message::CloseCanvasSizePopover)
                        .padding([6, 7])
                        .width(Length::FillPortion(1))
                        .style(|theme: &Theme, status| {
                            compact_action_button_style(theme, status, false)
                        }),
                    widget::button(widget::text(tr("Apply", "应用")).size(10))
                        .on_press(Message::ResizeCurrentCanvas)
                        .padding([6, 7])
                        .width(Length::FillPortion(1))
                        .style(|theme: &Theme, status| {
                            compact_action_button_style(theme, status, true)
                        }),
                ]
                .spacing(6),
            ]
            .spacing(8),
        )
        .width(Length::Fixed(220.0))
        .padding(12)
        .style(|_theme: &Theme| {
            widget::container::Style::default()
                .background(OVERLAY_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 3.0.into(),
                })
        });

        Some(widget::opaque(
            widget::mouse_area(
                widget::container(card)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center(Length::Fill),
            )
            .on_press(Message::CloseCanvasSizePopover)
            .on_right_press(Message::CloseCanvasSizePopover),
        ))
    }

    pub(super) fn editor_status_bar(&self) -> Element<'_, Message> {
        if !self.has_visible_canvas() {
            let settings_button: Element<'_, Message> = {
                #[cfg(any(target_os = "linux", windows))]
                {
                    widget::button(widget::text("☰").size(14))
                        .on_press(Message::ToggleAppMenu)
                        .padding(0)
                        .style(widget::button::text)
                        .into()
                }
                #[cfg(not(any(target_os = "linux", windows)))]
                {
                    widget::Space::new().into()
                }
            };
            return widget::container(
                widget::row![settings_button, widget::Space::new().width(Length::Fill),]
                    .spacing(12)
                    .align_y(iced::Alignment::Center),
            )
            .padding([4, 8])
            .height(Length::Fixed(24.0))
            .style(panel_style)
            .into();
        }

        let displayed_project = self.displayed_project();
        let save_status = if self.autosave_error.is_some() {
            tr("Autosave failed", "自动保存失败")
        } else if self.is_saved {
            tr("Saved", "已保存")
        } else {
            tr("Unsaved", "未保存")
        };

        let status_settings_button: Element<'_, Message> = {
            #[cfg(any(target_os = "linux", windows))]
            {
                widget::button(widget::text("☰").size(14))
                    .on_press(Message::ToggleAppMenu)
                    .padding(0)
                    .style(widget::button::text)
                    .into()
            }
            #[cfg(not(any(target_os = "linux", windows)))]
            {
                widget::Space::new().into()
            }
        };
        widget::container(
            widget::row![
                status_settings_button,
                widget::button(
                    widget::text(format!(
                        "{} {} × {} ▾",
                        tr("Grid", "网格"),
                        displayed_project.canvas_width,
                        displayed_project.canvas_height,
                    ))
                    .size(10)
                    .color(TEXT_MUTED)
                )
                .on_press(Message::ToggleCanvasSizePopover)
                .padding(0)
                .style(widget::button::text),
                widget::Space::new().width(Length::Fill),
                widget::text(format!("RGBA · Schema V6 · {save_status}"))
                    .size(10)
                    .color(TEXT_MUTED),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
        .padding([4, 8])
        .height(Length::Fixed(24.0))
        .style(panel_style)
        .into()
    }
}

fn context_menu_overlay(
    anchor: iced::Point,
    menu_width: f32,
    entries: Vec<(&'static str, &'static str, Option<Message>)>,
    close: Message,
) -> Element<'static, Message> {
    let menu_height = 26.0 * entries.len() as f32 + 8.0;

    let positioned_menu = widget::responsive(move |size| {
        let items = entries
            .iter()
            .map(|(label, shortcut, message)| {
                let enabled = message.is_some();
                widget::button(
                    widget::row![
                        widget::text(*label).size(11).color(if enabled {
                            TEXT_PRIMARY
                        } else {
                            TEXT_MUTED
                        }),
                        widget::Space::new().width(Length::Fill),
                        widget::text(*shortcut).size(10).color(TEXT_MUTED),
                    ]
                    .align_y(iced::Alignment::Center),
                )
                .on_press_maybe(message.clone())
                .padding([5, 10])
                .width(Length::Fill)
                .style(|theme: &Theme, status| {
                    let mut style = widget::button::text(theme, status);
                    if matches!(status, widget::button::Status::Hovered) {
                        style.background = Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.07).into());
                    }
                    style.border = iced::Border {
                        color: iced::Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    };
                    style
                })
                .into()
            })
            .collect::<Vec<Element<'static, Message>>>();
        let menu = widget::container(widget::column(items).spacing(0))
            .width(Length::Fixed(menu_width))
            .padding(4)
            .style(|_theme: &Theme| {
                widget::container::Style::default()
                    .background(OVERLAY_BACKGROUND)
                    .border(iced::Border {
                        color: BORDER_SUBTLE,
                        width: 1.0,
                        radius: 3.0.into(),
                    })
            });

        widget::column![
            widget::Space::new().height(Length::Fixed(
                anchor.y.min((size.height - menu_height).max(0.0))
            )),
            widget::row![
                widget::Space::new().width(Length::Fixed(
                    anchor.x.min((size.width - menu_width).max(0.0))
                )),
                menu,
                widget::Space::new().width(Length::Fill),
            ],
            widget::Space::new().height(Length::Fill),
        ]
        .into()
    })
    .width(Length::Fill)
    .height(Length::Fill);

    widget::opaque(
        widget::mouse_area(positioned_menu)
            .on_press(close.clone())
            .on_right_press(close),
    )
}

fn inspector_section_heading<'a>(
    label: String,
    trailing: Element<'a, Message>,
) -> widget::Container<'a, Message> {
    widget::container(
        widget::row![
            widget::text(label).size(11).color(TEXT_SECONDARY),
            widget::Space::new().width(Length::Fill),
            trailing,
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([5, 10])
    .height(Length::Fixed(34.0))
    .width(Length::Fill)
}

fn color_swatch(
    color: gridvana_core::model::Rgba,
    size: f32,
    message: Message,
) -> Element<'static, Message> {
    widget::button(
        widget::container(widget::Space::new())
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_| {
                widget::container::Style::default()
                    .background(iced::Color::from_rgba(color.r, color.g, color.b, color.a))
                    .border(iced::Border {
                        color: BORDER_STRONG,
                        width: 1.0,
                        radius: 1.0.into(),
                    })
            }),
    )
    .on_press(message)
    .padding(0)
    .style(widget::button::text)
    .into()
}
