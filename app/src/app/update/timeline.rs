use super::super::{Gridvana, TimelineCelClipboard, TimelineClipboardCel, TimelineDrag};
use crate::types::Message;
use gridvana_core::document::{CelRelocation, DocumentOp, DocumentPixel, apply_document_op};
use gridvana_core::model::{CelPosition, FrameId, LayerKind, Project, TagDirection};
use iced::Task;
use std::collections::HashSet;

impl Gridvana {
    pub(super) fn handle_timeline_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        if !self.has_canvas {
            return match message {
                Message::AddLayer
                | Message::AddLayerGroup
                | Message::RemoveLayer(_)
                | Message::SelectLayer(_)
                | Message::ToggleLayerVisibility(_)
                | Message::ToggleLayerLocked(_)
                | Message::SetLayerOpacity(_, _)
                | Message::SetLayerBlendMode(_, _)
                | Message::SetLayerKind(_, _)
                | Message::SetLayerParent(_, _)
                | Message::RenameLayer(_, _)
                | Message::AddFrame
                | Message::DuplicateFrame
                | Message::RemoveFrame(_)
                | Message::SelectFrame(_)
                | Message::SetFrameDuration(_, _)
                | Message::BeginFrameDrag(_)
                | Message::BeginLayerDrag(_)
                | Message::HoverTimelineDrag(_)
                | Message::FinishTimelineDrag
                | Message::BeginCelDrag(_, _)
                | Message::HoverCelDrag(_, _)
                | Message::OpenCelContextMenu(_, _)
                | Message::CloseCelContextMenu
                | Message::CopyTimelineCels
                | Message::PasteTimelineCels
                | Message::DeleteTimelineCels
                | Message::LinkTimelineCels
                | Message::UnlinkTimelineCels
                | Message::AddAnimationTag
                | Message::RemoveAnimationTag(_)
                | Message::SelectAnimationTag(_)
                | Message::ToggleAnimationTagSelection(_)
                | Message::RenameAnimationTag(_, _)
                | Message::BeginTagRangeDrag(_, _)
                | Message::HoverTagRangeDrag(_)
                | Message::CycleAnimationTagDirection(_)
                | Message::TogglePlayback
                | Message::ToggleOnionSkin
                | Message::SetOnionPreviousFrames(_)
                | Message::SetOnionNextFrames(_)
                | Message::SetOnionOpacity(_)
                | Message::ToggleOnionPreviousTint
                | Message::ToggleOnionNextTint
                | Message::ToggleOnionActiveLayerOnly
                | Message::Tick(_)
                | Message::SelectCel(_, _) => Ok(Task::none()),
                other => Err(other),
            };
        }

