use super::super::ui::{
    ACCENT, APP_BACKGROUND, BORDER_SUBTLE, OVERLAY_BACKGROUND, PANEL_BACKGROUND,
    SURFACE_BACKGROUND, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, compact_action_button_style,
    panel_style, text_input_style,
};
use super::super::{Gridvana, TimelineDrag};
use crate::icons::{Icon, icon_button};
use crate::types::Message;
use gridvana_core::model::{CelPosition, LayerKind, TagDirection};
use iced::{Element, Length, Theme, widget};

impl Gridvana {
    pub(super) fn editor_empty_bottom_panel(&self) -> Element<'_, Message> {
        let timeline_controls = widget::row![
            icon_button(Icon::Play, 12.0, 22.0, false, true),
            widget::text("时间轴").size(12).color(TEXT_PRIMARY),
            widget::text("帧 —").size(11).color(TEXT_SECONDARY),
        ]
        .spacing(6)
        .padding([4, 7])
        .align_y(iced::Alignment::Center);

        let empty_timeline = widget::row![
            widget::container(widget::text("图层 0").size(12))
                .width(Length::Fixed(208.0))
                .height(Length::Fill)
                .padding([6, 4])
                .style(|_theme: &Theme| {
                    widget::container::Style::default().background(SURFACE_BACKGROUND)
                }),
            widget::container(
                widget::text("创建画布后显示帧和图层")
                    .size(11)
                    .color(TEXT_MUTED),
            )
            .center(Length::Fill),
        ]
        .height(Length::Fill);

