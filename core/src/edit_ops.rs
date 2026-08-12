use crate::document::{DocumentOp, DocumentPixel, apply_document_ops};
use crate::grid::GridIndex;
use crate::model::{
    BlendMode, CelId, CelPosition, FrameId, GridConfig, LayerId, LayerKind, Palette, Project, Rgba,
    TagDirection, TagId,
};
use crate::transform::{PixelBounds, PixelTransform};
use serde::{Deserialize, Serialize};

const MAX_EDIT_OPS: usize = 128;
const MAX_PIXELS_PER_EDIT_OP: usize = 4096;
const MAX_TOTAL_PIXELS: usize = 16_384;
const MAX_FRAMES: usize = 32;
const MAX_LAYERS: usize = 32;
const MAX_TAGS: usize = 128;
const CEL_SAMPLE_LIMIT: usize = 24;
const CEL_SUMMARY_LIMIT: usize = 256;
const SELECTION_SAMPLE_LIMIT: usize = 128;
const SELECTION_INDEX_SAMPLE_LIMIT: usize = 256;
const SELECTION_NEARBY_SAMPLE_LIMIT: usize = 192;
const SELECTION_CONTEXT_PADDING: i32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub schema_version: u32,
    pub grid_config: GridConfig,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub active_layer_id: LayerId,
    pub active_frame_id: FrameId,
    pub active_tag_id: Option<TagId>,
    pub palette: Palette,
    pub foreground_color: Rgba,
    pub background_color: Rgba,
    pub total_colored_pixels: usize,
    pub cel_count: usize,
    pub cels_truncated: bool,
    pub layers: Vec<LayerSummary>,
    pub frames: Vec<FrameSummary>,
    pub tags: Vec<TagSummary>,
    pub cels: Vec<CelSummary>,
}

