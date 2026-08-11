use super::geometry::{
    constrained_shape_end_index, current_cel_world_pixels, ellipse_shape_indices, grid_cell_size,
    line_shape_indices, magic_wand_indices, min_selection_index, radial_indices,
    rectangle_shape_indices, same_color_indices, selection_pixels_in_box, tool_size_display,
};
use super::{
    ClipboardPixel, Gridvana, MAX_RECENT_COLORS, SelectionBoxDraft, SelectionClipboard, ShapeDraft,
    ShapeKind, StrokeBuilder, StrokeKind,
};
use crate::i18n::tr;
use crate::types::{ColorSlot, SelectionCombineMode, Tool, TransformTargetChoice};
use gridvana_core::commands::ReplaceProjectCommand;
use gridvana_core::commands::{StrokeCommand, StrokePixel};
use gridvana_core::document::{DocumentOp, DocumentPixel, apply_document_ops};
use gridvana_core::grid::GridIndex;
use gridvana_core::history::History;
use gridvana_core::model::{CelPosition, Project, Rgba};
use gridvana_core::transform::{PixelBounds, PixelTransform, transform_indices};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn combine_selection(
    existing: &HashSet<GridIndex>,
    candidate: HashSet<GridIndex>,
    mode: SelectionCombineMode,
) -> HashSet<GridIndex> {
    match mode {
        SelectionCombineMode::Replace => candidate,
        SelectionCombineMode::Add => existing.union(&candidate).copied().collect(),
        SelectionCombineMode::Subtract => existing.difference(&candidate).copied().collect(),
        SelectionCombineMode::Intersect => existing.intersection(&candidate).copied().collect(),
    }
}

impl StrokeBuilder {
    pub(super) fn new(project: &Project, kind: StrokeKind) -> Self {
        Self {
            kind,
            changes: HashMap::new(),
            layer_id: project.active_layer_id,
            frame_id: project.active_frame_id,
            cel_existed_before: project.current_cel().is_some(),
        }
    }

    pub(super) fn apply_point(&mut self, project: &mut Project, index: GridIndex) {
        let grid = project.grid_config.create_system();
        let cell_size = grid_cell_size(project.grid_config);
        let symmetry_x_world = project.symmetry_x.position * cell_size;
        let symmetry_y_world = project.symmetry_y.position * cell_size;

        let mut indices = match self.kind {
            StrokeKind::Brush { size, .. } => radial_indices(project, index, size),
            StrokeKind::Eraser { size } => radial_indices(project, index, size),
        };

        if project.symmetry_x.active {
            let mut mirrors = Vec::new();
            for &p in &indices {
                let center = grid.cell_center(p);
                let mirror_point =
                    gridvana_core::grid::Point::new(2.0 * symmetry_x_world - center.x, center.y);
                if let Some(mp) = grid.world_to_grid(mirror_point)
                    && project.is_index_in_bounds(mp)
                {
                    mirrors.push(mp);
                }
            }
            indices.extend(mirrors);
        }

        if project.symmetry_y.active {
            let mut mirrors = Vec::new();
            for &p in &indices {
                let center = grid.cell_center(p);
                let mirror_point =
                    gridvana_core::grid::Point::new(center.x, 2.0 * symmetry_y_world - center.y);
                if let Some(mp) = grid.world_to_grid(mirror_point)
                    && project.is_index_in_bounds(mp)
                {
                    mirrors.push(mp);
                }
            }
            indices.extend(mirrors);
        }

        for idx in indices {
            self.apply_raw_point(project, idx);
        }
    }

    fn apply_raw_point(&mut self, project: &mut Project, index: GridIndex) {
        if self.changes.contains_key(&index) {
            return;
        }

        let offset = project
            .cel(self.layer_id, self.frame_id)
            .map_or(GridIndex { x: 0, y: 0 }, |cel| cel.offset);
        let Some(local_index) = index
            .x
            .checked_sub(offset.x)
            .zip(index.y.checked_sub(offset.y))
            .map(|(x, y)| GridIndex { x, y })
        else {
            return;
        };
        let old_color = project
            .cel(self.layer_id, self.frame_id)
            .and_then(|cel| cel.pixels.get(&local_index).copied());

        self.changes.insert(index, old_color);

        match self.kind {
            StrokeKind::Brush { color, .. } => {
                if let Ok(cel) = project.ensure_cel(self.layer_id, self.frame_id) {
                    if cel.linked_cel_id.is_some() {
                        return;
                    }
                    cel.pixels.insert(local_index, color);
                }
            }
            StrokeKind::Eraser { .. } => {
                if let Some(cel) = project.cel_mut(self.layer_id, self.frame_id) {
                    cel.pixels.remove(&local_index);
                }
            }
        }
    }

