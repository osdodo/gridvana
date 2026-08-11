use crate::grid::GridIndex;
use crate::model::{
    BlendMode, CelId, CelPosition, DEFAULT_FRAME_DURATION_MS, FrameId, GridConfig, Layer, LayerId,
    LayerKind, Palette, Project, Rgba, SymmetryLine, TagDirection, TagId,
};
use crate::transform::{
    PixelBounds, PixelTransform, crop_canvas, transform_cel_pixels, trim_canvas,
};

const MAX_CANVAS_SIZE: u32 = 4096;

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentPixel {
    pub index: GridIndex,
    pub color: Rgba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CelRelocation {
    pub source: CelPosition,
    pub destination: CelPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CelCopy {
    pub source: CelPosition,
    pub destination: CelPosition,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentOp {
    ResizeCanvas {
        canvas_width: u32,
        canvas_height: u32,
    },
    SetForegroundColor {
        color: Rgba,
    },
    SetBackgroundColor {
        color: Rgba,
    },
    RenamePalette {
        name: String,
    },
    AddPaletteColor {
        color: Rgba,
        position: Option<usize>,
    },
    RemovePaletteColor {
        position: usize,
    },
    ReplacePaletteColor {
        position: usize,
        color: Rgba,
    },
    ReorderPaletteColor {
        from_position: usize,
        position: usize,
    },
    DeduplicatePalette,
    ReplacePalette {
        palette: Palette,
    },
    CreateCel {
        layer_id: LayerId,
        frame_id: FrameId,
    },
    SetCelPixels {
        layer_id: LayerId,
        frame_id: FrameId,
        pixels: Vec<DocumentPixel>,
    },
    EraseCelPixels {
        layer_id: LayerId,
        frame_id: FrameId,
        indices: Vec<GridIndex>,
    },
    ClearCel {
        layer_id: LayerId,
        frame_id: FrameId,
    },
    MoveCel {
        layer_id: LayerId,
        frame_id: FrameId,
        offset: GridIndex,
    },
    TransformCelPixels {
        targets: Vec<CelPosition>,
        selection: Vec<GridIndex>,
        transform: PixelTransform,
    },
    CropCanvas {
        bounds: PixelBounds,
    },
    TrimCanvas,
    RelocateCels {
        relocations: Vec<CelRelocation>,
    },
    CopyCels {
        copies: Vec<CelCopy>,
    },
    DeleteCel {
        layer_id: LayerId,
        frame_id: FrameId,
    },
    LinkCel {
        layer_id: LayerId,
        frame_id: FrameId,
        source_cel_id: CelId,
    },
    UnlinkCel {
        layer_id: LayerId,
        frame_id: FrameId,
    },
    AddLayer {
        name: Option<String>,
        position: Option<usize>,
        kind: LayerKind,
        parent_id: Option<LayerId>,
    },
    RemoveLayer {
        layer_id: LayerId,
    },
    RenameLayer {
        layer_id: LayerId,
        name: String,
    },
    ReorderLayer {
        layer_id: LayerId,
        position: usize,
    },
    SetLayerVisibility {
        layer_id: LayerId,
        visible: bool,
    },
    SetLayerLocked {
        layer_id: LayerId,
        locked: bool,
    },
    SetLayerOpacity {
        layer_id: LayerId,
        opacity: f32,
    },
    SetLayerBlendMode {
        layer_id: LayerId,
        blend_mode: BlendMode,
    },
    SetLayerKind {
        layer_id: LayerId,
        kind: LayerKind,
    },
    SetLayerParent {
        layer_id: LayerId,
        parent_id: Option<LayerId>,
    },
    AddFrame {
        position: Option<usize>,
        duration_ms: Option<u64>,
    },
    DuplicateFrame {
        frame_id: FrameId,
    },
    RemoveFrame {
        frame_id: FrameId,
    },
    ReorderFrame {
        frame_id: FrameId,
        position: usize,
    },
    SetFrameDuration {
        frame_id: FrameId,
        duration_ms: u64,
    },
    AddTag {
        name: String,
        from_frame_id: FrameId,
        to_frame_id: FrameId,
        direction: TagDirection,
    },
    RemoveTag {
        tag_id: TagId,
    },
    RenameTag {
        tag_id: TagId,
        name: String,
    },
    SetTagRange {
        tag_id: TagId,
        from_frame_id: FrameId,
        to_frame_id: FrameId,
    },
    SetTagDirection {
        tag_id: TagId,
        direction: TagDirection,
    },
    SetActiveTag {
        tag_id: Option<TagId>,
    },
    SetActiveLayer {
        layer_id: LayerId,
    },
    SetActiveFrame {
        frame_id: FrameId,
    },
    SetGridConfig {
        grid_config: GridConfig,
    },
    SetSymmetryX {
        line: SymmetryLine,
    },
    SetSymmetryY {
        line: SymmetryLine,
    },
    ReplaceProject {
        project: Project,
    },
}

pub fn apply_document_ops(project: &Project, ops: &[DocumentOp]) -> Result<Project, String> {
    if ops.is_empty() {
        return Err("a document transaction must contain at least one operation".to_string());
    }
    let mut next = project.clone();
    for (index, op) in ops.iter().enumerate() {
        apply_document_op(&mut next, op)
            .map_err(|error| format!("document operation {} failed: {error}", index + 1))?;
    }
    next.validate()?;
    Ok(next)
}

pub fn apply_document_op(project: &mut Project, op: &DocumentOp) -> Result<(), String> {
    match op {
        DocumentOp::ResizeCanvas {
            canvas_width,
            canvas_height,
        } => {
            if !(1..=MAX_CANVAS_SIZE).contains(canvas_width)
                || !(1..=MAX_CANVAS_SIZE).contains(canvas_height)
            {
                return Err(format!(
                    "canvas dimensions must be between 1 and {MAX_CANVAS_SIZE}"
                ));
            }
            let was_empty = project.canvas_width == 0 || project.canvas_height == 0;
            project.canvas_width = *canvas_width;
            project.canvas_height = *canvas_height;
            if was_empty {
                project.symmetry_x.position = *canvas_width as f32 / 2.0;
                project.symmetry_y.position = *canvas_height as f32 / 2.0;
            } else {
                project.symmetry_x.position =
                    project.symmetry_x.position.clamp(0.0, *canvas_width as f32);
                project.symmetry_y.position = project
                    .symmetry_y
                    .position
                    .clamp(0.0, *canvas_height as f32);
            }
        }
        DocumentOp::SetForegroundColor { color } => {
            project.foreground_color = clamped_color(*color);
        }
        DocumentOp::SetBackgroundColor { color } => {
            project.background_color = clamped_color(*color);
        }
        DocumentOp::RenamePalette { name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err("palette name cannot be empty".to_string());
            }
            project.palette.name = name.to_string();
        }
        DocumentOp::AddPaletteColor { color, position } => {
            let position = position.unwrap_or(project.palette.colors.len());
            if position > project.palette.colors.len() {
                return Err(format!(
                    "palette insertion position out of bounds: {position}"
                ));
            }
            project
                .palette
                .colors
                .insert(position, clamped_color(*color));
        }
        DocumentOp::RemovePaletteColor { position } => {
            if *position >= project.palette.colors.len() {
                return Err(format!("palette color position out of bounds: {position}"));
            }
            project.palette.colors.remove(*position);
        }
        DocumentOp::ReplacePaletteColor { position, color } => {
            let palette_color = project
                .palette
                .colors
                .get_mut(*position)
                .ok_or_else(|| format!("palette color position out of bounds: {position}"))?;
            *palette_color = clamped_color(*color);
        }
        DocumentOp::ReorderPaletteColor {
            from_position,
            position,
        } => {
            if *from_position >= project.palette.colors.len() {
                return Err(format!(
                    "palette source position out of bounds: {from_position}"
                ));
            }
            if *position >= project.palette.colors.len() {
                return Err(format!("palette target position out of bounds: {position}"));
            }
            let color = project.palette.colors.remove(*from_position);
            project.palette.colors.insert(*position, color);
        }
        DocumentOp::DeduplicatePalette => project.palette.deduplicate(),
        DocumentOp::ReplacePalette { palette } => {
            project.palette = palette.clone();
        }
        DocumentOp::CreateCel { layer_id, frame_id } => {
            require_editable_layer(project, *layer_id)?;
            project.ensure_cel(*layer_id, *frame_id)?;
        }
        DocumentOp::SetCelPixels {
            layer_id,
            frame_id,
            pixels,
        } => {
            require_editable_layer(project, *layer_id)?;
            let offset = project
                .cel(*layer_id, *frame_id)
                .map_or(GridIndex { x: 0, y: 0 }, |cel| cel.offset);
            let mut local_pixels = Vec::with_capacity(pixels.len());
            for pixel in pixels {
                if !project.is_index_in_bounds(pixel.index) {
                    return Err(format!(
                        "pixel is out of bounds: ({}, {})",
                        pixel.index.x, pixel.index.y
                    ));
                }
                local_pixels.push((
                    GridIndex {
                        x: pixel
                            .index
                            .x
                            .checked_sub(offset.x)
                            .ok_or_else(|| "pixel local x coordinate overflowed".to_string())?,
                        y: pixel
                            .index
                            .y
                            .checked_sub(offset.y)
                            .ok_or_else(|| "pixel local y coordinate overflowed".to_string())?,
                    },
                    clamped_color(pixel.color),
                ));
            }
            let cel = project.ensure_cel(*layer_id, *frame_id)?;
            if cel.linked_cel_id.is_some() {
                return Err("linked cels must be unlinked before editing pixels".to_string());
            }
            for (index, color) in local_pixels {
                cel.pixels.insert(index, color);
            }
        }
        DocumentOp::EraseCelPixels {
            layer_id,
            frame_id,
            indices,
        } => {
            require_editable_layer(project, *layer_id)?;
            let offset = project
                .cel(*layer_id, *frame_id)
                .map_or(GridIndex { x: 0, y: 0 }, |cel| cel.offset);
            let mut local_indices = Vec::with_capacity(indices.len());
            for index in indices {
                if !project.is_index_in_bounds(*index) {
                    return Err(format!(
                        "pixel is out of bounds: ({}, {})",
                        index.x, index.y
                    ));
                }
                local_indices.push(GridIndex {
                    x: index
                        .x
                        .checked_sub(offset.x)
                        .ok_or_else(|| "pixel local x coordinate overflowed".to_string())?,
                    y: index
                        .y
                        .checked_sub(offset.y)
                        .ok_or_else(|| "pixel local y coordinate overflowed".to_string())?,
                });
            }
            if let Some(cel) = project.cel_mut(*layer_id, *frame_id) {
                if cel.linked_cel_id.is_some() {
                    return Err("linked cels must be unlinked before editing pixels".to_string());
                }
                for index in local_indices {
                    cel.pixels.remove(&index);
                }
            }
        }
        DocumentOp::ClearCel { layer_id, frame_id } => {
            require_editable_layer(project, *layer_id)?;
            if let Some(cel) = project.cel_mut(*layer_id, *frame_id) {
                cel.pixels.clear();
                cel.linked_cel_id = None;
            }
        }
        DocumentOp::MoveCel {
            layer_id,
            frame_id,
            offset,
        } => {
            require_editable_layer(project, *layer_id)?;
            let cel = project
                .cel_mut(*layer_id, *frame_id)
                .ok_or_else(|| "cannot move an empty cel".to_string())?;
            cel.offset = *offset;
        }
        DocumentOp::TransformCelPixels {
            targets,
            selection,
            transform,
        } => transform_cel_pixels(project, targets, selection, *transform)?,
        DocumentOp::CropCanvas { bounds } => crop_canvas(project, *bounds)?,
        DocumentOp::TrimCanvas => {
            trim_canvas(project)?;
        }
        DocumentOp::RelocateCels { relocations } => {
            relocate_cels(project, relocations)?;
        }
        DocumentOp::CopyCels { copies } => {
            copy_cels(project, copies)?;
        }
        DocumentOp::DeleteCel { layer_id, frame_id } => {
            require_editable_layer(project, *layer_id)?;
            project.remove_cel_preserving_links(*layer_id, *frame_id);
        }
        DocumentOp::LinkCel {
            layer_id,
            frame_id,
            source_cel_id,
        } => {
            require_editable_layer(project, *layer_id)?;
            if project.cel_by_id(*source_cel_id).is_none() {
                return Err(format!("unknown source cel_id: {source_cel_id}"));
            }
            let destination_id = project.ensure_cel(*layer_id, *frame_id)?.id;
            let mut current_id = *source_cel_id;
            let mut visited = std::collections::HashSet::new();
            loop {
                if current_id == destination_id {
                    return Err("linking these cels would create a cycle".to_string());
                }
                if !visited.insert(current_id) {
                    return Err("source cel already contains a link cycle".to_string());
                }
                let current = project
                    .cel_by_id(current_id)
                    .ok_or_else(|| format!("unknown linked cel_id: {current_id}"))?;
                let Some(linked_id) = current.linked_cel_id else {
                    break;
                };
                current_id = linked_id;
            }
            let cel = project
                .cel_by_id_mut(destination_id)
                .expect("the destination cel was ensured");
            cel.pixels.clear();
            cel.linked_cel_id = Some(*source_cel_id);
        }
        DocumentOp::UnlinkCel { layer_id, frame_id } => {
            require_editable_layer(project, *layer_id)?;
            let Some(destination) = project.cel(*layer_id, *frame_id) else {
                return Ok(());
            };
            let Some(_) = destination.linked_cel_id else {
                return Ok(());
            };
            let source = project.resolved_cel(destination)?.clone();
            let destination = project
                .cel_mut(*layer_id, *frame_id)
                .expect("the destination cel still exists");
            destination.pixels = source.pixels;
            destination.linked_cel_id = None;
        }
        DocumentOp::AddLayer {
            name,
            position,
            kind,
            parent_id,
        } => {
            let name = normalized_layer_name(name.as_deref(), project.layers.len());
            let id = project.allocate_layer_id();
            let mut layer = Layer::new(id, name);
            layer.kind = *kind;
            layer.parent_id = *parent_id;
            let position = if *kind == LayerKind::Background {
                if project
                    .layers
                    .iter()
                    .any(|layer| layer.kind == LayerKind::Background)
                {
                    return Err("a project can contain at most one background layer".to_string());
                }
                if parent_id.is_some() {
                    return Err("a background layer cannot have a parent".to_string());
                }
                0
            } else {
                position.unwrap_or(project.layers.len())
            };
            if position > project.layers.len() {
                return Err(format!(
                    "layer insertion position out of bounds: {position}"
                ));
            }
            project.layers.insert(position, layer);
            project.active_layer_id = id;
        }
        DocumentOp::RemoveLayer { layer_id } => project.remove_layer_with_cels(*layer_id)?,
        DocumentOp::RenameLayer { layer_id, name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err("layer name cannot be empty".to_string());
            }
            project
                .layer_mut(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?
                .name = name.to_string();
        }
        DocumentOp::ReorderLayer { layer_id, position } => {
            if *position >= project.layers.len() {
                return Err(format!("layer position out of bounds: {position}"));
            }
            let current = project
                .layers
                .iter()
                .position(|layer| layer.id == *layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
            let layer = project.layers.remove(current);
            project.layers.insert(*position, layer);
        }
        DocumentOp::SetLayerVisibility { layer_id, visible } => {
            project
                .layer_mut(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?
                .visible = *visible;
        }
        DocumentOp::SetLayerLocked { layer_id, locked } => {
            project
                .layer_mut(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?
                .locked = *locked;
        }
        DocumentOp::SetLayerOpacity { layer_id, opacity } => {
            if !opacity.is_finite() || !(0.0..=1.0).contains(opacity) {
                return Err("layer opacity must be between 0 and 1".to_string());
            }
            project
                .layer_mut(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?
                .opacity = *opacity;
        }
        DocumentOp::SetLayerBlendMode {
            layer_id,
            blend_mode,
        } => {
            let layer = project
                .layer_mut(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
            if layer.kind == LayerKind::Background && *blend_mode != BlendMode::Normal {
                return Err("background layers must use normal blend mode".to_string());
            }
            layer.blend_mode = *blend_mode;
        }
        DocumentOp::SetLayerKind { layer_id, kind } => {
            let layer = project
                .layer(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
            if *kind == LayerKind::Group && project.cels.iter().any(|cel| cel.layer_id == *layer_id)
            {
                return Err("a layer with cels cannot be converted to a group".to_string());
            }
            if layer.kind == LayerKind::Group
                && *kind != LayerKind::Group
                && project
                    .layers
                    .iter()
                    .any(|candidate| candidate.parent_id == Some(*layer_id))
            {
                return Err(
                    "a group with children cannot be converted to a raster layer".to_string(),
                );
            }
            if *kind == LayerKind::Background {
                if project.layers.iter().any(|candidate| {
                    candidate.id != *layer_id && candidate.kind == LayerKind::Background
                }) {
                    return Err("a project can contain at most one background layer".to_string());
                }
                let position = project
                    .layers
                    .iter()
                    .position(|candidate| candidate.id == *layer_id)
                    .expect("the layer was found above");
                let mut layer = project.layers.remove(position);
                layer.kind = LayerKind::Background;
                layer.parent_id = None;
                layer.blend_mode = BlendMode::Normal;
                project.layers.insert(0, layer);
            } else {
                project
                    .layer_mut(*layer_id)
                    .expect("the layer was found above")
                    .kind = *kind;
            }
        }
        DocumentOp::SetLayerParent {
            layer_id,
            parent_id,
        } => {
            let layer = project
                .layer(*layer_id)
                .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
            if layer.kind == LayerKind::Background && parent_id.is_some() {
                return Err("a background layer cannot have a parent".to_string());
            }
            if let Some(parent_id) = parent_id {
                let parent = project
                    .layer(*parent_id)
                    .ok_or_else(|| format!("unknown parent layer_id: {parent_id}"))?;
                if parent.kind != LayerKind::Group {
                    return Err(format!("parent layer {parent_id} is not a group"));
                }
            }
            project
                .layer_mut(*layer_id)
                .expect("the layer was found above")
                .parent_id = *parent_id;
        }
        DocumentOp::AddFrame {
            position,
            duration_ms,
        } => {
            project.add_frame(*position, duration_ms.unwrap_or(DEFAULT_FRAME_DURATION_MS))?;
        }
        DocumentOp::DuplicateFrame { frame_id } => {
            project.duplicate_frame(*frame_id)?;
        }
        DocumentOp::RemoveFrame { frame_id } => project.remove_frame_with_cels(*frame_id)?,
        DocumentOp::ReorderFrame { frame_id, position } => {
            if *position >= project.frames.len() {
                return Err(format!("frame position out of bounds: {position}"));
            }
            let current = project
                .frames
                .iter()
                .position(|frame| frame.id == *frame_id)
                .ok_or_else(|| format!("unknown frame_id: {frame_id}"))?;
            let frame = project.frames.remove(current);
            project.frames.insert(*position, frame);
        }
        DocumentOp::SetFrameDuration {
            frame_id,
            duration_ms,
        } => {
            project
                .frame_mut(*frame_id)
                .ok_or_else(|| format!("unknown frame_id: {frame_id}"))?
                .duration_ms = *duration_ms;
        }
        DocumentOp::AddTag {
            name,
            from_frame_id,
            to_frame_id,
            direction,
        } => {
            project.add_tag(name, *from_frame_id, *to_frame_id, *direction)?;
        }
        DocumentOp::RemoveTag { tag_id } => project.remove_tag(*tag_id)?,
        DocumentOp::RenameTag { tag_id, name } => project.rename_tag(*tag_id, name)?,
        DocumentOp::SetTagRange {
            tag_id,
            from_frame_id,
            to_frame_id,
        } => project.set_tag_range(*tag_id, *from_frame_id, *to_frame_id)?,
        DocumentOp::SetTagDirection { tag_id, direction } => {
            project.set_tag_direction(*tag_id, *direction)?;
        }
        DocumentOp::SetActiveTag { tag_id } => project.set_active_tag(*tag_id)?,
        DocumentOp::SetActiveLayer { layer_id } => {
            if project.layer(*layer_id).is_none() {
                return Err(format!("unknown layer_id: {layer_id}"));
            }
            project.active_layer_id = *layer_id;
        }
        DocumentOp::SetActiveFrame { frame_id } => {
            if project.frame(*frame_id).is_none() {
                return Err(format!("unknown frame_id: {frame_id}"));
            }
            project.active_frame_id = *frame_id;
        }
        DocumentOp::SetGridConfig { grid_config } => project.grid_config = *grid_config,
        DocumentOp::SetSymmetryX { line } => project.symmetry_x = *line,
        DocumentOp::SetSymmetryY { line } => project.symmetry_y = *line,
        DocumentOp::ReplaceProject {
            project: replacement,
        } => {
            replacement.validate()?;
            *project = replacement.clone();
        }
    }
    Ok(())
}

fn require_editable_layer(project: &Project, layer_id: LayerId) -> Result<(), String> {
    let layer = project
        .layer(layer_id)
        .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
    if !layer.kind.supports_cels() {
        Err(format!("group layer {layer_id} cannot contain cels"))
    } else if project.layer_is_effectively_locked(layer_id)? {
        Err(format!("layer {layer_id} is locked"))
    } else {
        Ok(())
    }
}

fn relocate_cels(project: &mut Project, relocations: &[CelRelocation]) -> Result<(), String> {
    if relocations.is_empty() {
        return Err("cel relocation cannot be empty".to_string());
    }

    let mut sources = std::collections::HashSet::new();
    let mut destinations = std::collections::HashSet::new();
    for relocation in relocations {
        if !sources.insert(relocation.source) {
            return Err("cel relocation contains a duplicate source".to_string());
        }
        if !destinations.insert(relocation.destination) {
            return Err("cel relocation contains a duplicate destination".to_string());
        }
        require_cel_position(project, relocation.source)?;
        require_cel_position(project, relocation.destination)?;
        require_editable_layer(project, relocation.source.layer_id)?;
        require_editable_layer(project, relocation.destination.layer_id)?;
    }

    let moved_ids = relocations
        .iter()
        .filter_map(|relocation| {
            project
                .cel(relocation.source.layer_id, relocation.source.frame_id)
                .map(|cel| cel.id)
        })
        .collect::<std::collections::HashSet<_>>();
    for relocation in relocations {
        if let Some(destination) = project.cel(
            relocation.destination.layer_id,
            relocation.destination.frame_id,
        ) && !moved_ids.contains(&destination.id)
        {
            return Err(format!(
                "cel relocation destination is occupied: layer {}, frame {}",
                relocation.destination.layer_id, relocation.destination.frame_id
            ));
        }
    }

    let updates = relocations
        .iter()
        .filter_map(|relocation| {
            project
                .cel(relocation.source.layer_id, relocation.source.frame_id)
                .map(|cel| (cel.id, relocation.destination))
        })
        .collect::<Vec<_>>();
    for (cel_id, destination) in updates {
        let cel = project
            .cel_by_id_mut(cel_id)
            .expect("relocated cel was collected from the project");
        cel.layer_id = destination.layer_id;
        cel.frame_id = destination.frame_id;
    }
    Ok(())
}

fn copy_cels(project: &mut Project, copies: &[CelCopy]) -> Result<(), String> {
    if copies.is_empty() {
        return Err("cel copy cannot be empty".to_string());
    }

    let mut destinations = std::collections::HashSet::new();
    for copy in copies {
        if !destinations.insert(copy.destination) {
            return Err("cel copy contains a duplicate destination".to_string());
        }
        require_cel_position(project, copy.source)?;
        require_cel_position(project, copy.destination)?;
        require_editable_layer(project, copy.destination.layer_id)?;
        if project
            .cel(copy.destination.layer_id, copy.destination.frame_id)
            .is_some()
        {
            return Err(format!(
                "cel copy destination is occupied: layer {}, frame {}",
                copy.destination.layer_id, copy.destination.frame_id
            ));
        }
    }

    let copies = copies
        .iter()
        .filter_map(|copy| {
            let source = project.cel(copy.source.layer_id, copy.source.frame_id)?;
            let resolved = project.resolved_cel(source).ok()?;
            Some((copy.destination, source.offset, resolved.pixels.clone()))
        })
        .collect::<Vec<_>>();
    for (destination, offset, pixels) in copies {
        let cel = project.ensure_cel(destination.layer_id, destination.frame_id)?;
        cel.offset = offset;
        cel.pixels = pixels;
        cel.linked_cel_id = None;
    }
    Ok(())
}

fn require_cel_position(project: &Project, position: CelPosition) -> Result<(), String> {
    if project.layer(position.layer_id).is_none() {
        return Err(format!("unknown layer_id: {}", position.layer_id));
    }
    if project.frame(position.frame_id).is_none() {
        return Err(format!("unknown frame_id: {}", position.frame_id));
    }
    Ok(())
}

fn normalized_layer_name(name: Option<&str>, existing_layers: usize) -> String {
    let trimmed = name.unwrap_or_default().trim();
    if trimmed.is_empty() {
        format!("Layer {}", existing_layers + 1)
    } else {
        trimmed.to_string()
    }
}

fn clamped_color(mut color: Rgba) -> Rgba {
    color.r = color.r.clamp(0.0, 1.0);
    color.g = color.g.clamp(0.0, 1.0);
    color.b = color.b.clamp(0.0, 1.0);
    color.a = color.a.clamp(0.0, 1.0);
    color
}

#[cfg(test)]
mod tests {
    use super::{CelCopy, CelRelocation, DocumentOp, DocumentPixel, apply_document_ops};
    use crate::grid::GridIndex;
    use crate::model::{BlendMode, CelPosition, LayerKind, Palette, Project, Rgba, TagDirection};

    #[test]
    fn resizing_empty_canvas_centers_symmetry_axes() {
        let project = Project::new_square(8.0, 0, 0);
        let resized = apply_document_ops(
            &project,
            &[DocumentOp::ResizeCanvas {
                canvas_width: 10,
                canvas_height: 6,
            }],
        )
        .unwrap();

        assert_eq!(resized.symmetry_x.position, 5.0);
        assert_eq!(resized.symmetry_y.position, 3.0);
    }

    #[test]
    fn empty_cel_is_created_only_when_painted() {
        let mut project = Project::new_square(8.0, 8, 8);
        let layer_id = project.add_layer("Ink");
        let frame_id = project.add_frame(None, 100).unwrap();
        assert!(project.cel(layer_id, frame_id).is_none());
        let edited = apply_document_ops(
            &project,
            &[DocumentOp::SetCelPixels {
                layer_id,
                frame_id,
                pixels: vec![DocumentPixel {
                    index: GridIndex { x: 2, y: 3 },
                    color: Rgba::WHITE,
                }],
            }],
        )
        .unwrap();
        assert!(
            edited
                .cel(layer_id, frame_id)
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 2, y: 3 })
        );
    }

    #[test]
    fn locked_layers_reject_pixel_edits() {
        let mut project = Project::new_square(8.0, 8, 8);
        project.layers[0].locked = true;
        let result = apply_document_ops(
            &project,
            &[DocumentOp::ClearCel {
                layer_id: project.active_layer_id,
                frame_id: project.active_frame_id,
            }],
        );
        assert!(result.unwrap_err().contains("locked"));
    }

    #[test]
    fn parent_group_lock_rejects_child_pixel_edits() {
        let project = Project::new_square(8.0, 8, 8);
        let child_id = project.active_layer_id;
        let grouped = apply_document_ops(
            &project,
            &[
                DocumentOp::AddLayer {
                    name: Some("Group".to_string()),
                    position: None,
                    kind: LayerKind::Group,
                    parent_id: None,
                },
                DocumentOp::SetLayerLocked {
                    layer_id: crate::model::LayerId(4),
                    locked: true,
                },
                DocumentOp::SetLayerParent {
                    layer_id: child_id,
                    parent_id: Some(crate::model::LayerId(4)),
                },
            ],
        )
        .unwrap();
        let result = apply_document_ops(
            &grouped,
            &[DocumentOp::SetCelPixels {
                layer_id: child_id,
                frame_id: grouped.active_frame_id,
                pixels: vec![DocumentPixel {
                    index: GridIndex { x: 1, y: 1 },
                    color: Rgba::WHITE,
                }],
            }],
        );
        assert!(result.unwrap_err().contains("locked"));
    }

    #[test]
    fn layer_structure_operations_are_transactional() {
        let project = Project::new_square(8.0, 8, 8);
        let paint_id = project.active_layer_id;
        let grouped = apply_document_ops(
            &project,
            &[DocumentOp::AddLayer {
                name: Some("Group".to_string()),
                position: None,
                kind: LayerKind::Group,
                parent_id: None,
            }],
        )
        .unwrap();
        let group_id = grouped.active_layer_id;
        let two_groups = apply_document_ops(
            &grouped,
            &[DocumentOp::AddLayer {
                name: Some("Nested Group".to_string()),
                position: None,
                kind: LayerKind::Group,
                parent_id: None,
            }],
        )
        .unwrap();
        let nested_group_id = two_groups.active_layer_id;
        let cycle_error = apply_document_ops(
            &two_groups,
            &[
                DocumentOp::SetLayerParent {
                    layer_id: group_id,
                    parent_id: Some(nested_group_id),
                },
                DocumentOp::SetLayerParent {
                    layer_id: nested_group_id,
                    parent_id: Some(group_id),
                },
            ],
        )
        .unwrap_err();
        assert!(cycle_error.contains("parent cycle"));
        assert_eq!(two_groups.layer(group_id).unwrap().parent_id, None);

        let background = apply_document_ops(
            &grouped,
            &[DocumentOp::SetLayerKind {
                layer_id: paint_id,
                kind: LayerKind::Background,
            }],
        )
        .unwrap();
        assert_eq!(background.layers[0].id, paint_id);
        assert_eq!(background.layers[0].blend_mode, BlendMode::Normal);
        assert_eq!(background.layers[0].parent_id, None);
        assert!(
            apply_document_ops(
                &background,
                &[DocumentOp::SetLayerBlendMode {
                    layer_id: paint_id,
                    blend_mode: BlendMode::Screen,
                }],
            )
            .unwrap_err()
            .contains("normal blend mode")
        );
    }

    #[test]
    fn layer_and_frame_changes_are_global_and_keep_ids() {
        let project = Project::new_square(8.0, 8, 8);
        let layer_id = project.active_layer_id;
        let frame_id = project.active_frame_id;
        let edited = apply_document_ops(
            &project,
            &[
                DocumentOp::RenameLayer {
                    layer_id,
                    name: "Character".to_string(),
                },
                DocumentOp::SetLayerVisibility {
                    layer_id,
                    visible: false,
                },
                DocumentOp::SetFrameDuration {
                    frame_id,
                    duration_ms: 250,
                },
            ],
        )
        .unwrap();
        assert_eq!(edited.layer(layer_id).unwrap().name, "Character");
        assert!(!edited.layer(layer_id).unwrap().visible);
        assert_eq!(edited.frame(frame_id).unwrap().duration_ms, 250);
    }

    #[test]
    fn linked_cel_can_be_unlinked_into_an_independent_copy() {
        let mut project = Project::new_square(8.0, 8, 8);
        let layer_id = project.active_layer_id;
        let source_frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);
        let target_frame = project.add_frame(None, 100).unwrap();
        let source_cel_id = project.cel(layer_id, source_frame).unwrap().id;
        let linked = apply_document_ops(
            &project,
            &[DocumentOp::LinkCel {
                layer_id,
                frame_id: target_frame,
                source_cel_id,
            }],
        )
        .unwrap();
        let unlinked = apply_document_ops(
            &linked,
            &[DocumentOp::UnlinkCel {
                layer_id,
                frame_id: target_frame,
            }],
        )
        .unwrap();
        let cel = unlinked.cel(layer_id, target_frame).unwrap();
        assert_eq!(cel.linked_cel_id, None);
        assert!(cel.pixels.contains_key(&GridIndex { x: 1, y: 1 }));
    }

    #[test]
    fn cel_copy_and_relocation_are_atomic_and_preserve_expected_identity() {
        let mut project = Project::new_square(8.0, 8, 8);
        let source_layer = project.active_layer_id;
        let source_frame = project.active_frame_id;
        let target_frame = project.add_frame(None, 100).unwrap();
        let target_layer = project.add_layer("Target");
        project
            .cel_mut(source_layer, source_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 3 }, Rgba::WHITE);
        let source_id = project.cel(source_layer, source_frame).unwrap().id;

        let copied = apply_document_ops(
            &project,
            &[DocumentOp::CopyCels {
                copies: vec![CelCopy {
                    source: CelPosition {
                        layer_id: source_layer,
                        frame_id: source_frame,
                    },
                    destination: CelPosition {
                        layer_id: source_layer,
                        frame_id: target_frame,
                    },
                }],
            }],
        )
        .unwrap();
        let copied_cel = copied.cel(source_layer, target_frame).unwrap();
        assert_ne!(copied_cel.id, source_id);
        assert_eq!(copied_cel.linked_cel_id, None);
        assert!(copied_cel.pixels.contains_key(&GridIndex { x: 2, y: 3 }));

        let relocated = apply_document_ops(
            &copied,
            &[DocumentOp::RelocateCels {
                relocations: vec![CelRelocation {
                    source: CelPosition {
                        layer_id: source_layer,
                        frame_id: source_frame,
                    },
                    destination: CelPosition {
                        layer_id: target_layer,
                        frame_id: target_frame,
                    },
                }],
            }],
        )
        .unwrap();
        assert!(relocated.cel(source_layer, source_frame).is_none());
        assert_eq!(
            relocated.cel(target_layer, target_frame).unwrap().id,
            source_id
        );

        let occupied_error = apply_document_ops(
            &project,
            &[DocumentOp::RelocateCels {
                relocations: vec![CelRelocation {
                    source: CelPosition {
                        layer_id: source_layer,
                        frame_id: source_frame,
                    },
                    destination: CelPosition {
                        layer_id: source_layer,
                        frame_id: source_frame,
                    },
                }],
            }],
        )
        .unwrap();
        assert_eq!(
            occupied_error.cel(source_layer, source_frame).unwrap().id,
            source_id
        );
    }

    #[test]
    fn animation_tag_crud_is_transactional() {
        let mut project = Project::new_square(8.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 100).unwrap();
        let tagged = apply_document_ops(
            &project,
            &[DocumentOp::AddTag {
                name: "Walk".to_string(),
                from_frame_id: first,
                to_frame_id: second,
                direction: TagDirection::Forward,
            }],
        )
        .unwrap();
        let tag_id = tagged.tags[0].id;
        assert_eq!(tagged.active_tag_id, Some(tag_id));

        let edited = apply_document_ops(
            &tagged,
            &[
                DocumentOp::RenameTag {
                    tag_id,
                    name: "Walk Back".to_string(),
                },
                DocumentOp::SetTagDirection {
                    tag_id,
                    direction: TagDirection::Reverse,
                },
                DocumentOp::SetTagRange {
                    tag_id,
                    from_frame_id: second,
                    to_frame_id: second,
                },
                DocumentOp::SetActiveTag { tag_id: None },
            ],
        )
        .unwrap();
        let tag = edited.tag(tag_id).unwrap();
        assert_eq!(tag.name, "Walk Back");
        assert_eq!(tag.direction, TagDirection::Reverse);
        assert_eq!((tag.from_frame_id, tag.to_frame_id), (second, second));
        assert_eq!(edited.active_tag_id, None);

        let removed = apply_document_ops(&edited, &[DocumentOp::RemoveTag { tag_id }]).unwrap();
        assert!(removed.tags.is_empty());
    }

    #[test]
    fn palette_and_active_color_operations_are_transactional() {
        let project = Project::new_square(8.0, 8, 8);
        let red = Rgba::new(1.0, 0.0, 0.0, 0.5);
        let edited = apply_document_ops(
            &project,
            &[
                DocumentOp::SetForegroundColor { color: red },
                DocumentOp::SetBackgroundColor { color: Rgba::WHITE },
                DocumentOp::ReplacePalette {
                    palette: Palette {
                        name: "Custom".to_string(),
                        colors: vec![red, Rgba::BLACK, red],
                    },
                },
                DocumentOp::DeduplicatePalette,
                DocumentOp::ReorderPaletteColor {
                    from_position: 1,
                    position: 0,
                },
                DocumentOp::ReplacePaletteColor {
                    position: 1,
                    color: Rgba::WHITE,
                },
            ],
        )
        .unwrap();

        assert_eq!(edited.foreground_color, red);
        assert_eq!(edited.background_color, Rgba::WHITE);
        assert_eq!(edited.palette.name, "Custom");
        assert_eq!(edited.palette.colors, vec![Rgba::BLACK, Rgba::WHITE]);

        let error = apply_document_ops(
            &project,
            &[DocumentOp::RemovePaletteColor { position: 999 }],
        )
        .unwrap_err();
        assert!(error.contains("out of bounds"));
        assert_eq!(project.palette, Palette::pico8());
    }

    #[test]
    fn frame_reordering_keeps_tag_ids_or_rejects_an_invalid_range() {
        let mut project = Project::new_square(8.0, 8, 8);
        let first = project.active_frame_id;
        let middle = project.add_frame(None, 100).unwrap();
        let last = project.add_frame(None, 100).unwrap();
        let tag_id = project
            .add_tag("Range", first, last, TagDirection::Forward)
            .unwrap();

        let reordered = apply_document_ops(
            &project,
            &[DocumentOp::ReorderFrame {
                frame_id: middle,
                position: 0,
            }],
        )
        .unwrap();
        assert_eq!(reordered.tag(tag_id).unwrap().from_frame_id, first);
        assert_eq!(reordered.tag(tag_id).unwrap().to_frame_id, last);

        let error = apply_document_ops(
            &project,
            &[DocumentOp::ReorderFrame {
                frame_id: last,
                position: 0,
            }],
        )
        .unwrap_err();
        assert!(error.contains("start frame"));
        assert_eq!(project.frames[0].id, first);
    }
}