impl ProjectSummary {
    pub fn from_project(project: &Project) -> Self {
        Self {
            schema_version: project.schema_version,
            grid_config: project.grid_config,
            canvas_width: project.canvas_width,
            canvas_height: project.canvas_height,
            active_layer_id: project.active_layer_id,
            active_frame_id: project.active_frame_id,
            active_tag_id: project.active_tag_id,
            palette: project.palette.clone(),
            foreground_color: project.foreground_color,
            background_color: project.background_color,
            total_colored_pixels: project.cels.iter().map(|cel| cel.pixels.len()).sum(),
            cel_count: project.cels.len(),
            cels_truncated: project.cels.len() > CEL_SUMMARY_LIMIT,
            layers: project
                .layers
                .iter()
                .map(|layer| LayerSummary {
                    layer_id: layer.id,
                    name: layer.name.clone(),
                    visible: layer.visible,
                    locked: layer.locked,
                    opacity: layer.opacity,
                    blend_mode: layer.blend_mode,
                    kind: layer.kind,
                    parent_id: layer.parent_id,
                    depth: project.layer_depth(layer.id).unwrap_or(0),
                    effective_visible: project
                        .layer_is_effectively_visible(layer.id)
                        .unwrap_or(false),
                    effective_locked: project
                        .layer_is_effectively_locked(layer.id)
                        .unwrap_or(true),
                })
                .collect(),
            frames: project
                .frames
                .iter()
                .map(|frame| FrameSummary {
                    frame_id: frame.id,
                    duration_ms: frame.duration_ms,
                })
                .collect(),
            tags: project
                .tags
                .iter()
                .map(|tag| TagSummary {
                    tag_id: tag.id,
                    name: tag.name.clone(),
                    from_frame_id: tag.from_frame_id,
                    to_frame_id: tag.to_frame_id,
                    direction: tag.direction,
                })
                .collect(),
            cels: project
                .cels
                .iter()
                .take(CEL_SUMMARY_LIMIT)
                .map(|cel| CelSummary {
                    cel_id: cel.id,
                    layer_id: cel.layer_id,
                    frame_id: cel.frame_id,
                    offset: cel.offset,
                    linked_cel_id: cel.linked_cel_id,
                    colored_pixels: cel.pixels.len(),
                    sample_pixels: sorted_pixel_samples(
                        cel.pixels.iter().map(|(index, color)| PixelSample {
                            index: *index,
                            color: *color,
                        }),
                        CEL_SAMPLE_LIMIT,
                    ),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerSummary {
    pub layer_id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub kind: LayerKind,
    pub parent_id: Option<LayerId>,
    pub depth: usize,
    pub effective_visible: bool,
    pub effective_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSummary {
    pub frame_id: FrameId,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSummary {
    pub tag_id: TagId,
    pub name: String,
    pub from_frame_id: FrameId,
    pub to_frame_id: FrameId,
    pub direction: TagDirection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelSummary {
    pub cel_id: CelId,
    pub layer_id: LayerId,
    pub frame_id: FrameId,
    pub offset: GridIndex,
    pub linked_cel_id: Option<CelId>,
    pub colored_pixels: usize,
    pub sample_pixels: Vec<PixelSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PixelSample {
    pub index: GridIndex,
    pub color: Rgba,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionSummary {
    pub active: bool,
    pub active_layer_id: LayerId,
    pub active_frame_id: FrameId,
    pub active_cel_id: Option<CelId>,
    pub frame_duration_ms: u64,
    pub layer_name: String,
    pub layer_visible: bool,
    pub layer_locked: bool,
    pub selected_cells: usize,
    pub colored_pixels: usize,
    pub bounds: Option<SelectionBounds>,
    pub selected_indices: Vec<GridIndex>,
    pub pixels: Vec<PixelSample>,
    pub nearby_pixels: Vec<PixelSample>,
    pub timeline: TimelineSelectionSummary,
    pub scope_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineSelectionSummary {
    pub active: bool,
    pub selected_cels: usize,
    pub populated_cels: usize,
    pub linked_cels: usize,
    pub layer_ids: Vec<LayerId>,
    pub frame_ids: Vec<FrameId>,
    pub bounds: Option<TimelineSelectionBounds>,
    pub cells: Vec<TimelineCellSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineSelectionBounds {
    pub first_layer_id: LayerId,
    pub last_layer_id: LayerId,
    pub first_frame_id: FrameId,
    pub last_frame_id: FrameId,
    pub layer_span: usize,
    pub frame_span: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCellSummary {
    pub layer_id: LayerId,
    pub frame_id: FrameId,
    pub cel_id: Option<CelId>,
    pub has_content: bool,
    pub linked: bool,
}

impl TimelineSelectionSummary {
    pub fn from_project<I>(project: &Project, positions: I) -> Self
    where
        I: IntoIterator<Item = CelPosition>,
    {
        let layer_positions = project
            .layers
            .iter()
            .enumerate()
            .map(|(index, layer)| (layer.id, index))
            .collect::<std::collections::HashMap<_, _>>();
        let frame_positions = project
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| (frame.id, index))
            .collect::<std::collections::HashMap<_, _>>();
        let mut positions = positions
            .into_iter()
            .filter(|position| {
                layer_positions.contains_key(&position.layer_id)
                    && frame_positions.contains_key(&position.frame_id)
            })
            .collect::<Vec<_>>();
        positions.sort_by_key(|position| {
            (
                layer_positions[&position.layer_id],
                frame_positions[&position.frame_id],
            )
        });
        positions.dedup();

        let mut layer_ids = positions
            .iter()
            .map(|position| position.layer_id)
            .collect::<Vec<_>>();
        layer_ids.dedup();
        let mut frame_ids = positions
            .iter()
            .map(|position| position.frame_id)
            .collect::<Vec<_>>();
        frame_ids.sort_by_key(|frame_id| frame_positions[frame_id]);
        frame_ids.dedup();
        let bounds = if positions.is_empty() {
            None
        } else {
            let first_layer_position = positions
                .iter()
                .map(|position| layer_positions[&position.layer_id])
                .min()
                .expect("non-empty timeline selection has a layer");
            let last_layer_position = positions
                .iter()
                .map(|position| layer_positions[&position.layer_id])
                .max()
                .expect("non-empty timeline selection has a layer");
            let first_frame_position = positions
                .iter()
                .map(|position| frame_positions[&position.frame_id])
                .min()
                .expect("non-empty timeline selection has a frame");
            let last_frame_position = positions
                .iter()
                .map(|position| frame_positions[&position.frame_id])
                .max()
                .expect("non-empty timeline selection has a frame");
            Some(TimelineSelectionBounds {
                first_layer_id: project.layers[first_layer_position].id,
                last_layer_id: project.layers[last_layer_position].id,
                first_frame_id: project.frames[first_frame_position].id,
                last_frame_id: project.frames[last_frame_position].id,
                layer_span: last_layer_position - first_layer_position + 1,
                frame_span: last_frame_position - first_frame_position + 1,
            })
        };
        let cells = positions
            .iter()
            .map(|position| {
                let cel = project.cel(position.layer_id, position.frame_id);
                let has_content = cel
                    .and_then(|cel| project.resolved_cel(cel).ok())
                    .is_some_and(|cel| !cel.pixels.is_empty());
                TimelineCellSummary {
                    layer_id: position.layer_id,
                    frame_id: position.frame_id,
                    cel_id: cel.map(|cel| cel.id),
                    has_content,
                    linked: cel.is_some_and(|cel| cel.linked_cel_id.is_some()),
                }
            })
            .collect::<Vec<_>>();

        Self {
            active: !cells.is_empty(),
            selected_cels: cells.len(),
            populated_cels: cells.iter().filter(|cell| cell.has_content).count(),
            linked_cels: cells.iter().filter(|cell| cell.linked).count(),
            layer_ids,
            frame_ids,
            bounds,
            cells,
        }
    }
}

impl SelectionSummary {
    pub fn from_project_selection<I>(
        project: &Project,
        layer_id: LayerId,
        frame_id: FrameId,
        selection_indices: I,
    ) -> Self
    where
        I: IntoIterator<Item = GridIndex>,
    {
        Self::from_project_selections(
            project,
            layer_id,
            frame_id,
            selection_indices,
            std::iter::empty(),
        )
    }

    pub fn from_project_selections<I, T>(
        project: &Project,
        layer_id: LayerId,
        frame_id: FrameId,
        selection_indices: I,
        timeline_positions: T,
    ) -> Self
    where
        I: IntoIterator<Item = GridIndex>,
        T: IntoIterator<Item = CelPosition>,
    {
        let mut indices = selection_indices.into_iter().collect::<Vec<_>>();
        indices.sort_by_key(|index| (index.y, index.x));
        let layer = project.layer(layer_id);
        let frame_duration_ms = project
            .frame(frame_id)
            .map(|frame| frame.duration_ms)
            .unwrap_or(100);
        let display_pixels = display_pixels(project, layer_id, frame_id);
        let timeline = TimelineSelectionSummary::from_project(project, timeline_positions);
        let timeline_active = timeline.active;

        if indices.is_empty() {
            return Self {
                active: false,
                active_layer_id: layer_id,
                active_frame_id: frame_id,
                active_cel_id: project.cel(layer_id, frame_id).map(|cel| cel.id),
                frame_duration_ms,
                layer_name: layer
                    .map(|layer| layer.name.clone())
                    .unwrap_or_else(|| "Unknown layer".to_string()),
                layer_visible: project
                    .layer_is_effectively_visible(layer_id)
                    .unwrap_or(false),
                layer_locked: project
                    .layer_is_effectively_locked(layer_id)
                    .unwrap_or(true),
                selected_cells: 0,
                colored_pixels: 0,
                bounds: None,
                selected_indices: Vec::new(),
                pixels: Vec::new(),
                nearby_pixels: Vec::new(),
                scope_hint: if timeline_active {
                    "A timeline cel selection exists. Cel operations target timeline.cells; pixel edits still target the active layer/frame cel.".to_string()
                } else {
                    "No active selection. Global edits are allowed if they match the user request. Pixel edits target the active layer/frame cel and create it on demand.".to_string()
                },
                timeline,
            };
        }

        let bounds = SelectionBounds::from_indices(&indices);
        let pixels = sorted_pixel_samples(
            display_pixels
                .iter()
                .filter(|pixel| {
                    indices
                        .binary_search_by_key(&(pixel.index.y, pixel.index.x), |index| {
                            (index.y, index.x)
                        })
                        .is_ok()
                })
                .cloned(),
            SELECTION_SAMPLE_LIMIT,
        );
        let nearby_pixels = bounds
            .as_ref()
            .map(|bounds| {
                sorted_pixel_samples(
                    display_pixels
                        .iter()
                        .filter(|pixel| {
                            !bounds.contains(pixel.index)
                                && bounds
                                    .expanded(SELECTION_CONTEXT_PADDING)
                                    .contains(pixel.index)
                        })
                        .cloned(),
                    SELECTION_NEARBY_SAMPLE_LIMIT,
                )
            })
            .unwrap_or_default();

        Self {
            active: true,
            active_layer_id: layer_id,
            active_frame_id: frame_id,
            active_cel_id: project.cel(layer_id, frame_id).map(|cel| cel.id),
            frame_duration_ms,
            layer_name: layer
                .map(|layer| layer.name.clone())
                .unwrap_or_else(|| "Unknown layer".to_string()),
            layer_visible: project
                .layer_is_effectively_visible(layer_id)
                .unwrap_or(false),
            layer_locked: project
                .layer_is_effectively_locked(layer_id)
                .unwrap_or(true),
            selected_cells: indices.len(),
            colored_pixels: pixels.len(),
            bounds,
            selected_indices: indices
                .into_iter()
                .take(SELECTION_INDEX_SAMPLE_LIMIT)
                .collect(),
            pixels,
            nearby_pixels,
            timeline,
            scope_hint: if timeline_active {
                "Pixel and timeline cel selections are active. Pixel edits target selected_indices; cel operations target timeline.cells.".to_string()
            } else {
                "An active selection exists on the active cel. Prefer edits inside the selection first. Use nearby_pixels only as context unless the user explicitly asks for a broader change.".to_string()
            },
        }
    }
}

fn display_pixels(project: &Project, layer_id: LayerId, frame_id: FrameId) -> Vec<PixelSample> {
    let Some(destination) = project.cel(layer_id, frame_id) else {
        return Vec::new();
    };
    let Ok(source) = project.resolved_cel(destination) else {
        return Vec::new();
    };
    source
        .pixels
        .iter()
        .map(|(index, color)| PixelSample {
            index: GridIndex {
                x: index.x.saturating_add(destination.offset.x),
                y: index.y.saturating_add(destination.offset.y),
            },
            color: *color,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl SelectionBounds {
    fn from_indices(indices: &[GridIndex]) -> Option<Self> {
        let first = indices.first()?;
        let mut bounds = Self {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for index in indices.iter().skip(1) {
            bounds.min_x = bounds.min_x.min(index.x);
            bounds.min_y = bounds.min_y.min(index.y);
            bounds.max_x = bounds.max_x.max(index.x);
            bounds.max_y = bounds.max_y.max(index.y);
        }
        Some(bounds)
    }

    fn contains(&self, index: GridIndex) -> bool {
        index.x >= self.min_x
            && index.x <= self.max_x
            && index.y >= self.min_y
            && index.y <= self.max_y
    }

    fn expanded(&self, padding: i32) -> Self {
        Self {
            min_x: self.min_x.saturating_sub(padding),
            min_y: self.min_y.saturating_sub(padding),
            max_x: self.max_x.saturating_add(padding),
            max_y: self.max_y.saturating_add(padding),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PixelChange {
    pub index: GridIndex,
    pub color: Rgba,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditOp {
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
        #[serde(default)]
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
        name: String,
        colors: Vec<Rgba>,
    },
    SetCelPixels {
        layer_id: LayerId,
        frame_id: FrameId,
        pixels: Vec<PixelChange>,
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
        #[serde(default)]
        selection: Vec<GridIndex>,
        transform: PixelTransform,
    },
    CropCanvas {
        bounds: PixelBounds,
    },
    TrimCanvas,
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
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        position: Option<usize>,
        #[serde(default)]
        kind: Option<LayerKind>,
        #[serde(default)]
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
        #[serde(default)]
        position: Option<usize>,
        #[serde(default)]
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
    SetActiveFrame {
        frame_id: FrameId,
    },
    SetActiveLayer {
        layer_id: LayerId,
    },
}

impl EditOp {
    fn to_document_op(&self) -> DocumentOp {
        match self {
            EditOp::ResizeCanvas {
                canvas_width,
                canvas_height,
            } => DocumentOp::ResizeCanvas {
                canvas_width: *canvas_width,
                canvas_height: *canvas_height,
            },
            EditOp::SetForegroundColor { color } => {
                DocumentOp::SetForegroundColor { color: *color }
            }
            EditOp::SetBackgroundColor { color } => {
                DocumentOp::SetBackgroundColor { color: *color }
            }
            EditOp::RenamePalette { name } => DocumentOp::RenamePalette { name: name.clone() },
            EditOp::AddPaletteColor { color, position } => DocumentOp::AddPaletteColor {
                color: *color,
                position: *position,
            },
            EditOp::RemovePaletteColor { position } => DocumentOp::RemovePaletteColor {
                position: *position,
            },
            EditOp::ReplacePaletteColor { position, color } => DocumentOp::ReplacePaletteColor {
                position: *position,
                color: *color,
            },
            EditOp::ReorderPaletteColor {
                from_position,
                position,
            } => DocumentOp::ReorderPaletteColor {
                from_position: *from_position,
                position: *position,
            },
            EditOp::DeduplicatePalette => DocumentOp::DeduplicatePalette,
            EditOp::ReplacePalette { name, colors } => DocumentOp::ReplacePalette {
                palette: Palette {
                    name: name.clone(),
                    colors: colors.clone(),
                },
            },
            EditOp::SetCelPixels {
                layer_id,
                frame_id,
                pixels,
            } => DocumentOp::SetCelPixels {
                layer_id: *layer_id,
                frame_id: *frame_id,
                pixels: pixels
                    .iter()
                    .map(|pixel| DocumentPixel {
                        index: pixel.index,
                        color: pixel.color,
                    })
                    .collect(),
            },
            EditOp::EraseCelPixels {
                layer_id,
                frame_id,
                indices,
            } => DocumentOp::EraseCelPixels {
                layer_id: *layer_id,
                frame_id: *frame_id,
                indices: indices.clone(),
            },
            EditOp::ClearCel { layer_id, frame_id } => DocumentOp::ClearCel {
                layer_id: *layer_id,
                frame_id: *frame_id,
            },
            EditOp::MoveCel {
                layer_id,
                frame_id,
                offset,
            } => DocumentOp::MoveCel {
                layer_id: *layer_id,
                frame_id: *frame_id,
                offset: *offset,
            },
            EditOp::TransformCelPixels {
                targets,
                selection,
                transform,
            } => DocumentOp::TransformCelPixels {
                targets: targets.clone(),
                selection: selection.clone(),
                transform: *transform,
            },
            EditOp::CropCanvas { bounds } => DocumentOp::CropCanvas { bounds: *bounds },
            EditOp::TrimCanvas => DocumentOp::TrimCanvas,
            EditOp::DeleteCel { layer_id, frame_id } => DocumentOp::DeleteCel {
                layer_id: *layer_id,
                frame_id: *frame_id,
            },
            EditOp::LinkCel {
                layer_id,
                frame_id,
                source_cel_id,
            } => DocumentOp::LinkCel {
                layer_id: *layer_id,
                frame_id: *frame_id,
                source_cel_id: *source_cel_id,
            },
            EditOp::UnlinkCel { layer_id, frame_id } => DocumentOp::UnlinkCel {
                layer_id: *layer_id,
                frame_id: *frame_id,
            },
            EditOp::AddLayer {
                name,
                position,
                kind,
                parent_id,
            } => DocumentOp::AddLayer {
                name: name.clone(),
                position: *position,
                kind: kind.unwrap_or(LayerKind::Paint),
                parent_id: *parent_id,
            },
            EditOp::RemoveLayer { layer_id } => DocumentOp::RemoveLayer {
                layer_id: *layer_id,
            },
            EditOp::RenameLayer { layer_id, name } => DocumentOp::RenameLayer {
                layer_id: *layer_id,
                name: name.clone(),
            },
            EditOp::ReorderLayer { layer_id, position } => DocumentOp::ReorderLayer {
                layer_id: *layer_id,
                position: *position,
            },
            EditOp::SetLayerVisibility { layer_id, visible } => DocumentOp::SetLayerVisibility {
                layer_id: *layer_id,
                visible: *visible,
            },
            EditOp::SetLayerLocked { layer_id, locked } => DocumentOp::SetLayerLocked {
                layer_id: *layer_id,
                locked: *locked,
            },
            EditOp::SetLayerOpacity { layer_id, opacity } => DocumentOp::SetLayerOpacity {
                layer_id: *layer_id,
                opacity: *opacity,
            },
            EditOp::SetLayerBlendMode {
                layer_id,
                blend_mode,
            } => DocumentOp::SetLayerBlendMode {
                layer_id: *layer_id,
                blend_mode: *blend_mode,
            },
            EditOp::SetLayerKind { layer_id, kind } => DocumentOp::SetLayerKind {
                layer_id: *layer_id,
                kind: *kind,
            },
            EditOp::SetLayerParent {
                layer_id,
                parent_id,
            } => DocumentOp::SetLayerParent {
                layer_id: *layer_id,
                parent_id: *parent_id,
            },
            EditOp::AddFrame {
                position,
                duration_ms,
            } => DocumentOp::AddFrame {
                position: *position,
                duration_ms: *duration_ms,
            },
            EditOp::DuplicateFrame { frame_id } => DocumentOp::DuplicateFrame {
                frame_id: *frame_id,
            },
            EditOp::RemoveFrame { frame_id } => DocumentOp::RemoveFrame {
                frame_id: *frame_id,
            },
            EditOp::ReorderFrame { frame_id, position } => DocumentOp::ReorderFrame {
                frame_id: *frame_id,
                position: *position,
            },
            EditOp::SetFrameDuration {
                frame_id,
                duration_ms,
            } => DocumentOp::SetFrameDuration {
                frame_id: *frame_id,
                duration_ms: *duration_ms,
            },
            EditOp::AddTag {
                name,
                from_frame_id,
                to_frame_id,
                direction,
            } => DocumentOp::AddTag {
                name: name.clone(),
                from_frame_id: *from_frame_id,
                to_frame_id: *to_frame_id,
                direction: *direction,
            },
            EditOp::RemoveTag { tag_id } => DocumentOp::RemoveTag { tag_id: *tag_id },
            EditOp::RenameTag { tag_id, name } => DocumentOp::RenameTag {
                tag_id: *tag_id,
                name: name.clone(),
            },
            EditOp::SetTagRange {
                tag_id,
                from_frame_id,
                to_frame_id,
            } => DocumentOp::SetTagRange {
                tag_id: *tag_id,
                from_frame_id: *from_frame_id,
                to_frame_id: *to_frame_id,
            },
            EditOp::SetTagDirection { tag_id, direction } => DocumentOp::SetTagDirection {
                tag_id: *tag_id,
                direction: *direction,
            },
            EditOp::SetActiveTag { tag_id } => DocumentOp::SetActiveTag { tag_id: *tag_id },
            EditOp::SetActiveFrame { frame_id } => DocumentOp::SetActiveFrame {
                frame_id: *frame_id,
            },
            EditOp::SetActiveLayer { layer_id } => DocumentOp::SetActiveLayer {
                layer_id: *layer_id,
            },
        }
    }
}

pub fn apply_edit_ops(project: &Project, ops: &[EditOp]) -> Result<Project, String> {
    if ops.is_empty() {
        return Err("no edit operations were provided".to_string());
    }
    if ops.len() > MAX_EDIT_OPS {
        return Err(format!(
            "too many edit operations: {} > {MAX_EDIT_OPS}",
            ops.len()
        ));
    }
    let changed_pixels = ops.iter().try_fold(0usize, |total, op| {
        let count = match op {
            EditOp::SetCelPixels { pixels, .. } => pixels.len(),
            EditOp::EraseCelPixels { indices, .. } => indices.len(),
            _ => 0,
        };
        if count > MAX_PIXELS_PER_EDIT_OP {
            return Err(format!(
                "one operation contains too many pixels: {count} > {MAX_PIXELS_PER_EDIT_OP}"
            ));
        }
        if count == 0
            && matches!(
                op,
                EditOp::SetCelPixels { .. } | EditOp::EraseCelPixels { .. }
            )
        {
            return Err("pixel operations cannot contain an empty list".to_string());
        }
        Ok(total + count)
    })?;
    if changed_pixels > MAX_TOTAL_PIXELS {
        return Err(format!(
            "too many changed pixels: {changed_pixels} > {MAX_TOTAL_PIXELS}"
        ));
    }
    let document_ops = ops.iter().map(EditOp::to_document_op).collect::<Vec<_>>();
    let edited = apply_document_ops(project, &document_ops)?;
    if edited.layers.len() > MAX_LAYERS {
        return Err(format!(
            "too many layers: {} > {MAX_LAYERS}",
            edited.layers.len()
        ));
    }
    if edited.frames.len() > MAX_FRAMES {
        return Err(format!(
            "too many frames: {} > {MAX_FRAMES}",
            edited.frames.len()
        ));
    }
    if edited.tags.len() > MAX_TAGS {
        return Err(format!(
            "too many animation tags: {} > {MAX_TAGS}",
            edited.tags.len()
        ));
    }
    let total_pixels = edited
        .cels
        .iter()
        .map(|cel| cel.pixels.len())
        .sum::<usize>();
    if total_pixels > MAX_TOTAL_PIXELS {
        return Err(format!(
            "project contains too many pixels: {total_pixels} > {MAX_TOTAL_PIXELS}"
        ));
    }
    Ok(edited)
}

fn sorted_pixel_samples<I>(pixels: I, limit: usize) -> Vec<PixelSample>
where
    I: IntoIterator<Item = PixelSample>,
{
    let mut pixels = pixels.into_iter().collect::<Vec<_>>();
    pixels.sort_by_key(|pixel| (pixel.index.y, pixel.index.x));
    pixels.truncate(limit);
    pixels
}

pub const EDIT_OP_JSON_SCHEMA: &str = r##"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "gridvana://schema/edit-op",
  "title": "Gridvana EditOp V6 array",
  "type": "array", "minItems": 1, "maxItems": 128,
  "items": {
    "type": "object", "additionalProperties": false,
    "properties": {
      "type": {"type":"string"},
      "canvas_width": {"type":"integer","minimum":1,"maximum":4096},
      "canvas_height": {"type":"integer","minimum":1,"maximum":4096},
      "layer_id": {"type":"integer","minimum":1}, "frame_id": {"type":"integer","minimum":1},
      "tag_id": {"type":["integer","null"],"minimum":1},
      "from_frame_id": {"type":"integer","minimum":1}, "to_frame_id": {"type":"integer","minimum":1},
      "direction": {"type":"string","enum":["forward","reverse","ping_pong"]},
      "source_cel_id": {"type":"integer","minimum":1},
      "targets": {"type":"array","minItems":1,"maxItems":256,"items":{"$ref":"#/$defs/cel_position"}},
      "selection": {"type":"array","maxItems":16384,"items":{"$ref":"#/$defs/grid_index"}},
      "transform": {"$ref":"#/$defs/pixel_transform"},
      "bounds": {"$ref":"#/$defs/pixel_bounds"},
      "pixels": {"type":"array","minItems":1,"maxItems":4096,"items":{"$ref":"#/$defs/pixel_change"}},
      "indices": {"type":"array","minItems":1,"maxItems":4096,"items":{"$ref":"#/$defs/grid_index"}},
      "offset": {"$ref":"#/$defs/grid_index"}, "name": {"type":["string","null"]},
      "color": {"$ref":"#/$defs/rgba"},
      "colors": {"type":"array","maxItems":256,"items":{"$ref":"#/$defs/rgba"}},
      "position": {"type":["integer","null"],"minimum":0},
      "from_position": {"type":"integer","minimum":0}, "visible": {"type":"boolean"},
      "locked": {"type":"boolean"}, "opacity": {"type":"number","minimum":0,"maximum":1},
      "blend_mode": {"type":"string","enum":["normal","multiply","screen","overlay"]},
      "kind": {"type":["string","null"],"enum":["paint","group","background","reference",null]},
      "parent_id": {"type":["integer","null"],"minimum":1},
      "duration_ms": {"type":["integer","null"],"minimum":1}
    },
    "oneOf": [
      {"properties":{"type":{"const":"resize_canvas"}},"required":["type","canvas_width","canvas_height"]},
      {"properties":{"type":{"const":"set_foreground_color"}},"required":["type","color"]},
      {"properties":{"type":{"const":"set_background_color"}},"required":["type","color"]},
      {"properties":{"type":{"const":"rename_palette"},"name":{"type":"string","minLength":1,"maxLength":128}},"required":["type","name"]},
      {"properties":{"type":{"const":"add_palette_color"}},"required":["type","color"]},
      {"properties":{"type":{"const":"remove_palette_color"},"position":{"type":"integer","minimum":0}},"required":["type","position"]},
      {"properties":{"type":{"const":"replace_palette_color"},"position":{"type":"integer","minimum":0}},"required":["type","position","color"]},
      {"properties":{"type":{"const":"reorder_palette_color"},"position":{"type":"integer","minimum":0}},"required":["type","from_position","position"]},
      {"properties":{"type":{"const":"deduplicate_palette"}},"required":["type"]},
      {"properties":{"type":{"const":"replace_palette"},"name":{"type":"string","minLength":1,"maxLength":128}},"required":["type","name","colors"]},
      {"properties":{"type":{"const":"set_cel_pixels"}},"required":["type","layer_id","frame_id","pixels"]},
      {"properties":{"type":{"const":"erase_cel_pixels"}},"required":["type","layer_id","frame_id","indices"]},
      {"properties":{"type":{"const":"clear_cel"}},"required":["type","layer_id","frame_id"]},
      {"properties":{"type":{"const":"move_cel"}},"required":["type","layer_id","frame_id","offset"]},
      {"properties":{"type":{"const":"transform_cel_pixels"}},"required":["type","targets","transform"]},
      {"properties":{"type":{"const":"crop_canvas"}},"required":["type","bounds"]},
      {"properties":{"type":{"const":"trim_canvas"}},"required":["type"]},
      {"properties":{"type":{"const":"delete_cel"}},"required":["type","layer_id","frame_id"]},
      {"properties":{"type":{"const":"link_cel"}},"required":["type","layer_id","frame_id","source_cel_id"]},
      {"properties":{"type":{"const":"unlink_cel"}},"required":["type","layer_id","frame_id"]},
      {"properties":{"type":{"const":"add_layer"}},"required":["type"]},
      {"properties":{"type":{"const":"remove_layer"}},"required":["type","layer_id"]},
      {"properties":{"type":{"const":"rename_layer"},"name":{"type":"string","minLength":1}},"required":["type","layer_id","name"]},
      {"properties":{"type":{"const":"reorder_layer"}},"required":["type","layer_id","position"]},
      {"properties":{"type":{"const":"set_layer_visibility"}},"required":["type","layer_id","visible"]},
      {"properties":{"type":{"const":"set_layer_locked"}},"required":["type","layer_id","locked"]},
      {"properties":{"type":{"const":"set_layer_opacity"}},"required":["type","layer_id","opacity"]},
      {"properties":{"type":{"const":"set_layer_blend_mode"}},"required":["type","layer_id","blend_mode"]},
      {"properties":{"type":{"const":"set_layer_kind"},"kind":{"type":"string","enum":["paint","group","background","reference"]}},"required":["type","layer_id","kind"]},
      {"properties":{"type":{"const":"set_layer_parent"}},"required":["type","layer_id","parent_id"]},
      {"properties":{"type":{"const":"add_frame"}},"required":["type"]},
      {"properties":{"type":{"const":"duplicate_frame"}},"required":["type","frame_id"]},
      {"properties":{"type":{"const":"remove_frame"}},"required":["type","frame_id"]},
      {"properties":{"type":{"const":"reorder_frame"}},"required":["type","frame_id","position"]},
      {"properties":{"type":{"const":"set_frame_duration"},"duration_ms":{"type":"integer","minimum":1}},"required":["type","frame_id","duration_ms"]},
      {"properties":{"type":{"const":"add_tag"},"name":{"type":"string","minLength":1}},"required":["type","name","from_frame_id","to_frame_id","direction"]},
      {"properties":{"type":{"const":"remove_tag"},"tag_id":{"type":"integer","minimum":1}},"required":["type","tag_id"]},
      {"properties":{"type":{"const":"rename_tag"},"tag_id":{"type":"integer","minimum":1},"name":{"type":"string","minLength":1}},"required":["type","tag_id","name"]},
      {"properties":{"type":{"const":"set_tag_range"},"tag_id":{"type":"integer","minimum":1}},"required":["type","tag_id","from_frame_id","to_frame_id"]},
      {"properties":{"type":{"const":"set_tag_direction"},"tag_id":{"type":"integer","minimum":1}},"required":["type","tag_id","direction"]},
      {"properties":{"type":{"const":"set_active_tag"}},"required":["type","tag_id"]},
      {"properties":{"type":{"const":"set_active_frame"}},"required":["type","frame_id"]},
      {"properties":{"type":{"const":"set_active_layer"}},"required":["type","layer_id"]}
    ]
  },
  "$defs": {
    "grid_index":{"type":"object","additionalProperties":false,"properties":{"x":{"type":"integer"},"y":{"type":"integer"}},"required":["x","y"]},
    "cel_position":{"type":"object","additionalProperties":false,"properties":{"layer_id":{"type":"integer","minimum":1},"frame_id":{"type":"integer","minimum":1}},"required":["layer_id","frame_id"]},
    "pixel_bounds":{"type":"object","additionalProperties":false,"properties":{"min_x":{"type":"integer"},"min_y":{"type":"integer"},"max_x":{"type":"integer"},"max_y":{"type":"integer"}},"required":["min_x","min_y","max_x","max_y"]},
    "pixel_transform":{"type":"object","additionalProperties":false,"properties":{"type":{"type":"string"},"dx":{"type":"integer"},"dy":{"type":"integer"},"factor":{"type":"integer","minimum":2,"maximum":8},"width":{"type":"integer","minimum":1,"maximum":4096},"height":{"type":"integer","minimum":1,"maximum":4096}},"oneOf":[
      {"properties":{"type":{"const":"translate"}},"required":["type","dx","dy"]},
      {"properties":{"type":{"const":"flip_horizontal"}},"required":["type"]},
      {"properties":{"type":{"const":"flip_vertical"}},"required":["type"]},
      {"properties":{"type":{"const":"rotate_clockwise"}},"required":["type"]},
      {"properties":{"type":{"const":"rotate_counter_clockwise"}},"required":["type"]},
      {"properties":{"type":{"const":"scale_integer"}},"required":["type","factor"]},
      {"properties":{"type":{"const":"resize_nearest"}},"required":["type","width","height"]}
    ]},
    "rgba":{"type":"object","additionalProperties":false,"properties":{"r":{"type":"number","minimum":0,"maximum":1},"g":{"type":"number","minimum":0,"maximum":1},"b":{"type":"number","minimum":0,"maximum":1},"a":{"type":"number","minimum":0,"maximum":1}},"required":["r","g","b","a"]},
    "pixel_change":{"type":"object","additionalProperties":false,"properties":{"index":{"$ref":"#/$defs/grid_index"},"color":{"$ref":"#/$defs/rgba"}},"required":["index","color"]}
  }
}"##;

#[cfg(test)]
mod tests {
    use super::{
        CEL_SUMMARY_LIMIT, EDIT_OP_JSON_SCHEMA, EditOp, PixelChange, ProjectSummary,
        SelectionSummary, apply_edit_ops,
    };
    use crate::document::{DocumentOp, apply_document_ops};
    use crate::grid::GridIndex;
    use crate::model::{
        BlendMode, CURRENT_SCHEMA_VERSION, CelPosition, LayerId, LayerKind, Project, Rgba,
        TagDirection,
    };
    use crate::transform::{PixelBounds, PixelTransform};

    #[test]
    fn summaries_use_global_collections_and_stable_ids() {
        let mut project = Project::new_square(20.0, 8, 8);
        project
            .ensure_current_cel()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 2 }, Rgba::WHITE);
        let summary = ProjectSummary::from_project(&project);
        assert_eq!(summary.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(summary.layers.len(), 1);
        assert_eq!(summary.frames.len(), 1);
        assert_eq!(summary.cels.len(), 1);
        assert_eq!(summary.cels[0].layer_id, project.active_layer_id);
    }

    #[test]
    fn edit_ops_target_stable_ids_and_create_sparse_cels() {
        let mut project = Project::new_square(20.0, 8, 8);
        let layer_id = project.add_layer("Glow");
        let frame_id = project.active_frame_id;
        let edited = apply_edit_ops(
            &project,
            &[EditOp::SetCelPixels {
                layer_id,
                frame_id,
                pixels: vec![PixelChange {
                    index: GridIndex { x: 1, y: 2 },
                    color: Rgba::new(0.2, 0.3, 0.4, 1.0),
                }],
            }],
        )
        .unwrap();
        assert!(
            edited
                .cel(layer_id, frame_id)
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 1, y: 2 })
        );
    }

    #[test]
    fn edit_ops_reject_out_of_bounds_pixels() {
        let project = Project::new_square(20.0, 8, 8);
        let error = apply_edit_ops(
            &project,
            &[EditOp::SetCelPixels {
                layer_id: project.active_layer_id,
                frame_id: project.active_frame_id,
                pixels: vec![PixelChange {
                    index: GridIndex { x: 99, y: 99 },
                    color: Rgba::BLACK,
                }],
            }],
        )
        .unwrap_err();
        assert!(error.contains("out of bounds"));
    }

    #[test]
    fn selection_summary_uses_active_cel_identity_and_nearby_pixels() {
        let mut project = Project::new_square(20.0, 8, 8);
        let layer_id = project.active_layer_id;
        let frame_id = project.active_frame_id;
        let cel = project.ensure_current_cel().unwrap();
        cel.pixels.insert(GridIndex { x: 2, y: 2 }, Rgba::WHITE);
        cel.pixels.insert(GridIndex { x: 4, y: 4 }, Rgba::BLACK);
        let summary = SelectionSummary::from_project_selection(
            &project,
            layer_id,
            frame_id,
            [GridIndex { x: 2, y: 2 }, GridIndex { x: 2, y: 3 }],
        );
        assert!(summary.active);
        assert_eq!(
            summary.active_cel_id,
            project.current_cel().map(|cel| cel.id)
        );
        assert_eq!(summary.pixels.len(), 1);
        assert_eq!(summary.nearby_pixels.len(), 1);
    }

    #[test]
    fn selection_summary_reports_timeline_range_in_stable_ids() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first_layer = project.active_layer_id;
        let first_frame = project.active_frame_id;
        let second_frame = project.add_frame(None, 100).unwrap();
        let second_layer = project.add_layer("Top");
        project
            .ensure_cel(second_layer, second_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);

        let summary = SelectionSummary::from_project_selections(
            &project,
            second_layer,
            second_frame,
            std::iter::empty(),
            [
                CelPosition {
                    layer_id: first_layer,
                    frame_id: first_frame,
                },
                CelPosition {
                    layer_id: first_layer,
                    frame_id: second_frame,
                },
                CelPosition {
                    layer_id: second_layer,
                    frame_id: first_frame,
                },
                CelPosition {
                    layer_id: second_layer,
                    frame_id: second_frame,
                },
            ],
        );

        assert!(!summary.active);
        assert!(summary.timeline.active);
        assert_eq!(summary.timeline.selected_cels, 4);
        assert_eq!(summary.timeline.populated_cels, 1);
        assert_eq!(summary.timeline.layer_ids, vec![first_layer, second_layer]);
        assert_eq!(summary.timeline.frame_ids, vec![first_frame, second_frame]);
        let bounds = summary.timeline.bounds.unwrap();
        assert_eq!((bounds.layer_span, bounds.frame_span), (2, 2));
        assert_eq!(bounds.first_layer_id, first_layer);
        assert_eq!(bounds.last_frame_id, second_frame);
    }

    #[test]
    fn edit_op_schema_is_valid_and_lists_all_v6_variants() {
        let schema: serde_json::Value = serde_json::from_str(EDIT_OP_JSON_SCHEMA).unwrap();
        assert_eq!(schema["items"]["oneOf"].as_array().unwrap().len(), 43);
        for name in [
            "resize_canvas",
            "set_foreground_color",
            "set_background_color",
            "rename_palette",
            "add_palette_color",
            "remove_palette_color",
            "replace_palette_color",
            "reorder_palette_color",
            "deduplicate_palette",
            "replace_palette",
            "set_cel_pixels",
            "erase_cel_pixels",
            "clear_cel",
            "move_cel",
            "transform_cel_pixels",
            "crop_canvas",
            "trim_canvas",
            "link_cel",
            "add_layer",
            "set_layer_locked",
            "set_layer_opacity",
            "set_layer_blend_mode",
            "set_layer_kind",
            "set_layer_parent",
            "add_frame",
            "reorder_frame",
            "add_tag",
            "set_tag_range",
            "set_tag_direction",
            "set_active_tag",
            "set_active_layer",
        ] {
            assert!(EDIT_OP_JSON_SCHEMA.contains(&format!("\"{name}\"")));
        }
    }

    #[test]
    fn palette_edit_ops_and_project_summary_share_document_semantics() {
        let project = Project::new_square(20.0, 8, 8);
        let color = Rgba::new(0.2, 0.4, 0.6, 0.8);
        let edited = apply_edit_ops(
            &project,
            &[
                EditOp::SetForegroundColor { color },
                EditOp::ReplacePalette {
                    name: "Agent Palette".to_string(),
                    colors: vec![color, Rgba::BLACK],
                },
                EditOp::ReorderPaletteColor {
                    from_position: 1,
                    position: 0,
                },
            ],
        )
        .unwrap();
        let summary = ProjectSummary::from_project(&edited);

        assert_eq!(summary.foreground_color, color);
        assert_eq!(summary.palette.name, "Agent Palette");
        assert_eq!(summary.palette.colors, vec![Rgba::BLACK, color]);
        assert_eq!(summary.background_color, Rgba::BLACK);
    }

    #[test]
    fn transform_edit_ops_use_stable_targets_and_document_semantics() {
        let mut project = Project::new_square(20.0, 6, 6);
        let target = CelPosition {
            layer_id: project.active_layer_id,
            frame_id: project.active_frame_id,
        };
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);
        let ops = vec![EditOp::TransformCelPixels {
            targets: vec![target],
            selection: vec![GridIndex { x: 1, y: 1 }],
            transform: PixelTransform::Translate { dx: 2, dy: 1 },
        }];

        let via_edit_ops = apply_edit_ops(&project, &ops).unwrap();
        let via_document_ops = apply_document_ops(
            &project,
            &[DocumentOp::TransformCelPixels {
                targets: vec![target],
                selection: vec![GridIndex { x: 1, y: 1 }],
                transform: PixelTransform::Translate { dx: 2, dy: 1 },
            }],
        )
        .unwrap();
        assert_eq!(via_edit_ops, via_document_ops);
        assert!(
            via_edit_ops
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 3, y: 2 })
        );

        let cropped = apply_edit_ops(
            &via_edit_ops,
            &[EditOp::CropCanvas {
                bounds: PixelBounds {
                    min_x: 2,
                    min_y: 1,
                    max_x: 4,
                    max_y: 3,
                },
            }],
        )
        .unwrap();
        assert_eq!((cropped.canvas_width, cropped.canvas_height), (3, 3));
    }

    #[test]
    fn layer_edit_ops_share_document_operations_and_summary_semantics() {
        let project = Project::new_square(20.0, 8, 8);
        let group_id = LayerId(4);
        let child_id = LayerId(5);
        let edit_ops = vec![
            EditOp::AddLayer {
                name: Some("Group".to_string()),
                position: None,
                kind: Some(LayerKind::Group),
                parent_id: None,
            },
            EditOp::AddLayer {
                name: Some("Child".to_string()),
                position: None,
                kind: Some(LayerKind::Paint),
                parent_id: Some(group_id),
            },
            EditOp::SetLayerOpacity {
                layer_id: group_id,
                opacity: 0.5,
            },
            EditOp::SetLayerBlendMode {
                layer_id: child_id,
                blend_mode: BlendMode::Multiply,
            },
            EditOp::SetLayerLocked {
                layer_id: group_id,
                locked: true,
            },
        ];
        let via_edit_ops = apply_edit_ops(&project, &edit_ops).unwrap();
        let document_ops = edit_ops
            .iter()
            .map(EditOp::to_document_op)
            .collect::<Vec<_>>();
        let via_document_ops = apply_document_ops(&project, &document_ops).unwrap();
        assert_eq!(via_edit_ops, via_document_ops);

        let summary = ProjectSummary::from_project(&via_edit_ops);
        let child = summary
            .layers
            .iter()
            .find(|layer| layer.layer_id == child_id)
            .unwrap();
        assert_eq!(child.kind, LayerKind::Paint);
        assert_eq!(child.blend_mode, BlendMode::Multiply);
        assert_eq!(child.parent_id, Some(group_id));
        assert_eq!(child.depth, 1);
        assert!(child.effective_locked);
    }

    #[test]
    fn edit_ops_and_project_summary_include_animation_tags() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first = project.active_frame_id;
        let last = project.add_frame(None, 120).unwrap();
        let tagged = apply_edit_ops(
            &project,
            &[EditOp::AddTag {
                name: "Run".to_string(),
                from_frame_id: first,
                to_frame_id: last,
                direction: TagDirection::PingPong,
            }],
        )
        .unwrap();
        let tag_id = tagged.tags[0].id;
        let summary = ProjectSummary::from_project(&tagged);

        assert_eq!(summary.active_tag_id, Some(tag_id));
        assert_eq!(summary.tags.len(), 1);
        assert_eq!(summary.tags[0].tag_id, tag_id);
        assert_eq!(summary.tags[0].direction, TagDirection::PingPong);

        let edited = apply_edit_ops(
            &tagged,
            &[
                EditOp::RenameTag {
                    tag_id,
                    name: "Run Reverse".to_string(),
                },
                EditOp::SetTagDirection {
                    tag_id,
                    direction: TagDirection::Reverse,
                },
            ],
        )
        .unwrap();
        assert_eq!(edited.tag(tag_id).unwrap().name, "Run Reverse");
        assert_eq!(edited.tag(tag_id).unwrap().direction, TagDirection::Reverse);
    }

    #[test]
    fn stable_id_pixel_target_survives_layer_reordering() {
        let mut project = Project::new_square(20.0, 8, 8);
        let original_layer = project.active_layer_id;
        let frame_id = project.active_frame_id;
        let other_layer = project.add_layer("Other");
        let edited = apply_edit_ops(
            &project,
            &[
                EditOp::ReorderLayer {
                    layer_id: original_layer,
                    position: 1,
                },
                EditOp::SetCelPixels {
                    layer_id: original_layer,
                    frame_id,
                    pixels: vec![PixelChange {
                        index: GridIndex { x: 3, y: 3 },
                        color: Rgba::WHITE,
                    }],
                },
            ],
        )
        .unwrap();
        assert_eq!(edited.layers[0].id, other_layer);
        assert!(
            edited
                .cel(original_layer, frame_id)
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 3, y: 3 })
        );
    }

    #[test]
    fn project_summary_limits_large_cel_lists() {
        let mut project = Project::new_square(20.0, 8, 8);
        let layer_id = project.active_layer_id;
        for _ in 0..CEL_SUMMARY_LIMIT {
            let frame_id = project.add_frame(None, 100).unwrap();
            project.ensure_cel(layer_id, frame_id).unwrap();
        }
        let summary = ProjectSummary::from_project(&project);
        assert_eq!(summary.cel_count, CEL_SUMMARY_LIMIT + 1);
        assert_eq!(summary.cels.len(), CEL_SUMMARY_LIMIT);
        assert!(summary.cels_truncated);
    }

    #[test]
    fn edit_ops_can_initialize_an_empty_canvas_before_drawing() {
        let project = Project::new_square(20.0, 0, 0);
        let layer_id = project.active_layer_id;
        let frame_id = project.active_frame_id;
        let edited = apply_edit_ops(
            &project,
            &[
                EditOp::ResizeCanvas {
                    canvas_width: 16,
                    canvas_height: 16,
                },
                EditOp::RenameLayer {
                    layer_id,
                    name: "Ball".to_string(),
                },
                EditOp::SetCelPixels {
                    layer_id,
                    frame_id,
                    pixels: vec![PixelChange {
                        index: GridIndex { x: 8, y: 8 },
                        color: Rgba::WHITE,
                    }],
                },
            ],
        )
        .unwrap();
        assert_eq!((edited.canvas_width, edited.canvas_height), (16, 16));
        assert_eq!(edited.layer(layer_id).unwrap().name, "Ball");
        assert!(
            edited
                .cel(layer_id, frame_id)
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 8, y: 8 })
        );
    }

    #[test]
    fn resize_canvas_rejects_invalid_sizes_and_destructive_shrinks() {
        let mut project = Project::new_square(20.0, 16, 16);
        project
            .ensure_current_cel()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 15, y: 15 }, Rgba::WHITE);

        let invalid_size = apply_edit_ops(
            &project,
            &[EditOp::ResizeCanvas {
                canvas_width: 0,
                canvas_height: 16,
            }],
        )
        .unwrap_err();
        assert!(invalid_size.contains("between 1 and 4096"));

        let destructive_shrink = apply_edit_ops(
            &project,
            &[EditOp::ResizeCanvas {
                canvas_width: 8,
                canvas_height: 8,
            }],
        )
        .unwrap_err();
        assert!(destructive_shrink.contains("out-of-bounds"));
        assert_eq!((project.canvas_width, project.canvas_height), (16, 16));
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 15, y: 15 })
        );
    }
}