    pub(super) fn into_command(self) -> Option<StrokeCommand> {
        if self.changes.is_empty() {
            return None;
        }

        let new_color = match self.kind {
            StrokeKind::Brush { color, .. } => Some(color),
            StrokeKind::Eraser { .. } => None,
        };

        let pixels = self
            .changes
            .into_iter()
            .filter_map(|(index, old_color)| {
                (old_color != new_color).then_some(StrokePixel {
                    index,
                    old_color,
                    new_color,
                })
            })
            .collect::<Vec<_>>();

        if pixels.is_empty() {
            return None;
        }

        Some(StrokeCommand::new(
            self.frame_id,
            self.layer_id,
            pixels,
            self.cel_existed_before,
        ))
    }
}

impl Gridvana {
    pub(super) fn apply_document_transaction(&mut self, ops: Vec<DocumentOp>) -> bool {
        self.try_apply_document_transaction(ops).unwrap_or(false)
    }

    pub(super) fn try_apply_document_transaction(
        &mut self,
        ops: Vec<DocumentOp>,
    ) -> Result<bool, String> {
        let after = apply_document_ops(&self.project, &ops)?;
        if after == self.project {
            return Ok(false);
        }
        let before = self.project.clone();
        let invalidate_selection = before.active_layer_id != after.active_layer_id
            || before.active_frame_id != after.active_frame_id
            || before.grid_config != after.grid_config
            || before.canvas_width != after.canvas_width
            || before.canvas_height != after.canvas_height
            || before.current_cel().map(|cel| cel.id) != after.current_cel().map(|cel| cel.id);
        self.history.push(
            Box::new(ReplaceProjectCommand::new(before, after)),
            &mut self.project,
        );
        self.normalize_project_state();
        if invalidate_selection {
            self.clear_selection_state();
        }
        self.is_saved = false;
        Ok(true)
    }

    pub(super) fn ensure_mcp_service_started(&mut self) {
        if self.mcp_service.is_some() {
            return;
        }

        match crate::mcp::EmbeddedMcpService::start(
            &self.project,
            self.selection_indices.iter().copied(),
        ) {
            Ok(service) => {
                self.mcp_status = format!(
                    "{} · {}",
                    tr("MCP connected", "MCP 已连接"),
                    service.endpoint()
                );
                self.mcp_service = Some(service);
            }
            Err(error) => {
                self.mcp_status = format!("{} · {error}", tr("MCP did not start", "MCP 未启动"));
            }
        }
    }

    pub(super) fn selection_tool_active(&self) -> bool {
        matches!(
            self.current_tool,
            Tool::HandPoint | Tool::MagicWand | Tool::ColorSelect
        )
    }

    pub(super) fn clear_active_drawing(&mut self) {
        self.current_stroke = None;
        self.current_shape = None;
        self.shape_preview_indices.clear();
    }

    fn active_layer_accepts_pixel_edits(&self) -> bool {
        self.project
            .current_layer()
            .is_some_and(|layer| layer.kind.supports_cels())
            && !self
                .project
                .layer_is_effectively_locked(self.project.active_layer_id)
                .unwrap_or(true)
    }

    pub(super) fn begin_stroke(&mut self, kind: StrokeKind, index: GridIndex) {
        if !self.active_layer_accepts_pixel_edits()
            || self
                .project
                .current_cel()
                .is_some_and(|cel| cel.linked_cel_id.is_some())
        {
            return;
        }
        self.current_shape = None;
        self.shape_preview_indices.clear();
        self.current_stroke = Some(StrokeBuilder::new(&self.project, kind));

        if let Some(stroke) = self.current_stroke.as_mut() {
            stroke.apply_point(&mut self.project, index);
        }
    }