        match message {
            Message::AddLayer => {
                self.apply_document_transaction(vec![DocumentOp::AddLayer {
                    name: None,
                    position: None,
                    kind: LayerKind::Paint,
                    parent_id: None,
                }]);
                Ok(Task::none())
            }
            Message::AddLayerGroup => {
                self.apply_document_transaction(vec![DocumentOp::AddLayer {
                    name: Some("Group".to_string()),
                    position: None,
                    kind: LayerKind::Group,
                    parent_id: None,
                }]);
                Ok(Task::none())
            }
            Message::RemoveLayer(layer_id) => {
                self.apply_document_transaction(vec![DocumentOp::RemoveLayer { layer_id }]);
                Ok(Task::none())
            }
            Message::SelectLayer(layer_id) => {
                if apply_document_op(&mut self.project, &DocumentOp::SetActiveLayer { layer_id })
                    .is_ok()
                {
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::ToggleLayerVisibility(layer_id) => {
                if let Some(layer) = self.project.layer(layer_id) {
                    self.apply_document_transaction(vec![DocumentOp::SetLayerVisibility {
                        layer_id,
                        visible: !layer.visible,
                    }]);
                }
                Ok(Task::none())
            }
            Message::ToggleLayerLocked(layer_id) => {
                if let Some(layer) = self.project.layer(layer_id) {
                    self.apply_document_transaction(vec![DocumentOp::SetLayerLocked {
                        layer_id,
                        locked: !layer.locked,
                    }]);
                }
                Ok(Task::none())
            }
            Message::SetLayerOpacity(layer_id, percent) => {
                self.apply_document_transaction(vec![DocumentOp::SetLayerOpacity {
                    layer_id,
                    opacity: percent as f32 / 100.0,
                }]);
                Ok(Task::none())
            }
            Message::SetLayerBlendMode(layer_id, blend_mode) => {
                self.apply_document_transaction(vec![DocumentOp::SetLayerBlendMode {
                    layer_id,
                    blend_mode,
                }]);
                Ok(Task::none())
            }
            Message::SetLayerKind(layer_id, kind) => {
                self.apply_document_transaction(vec![DocumentOp::SetLayerKind { layer_id, kind }]);
                Ok(Task::none())
            }
            Message::SetLayerParent(layer_id, parent_id) => {
                self.apply_document_transaction(vec![DocumentOp::SetLayerParent {
                    layer_id,
                    parent_id,
                }]);
                Ok(Task::none())
            }
            Message::RenameLayer(layer_id, name) => {
                self.apply_document_transaction(vec![
                    DocumentOp::RenameLayer { layer_id, name },
                    DocumentOp::SetActiveLayer { layer_id },
                ]);
                Ok(Task::none())
            }
            Message::AddFrame => {
                self.apply_document_transaction(vec![DocumentOp::AddFrame {
                    position: None,
                    duration_ms: None,
                }]);
                Ok(Task::none())
            }
            Message::DuplicateFrame => {
                self.apply_document_transaction(vec![DocumentOp::DuplicateFrame {
                    frame_id: self.project.active_frame_id,
                }]);
                Ok(Task::none())
            }
            Message::RemoveFrame(frame_id) => {
                self.apply_document_transaction(vec![DocumentOp::RemoveFrame { frame_id }]);
                Ok(Task::none())
            }
            Message::SelectFrame(frame_id) => {
                if apply_document_op(&mut self.project, &DocumentOp::SetActiveFrame { frame_id })
                    .is_ok()
                {
                    self.reset_playback_timing();
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::SetFrameDuration(frame_id, value) => {
                if let Ok(duration_ms) = value.parse::<u64>()
                    && duration_ms > 0
                {
                    self.apply_document_transaction(vec![DocumentOp::SetFrameDuration {
                        frame_id,
                        duration_ms,
                    }]);
                }
                Ok(Task::none())
            }
            Message::BeginFrameDrag(frame_id) => {
                if let Some(target) = self
                    .project
                    .frames
                    .iter()
                    .position(|frame| frame.id == frame_id)
                {
                    self.timeline_drag = Some(TimelineDrag::Frame { frame_id, target });
                    let _ = apply_document_op(
                        &mut self.project,
                        &DocumentOp::SetActiveFrame { frame_id },
                    );
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::BeginLayerDrag(layer_id) => {
                if let Some(target) = self
                    .project
                    .layers
                    .iter()
                    .position(|layer| layer.id == layer_id)
                {
                    self.timeline_drag = Some(TimelineDrag::Layer { layer_id, target });
                    let _ = apply_document_op(
                        &mut self.project,
                        &DocumentOp::SetActiveLayer { layer_id },
                    );
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::HoverTimelineDrag(target) => {
                if let Some(drag) = self.timeline_drag.as_mut() {
                    match drag {
                        TimelineDrag::Frame {
                            target: current, ..
                        } => {
                            *current = target.min(self.project.frames.len().saturating_sub(1));
                        }
                        TimelineDrag::Layer {
                            target: current, ..
                        } => {
                            *current = target.min(self.project.layers.len().saturating_sub(1));
                        }
                        TimelineDrag::Cel { .. } | TimelineDrag::TagRange { .. } => {}
                    }
                }
                Ok(Task::none())
            }
            Message::FinishTimelineDrag => {
                self.finish_timeline_drag();
                Ok(Task::none())
            }
            Message::BeginCelDrag(layer_id, frame_id) => {
                let position = CelPosition { layer_id, frame_id };
                if self
                    .project
                    .layer(layer_id)
                    .is_some_and(|layer| layer.kind.supports_cels())
                    && self.project.frame(frame_id).is_some()
                {
                    if !self.timeline_selection.contains(&position)
                        || self.shift_pressed
                        || self.zoom_modifier_pressed
                    {
                        self.select_timeline_cel(position);
                    } else {
                        let _ = apply_document_op(
                            &mut self.project,
                            &DocumentOp::SetActiveLayer { layer_id },
                        );
                        let _ = apply_document_op(
                            &mut self.project,
                            &DocumentOp::SetActiveFrame { frame_id },
                        );
                        self.clear_selection_state();
                    }
                    self.timeline_drag = Some(TimelineDrag::Cel {
                        origin: position,
                        target: position,
                    });
                }
                Ok(Task::none())
            }
            Message::HoverCelDrag(layer_id, frame_id) => {
                if let Some(TimelineDrag::Cel { target, .. }) = self.timeline_drag.as_mut()
                    && self
                        .project
                        .layer(layer_id)
                        .is_some_and(|layer| layer.kind.supports_cels())
                    && self.project.frame(frame_id).is_some()
                {
                    *target = CelPosition { layer_id, frame_id };
                }
                Ok(Task::none())
            }
            Message::OpenCelContextMenu(layer_id, frame_id) => {
                let position = CelPosition { layer_id, frame_id };
                if self
                    .project
                    .layer(layer_id)
                    .is_some_and(|layer| layer.kind.supports_cels())
                    && self.project.frame(frame_id).is_some()
                {
                    if !self.timeline_selection.contains(&position) {
                        self.select_timeline_cel(position);
                    }
                    self.cel_context_menu = Some(position);
                }
                Ok(Task::none())
            }
            Message::CloseCelContextMenu => {
                self.cel_context_menu = None;
                Ok(Task::none())
            }
            Message::CopyTimelineCels => {
                self.cel_context_menu = None;
                self.copy_timeline_cels();
                Ok(Task::none())
            }
            Message::PasteTimelineCels => {
                self.cel_context_menu = None;
                self.paste_timeline_cels();
                Ok(Task::none())
            }
            Message::DeleteTimelineCels => {
                self.cel_context_menu = None;
                self.delete_timeline_cels();
                Ok(Task::none())
            }
            Message::LinkTimelineCels => {
                self.cel_context_menu = None;
                self.link_timeline_cels();
                Ok(Task::none())
            }
            Message::UnlinkTimelineCels => {
                self.cel_context_menu = None;
                self.unlink_timeline_cels();
                Ok(Task::none())
            }
            Message::AddAnimationTag => {
                let frame_id = self.project.active_frame_id;
                let name = format!("Tag {}", self.project.tags.len() + 1);
                if self.apply_document_transaction(vec![DocumentOp::AddTag {
                    name,
                    from_frame_id: frame_id,
                    to_frame_id: frame_id,
                    direction: TagDirection::Forward,
                }]) {
                    self.reset_playback_timing();
                }
                Ok(Task::none())
            }
            Message::RemoveAnimationTag(tag_id) => {
                if self.apply_document_transaction(vec![DocumentOp::RemoveTag { tag_id }]) {
                    self.reset_playback_timing();
                }
                Ok(Task::none())
            }
            Message::SelectAnimationTag(tag_id) => {
                let mut ops = vec![DocumentOp::SetActiveTag { tag_id }];
                if let Some(tag_id) = tag_id
                    && let Ok(frame_ids) = self.project.frame_ids_for_tag(tag_id)
                    && let Some(frame_id) = frame_ids.first()
                {
                    ops.push(DocumentOp::SetActiveFrame {
                        frame_id: *frame_id,
                    });
                }
                if self.apply_document_transaction(ops) {
                    self.reset_playback_timing();
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::ToggleAnimationTagSelection(tag_id) => {
                let next = (self.project.active_tag_id != Some(tag_id)).then_some(tag_id);
                let mut ops = vec![DocumentOp::SetActiveTag { tag_id: next }];
                if let Some(tag_id) = next
                    && let Ok(frame_ids) = self.project.frame_ids_for_tag(tag_id)
                    && let Some(frame_id) = frame_ids.first()
                {
                    ops.push(DocumentOp::SetActiveFrame {
                        frame_id: *frame_id,
                    });
                }
                if self.apply_document_transaction(ops) {
                    self.reset_playback_timing();
                    self.clear_selection_state();
                }
                Ok(Task::none())
            }
            Message::RenameAnimationTag(tag_id, name) => {
                self.apply_document_transaction(vec![DocumentOp::RenameTag { tag_id, name }]);
                Ok(Task::none())
            }
            Message::BeginTagRangeDrag(tag_id, frame_id) => {
                if self.project.tag(tag_id).is_some() && self.project.frame(frame_id).is_some() {
                    self.timeline_drag = Some(TimelineDrag::TagRange {
                        tag_id,
                        anchor: frame_id,
                        target: frame_id,
                    });
                }
                Ok(Task::none())
            }
            Message::HoverTagRangeDrag(frame_id) => {
                if let Some(TimelineDrag::TagRange { target, .. }) = self.timeline_drag.as_mut()
                    && self.project.frame(frame_id).is_some()
                {
                    *target = frame_id;
                }
                Ok(Task::none())
            }
            Message::CycleAnimationTagDirection(tag_id) => {
                if let Some(tag) = self.project.tag(tag_id) {
                    let direction = match tag.direction {
                        TagDirection::Forward => TagDirection::Reverse,
                        TagDirection::Reverse => TagDirection::PingPong,
                        TagDirection::PingPong => TagDirection::Forward,
                    };
                    if self.apply_document_transaction(vec![DocumentOp::SetTagDirection {
                        tag_id,
                        direction,
                    }]) {
                        self.reset_playback_timing();
                    }
                }
                Ok(Task::none())
            }
            Message::TogglePlayback => {
                self.is_playing = !self.is_playing;
                self.reset_playback_timing();
                synchronize_playback_cursor(&mut self.project, &mut self.playback_sequence_index);
                Ok(Task::none())
            }
            Message::ToggleOnionSkin => {
                self.onion_skin_enabled = !self.onion_skin_enabled;
                Ok(Task::none())
            }
            Message::SetOnionPreviousFrames(frames) => {
                self.onion_skin_settings.previous_frames = frames.min(4);
                Ok(Task::none())
            }
            Message::SetOnionNextFrames(frames) => {
                self.onion_skin_settings.next_frames = frames.min(4);
                Ok(Task::none())
            }
            Message::SetOnionOpacity(opacity) => {
                self.onion_skin_settings.opacity_percent = opacity.min(100);
                Ok(Task::none())
            }
            Message::ToggleOnionPreviousTint => {
                self.onion_skin_settings.tint_previous = !self.onion_skin_settings.tint_previous;
                Ok(Task::none())
            }
            Message::ToggleOnionNextTint => {
                self.onion_skin_settings.tint_next = !self.onion_skin_settings.tint_next;
                Ok(Task::none())
            }
            Message::ToggleOnionActiveLayerOnly => {
                self.onion_skin_settings.active_layer_only =
                    !self.onion_skin_settings.active_layer_only;
                Ok(Task::none())
            }
            Message::Tick(now) => {
                if self.is_playing && !self.project.frames.is_empty() {
                    let Some(last_tick) = self.playback_last_tick.replace(now) else {
                        return Ok(Task::none());
                    };
                    self.playback_elapsed += now.saturating_duration_since(last_tick);
                    advance_playback(
                        &mut self.project,
                        &mut self.playback_elapsed,
                        &mut self.playback_sequence_index,
                    );
                }
                Ok(Task::none())
            }
            Message::SelectCel(layer_id, frame_id) => {
                if self
                    .project
                    .layer(layer_id)
                    .is_some_and(|layer| layer.kind.supports_cels())
                    && self.project.frame(frame_id).is_some()
                {
                    let _ = apply_document_op(
                        &mut self.project,
                        &DocumentOp::SetActiveLayer { layer_id },
                    );
                    let _ = apply_document_op(
                        &mut self.project,
                        &DocumentOp::SetActiveFrame { frame_id },
                    );
                    self.select_timeline_cel(CelPosition { layer_id, frame_id });
                }
                Ok(Task::none())
            }
            other => Err(other),
        }
    }

    fn reset_playback_timing(&mut self) {
        self.playback_last_tick = None;
        self.playback_elapsed = std::time::Duration::ZERO;
        self.playback_sequence_index = 0;
    }

    pub(in crate::app) fn finish_timeline_drag(&mut self) {
        let Some(drag) = self.timeline_drag.take() else {
            return;
        };
        if let TimelineDrag::Cel { origin, target } = drag {
            self.finish_cel_drag(origin, target);
            return;
        }
        let Some(op) = timeline_drag_op(&self.project, drag) else {
            return;
        };
        if self.apply_document_transaction(vec![op])
            && matches!(drag, TimelineDrag::TagRange { .. })
        {
            self.reset_playback_timing();
        }
    }

    fn select_timeline_cel(&mut self, position: CelPosition) {
        let previous_anchor = self.timeline_selection_anchor;
        if self.shift_pressed {
            let anchor = previous_anchor.unwrap_or(position);
            let range = timeline_selection_range(&self.project, anchor, position);
            if !self.zoom_modifier_pressed {
                self.timeline_selection.clear();
            }
            self.timeline_selection.extend(range);
        } else if self.zoom_modifier_pressed {
            if !self.timeline_selection.insert(position) {
                self.timeline_selection.remove(&position);
            }
            self.timeline_selection_anchor = Some(position);
        } else {
            self.timeline_selection.clear();
            self.timeline_selection.insert(position);
            self.timeline_selection_anchor = Some(position);
        }
        if self.timeline_selection.is_empty() {
            self.timeline_selection_anchor = None;
        }
        let _ = apply_document_op(
            &mut self.project,
            &DocumentOp::SetActiveLayer {
                layer_id: position.layer_id,
            },
        );
        let _ = apply_document_op(
            &mut self.project,
            &DocumentOp::SetActiveFrame {
                frame_id: position.frame_id,
            },
        );
        self.clear_selection_state();
        self.reset_playback_timing();
    }

    fn copy_timeline_cels(&mut self) {
        let Some((min_layer, min_frame)) =
            timeline_selection_origin(&self.project, self.timeline_selection.iter().copied())
        else {
            return;
        };
        let cells =
            sorted_timeline_positions(&self.project, self.timeline_selection.iter().copied())
                .into_iter()
                .filter_map(|position| {
                    let layer_position = self
                        .project
                        .layers
                        .iter()
                        .position(|layer| layer.id == position.layer_id)?;
                    let frame_position = self
                        .project
                        .frames
                        .iter()
                        .position(|frame| frame.id == position.frame_id)?;
                    let cel = self.project.cel(position.layer_id, position.frame_id);
                    let resolved = cel.and_then(|cel| self.project.resolved_cel(cel).ok());
                    Some(TimelineClipboardCel {
                        layer_offset: layer_position as isize - min_layer as isize,
                        frame_offset: frame_position as isize - min_frame as isize,
                        cel_offset: cel
                            .map_or(gridvana_core::grid::GridIndex { x: 0, y: 0 }, |cel| {
                                cel.offset
                            }),
                        pixels: resolved
                            .map(|cel| {
                                cel.pixels
                                    .iter()
                                    .map(|(index, color)| (*index, *color))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        populated: cel.is_some(),
                    })
                })
                .collect();
        self.timeline_cel_clipboard = Some(TimelineCelClipboard { cells });
    }

    fn paste_timeline_cels(&mut self) {
        let Some(clipboard) = self.timeline_cel_clipboard.clone() else {
            return;
        };
        let Some(anchor) = self
            .timeline_selection_anchor
            .or_else(|| self.timeline_selection.iter().copied().next())
        else {
            return;
        };
        let Some(anchor_layer) = self
            .project
            .layers
            .iter()
            .position(|layer| layer.id == anchor.layer_id)
        else {
            return;
        };
        let Some(anchor_frame) = self
            .project
            .frames
            .iter()
            .position(|frame| frame.id == anchor.frame_id)
        else {
            return;
        };
        let mut ops = Vec::new();
        let mut pasted = HashSet::new();
        for cell in clipboard.cells {
            let Some(layer_position) = anchor_layer.checked_add_signed(cell.layer_offset) else {
                return;
            };
            let Some(frame_position) = anchor_frame.checked_add_signed(cell.frame_offset) else {
                return;
            };
            let (Some(layer), Some(frame)) = (
                self.project.layers.get(layer_position),
                self.project.frames.get(frame_position),
            ) else {
                return;
            };
            let destination = CelPosition {
                layer_id: layer.id,
                frame_id: frame.id,
            };
            if self
                .project
                .cel(destination.layer_id, destination.frame_id)
                .is_some()
            {
                return;
            }
            pasted.insert(destination);
            if !cell.populated {
                continue;
            }
            ops.push(DocumentOp::CreateCel {
                layer_id: destination.layer_id,
                frame_id: destination.frame_id,
            });
            if !cell.pixels.is_empty() {
                ops.push(DocumentOp::SetCelPixels {
                    layer_id: destination.layer_id,
                    frame_id: destination.frame_id,
                    pixels: cell
                        .pixels
                        .into_iter()
                        .map(|(index, color)| DocumentPixel { index, color })
                        .collect(),
                });
            }
            if cell.cel_offset != (gridvana_core::grid::GridIndex { x: 0, y: 0 }) {
                ops.push(DocumentOp::MoveCel {
                    layer_id: destination.layer_id,
                    frame_id: destination.frame_id,
                    offset: cell.cel_offset,
                });
            }
        }
        if ops.is_empty() || self.apply_document_transaction(ops) {
            self.timeline_selection = pasted;
            self.timeline_selection_anchor = self.timeline_selection.iter().copied().next();
        }
    }

    fn delete_timeline_cels(&mut self) {
        let ops = sorted_timeline_positions(&self.project, self.timeline_selection.iter().copied())
            .into_iter()
            .filter(|position| {
                self.project
                    .cel(position.layer_id, position.frame_id)
                    .is_some()
            })
            .map(|position| DocumentOp::DeleteCel {
                layer_id: position.layer_id,
                frame_id: position.frame_id,
            })
            .collect::<Vec<_>>();
        if !ops.is_empty() {
            self.apply_document_transaction(ops);
        }
    }

    fn link_timeline_cels(&mut self) {
        let positions =
            sorted_timeline_positions(&self.project, self.timeline_selection.iter().copied());
        let source_position = positions
            .iter()
            .copied()
            .find(|position| {
                position.layer_id == self.project.active_layer_id
                    && position.frame_id == self.project.active_frame_id
                    && self
                        .project
                        .cel(position.layer_id, position.frame_id)
                        .and_then(|cel| self.project.resolved_cel(cel).ok())
                        .is_some_and(|cel| !cel.pixels.is_empty())
            })
            .or_else(|| {
                positions.iter().copied().find(|position| {
                    self.project
                        .cel(position.layer_id, position.frame_id)
                        .and_then(|cel| self.project.resolved_cel(cel).ok())
                        .is_some_and(|cel| !cel.pixels.is_empty())
                })
            });
        let Some(source_position) = source_position else {
            return;
        };
        let source_cel = self
            .project
            .cel(source_position.layer_id, source_position.frame_id)
            .expect("link source was selected from an existing cel");
        let source_id = self
            .project
            .resolved_cel(source_cel)
            .map_or(source_cel.id, |cel| cel.id);
        let ops = positions
            .into_iter()
            .filter(|position| {
                self.project
                    .cel(position.layer_id, position.frame_id)
                    .is_none_or(|cel| cel.id != source_id)
            })
            .map(|position| DocumentOp::LinkCel {
                layer_id: position.layer_id,
                frame_id: position.frame_id,
                source_cel_id: source_id,
            })
            .collect::<Vec<_>>();
        if !ops.is_empty() {
            self.apply_document_transaction(ops);
        }
    }

    fn unlink_timeline_cels(&mut self) {
        let ops = sorted_timeline_positions(&self.project, self.timeline_selection.iter().copied())
            .into_iter()
            .filter(|position| {
                self.project
                    .cel(position.layer_id, position.frame_id)
                    .is_some_and(|cel| cel.linked_cel_id.is_some())
            })
            .map(|position| DocumentOp::UnlinkCel {
                layer_id: position.layer_id,
                frame_id: position.frame_id,
            })
            .collect::<Vec<_>>();
        if !ops.is_empty() {
            self.apply_document_transaction(ops);
        }
    }

    fn finish_cel_drag(&mut self, origin: CelPosition, target: CelPosition) {
        if origin == target {
            return;
        }
        let selected = if self.timeline_selection.contains(&origin) {
            self.timeline_selection.clone()
        } else {
            HashSet::from([origin])
        };
        let Some((source_layer, source_frame)) = timeline_position_indices(&self.project, origin)
        else {
            return;
        };
        let Some((target_layer, target_frame)) = timeline_position_indices(&self.project, target)
        else {
            return;
        };
        let layer_delta = target_layer as isize - source_layer as isize;
        let frame_delta = target_frame as isize - source_frame as isize;
        let Some(relocations) = timeline_relocations(
            &self.project,
            selected.iter().copied(),
            layer_delta,
            frame_delta,
        ) else {
            return;
        };
        if relocations.is_empty() {
            return;
        }
        let next_selection = relocations
            .iter()
            .map(|relocation| relocation.destination)
            .collect::<HashSet<_>>();
        let active_destination = relocations
            .iter()
            .find(|relocation| {
                relocation.source.layer_id == self.project.active_layer_id
                    && relocation.source.frame_id == self.project.active_frame_id
            })
            .map_or(target, |relocation| relocation.destination);
        if self.apply_document_transaction(vec![
            DocumentOp::RelocateCels { relocations },
            DocumentOp::SetActiveLayer {
                layer_id: active_destination.layer_id,
            },
            DocumentOp::SetActiveFrame {
                frame_id: active_destination.frame_id,
            },
        ]) {
            self.timeline_selection = next_selection;
            self.timeline_selection_anchor = Some(active_destination);
        }
    }
}

fn timeline_drag_op(project: &Project, drag: TimelineDrag) -> Option<DocumentOp> {
    match drag {
        TimelineDrag::Frame { frame_id, target } => {
            let current = project
                .frames
                .iter()
                .position(|frame| frame.id == frame_id)?;
            (current != target).then_some(DocumentOp::ReorderFrame {
                frame_id,
                position: target,
            })
        }
        TimelineDrag::Layer { layer_id, target } => {
            let current = project
                .layers
                .iter()
                .position(|layer| layer.id == layer_id)?;
            (current != target).then_some(DocumentOp::ReorderLayer {
                layer_id,
                position: target,
            })
        }
        TimelineDrag::Cel { .. } => None,
        TimelineDrag::TagRange {
            tag_id,
            anchor,
            target,
        } => {
            let anchor_position = project.frames.iter().position(|frame| frame.id == anchor)?;
            let target_position = project.frames.iter().position(|frame| frame.id == target)?;
            let (from_frame_id, to_frame_id) = if anchor_position <= target_position {
                (anchor, target)
            } else {
                (target, anchor)
            };
            let tag = project.tag(tag_id)?;
            (tag.from_frame_id != from_frame_id || tag.to_frame_id != to_frame_id).then_some(
                DocumentOp::SetTagRange {
                    tag_id,
                    from_frame_id,
                    to_frame_id,
                },
            )
        }
    }
}

fn timeline_position_indices(project: &Project, position: CelPosition) -> Option<(usize, usize)> {
    Some((
        project
            .layers
            .iter()
            .position(|layer| layer.id == position.layer_id)?,
        project
            .frames
            .iter()
            .position(|frame| frame.id == position.frame_id)?,
    ))
}

fn timeline_selection_range(
    project: &Project,
    anchor: CelPosition,
    end: CelPosition,
) -> HashSet<CelPosition> {
    let Some((anchor_layer, anchor_frame)) = timeline_position_indices(project, anchor) else {
        return HashSet::new();
    };
    let Some((end_layer, end_frame)) = timeline_position_indices(project, end) else {
        return HashSet::new();
    };
    let layer_range = anchor_layer.min(end_layer)..=anchor_layer.max(end_layer);
    let frame_start = anchor_frame.min(end_frame);
    let frame_end = anchor_frame.max(end_frame);
    layer_range
        .filter(|layer_position| project.layers[*layer_position].kind.supports_cels())
        .flat_map(|layer_position| {
            (frame_start..=frame_end).map(move |frame_position| CelPosition {
                layer_id: project.layers[layer_position].id,
                frame_id: project.frames[frame_position].id,
            })
        })
        .collect()
}

fn sorted_timeline_positions<I>(project: &Project, positions: I) -> Vec<CelPosition>
where
    I: IntoIterator<Item = CelPosition>,
{
    let mut positions = positions
        .into_iter()
        .filter(|position| timeline_position_indices(project, *position).is_some())
        .collect::<Vec<_>>();
    positions.sort_by_key(|position| timeline_position_indices(project, *position));
    positions.dedup();
    positions
}

fn timeline_selection_origin<I>(project: &Project, positions: I) -> Option<(usize, usize)>
where
    I: IntoIterator<Item = CelPosition>,
{
    let indices = positions
        .into_iter()
        .filter_map(|position| timeline_position_indices(project, position))
        .collect::<Vec<_>>();
    Some((
        indices.iter().map(|(layer, _)| *layer).min()?,
        indices.iter().map(|(_, frame)| *frame).min()?,
    ))
}

fn timeline_relocations<I>(
    project: &Project,
    positions: I,
    layer_delta: isize,
    frame_delta: isize,
) -> Option<Vec<CelRelocation>>
where
    I: IntoIterator<Item = CelPosition>,
{
    sorted_timeline_positions(project, positions)
        .into_iter()
        .filter(|source| project.cel(source.layer_id, source.frame_id).is_some())
        .map(|source| {
            let (layer_position, frame_position) = timeline_position_indices(project, source)?;
            let destination_layer = layer_position.checked_add_signed(layer_delta)?;
            let destination_frame = frame_position.checked_add_signed(frame_delta)?;
            let destination_layer = project.layers.get(destination_layer)?;
            if !destination_layer.kind.supports_cels() {
                return None;
            }
            Some(CelRelocation {
                source,
                destination: CelPosition {
                    layer_id: destination_layer.id,
                    frame_id: project.frames.get(destination_frame)?.id,
                },
            })
        })
        .collect()
}

fn playback_frame_ids(project: &Project) -> Vec<FrameId> {
    project
        .active_tag_id
        .and_then(|tag_id| project.frame_ids_for_tag(tag_id).ok())
        .filter(|frames| !frames.is_empty())
        .unwrap_or_else(|| project.frames.iter().map(|frame| frame.id).collect())
}

fn synchronize_playback_cursor(project: &mut Project, sequence_index: &mut usize) {
    let frame_ids = playback_frame_ids(project);
    if frame_ids.is_empty() {
        *sequence_index = 0;
        return;
    }
    if *sequence_index >= frame_ids.len() || frame_ids[*sequence_index] != project.active_frame_id {
        *sequence_index = frame_ids
            .iter()
            .position(|frame_id| *frame_id == project.active_frame_id)
            .unwrap_or(0);
    }
    project.active_frame_id = frame_ids[*sequence_index];
}

fn advance_one_frame(project: &mut Project, sequence_index: &mut usize) {
    let frame_ids = playback_frame_ids(project);
    if frame_ids.is_empty() {
        return;
    }
    synchronize_playback_cursor(project, sequence_index);
    *sequence_index = (*sequence_index + 1) % frame_ids.len();
    let _ = apply_document_op(
        project,
        &DocumentOp::SetActiveFrame {
            frame_id: frame_ids[*sequence_index],
        },
    );
}

fn advance_playback(
    project: &mut Project,
    elapsed: &mut std::time::Duration,
    sequence_index: &mut usize,
) {
    synchronize_playback_cursor(project, sequence_index);
    loop {
        let duration = std::time::Duration::from_millis(
            project
                .current_frame()
                .map_or(100, |frame| frame.duration_ms.max(1)),
        );
        if *elapsed < duration {
            break;
        }
        *elapsed -= duration;
        advance_one_frame(project, sequence_index);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        advance_one_frame, advance_playback, timeline_drag_op, timeline_relocations,
        timeline_selection_range,
    };
    use crate::app::TimelineDrag;
    use gridvana_core::commands::ReplaceProjectCommand;
    use gridvana_core::document::{CelCopy, DocumentOp, apply_document_ops};
    use gridvana_core::grid::GridIndex;
    use gridvana_core::history::History;
    use gridvana_core::model::{CelPosition, Layer, LayerKind, Project, Rgba, TagDirection};

    #[test]
    fn playback_advances_in_frame_order_without_changing_ids() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first_frame = project.active_frame_id;
        let second_frame = project.add_frame(None, 240).unwrap();
        project.active_frame_id = first_frame;
        let mut sequence_index = 0;
        advance_one_frame(&mut project, &mut sequence_index);
        assert_eq!(project.active_frame_id, second_frame);
        advance_one_frame(&mut project, &mut sequence_index);
        assert_eq!(project.active_frame_id, first_frame);
    }

    #[test]
    fn playback_honors_each_frames_duration() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first_frame = project.active_frame_id;
        project.frame_mut(first_frame).unwrap().duration_ms = 120;
        let second_frame = project.add_frame(None, 340).unwrap();
        project.active_frame_id = first_frame;
        let mut elapsed = std::time::Duration::from_millis(119);
        let mut sequence_index = 0;
        advance_playback(&mut project, &mut elapsed, &mut sequence_index);
        assert_eq!(project.active_frame_id, first_frame);
        elapsed += std::time::Duration::from_millis(1);
        advance_playback(&mut project, &mut elapsed, &mut sequence_index);
        assert_eq!(project.active_frame_id, second_frame);
        assert_eq!(elapsed, std::time::Duration::ZERO);
        elapsed = std::time::Duration::from_millis(339);
        advance_playback(&mut project, &mut elapsed, &mut sequence_index);
        assert_eq!(project.active_frame_id, second_frame);
    }

    #[test]
    fn playback_follows_reverse_and_ping_pong_tag_sequences_without_repeated_endpoints() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 100).unwrap();
        let third = project.add_frame(None, 100).unwrap();
        let tag = project
            .add_tag("Action", first, third, TagDirection::Reverse)
            .unwrap();
        project.active_frame_id = third;
        let mut sequence_index = 0;

        advance_one_frame(&mut project, &mut sequence_index);
        assert_eq!(project.active_frame_id, second);
        advance_one_frame(&mut project, &mut sequence_index);
        assert_eq!(project.active_frame_id, first);
        advance_one_frame(&mut project, &mut sequence_index);
        assert_eq!(project.active_frame_id, third);

        project
            .set_tag_direction(tag, TagDirection::PingPong)
            .unwrap();
        project.active_frame_id = first;
        sequence_index = 0;
        let mut visited = Vec::new();
        for _ in 0..4 {
            visited.push(project.active_frame_id);
            advance_one_frame(&mut project, &mut sequence_index);
        }
        assert_eq!(visited, vec![first, second, third, second]);
        assert_eq!(project.active_frame_id, first);
    }

    #[test]
    fn timeline_drag_preview_is_non_mutating_and_commit_is_one_undoable_reorder() {
        let mut before = Project::new_square(20.0, 8, 8);
        let first = before.active_frame_id;
        let middle = before.add_frame(None, 100).unwrap();
        let last = before.add_frame(None, 100).unwrap();
        before
            .add_tag("All", first, last, TagDirection::Forward)
            .unwrap();
        let snapshot = before.clone();
        let drag = TimelineDrag::Frame {
            frame_id: middle,
            target: 0,
        };

        let op = timeline_drag_op(&before, drag).unwrap();
        assert_eq!(before, snapshot);
        let after = apply_document_ops(&before, &[op]).unwrap();
        assert_eq!(after.frames[0].id, middle);
        assert_eq!(after.tags[0].from_frame_id, first);

        let mut project = before.clone();
        let mut history = History::new();
        history.push(
            Box::new(ReplaceProjectCommand::new(before.clone(), after)),
            &mut project,
        );
        assert!(history.undo(&mut project));
        assert_eq!(project, before);
        assert!(history.redo(&mut project));
        assert_eq!(project.frames[0].id, middle);
    }

    #[test]
    fn layer_drag_commits_one_stable_id_reorder_and_one_undo_item() {
        let mut before = Project::new_square(20.0, 8, 8);
        let bottom = before.active_layer_id;
        let top = before.add_layer("Top");
        let snapshot = before.clone();
        let drag = TimelineDrag::Layer {
            layer_id: bottom,
            target: 1,
        };

        let op = timeline_drag_op(&before, drag).unwrap();
        assert_eq!(
            op,
            DocumentOp::ReorderLayer {
                layer_id: bottom,
                position: 1
            }
        );
        assert_eq!(before, snapshot);
        let after = apply_document_ops(&before, &[op]).unwrap();
        assert_eq!(after.layers[0].id, top);
        assert_eq!(after.layers[1].id, bottom);

        let mut project = before.clone();
        let mut history = History::new();
        history.push(
            Box::new(ReplaceProjectCommand::new(before.clone(), after)),
            &mut project,
        );
        assert!(history.undo(&mut project));
        assert_eq!(project, before);
        assert!(history.redo(&mut project));
        assert_eq!(project.layers[1].id, bottom);
    }

    #[test]
    fn timeline_range_selection_uses_stable_layer_and_frame_ids() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first_layer = project.active_layer_id;
        let first_frame = project.active_frame_id;
        let second_frame = project.add_frame(None, 100).unwrap();
        let third_frame = project.add_frame(None, 100).unwrap();
        let second_layer = project.add_layer("Top");

        let range = timeline_selection_range(
            &project,
            CelPosition {
                layer_id: first_layer,
                frame_id: second_frame,
            },
            CelPosition {
                layer_id: second_layer,
                frame_id: third_frame,
            },
        );
        assert_eq!(range.len(), 4);
        assert!(range.contains(&CelPosition {
            layer_id: first_layer,
            frame_id: third_frame,
        }));
        assert!(!range.contains(&CelPosition {
            layer_id: second_layer,
            frame_id: first_frame,
        }));
    }

    #[test]
    fn timeline_range_selection_skips_group_rows() {
        let mut project = Project::new_square(20.0, 8, 8);
        let bottom = project.active_layer_id;
        let frame = project.active_frame_id;
        let group_id = project.allocate_layer_id();
        let mut group = Layer::new(group_id, "Group");
        group.kind = LayerKind::Group;
        project.layers.push(group);
        let top = project.add_layer("Top");

        let range = timeline_selection_range(
            &project,
            CelPosition {
                layer_id: bottom,
                frame_id: frame,
            },
            CelPosition {
                layer_id: top,
                frame_id: frame,
            },
        );

        assert_eq!(range.len(), 2);
        assert!(!range.iter().any(|position| position.layer_id == group_id));
    }

    #[test]
    fn multi_cel_drag_relocation_preserves_cel_ids_and_rejects_out_of_bounds() {
        let mut project = Project::new_square(20.0, 8, 8);
        let layer = project.active_layer_id;
        let first = project.active_frame_id;
        let second = project.add_frame(None, 100).unwrap();
        let third = project.add_frame(None, 100).unwrap();
        project.ensure_cel(layer, second).unwrap();
        let first_id = project.cel(layer, first).unwrap().id;
        let second_id = project.cel(layer, second).unwrap().id;

        let relocations = timeline_relocations(
            &project,
            [
                CelPosition {
                    layer_id: layer,
                    frame_id: first,
                },
                CelPosition {
                    layer_id: layer,
                    frame_id: second,
                },
            ],
            0,
            1,
        )
        .unwrap();
        let moved = apply_document_ops(
            &project,
            &[gridvana_core::document::DocumentOp::RelocateCels { relocations }],
        )
        .unwrap();
        assert_eq!(moved.cel(layer, second).unwrap().id, first_id);
        assert_eq!(moved.cel(layer, third).unwrap().id, second_id);

        assert!(
            timeline_relocations(
                &project,
                [CelPosition {
                    layer_id: layer,
                    frame_id: first,
                }],
                0,
                -1,
            )
            .is_none()
        );
    }

    #[test]
    fn multi_cel_copy_link_and_delete_are_each_one_undoable_transaction() {
        let mut source = Project::new_square(20.0, 8, 8);
        let layer = source.active_layer_id;
        let first = source.active_frame_id;
        let second = source.add_frame(None, 100).unwrap();
        let third = source.add_frame(None, 100).unwrap();
        source
            .cel_mut(layer, first)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);
        let source_id = source.cel(layer, first).unwrap().id;

        let copied = apply_document_ops(
            &source,
            &[DocumentOp::CopyCels {
                copies: vec![CelCopy {
                    source: CelPosition {
                        layer_id: layer,
                        frame_id: first,
                    },
                    destination: CelPosition {
                        layer_id: layer,
                        frame_id: second,
                    },
                }],
            }],
        )
        .unwrap();
        assert_one_undoable_transaction(&source, &copied);

        let linked = apply_document_ops(
            &source,
            &[
                DocumentOp::LinkCel {
                    layer_id: layer,
                    frame_id: second,
                    source_cel_id: source_id,
                },
                DocumentOp::LinkCel {
                    layer_id: layer,
                    frame_id: third,
                    source_cel_id: source_id,
                },
            ],
        )
        .unwrap();
        assert_one_undoable_transaction(&source, &linked);

        let deleted = apply_document_ops(
            &linked,
            &[
                DocumentOp::DeleteCel {
                    layer_id: layer,
                    frame_id: second,
                },
                DocumentOp::DeleteCel {
                    layer_id: layer,
                    frame_id: third,
                },
            ],
        )
        .unwrap();
        assert_one_undoable_transaction(&linked, &deleted);
    }

    fn assert_one_undoable_transaction(before: &Project, after: &Project) {
        let mut project = before.clone();
        let mut history = History::new();
        history.push(
            Box::new(ReplaceProjectCommand::new(before.clone(), after.clone())),
            &mut project,
        );
        assert_eq!(&project, after);
        assert!(history.undo(&mut project));
        assert_eq!(&project, before);
        assert!(!history.undo(&mut project));
        assert!(history.redo(&mut project));
        assert_eq!(&project, after);
    }
}
