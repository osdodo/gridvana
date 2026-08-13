use super::super::ui::parse_hex_color_input;
use super::super::{
    Gridvana, InspectorResize, MAX_BRUSH_SIZE, MAX_ERASER_SIZE, MAX_INSPECTOR_WIDTH,
    MIN_AI_INSPECTOR_WIDTH, MIN_BRUSH_SIZE, MIN_ERASER_SIZE, MIN_INSPECTOR_WIDTH,
    SelectionMoveDraft, ShapeKind, StrokeKind,
};
use crate::types::{ColorSlot, InspectorPanel, Message, SelectionCombineMode, Tool};
use iced::Task;

impl Gridvana {
    /// Whether a modal surface with focused text inputs is open (CLI settings
    /// or the new project dialog). Used to suppress canvas keyboard
    /// shortcuts so focused text fields can handle Cmd+C/V/D etc. themselves.
    pub(super) fn text_entry_active(&self) -> bool {
        self.cli_settings_open || self.new_project_dialog_open
    }

    pub(super) fn handle_canvas_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        match message {
            Message::CanvasEvent => Ok(Task::none()),
            Message::SetInspectorPanel(panel) => {
                self.inspector_panel = panel;
                if panel == crate::types::InspectorPanel::Export {
                    self.refresh_sprite_sheet_export_estimate();
                } else {
                    self.pending_sprite_sheet_export_path = None;
                }
                self.sync_terminal_webview_visibility();
                Ok(Task::none())
            }
            Message::BeginInspectorResize => {
                if let Some(cursor_position) = self.cursor_position {
                    self.inspector_resize = Some(InspectorResize {
                        start_cursor_x: cursor_position.x,
                        start_width: self.active_inspector_width(),
                        target: self.inspector_panel,
                    });
                }
                Ok(Task::none())
            }
            Message::EndInspectorResize => {
                self.inspector_resize = None;
                Ok(Task::none())
            }
            Message::GlobalLeftPressed
            | Message::GlobalLeftReleased
            | Message::UpdateCursorPosition(_)
                if self.cli_settings_open
                    || self.new_project_dialog_open
                    || self.about_dialog_open =>
            {
                Ok(Task::none())
            }
            Message::StrokeStart(index, color_slot) => {
                if color_slot == ColorSlot::Background && self.selection_tool_active() {
                    return Ok(Task::none());
                }
                let drawing_color = self.color_for_slot(color_slot);
                if self.selection_tool_active() {
                    self.clear_active_drawing();

                    if self.selection_combine_mode() == SelectionCombineMode::Replace
                        && self.move_mode_active
                        && self.selection_indices.contains(&index)
                    {
                        self.selection_box_draft = None;
                        self.selection_move_draft = Some(SelectionMoveDraft {
                            start: index,
                            current: index,
                        });
                    } else {
                        match self.current_tool {
                            Tool::HandPoint => self.begin_selection_box(index),
                            Tool::MagicWand => self.select_magic_wand(index),
                            Tool::ColorSelect => self.select_same_color(index),
                            _ => unreachable!("selection_tool_active only accepts selection tools"),
                        }
                    }
                    return Ok(Task::none());
                }

                if color_slot == ColorSlot::Foreground
                    && self.move_mode_active
                    && self.selection_indices.contains(&index)
                {
                    self.clear_active_drawing();
                    self.selection_move_draft = Some(SelectionMoveDraft {
                        start: index,
                        current: index,
                    });
                    return Ok(Task::none());
                }
                if color_slot == ColorSlot::Foreground && self.move_mode_active {
                    self.deselect();
                }

                match self.current_tool {
                    Tool::Brush => {
                        self.begin_stroke(
                            StrokeKind::Brush {
                                color: drawing_color,
                                size: self.brush_size,
                            },
                            index,
                        );
                    }
                    Tool::Eraser => {
                        self.begin_stroke(
                            StrokeKind::Eraser {
                                size: self.eraser_size,
                            },
                            index,
                        );
                    }
                    Tool::PaintBucket => {
                        self.clear_active_drawing();
                        self.flood_fill(index, drawing_color);
                    }
                    Tool::Picker => {
                        self.clear_active_drawing();
                        return Ok(Task::perform(
                            async move { Message::PickColor(index, color_slot) },
                            |m| m,
                        ));
                    }
                    Tool::Rectangle => {
                        self.begin_shape(ShapeKind::Rectangle, index, drawing_color);
                    }
                    Tool::RectangleHollow => {
                        self.begin_shape(ShapeKind::RectangleHollow, index, drawing_color);
                    }
                    Tool::Circle => {
                        self.begin_shape(ShapeKind::Circle, index, drawing_color);
                    }
                    Tool::CircleHollow => {
                        self.begin_shape(ShapeKind::CircleHollow, index, drawing_color);
                    }
                    Tool::Line => {
                        self.begin_shape(ShapeKind::Line, index, drawing_color);
                    }
                    Tool::HandPoint | Tool::MagicWand | Tool::ColorSelect => {
                        unreachable!("selection tools are handled before paint tools")
                    }
                }
                Ok(Task::none())
            }
            Message::StrokeAdd(index) => {
                if let Some(box_draft) = self.selection_box_draft.as_mut() {
                    box_draft.current = index;
                } else if let Some(move_draft) = self.selection_move_draft.as_mut() {
                    move_draft.current = index;
                } else if let Some(shape) = self.current_shape.as_mut() {
                    shape.current = index;
                    self.refresh_shape_preview();
                } else if let Some(stroke) = self.current_stroke.as_mut() {
                    stroke.apply_point(&mut self.project, index);
                }
                Ok(Task::none())
            }
            Message::StrokeEnd => {
                if self.selection_box_draft.is_some() {
                    self.finalize_selection_box();
                } else if self.selection_move_draft.is_some() {
                    self.finish_selection_gesture();
                } else {
                    self.finalize_active_stroke();
                }
                Ok(Task::none())
            }
            Message::DeactivateMoveSelection => {
                if self.about_dialog_open {
                    self.about_dialog_open = false;
                    self.sync_terminal_webview_visibility();
                    return Ok(Task::none());
                }
                if self.timeline_selection.is_empty() {
                    self.deselect();
                } else {
                    self.clear_timeline_selection();
                }
                Ok(Task::none())
            }
            Message::CopySelection => {
                self.selection_context_menu = None;
                if !self.text_entry_active() {
                    if self.timeline_selection.is_empty() {
                        self.copy_selection();
                    } else {
                        return Err(Message::CopyTimelineCels);
                    }
                }
                Ok(Task::none())
            }
            Message::PasteSelection => {
                self.selection_context_menu = None;
                // When a modal text field is focused, let it handle Cmd+V paste
                // instead of pasting a selection onto the canvas.
                if !self.text_entry_active() {
                    if self.timeline_cel_clipboard.is_some() && !self.timeline_selection.is_empty()
                    {
                        return Err(Message::PasteTimelineCels);
                    } else if self.selection_clipboard.is_some() {
                        self.paste_selection();
                    }
                }
                Ok(Task::none())
            }
            Message::DuplicateSelection => {
                self.selection_context_menu = None;
                if !self.text_entry_active() {
                    self.duplicate_selection();
                }
                Ok(Task::none())
            }
            Message::SelectAllPixels => {
                if self.has_canvas && !self.text_entry_active() {
                    self.select_all_pixels();
                }
                Ok(Task::none())
            }
            Message::InvertPixelSelection => {
                if self.has_canvas && !self.text_entry_active() {
                    self.invert_pixel_selection();
                }
                Ok(Task::none())
            }
            Message::ClearPixelSelection => {
                self.selection_context_menu = None;
                if !self.text_entry_active() {
                    self.deselect();
                }
                Ok(Task::none())
            }
            Message::DeletePixelSelection => {
                self.selection_context_menu = None;
                if self.text_entry_active() {
                    return Ok(Task::none());
                }
                if self.timeline_selection.is_empty() {
                    self.delete_pixel_selection();
                    Ok(Task::none())
                } else {
                    Err(Message::DeleteTimelineCels)
                }
            }
            Message::CutPixelSelection => {
                self.selection_context_menu = None;
                if !self.text_entry_active() && self.timeline_selection.is_empty() {
                    self.cut_pixel_selection();
                }
                Ok(Task::none())
            }
            Message::OpenSelectionContextMenu => {
                self.selection_context_menu = self.cursor_position;
                Ok(Task::none())
            }
            Message::CloseSelectionContextMenu => {
                self.selection_context_menu = None;
                Ok(Task::none())
            }
            Message::TransformPixelSelectionSequence(transforms) => {
                self.selection_context_menu = None;
                self.transform_pixel_selection_sequence(&transforms);
                Ok(Task::none())
            }
            Message::CropCanvasToSelection => {
                self.crop_canvas_to_selection();
                Ok(Task::none())
            }
            Message::TrimCanvas => {
                self.trim_current_canvas();
                Ok(Task::none())
            }
            Message::UpdateResizeCanvasWidth(value) => {
                self.resize_canvas_width = value;
                Ok(Task::none())
            }
            Message::UpdateResizeCanvasHeight(value) => {
                self.resize_canvas_height = value;
                Ok(Task::none())
            }
            Message::ResizeCurrentCanvas => {
                self.resize_current_canvas();
                Ok(Task::none())
            }
            Message::MoveSelectionBy(dx, dy) => {
                if self.move_mode_active && !self.text_entry_active() {
                    self.move_selection_by(dx, dy);
                }
                Ok(Task::none())
            }
            Message::SetSpacePressed(pressed) => {
                self.space_pressed = pressed;
                Ok(Task::none())
            }
            Message::UpdateHoveredGridIndex(index) => {
                self.hovered_grid_index = index;
                Ok(Task::none())
            }
            Message::PickColor(index, color_slot) => {
                if let Some(color) = self.color_at_index(index) {
                    self.set_color_for_slot(color_slot, color);
                }
                Ok(Task::none())
            }
            Message::SelectTool(tool) => {
                if !matches!(tool, Tool::HandPoint | Tool::MagicWand | Tool::ColorSelect) {
                    self.commit_floating_selection();
                }
                self.current_tool = tool;
                self.clear_active_drawing();
                self.selection_box_draft = None;
                self.selection_move_draft = None;
                self.move_mode_active =
                    self.selection_tool_active() && !self.selection_indices.is_empty();
                Ok(Task::none())
            }
            Message::UpdateBrushSize(size) => {
                self.brush_size = size.clamp(MIN_BRUSH_SIZE, MAX_BRUSH_SIZE);
                Ok(Task::none())
            }
            Message::UpdateEraserSize(size) => {
                self.eraser_size = size.clamp(MIN_ERASER_SIZE, MAX_ERASER_SIZE);
                Ok(Task::none())
            }
            Message::SelectColor(color) => {
                self.set_current_color(color);
                Ok(Task::none())
            }
            Message::SetActiveColorSlot(slot) => {
                self.active_color_slot = slot;
                self.current_color_hex_input = super::super::ui::color_to_hex(self.active_color());
                Ok(Task::none())
            }
            Message::SwapForegroundBackground => {
                self.swap_foreground_background();
                Ok(Task::none())
            }
            Message::UpdateColorHexInput(value) => {
                self.current_color_hex_input = value;
                Ok(Task::none())
            }
            Message::SubmitColorHexInput => {
                if let Some(color) = parse_hex_color_input(&self.current_color_hex_input) {
                    self.set_current_color(color);
                }
                Ok(Task::none())
            }
            Message::UpdateColorAlpha(value) => {
                let mut color = self.active_color();
                color.a = f32::from(value) / 255.0;
                self.set_current_color(color);
                Ok(Task::none())
            }
            Message::GlobalLeftPressed => {
                self.global_left_button_down = true;
                Ok(Task::none())
            }
            Message::GlobalLeftReleased => {
                self.global_left_button_down = false;
                self.inspector_resize = None;
                self.finish_timeline_drag();
                if self.selection_box_draft.is_some() {
                    self.finalize_selection_box();
                } else if self.selection_move_draft.is_some() {
                    self.finish_selection_gesture();
                } else {
                    self.finalize_active_stroke();
                }
                Ok(Task::none())
            }
            Message::UpdateKeyboardModifiers {
                shift_pressed,
                alt_pressed,
                zoom_modifier_pressed,
            } => {
                self.shift_pressed = shift_pressed;
                self.alt_pressed = alt_pressed;
                self.zoom_modifier_pressed = zoom_modifier_pressed;
                if self.current_shape.is_some() {
                    self.refresh_shape_preview();
                }
                Ok(Task::none())
            }
            Message::UpdateCursorPosition(cursor_position) => {
                self.cursor_position = Some(cursor_position);
                if let Some(resize) = self.inspector_resize {
                    let width = resized_inspector_width(resize, cursor_position.x);
                    if resize.target == InspectorPanel::AiAgent {
                        self.ai_inspector_width = width;
                    } else {
                        self.inspector_width = width;
                    }
                }
                Ok(Task::none())
            }
            other => Err(other),
        }
    }
}

fn resized_inspector_width(resize: InspectorResize, cursor_x: f32) -> f32 {
    let minimum = if resize.target == InspectorPanel::AiAgent {
        MIN_AI_INSPECTOR_WIDTH
    } else {
        MIN_INSPECTOR_WIDTH
    };

    (resize.start_width + resize.start_cursor_x - cursor_x).clamp(minimum, MAX_INSPECTOR_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_resize_tracks_drag_direction_and_limits() {
        let resize = InspectorResize {
            start_cursor_x: 600.0,
            start_width: 304.0,
            target: InspectorPanel::Layers,
        };

        assert_eq!(resized_inspector_width(resize, 500.0), 404.0);
        assert_eq!(resized_inspector_width(resize, 700.0), 260.0);
        assert_eq!(resized_inspector_width(resize, 0.0), 720.0);
    }

    #[test]
    fn ai_inspector_keeps_a_terminal_friendly_minimum_width() {
        let resize = InspectorResize {
            start_cursor_x: 400.0,
            start_width: 520.0,
            target: InspectorPanel::AiAgent,
        };

        assert_eq!(resized_inspector_width(resize, 700.0), 360.0);
    }
}