    pub(super) fn begin_shape(&mut self, kind: ShapeKind, index: GridIndex, color: Rgba) {
        self.current_stroke = None;
        self.current_shape = Some(ShapeDraft {
            kind,
            start: index,
            current: index,
            color,
        });
        self.refresh_shape_preview();
    }

    pub(super) fn begin_selection_box(&mut self, index: GridIndex) {
        self.clear_active_drawing();
        self.selection_move_draft = None;
        self.selection_box_draft = Some(SelectionBoxDraft {
            start: index,
            current: index,
        });
    }

    pub(super) fn select_magic_wand(&mut self, index: GridIndex) {
        self.combine_pixel_selection(magic_wand_indices(&self.project, index));
    }

    pub(super) fn select_same_color(&mut self, index: GridIndex) {
        self.combine_pixel_selection(same_color_indices(&self.project, index));
    }

    pub(super) fn active_color(&self) -> Rgba {
        self.color_for_slot(self.active_color_slot)
    }

    pub(super) fn color_for_slot(&self, slot: ColorSlot) -> Rgba {
        match slot {
            ColorSlot::Foreground => self.project.foreground_color,
            ColorSlot::Background => self.project.background_color,
        }
    }

    pub(super) fn set_color_for_slot(&mut self, slot: ColorSlot, color: Rgba) {
        self.active_color_slot = slot;
        let op = match slot {
            ColorSlot::Foreground => DocumentOp::SetForegroundColor { color },
            ColorSlot::Background => DocumentOp::SetBackgroundColor { color },
        };
        self.apply_document_transaction(vec![op]);
        self.current_color_hex_input = super::ui::color_to_hex(self.color_for_slot(slot));
        self.current_tool = Tool::Brush;
        self.clear_active_drawing();
        self.move_mode_active = false;
        self.selection_box_draft = None;
    }

    pub(super) fn set_current_color(&mut self, color: Rgba) {
        self.set_color_for_slot(self.active_color_slot, color);
    }

    pub(super) fn swap_foreground_background(&mut self) {
        let foreground = self.project.foreground_color;
        let background = self.project.background_color;
        if self.apply_document_transaction(vec![
            DocumentOp::SetForegroundColor { color: background },
            DocumentOp::SetBackgroundColor { color: foreground },
        ]) {
            self.current_color_hex_input = super::ui::color_to_hex(self.active_color());
        }
    }

    pub(super) fn remember_color(&mut self, color: Rgba) {
        self.recent_colors.retain(|recent| *recent != color);
        self.recent_colors.insert(0, color);
        self.recent_colors.truncate(MAX_RECENT_COLORS);
    }

    pub(super) fn refresh_shape_preview(&mut self) {
        self.shape_preview_indices = if let Some(shape) = self.current_shape {
            let constrained_end =
                constrained_shape_end_index(&self.project, shape, self.shift_pressed);
            match shape.kind {
                ShapeKind::Rectangle => {
                    rectangle_shape_indices(&self.project, shape.start, constrained_end, true)
                }
                ShapeKind::RectangleHollow => {
                    rectangle_shape_indices(&self.project, shape.start, constrained_end, false)
                }
                ShapeKind::Circle => {
                    ellipse_shape_indices(&self.project, shape.start, constrained_end, true)
                }
                ShapeKind::CircleHollow => {
                    ellipse_shape_indices(&self.project, shape.start, constrained_end, false)
                }
                ShapeKind::Line => line_shape_indices(&self.project, shape.start, constrained_end),
            }
        } else {
            Vec::new()
        };
    }