        widget::container(
            widget::column![
                timeline_controls,
                widget::rule::horizontal(1),
                empty_timeline
            ]
            .spacing(5),
        )
        .width(Length::Fill)
        .height(Length::Fixed(220.0))
        .padding(0)
        .style(panel_style)
        .into()
    }

    pub(super) fn editor_bottom_panel(&self) -> Element<'_, Message> {
        let active_layer_count = self.project.layers.len();
        let active_frame_id = self.project.active_frame_id;
        let frame_duration = self
            .project
            .current_frame()
            .map_or_else(|| "100".to_string(), |frame| frame.duration_ms.to_string());
        let onion_settings = self.onion_skin_settings;
        let onion_settings_controls: Vec<Element<'_, Message>> = if self.onion_skin_enabled {
            vec![
                widget::text(format!("前 {}", onion_settings.previous_frames))
                    .size(9)
                    .color(TEXT_SECONDARY)
                    .into(),
                widget::slider(
                    0..=4,
                    onion_settings.previous_frames,
                    Message::SetOnionPreviousFrames,
                )
                .width(Length::Fixed(56.0))
                .into(),
                widget::text(format!("后 {}", onion_settings.next_frames))
                    .size(9)
                    .color(TEXT_SECONDARY)
                    .into(),
                widget::slider(
                    0..=4,
                    onion_settings.next_frames,
                    Message::SetOnionNextFrames,
                )
                .width(Length::Fixed(56.0))
                .into(),
                widget::text(format!("强度 {}%", onion_settings.opacity_percent))
                    .size(9)
                    .color(TEXT_SECONDARY)
                    .into(),
                widget::slider(
                    0..=100,
                    onion_settings.opacity_percent,
                    Message::SetOnionOpacity,
                )
                .width(Length::Fixed(76.0))
                .into(),
                timeline_toggle(
                    "前帧着色",
                    onion_settings.tint_previous,
                    Message::ToggleOnionPreviousTint,
                )
                .into(),
                timeline_toggle(
                    "后帧着色",
                    onion_settings.tint_next,
                    Message::ToggleOnionNextTint,
                )
                .into(),
                timeline_toggle(
                    "仅活动图层",
                    onion_settings.active_layer_only,
                    Message::ToggleOnionActiveLayerOnly,
                )
                .into(),
            ]
        } else {
            Vec::new()
        };

        let mut control_items: Vec<Element<'_, Message>> = vec![
            widget::text("时间轴").size(11).color(TEXT_PRIMARY).into(),
            icon_button(
                if self.is_playing {
                    Icon::Pause
                } else {
                    Icon::Play
                },
                12.0,
                22.0,
                self.is_playing,
                true,
            )
            .on_press(Message::TogglePlayback)
            .into(),
            widget::text_input("ms", &frame_duration)
                .on_input(move |value| Message::SetFrameDuration(active_frame_id, value))
                .padding([4, 6])
                .size(9)
                .width(Length::Fixed(54.0))
                .style(text_input_style)
                .into(),
            widget::text("ms").size(9).color(TEXT_MUTED).into(),
            widget::Space::new().width(Length::Fixed(6.0)).into(),
            timeline_action("＋标签", Message::AddAnimationTag).into(),
            timeline_action("＋组", Message::AddLayerGroup).into(),
            widget::Space::new().width(Length::Fill).into(),
        ];
        control_items.push(
            timeline_toggle("洋葱皮", self.onion_skin_enabled, Message::ToggleOnionSkin).into(),
        );
        control_items.extend(onion_settings_controls);

        let timeline_controls = widget::row(control_items)
            .spacing(6)
            .padding([4, 7])
            .height(Length::Fixed(36.0))
            .align_y(iced::Alignment::Center);

        let layer_header_width = 208.0;
        let cel_width = 64.0;
        let row_height = 27.0;

        let frame_numbers = self
            .project
            .frames
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                let is_active_frame = frame.id == self.project.active_frame_id;
                let is_drop_target = matches!(
                    self.timeline_drag,
                    Some(TimelineDrag::Frame { target, .. }) if target == i
                );
                let frame_label = widget::container(
                    widget::column![
                        widget::text(format!("{}", i + 1)).size(9),
                        widget::text(format!("{}ms", frame.duration_ms)).size(7),
                    ]
                    .spacing(0)
                    .align_x(iced::Alignment::Center),
                )
                .width(Length::Fixed(cel_width))
                .height(Length::Fixed(row_height))
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
                .style(move |_theme: &Theme| {
                    let bg = if is_drop_target {
                        iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.38)
                    } else if is_active_frame {
                        ACCENT
                    } else {
                        PANEL_BACKGROUND
                    };
                    widget::container::Style::default()
                        .color(if is_active_frame || is_drop_target {
                            APP_BACKGROUND
                        } else {
                            TEXT_SECONDARY
                        })
                        .background(bg)
                        .border(iced::Border {
                            color: if is_drop_target {
                                ACCENT
                            } else {
                                BORDER_SUBTLE
                            },
                            width: 1.0,
                            radius: 0.0.into(),
                        })
                });

                widget::mouse_area(frame_label)
                    .on_press(Message::BeginFrameDrag(frame.id))
                    .on_enter(Message::HoverTimelineDrag(i))
                    .on_release(Message::FinishTimelineDrag)
                    .interaction(iced::mouse::Interaction::Grab)
                    .into()
            })
            .collect::<Vec<Element<Message>>>();

        let top_left_spacer = widget::container(
            widget::row![
                widget::text(format!("图层 {}", active_layer_count)).size(12),
                widget::Space::new().width(Length::Fill),
                icon_button(Icon::Add, 10.0, 18.0, false, true).on_press(Message::AddLayer),
            ]
            .align_y(iced::Alignment::Center)
            .padding([2, 4]),
        )
        .width(Length::Fixed(layer_header_width))
        .height(Length::Fixed(row_height))
        .style(|_t: &Theme| {
            widget::container::Style::default()
                .background(SURFACE_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 0.0.into(),
                })
        });

        let frame_actions = widget::container(
            widget::row![
                icon_button(Icon::Add, 10.0, 18.0, false, true).on_press(Message::AddFrame),
                icon_button(Icon::Copy, 10.0, 18.0, false, true).on_press(Message::DuplicateFrame),
                icon_button(Icon::Remove, 10.0, 18.0, false, true)
                    .on_press(Message::RemoveFrame(self.project.active_frame_id)),
            ]
            .spacing(2)
            .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(row_height))
        .padding([2, 6])
        .align_y(iced::Alignment::Center);

        let timeline_header_row = widget::row![
            top_left_spacer,
            widget::row(frame_numbers).spacing(0),
            frame_actions,
        ]
        .spacing(0);

        let mut table_children = vec![timeline_header_row.into()];
        for tag in &self.project.tags {
            let tag_id = tag.id;
            let is_active_tag = self.project.active_tag_id == Some(tag_id);
            let from_position = self
                .project
                .frames
                .iter()
                .position(|frame| frame.id == tag.from_frame_id)
                .unwrap_or(0);
            let to_position = self
                .project
                .frames
                .iter()
                .position(|frame| frame.id == tag.to_frame_id)
                .unwrap_or(from_position);
            let (from_position, to_position) = match self.timeline_drag {
                Some(TimelineDrag::TagRange {
                    tag_id: dragged,
                    anchor,
                    target,
                }) if dragged == tag_id => {
                    let anchor_position = self
                        .project
                        .frames
                        .iter()
                        .position(|frame| frame.id == anchor)
                        .unwrap_or(from_position);
                    let target_position = self
                        .project
                        .frames
                        .iter()
                        .position(|frame| frame.id == target)
                        .unwrap_or(anchor_position);
                    (
                        anchor_position.min(target_position),
                        anchor_position.max(target_position),
                    )
                }
                _ => (from_position, to_position),
            };
            let tag_header_content: Element<'_, Message> = if is_active_tag {
                widget::text_input("标签名", &tag.name)
                    .on_input(move |name| Message::RenameAnimationTag(tag_id, name))
                    .padding(2)
                    .size(11)
                    .style(text_input_style)
                    .width(Length::Fill)
                    .into()
            } else {
                widget::button(widget::text(&tag.name).size(11))
                    .on_press(Message::SelectAnimationTag(Some(tag_id)))
                    .padding([2, 4])
                    .style(widget::button::text)
                    .width(Length::Fill)
                    .into()
            };
            let tag_header = widget::container(
                widget::row![
                    widget::button(
                        widget::text(tag_direction_symbol(tag.direction))
                            .size(10)
                            .color(TEXT_SECONDARY)
                    )
                    .on_press(Message::CycleAnimationTagDirection(tag_id))
                    .padding([1, 4])
                    .style(widget::button::text),
                    tag_header_content,
                    icon_button(Icon::Remove, 10.0, 18.0, false, true)
                        .on_press(Message::RemoveAnimationTag(tag_id)),
                ]
                .spacing(3)
                .align_y(iced::Alignment::Center)
                .padding(2),
            )
            .width(Length::Fixed(layer_header_width))
            .height(Length::Fixed(row_height))
            .style(move |_theme: &Theme| {
                widget::container::Style::default().background(if is_active_tag {
                    iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.16)
                } else {
                    SURFACE_BACKGROUND
                })
            });
            let tag_cells = self
                .project
                .frames
                .iter()
                .enumerate()
                .map(|(position, frame)| {
                    let frame_id = frame.id;
                    let in_range = (from_position..=to_position).contains(&position);
                    let is_start = position == from_position;
                    let is_end = position == to_position;
                    let marker = if is_start && is_end {
                        "◆"
                    } else if is_start {
                        "◀"
                    } else if is_end {
                        "▶"
                    } else if in_range {
                        "━"
                    } else {
                        ""
                    };
                    let cell = widget::container(widget::text(marker).size(9))
                        .width(Length::Fixed(cel_width))
                        .height(Length::Fixed(row_height))
                        .align_x(iced::Alignment::Center)
                        .align_y(iced::Alignment::Center)
                        .style(move |_theme: &Theme| {
                            widget::container::Style::default().background(if in_range {
                                iced::Color::from_rgba(
                                    ACCENT.r,
                                    ACCENT.g,
                                    ACCENT.b,
                                    if is_active_tag { 0.28 } else { 0.14 },
                                )
                            } else {
                                PANEL_BACKGROUND
                            })
                        });
                    widget::mouse_area(cell)
                        .on_press(Message::BeginTagRangeDrag(tag_id, frame_id))
                        .on_enter(Message::HoverTagRangeDrag(frame_id))
                        .on_release(Message::FinishTimelineDrag)
                        .on_right_press(Message::ToggleAnimationTagSelection(tag_id))
                        .interaction(iced::mouse::Interaction::ResizingHorizontally)
                        .into()
                })
                .collect::<Vec<Element<Message>>>();
            table_children.push(widget::row![tag_header, widget::row(tag_cells).spacing(0)].into());
        }
        if self.project.current_frame().is_some() {
            let layer_count = self.project.layers.len();
            for (row_index, layer_idx) in (0..layer_count).rev().enumerate() {
                let layer = &self.project.layers[layer_idx];
                let layer_id = layer.id;
                let layer_kind = layer.kind;
                let layer_depth = self.project.layer_depth(layer_id).unwrap_or(0);
                let is_active_layer = layer_id == self.project.active_layer_id;
                let is_even_row = row_index % 2 == 0;
                let is_drop_target = matches!(
                    self.timeline_drag,
                    Some(TimelineDrag::Layer { target, .. }) if target == layer_idx
                );

                let layer_header_content: Element<'_, Message> = if is_active_layer {
                    widget::container(
                        widget::text_input("", &layer.name)
                            .on_input(move |value| Message::RenameLayer(layer_id, value))
                            .padding(2)
                            .size(12)
                            .style(text_input_style)
                            .width(iced::Length::Fill),
                    )
                    .width(iced::Length::Fill)
                    .align_y(iced::Alignment::Center)
                    .into()
                } else {
                    widget::button(widget::text(&layer.name).size(12))
                        .on_press(Message::SelectLayer(layer_id))
                        .padding([2, 4])
                        .style(move |t: &Theme, status| {
                            let mut style = widget::button::text(t, status);
                            if matches!(status, widget::button::Status::Hovered) {
                                style.background =
                                    Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.08).into());
                            }
                            style.border = iced::Border {
                                color: iced::Color::TRANSPARENT,
                                width: 0.0,
                                radius: iced::border::Radius::from(5.0),
                            };
                            style
                        })
                        .width(iced::Length::Fill)
                        .into()
                };

                let drag_handle = widget::mouse_area(
                    widget::container(widget::text("≡").size(10).color(TEXT_MUTED))
                        .width(Length::Fixed(14.0))
                        .center_y(Length::Fill),
                )
                .on_press(Message::BeginLayerDrag(layer_id))
                .on_release(Message::FinishTimelineDrag)
                .interaction(iced::mouse::Interaction::Grab);
                let layer_header = widget::mouse_area(
                    widget::container(
                        widget::row![
                            widget::Space::new()
                                .width(Length::Fixed((layer_depth.min(6) * 8) as f32)),
                            drag_handle,
                            widget::text(layer_kind_symbol(layer_kind))
                                .size(9)
                                .color(TEXT_SECONDARY),
                            icon_button(
                                if layer.visible {
                                    Icon::Eye
                                } else {
                                    Icon::EyeSlash
                                },
                                10.0,
                                18.0,
                                layer.visible,
                                false
                            )
                            .on_press(Message::ToggleLayerVisibility(layer_id)),
                            widget::button(
                                widget::text(if layer.locked { "锁" } else { "开" }).size(9)
                            )
                            .on_press(Message::ToggleLayerLocked(layer_id))
                            .padding([1, 3]),
                            layer_header_content,
                            icon_button(Icon::Remove, 10.0, 18.0, false, true)
                                .on_press(Message::RemoveLayer(layer_id)),
                        ]
                        .spacing(2)
                        .align_y(iced::Alignment::Center)
                        .padding(2),
                    )
                    .width(Length::Fixed(layer_header_width))
                    .height(Length::Fixed(row_height))
                    .style(move |t: &Theme| {
                        let p = t.extended_palette();
                        let stripe_bg = if is_even_row {
                            p.background.weak.color
                        } else {
                            p.background.base.color
                        };
                        widget::container::Style::default()
                            .background(if is_drop_target {
                                iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.30)
                            } else if is_active_layer {
                                iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.16)
                            } else {
                                stripe_bg
                            })
                            .border(iced::Border {
                                color: if is_drop_target {
                                    ACCENT
                                } else {
                                    iced::Color::TRANSPARENT
                                },
                                width: if is_drop_target { 1.0 } else { 0.0 },
                                radius: 0.0.into(),
                            })
                    }),
                )
                .on_enter(Message::HoverTimelineDrag(layer_idx))
                .on_release(Message::FinishTimelineDrag);

                let cels = self
                    .project
                    .frames
                    .iter()
                    .map(|frm| {
                        if layer_kind == LayerKind::Group {
                            return widget::container(widget::text("-").size(9).color(TEXT_MUTED))
                                .width(Length::Fixed(cel_width))
                                .height(Length::Fixed(row_height))
                                .align_x(iced::Alignment::Center)
                                .align_y(iced::Alignment::Center)
                                .style(move |t: &Theme| {
                                    let p = t.extended_palette();
                                    widget::container::Style::default().background(
                                        if frm.id == self.project.active_frame_id {
                                            p.background.strong.color
                                        } else if is_even_row {
                                            p.background.base.color
                                        } else {
                                            p.background.weak.color
                                        },
                                    )
                                })
                                .into();
                        }
                        let cel = self.project.cel(layer_id, frm.id);
                        let has_cel = cel.is_some();
                        let linked = cel.is_some_and(|cel| cel.linked_cel_id.is_some());
                        let has_content = cel
                            .and_then(|cel| self.project.resolved_cel(cel).ok())
                            .is_some_and(|cel| !cel.pixels.is_empty());
                        let is_active_cel =
                            is_active_layer && frm.id == self.project.active_frame_id;
                        let position = CelPosition {
                            layer_id,
                            frame_id: frm.id,
                        };
                        let is_selected = self.timeline_selection.contains(&position);
                        let is_cel_drop_target = matches!(
                            self.timeline_drag,
                            Some(TimelineDrag::Cel { target, .. }) if target == position
                        );

                        let content_marker = widget::container(
                            widget::text(if linked {
                                "▧"
                            } else if has_content {
                                "■"
                            } else if has_cel {
                                "□"
                            } else {
                                ""
                            })
                            .size(10)
                            .color(
                                if has_content || linked || is_selected {
                                    TEXT_PRIMARY
                                } else {
                                    TEXT_MUTED
                                },
                            ),
                        )
                        .width(Length::Fixed(cel_width))
                        .height(Length::Fixed(row_height))
                        .align_x(iced::Alignment::Center)
                        .align_y(iced::Alignment::Center)
                        .style(move |t: &Theme| {
                            let p = t.extended_palette();
                            let stripe_bg = if is_even_row {
                                p.background.base.color
                            } else {
                                p.background.weak.color
                            };
                            let background = if is_cel_drop_target {
                                iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.38)
                            } else if is_selected {
                                iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.24)
                            } else if is_active_cel {
                                iced::Color::from_rgba(ACCENT.r, ACCENT.g, ACCENT.b, 0.12)
                            } else if frm.id == self.project.active_frame_id {
                                p.background.strong.color
                            } else {
                                stripe_bg
                            };
                            widget::container::Style::default()
                                .background(background)
                                .border(iced::Border {
                                    color: if is_cel_drop_target || is_selected || is_active_cel {
                                        ACCENT
                                    } else {
                                        BORDER_SUBTLE
                                    },
                                    width: 1.0,
                                    radius: 0.0.into(),
                                })
                        });

                        widget::mouse_area(content_marker)
                            .on_press(Message::BeginCelDrag(layer_id, frm.id))
                            .on_enter(Message::HoverCelDrag(layer_id, frm.id))
                            .on_release(Message::FinishTimelineDrag)
                            .on_right_press(Message::OpenCelContextMenu(layer_id, frm.id))
                            .interaction(iced::mouse::Interaction::Grab)
                            .into()
                    })
                    .collect::<Vec<Element<Message>>>();

                let row = widget::row![layer_header, widget::row(cels).spacing(0)].spacing(0);
                table_children.push(row.into());
            }
        }

        let bottom_panel_content = widget::column![
            timeline_controls,
            widget::rule::horizontal(1),
            widget::scrollable(widget::column(table_children).spacing(0))
                .height(Length::Fill)
                .direction(widget::scrollable::Direction::Both {
                    vertical: widget::scrollable::Scrollbar::new(),
                    horizontal: widget::scrollable::Scrollbar::new(),
                }),
        ]
        .spacing(0);

        widget::container(bottom_panel_content)
            .width(Length::Fill)
            .height(Length::Fixed(274.0))
            .padding(0)
            .style(panel_style)
            .into()
    }

    pub(super) fn cel_context_menu_overlay(&self) -> Option<Element<'_, Message>> {
        self.cel_context_menu?;
        let cursor_position = self.cursor_position?;
        let selected_cels = self.timeline_selection.len();
        let populated_cels = self
            .timeline_selection
            .iter()
            .filter(|position| {
                self.project
                    .cel(position.layer_id, position.frame_id)
                    .and_then(|cel| self.project.resolved_cel(cel).ok())
                    .is_some_and(|cel| !cel.pixels.is_empty())
            })
            .count();

        let entries: Vec<(&'static str, Option<Message>)> = vec![
            (
                "复制",
                (selected_cels > 0).then_some(Message::CopyTimelineCels),
            ),
            (
                "粘贴",
                (selected_cels > 0 && self.timeline_cel_clipboard.is_some())
                    .then_some(Message::PasteTimelineCels),
            ),
            (
                "删除",
                (selected_cels > 0).then_some(Message::DeleteTimelineCels),
            ),
            (
                "链接",
                (selected_cels > 1 && populated_cels > 0).then_some(Message::LinkTimelineCels),
            ),
            (
                "取消链接",
                (selected_cels > 0).then_some(Message::UnlinkTimelineCels),
            ),
        ];
        let menu_width = 128.0;
        let menu_height = 26.0 * entries.len() as f32 + 8.0;

        let positioned_menu = widget::responsive(move |size| {
            let items = entries
                .iter()
                .map(|(label, message)| {
                    let enabled = message.is_some();
                    widget::button(widget::text(*label).size(11).color(if enabled {
                        TEXT_PRIMARY
                    } else {
                        TEXT_MUTED
                    }))
                    .on_press_maybe(message.clone())
                    .padding([5, 10])
                    .width(Length::Fill)
                    .style(|theme: &Theme, status| {
                        let mut style = widget::button::text(theme, status);
                        if matches!(status, widget::button::Status::Hovered) {
                            style.background =
                                Some(iced::Color::from_rgba(1.0, 1.0, 1.0, 0.07).into());
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
                    cursor_position.y.min((size.height - menu_height).max(0.0))
                )),
                widget::row![
                    widget::Space::new().width(Length::Fixed(
                        cursor_position.x.min((size.width - menu_width).max(0.0))
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

        Some(widget::opaque(
            widget::mouse_area(positioned_menu)
                .on_press(Message::CloseCelContextMenu)
                .on_right_press(Message::CloseCelContextMenu),
        ))
    }
}

fn timeline_toggle(
    label: &'static str,
    active: bool,
    message: Message,
) -> widget::Button<'static, Message> {
    widget::button(widget::text(label).size(9))
        .on_press(message)
        .padding([3, 7])
        .style(move |theme: &Theme, status| compact_action_button_style(theme, status, active))
}

fn timeline_action(label: &'static str, message: Message) -> widget::Button<'static, Message> {
    widget::button(widget::text(label).size(9))
        .on_press(message)
        .padding([3, 7])
        .style(|theme: &Theme, status| compact_action_button_style(theme, status, false))
}

fn tag_direction_symbol(direction: TagDirection) -> &'static str {
    match direction {
        TagDirection::Forward => "→",
        TagDirection::Reverse => "←",
        TagDirection::PingPong => "↔",
    }
}

fn layer_kind_symbol(kind: LayerKind) -> &'static str {
    match kind {
        LayerKind::Paint => "P",
        LayerKind::Group => "G",
        LayerKind::Background => "B",
        LayerKind::Reference => "R",
    }
}