    pub(super) fn finalize_active_stroke(&mut self) {
        if let Some(shape) = self.current_shape.take() {
            let constrained_end =
                constrained_shape_end_index(&self.project, shape, self.shift_pressed);
            let mut stroke = StrokeBuilder::new(
                &self.project,
                StrokeKind::Brush {
                    color: shape.color,
                    size: 1,
                },
            );
            let shape_indices = match shape.kind {
                ShapeKind::Rectangle => {
                    rectangle_shape_indices(&self.project, shape.start, constrained_end, true)
                }
                ShapeKind::RectangleHollow => {
                    rectangle_shape_indices(&self.project, shape.start, constrained_end, false)
                }
                ShapeKind::Circle => {
                    ellipse_shape_indices(&self.project, shape.start, constrained_end, true)
                }
                ShapeKind::CircleHollow => {
                    ellipse_shape_indices(&self.project, shape.start, constrained_end, false)
                }
                ShapeKind::Line => line_shape_indices(&self.project, shape.start, constrained_end),
            };

            for index in shape_indices {
                stroke.apply_point(&mut self.project, index);
            }

            if let Some(cmd) = stroke.into_command() {
                let cmd: Box<dyn gridvana_core::history::EditCommand> = Box::new(cmd);
                self.history.push(cmd, &mut self.project);
                self.remember_color(shape.color);
                self.is_saved = false;
            }
            self.shape_preview_indices.clear();
        } else if let Some(stroke) = self.current_stroke.take() {
            let used_color = match stroke.kind {
                StrokeKind::Brush { color, .. } => Some(color),
                StrokeKind::Eraser { .. } => None,
            };

            if let Some(cmd) = stroke.into_command() {
                let cmd: Box<dyn gridvana_core::history::EditCommand> = Box::new(cmd);
                self.history.push(cmd, &mut self.project);
                if let Some(color) = used_color {
                    self.remember_color(color);
                }
                self.is_saved = false;
            }
        }
    }

    pub(super) fn shape_preview_hint(&self) -> Option<String> {
        if self.current_tool == Tool::Brush && self.hovered_grid_index.is_some() {
            return Some(format!(
                "{} {}",
                tr("Brush", "画笔"),
                tool_size_display(self.project.grid_config, self.brush_size)
            ));
        }

        if self.current_tool == Tool::Eraser && self.hovered_grid_index.is_some() {
            return Some(format!(
                "{} {}",
                tr("Eraser", "橡皮擦"),
                tool_size_display(self.project.grid_config, self.eraser_size)
            ));
        }

        let shape = self.current_shape?;
        let grid = self.project.grid_config.create_system();
        let constrained_end = constrained_shape_end_index(&self.project, shape, self.shift_pressed);
        let start_center = grid.cell_center(shape.start);
        let end_center = grid.cell_center(constrained_end);

        let dx = (end_center.x - start_center.x).abs();
        let dy = (end_center.y - start_center.y).abs();
        let cell_size = grid_cell_size(self.project.grid_config).max(0.001);

        let hint = match shape.kind {
            ShapeKind::Rectangle => {
                let width_cells = (dx / cell_size).round() as i32 + 1;
                let height_cells = (dy / cell_size).round() as i32 + 1;
                format!(
                    "{} {} × {}",
                    tr("Rectangle", "矩形"),
                    width_cells.max(1),
                    height_cells.max(1)
                )
            }
            ShapeKind::RectangleHollow => {
                let width_cells = (dx / cell_size).round() as i32 + 1;
                let height_cells = (dy / cell_size).round() as i32 + 1;
                format!(
                    "{} {} × {}",
                    tr("Rectangle outline", "空心矩形"),
                    width_cells.max(1),
                    height_cells.max(1)
                )
            }
            ShapeKind::Circle => {
                let radius_x = dx * 0.5 / cell_size;
                let radius_y = dy * 0.5 / cell_size;
                format!(
                    "{} {} {:.1} / {:.1}",
                    tr("Circle", "圆形"),
                    tr("radius", "半径"),
                    radius_x,
                    radius_y
                )
            }
            ShapeKind::CircleHollow => {
                let radius_x = dx * 0.5 / cell_size;
                let radius_y = dy * 0.5 / cell_size;
                format!(
                    "{} {} {:.1} / {:.1}",
                    tr("Circle outline", "空心圆"),
                    tr("radius", "半径"),
                    radius_x,
                    radius_y
                )
            }
            ShapeKind::Line => {
                let length_cells = ((dx * dx + dy * dy).sqrt() / cell_size).max(0.0);
                format!(
                    "{} {} {:.1}",
                    tr("Line", "线条"),
                    tr("length", "长度"),
                    length_cells
                )
            }
        };

        Some(hint)
    }

    pub(super) fn brush_preview_indices(&self) -> Vec<GridIndex> {
        if self.current_tool != Tool::Brush {
            return Vec::new();
        }

        self.hovered_grid_index
            .map(|index| radial_indices(&self.project, index, self.brush_size))
            .unwrap_or_default()
    }

    pub(super) fn eraser_preview_indices(&self) -> Vec<GridIndex> {
        if self.current_tool != Tool::Eraser {
            return Vec::new();
        }

        self.hovered_grid_index
            .map(|index| radial_indices(&self.project, index, self.eraser_size))
            .unwrap_or_default()
    }

    pub(super) fn clear_selection_state(&mut self) {
        self.selection_indices.clear();
        self.selection_box_draft = None;
        self.selection_move_draft = None;
        self.move_mode_active = false;
    }

    fn combined_pixel_selection(&self, candidate: HashSet<GridIndex>) -> HashSet<GridIndex> {
        combine_selection(
            &self.selection_indices,
            candidate,
            self.selection_combine_mode,
        )
    }

    pub(super) fn combine_pixel_selection(&mut self, candidate: HashSet<GridIndex>) {
        self.selection_indices = self.combined_pixel_selection(candidate);
        self.move_mode_active = self.selection_tool_active()
            && self.selection_combine_mode == SelectionCombineMode::Replace
            && !self.selection_indices.is_empty();
        self.selection_move_draft = None;
    }

    pub(super) fn select_all_pixels(&mut self) {
        self.selection_indices = self.project.canvas_grid_indices().into_iter().collect();
        self.move_mode_active = self.selection_tool_active()
            && self.selection_combine_mode == SelectionCombineMode::Replace
            && !self.selection_indices.is_empty();
    }

    pub(super) fn invert_pixel_selection(&mut self) {
        self.selection_indices = self
            .project
            .canvas_grid_indices()
            .into_iter()
            .filter(|index| !self.selection_indices.contains(index))
            .collect();
        self.move_mode_active = self.selection_tool_active()
            && self.selection_combine_mode == SelectionCombineMode::Replace
            && !self.selection_indices.is_empty();
    }

    pub(super) fn selection_display_indices(&self) -> Vec<GridIndex> {
        if let Some(draft) = self.selection_box_draft {
            let candidate = selection_pixels_in_box(&self.project, draft.start, draft.current);
            return self
                .combined_pixel_selection(candidate)
                .into_iter()
                .collect();
        }

        if let Some(draft) = self.selection_move_draft {
            let dx = draft.current.x - draft.start.x;
            let dy = draft.current.y - draft.start.y;
            return self
                .selection_indices
                .iter()
                .map(|index| GridIndex {
                    x: index.x + dx,
                    y: index.y + dy,
                })
                .filter(|index| self.project.is_index_in_bounds(*index))
                .collect();
        }

        self.selection_indices.iter().copied().collect()
    }

    pub(super) fn finalize_selection_box(&mut self) {
        let Some(draft) = self.selection_box_draft.take() else {
            return;
        };

        let candidate = selection_pixels_in_box(&self.project, draft.start, draft.current);
        self.combine_pixel_selection(candidate);

        if self.move_mode_active {
            self.copy_selection();
        }
    }

    fn apply_selection_changes(&mut self, changes: &HashMap<GridIndex, Option<Rgba>>) -> bool {
        let frame_id = self.project.active_frame_id;
        let layer_id = self.project.active_layer_id;
        if !self.active_layer_accepts_pixel_edits() {
            return false;
        }
        let mut erase = Vec::new();
        let mut set = Vec::new();
        for (&index, &color) in changes {
            match color {
                Some(color) => set.push(DocumentPixel { index, color }),
                None => erase.push(index),
            }
        }
        let mut ops = Vec::new();
        if !erase.is_empty() {
            ops.push(DocumentOp::EraseCelPixels {
                layer_id,
                frame_id,
                indices: erase,
            });
        }
        if !set.is_empty() {
            ops.push(DocumentOp::SetCelPixels {
                layer_id,
                frame_id,
                pixels: set,
            });
        }
        !ops.is_empty() && self.apply_document_transaction(ops)
    }

    pub(super) fn pixel_transform_targets(&self) -> Vec<CelPosition> {
        let mut targets = match self.transform_target {
            TransformTargetChoice::CurrentCel => vec![CelPosition {
                layer_id: self.project.active_layer_id,
                frame_id: self.project.active_frame_id,
            }],
            TransformTargetChoice::SelectedCels => {
                self.timeline_selection.iter().copied().collect()
            }
            TransformTargetChoice::CompositedFrame => self
                .project
                .layers
                .iter()
                .filter(|layer| layer.kind.supports_cels())
                .filter(|layer| {
                    self.project
                        .layer_is_effectively_visible(layer.id)
                        .unwrap_or(false)
                })
                .filter(|layer| {
                    self.project
                        .cel(layer.id, self.project.active_frame_id)
                        .is_some()
                })
                .map(|layer| CelPosition {
                    layer_id: layer.id,
                    frame_id: self.project.active_frame_id,
                })
                .collect(),
        };
        targets.sort_by_key(|target| (target.layer_id, target.frame_id));
        targets.dedup();
        targets
    }

    pub(super) fn transform_pixel_selection(&mut self, transform: PixelTransform) {
        let targets = self.pixel_transform_targets();
        if targets.is_empty() {
            return;
        }
        let mut selection = self.selection_indices.iter().copied().collect::<Vec<_>>();
        selection.sort_by_key(|index| (index.y, index.x));
        let bounds = PixelBounds::from_indices(selection.iter().copied());
        if !self.apply_document_transaction(vec![DocumentOp::TransformCelPixels {
            targets,
            selection,
            transform,
        }]) {
            return;
        }
        if let Some(bounds) = bounds
            && let Ok(transformed) =
                transform_indices(self.selection_indices.iter().copied(), transform, bounds)
        {
            self.selection_indices = transformed
                .into_iter()
                .filter(|index| self.project.is_index_in_bounds(*index))
                .collect();
        }
        self.move_mode_active = self.selection_tool_active()
            && self.selection_combine_mode == SelectionCombineMode::Replace
            && !self.selection_indices.is_empty();
    }

    pub(super) fn move_selection_by(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }

        if self.selection_indices.is_empty() {
            return;
        }

        self.transform_pixel_selection(PixelTransform::Translate { dx, dy });
    }

    pub(super) fn copy_selection(&mut self) {
        if self.selection_indices.is_empty() {
            return;
        }

        let Some(anchor) = min_selection_index(&self.selection_indices) else {
            return;
        };

        let selected_offsets = self
            .selection_indices
            .iter()
            .map(|index| (index.x - anchor.x, index.y - anchor.y))
            .collect::<Vec<_>>();

        let world_pixels = current_cel_world_pixels(&self.project);

        let mut pixels = Vec::new();
        for index in &self.selection_indices {
            if let Some(color) = world_pixels.get(index) {
                pixels.push(ClipboardPixel {
                    offset_x: index.x - anchor.x,
                    offset_y: index.y - anchor.y,
                    color: *color,
                });
            }
        }

        self.selection_clipboard = Some(SelectionClipboard {
            anchor,
            source_layer_id: self.project.active_layer_id,
            source_frame_id: self.project.active_frame_id,
            selected_offsets,
            pixels,
        });
        self.paste_offset = GridIndex { x: 1, y: 1 };
    }

    pub(super) fn paste_selection(&mut self) {
        let Some(clipboard) = self.selection_clipboard.clone() else {
            return;
        };

        let same_source_cel = clipboard.source_layer_id == self.project.active_layer_id
            && clipboard.source_frame_id == self.project.active_frame_id;
        let anchor = if same_source_cel
            && let Some(selection_anchor) = min_selection_index(&self.selection_indices)
        {
            GridIndex {
                x: selection_anchor.x + self.paste_offset.x,
                y: selection_anchor.y + self.paste_offset.y,
            }
        } else {
            GridIndex {
                x: clipboard.anchor.x + self.paste_offset.x,
                y: clipboard.anchor.y + self.paste_offset.y,
            }
        };

        let mut paint_pixels = HashMap::new();
        for pixel in &clipboard.pixels {
            let target = GridIndex {
                x: anchor.x + pixel.offset_x,
                y: anchor.y + pixel.offset_y,
            };
            if self.project.is_index_in_bounds(target) {
                paint_pixels.insert(target, pixel.color);
            }
        }

        let changes = paint_pixels
            .iter()
            .map(|(&index, &color)| (index, Some(color)))
            .collect::<HashMap<_, _>>();
        self.apply_selection_changes(&changes);

        self.selection_indices = clipboard
            .selected_offsets
            .iter()
            .map(|(offset_x, offset_y)| GridIndex {
                x: anchor.x + offset_x,
                y: anchor.y + offset_y,
            })
            .filter(|index| self.project.is_index_in_bounds(*index))
            .collect();
        self.move_mode_active = true;
        self.selection_move_draft = None;
        self.paste_offset.x += 1;
        self.paste_offset.y += 1;
    }

    pub(super) fn duplicate_selection(&mut self) {
        if self.selection_indices.is_empty() {
            return;
        }

        self.copy_selection();
        self.paste_selection();
    }

    pub(super) fn delete_pixel_selection(&mut self) {
        if self.selection_indices.is_empty() {
            return;
        }
        let mut indices = self.selection_indices.iter().copied().collect::<Vec<_>>();
        indices.sort_by_key(|index| (index.y, index.x));
        self.apply_document_transaction(vec![DocumentOp::EraseCelPixels {
            layer_id: self.project.active_layer_id,
            frame_id: self.project.active_frame_id,
            indices,
        }]);
    }

    pub(super) fn cut_pixel_selection(&mut self) {
        self.copy_selection();
        self.delete_pixel_selection();
    }

    pub(super) fn crop_canvas_to_selection(&mut self) {
        let Some(bounds) = PixelBounds::from_indices(self.selection_indices.iter().copied()) else {
            return;
        };
        if self.apply_document_transaction(vec![DocumentOp::CropCanvas { bounds }]) {
            self.clear_selection_state();
        }
    }

    pub(super) fn trim_current_canvas(&mut self) {
        if self.apply_document_transaction(vec![DocumentOp::TrimCanvas]) {
            self.clear_selection_state();
        }
    }

    pub(super) fn resize_current_canvas(&mut self) {
        let (Ok(canvas_width), Ok(canvas_height)) = (
            self.resize_canvas_width.parse::<u32>(),
            self.resize_canvas_height.parse::<u32>(),
        ) else {
            return;
        };
        if self.apply_document_transaction(vec![DocumentOp::ResizeCanvas {
            canvas_width,
            canvas_height,
        }]) {
            self.clear_selection_state();
        }
    }

    pub(super) fn finish_selection_gesture(&mut self) {
        if let Some(move_draft) = self.selection_move_draft.take() {
            let dx = move_draft.current.x - move_draft.start.x;
            let dy = move_draft.current.y - move_draft.start.y;
            self.move_selection_by(dx, dy);
        }
    }
    pub(super) fn color_at_index(&self, index: GridIndex) -> Option<Rgba> {
        current_cel_world_pixels(&self.project).get(&index).copied()
    }

    pub(super) fn flood_fill(&mut self, start: GridIndex, new_color: Rgba) {
        if !self.project.is_index_in_bounds(start) {
            return;
        }

        if !self.active_layer_accepts_pixel_edits() {
            return;
        }
        let cel_pixels = current_cel_world_pixels(&self.project);
        let target_color = cel_pixels.get(&start).copied();
        if target_color == Some(new_color) {
            return;
        }

        let grid = self.project.grid_config.create_system();
        let mut visited = HashSet::new();
        let mut region = Vec::new();
        let mut stack = vec![start];

        while let Some(index) = stack.pop() {
            if !visited.insert(index) {
                continue;
            }

            if cel_pixels.get(&index).copied() != target_color {
                continue;
            }

            region.push(index);

            for neighbor in grid.neighbors(index) {
                if self.project.is_index_in_bounds(neighbor) && !visited.contains(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }

        let mut changes = HashMap::new();
        for index in region {
            changes.insert(index, Some(new_color));
        }

        if self.apply_selection_changes(&changes) {
            self.remember_color(new_color);
        }
    }

    pub(super) fn normalize_project_state(&mut self) {
        if self.project.frame(self.project.active_frame_id).is_none()
            && let Some(frame) = self.project.frames.first()
        {
            self.project.active_frame_id = frame.id;
        }
        if self.project.layer(self.project.active_layer_id).is_none()
            && let Some(layer) = self.project.layers.first()
        {
            self.project.active_layer_id = layer.id;
        }
        self.timeline_selection.retain(|position| {
            self.project
                .layer(position.layer_id)
                .is_some_and(|layer| layer.kind.supports_cels())
                && self.project.frame(position.frame_id).is_some()
        });
        if self.timeline_selection_anchor.is_some_and(|position| {
            !self
                .project
                .layer(position.layer_id)
                .is_some_and(|layer| layer.kind.supports_cels())
                || self.project.frame(position.frame_id).is_none()
        }) {
            self.timeline_selection_anchor = None;
        }
        self.resize_canvas_width = self.project.canvas_width.to_string();
        self.resize_canvas_height = self.project.canvas_height.to_string();
        self.current_color_hex_input = super::ui::color_to_hex(self.active_color());
    }

    pub(super) fn clear_timeline_selection(&mut self) {
        self.timeline_selection.clear();
        self.timeline_selection_anchor = None;
    }

    pub(super) fn sync_editor_after_external_edit(&mut self) {
        self.has_canvas = true;
        self.ai_preview_project = None;
        self.current_stroke = None;
        self.current_shape = None;
        self.shape_preview_indices.clear();
        self.clear_selection_state();
        self.is_playing = false;
        self.playback_last_tick = None;
        self.playback_elapsed = std::time::Duration::ZERO;
        self.playback_sequence_index = 0;
        self.timeline_drag = None;
        self.clear_timeline_selection();
        self.normalize_project_state();
        self.is_saved = false;
    }

    pub(super) fn load_project_into_editor(
        &mut self,
        project: Project,
        project_path: Option<PathBuf>,
        is_saved: bool,
    ) {
        self.ai_preview_project = None;
        self.pending_sprite_sheet_export_path = None;
        self.project = project;
        self.history = History::new();
        self.current_stroke = None;
        self.current_shape = None;
        self.shape_preview_indices.clear();
        self.clear_selection_state();
        self.is_playing = false;
        self.playback_last_tick = None;
        self.playback_elapsed = std::time::Duration::ZERO;
        self.playback_sequence_index = 0;
        self.timeline_drag = None;
        self.clear_timeline_selection();
        self.has_canvas = true;
        self.ensure_mcp_service_started();
        if let Some(service) = self.mcp_service.as_mut()
            && let Err(error) = service.replace_editor_project(&self.project)
        {
            self.mcp_status = format!(
                "{} · {error}",
                tr("MCP project reset failed", "MCP 项目重置失败")
            );
        }
        self.project_path = project_path;
        self.normalize_project_state();
        self.is_saved = is_saved;
        self.sync_terminal_webview_visibility();
    }
}

#[cfg(test)]
mod tests {
    use super::{StrokeBuilder, StrokeKind, combine_selection};
    use crate::types::SelectionCombineMode;
    use gridvana_core::grid::GridIndex;
    use gridvana_core::history::History;
    use gridvana_core::model::{Project, Rgba};
    use std::collections::HashSet;

    #[test]
    fn selection_combine_modes_have_set_semantics() {
        let a = GridIndex { x: 0, y: 0 };
        let b = GridIndex { x: 1, y: 0 };
        let c = GridIndex { x: 2, y: 0 };
        let existing = HashSet::from([a, b]);
        let candidate = HashSet::from([b, c]);

        assert_eq!(
            combine_selection(&existing, candidate.clone(), SelectionCombineMode::Replace),
            HashSet::from([b, c])
        );
        assert_eq!(
            combine_selection(&existing, candidate.clone(), SelectionCombineMode::Add),
            HashSet::from([a, b, c])
        );
        assert_eq!(
            combine_selection(&existing, candidate.clone(), SelectionCombineMode::Subtract,),
            HashSet::from([a])
        );
        assert_eq!(
            combine_selection(&existing, candidate, SelectionCombineMode::Intersect),
            HashSet::from([b])
        );
    }

    #[test]
    fn stroke_builder_uses_world_coordinates_for_offset_cels_and_history() {
        let mut project = Project::new_square(1.0, 5, 4);
        project.current_cel_mut().unwrap().offset = GridIndex { x: 2, y: 1 };
        let mut stroke = StrokeBuilder::new(
            &project,
            StrokeKind::Brush {
                color: Rgba::WHITE,
                size: 1,
            },
        );
        stroke.apply_point(&mut project, GridIndex { x: 3, y: 2 });
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 1, y: 1 })
        );

        let mut history = History::new();
        history.push(Box::new(stroke.into_command().unwrap()), &mut project);
        assert!(history.undo(&mut project));
        assert!(project.current_cel().unwrap().pixels.is_empty());
        assert!(history.redo(&mut project));
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 1, y: 1 })
        );
    }
}
