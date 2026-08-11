use crate::grid::GridIndex;
use crate::model::{CelPosition, GridConfig, Project};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_TRANSFORM_TARGETS: usize = 256;
const MAX_TRANSFORM_SELECTION: usize = 1_048_576;
const MAX_GENERATED_PIXELS: usize = 4_194_304;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PixelBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl PixelBounds {
    pub fn from_indices(indices: impl IntoIterator<Item = GridIndex>) -> Option<Self> {
        let mut indices = indices.into_iter();
        let first = indices.next()?;
        let mut bounds = Self {
            min_x: first.x,
            min_y: first.y,
            max_x: first.x,
            max_y: first.y,
        };
        for index in indices {
            bounds.min_x = bounds.min_x.min(index.x);
            bounds.min_y = bounds.min_y.min(index.y);
            bounds.max_x = bounds.max_x.max(index.x);
            bounds.max_y = bounds.max_y.max(index.y);
        }
        Some(bounds)
    }

    pub fn width(self) -> Result<u32, String> {
        u32::try_from(i64::from(self.max_x) - i64::from(self.min_x) + 1)
            .map_err(|_| "pixel bounds width overflowed".to_string())
    }

    pub fn height(self) -> Result<u32, String> {
        u32::try_from(i64::from(self.max_y) - i64::from(self.min_y) + 1)
            .map_err(|_| "pixel bounds height overflowed".to_string())
    }

    pub fn contains(self, index: GridIndex) -> bool {
        index.x >= self.min_x
            && index.x <= self.max_x
            && index.y >= self.min_y
            && index.y <= self.max_y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PixelTransform {
    Translate { dx: i32, dy: i32 },
    FlipHorizontal,
    FlipVertical,
    RotateClockwise,
    RotateCounterClockwise,
    ScaleInteger { factor: u8 },
}

pub fn transform_indices(
    indices: impl IntoIterator<Item = GridIndex>,
    transform: PixelTransform,
    bounds: PixelBounds,
) -> Result<HashSet<GridIndex>, String> {
    let mut transformed = HashSet::new();
    for index in indices {
        transformed.extend(transformed_positions(index, transform, bounds)?);
        if transformed.len() > MAX_TRANSFORM_SELECTION {
            return Err("transformed selection is too large".to_string());
        }
    }
    Ok(transformed)
}

pub fn transform_cel_pixels(
    project: &mut Project,
    targets: &[CelPosition],
    selection: &[GridIndex],
    transform: PixelTransform,
) -> Result<(), String> {
    if targets.is_empty() {
        return Err("pixel transform requires at least one target cel".to_string());
    }
    if targets.len() > MAX_TRANSFORM_TARGETS {
        return Err(format!(
            "too many pixel transform targets: {} > {MAX_TRANSFORM_TARGETS}",
            targets.len()
        ));
    }
    if selection.len() > MAX_TRANSFORM_SELECTION {
        return Err(format!(
            "pixel transform selection is too large: {} > {MAX_TRANSFORM_SELECTION}",
            selection.len()
        ));
    }
    validate_transform(transform)?;
    if !matches!(project.grid_config, GridConfig::Square { .. })
        && !matches!(transform, PixelTransform::Translate { .. })
    {
        return Err("flip, rotate, and scale transforms require a square grid".to_string());
    }

    let mut unique_targets = HashSet::new();
    for target in targets {
        if !unique_targets.insert(*target) {
            return Err(format!(
                "pixel transform contains duplicate target layer {}, frame {}",
                target.layer_id, target.frame_id
            ));
        }
        let layer = project
            .layer(target.layer_id)
            .ok_or_else(|| format!("unknown layer_id: {}", target.layer_id))?;
        if !layer.kind.supports_cels() {
            return Err(format!(
                "group layer {} cannot contain cels",
                target.layer_id
            ));
        }
        if project.layer_is_effectively_locked(target.layer_id)? {
            return Err(format!("layer {} is locked", target.layer_id));
        }
        if project.frame(target.frame_id).is_none() {
            return Err(format!("unknown frame_id: {}", target.frame_id));
        }
        if project
            .cel(target.layer_id, target.frame_id)
            .is_some_and(|cel| cel.linked_cel_id.is_some())
        {
            return Err(format!(
                "linked cel at layer {}, frame {} must be unlinked before transforming pixels",
                target.layer_id, target.frame_id
            ));
        }
    }

    let selection_set = selection.iter().copied().collect::<HashSet<_>>();
    if let Some(index) = selection_set
        .iter()
        .find(|index| !project.is_index_in_bounds(**index))
    {
        return Err(format!(
            "pixel transform selection is out of bounds: ({}, {})",
            index.x, index.y
        ));
    }

    let bounds = if selection_set.is_empty() {
        let world_pixels = targets.iter().flat_map(|target| {
            project
                .cel(target.layer_id, target.frame_id)
                .into_iter()
                .flat_map(|cel| {
                    cel.pixels
                        .keys()
                        .filter_map(|index| world_index(*index, cel.offset).ok())
                })
        });
        PixelBounds::from_indices(world_pixels)
    } else {
        PixelBounds::from_indices(selection_set.iter().copied())
    };
    let Some(bounds) = bounds else {
        return Ok(());
    };

    let mut updates = Vec::new();
    let mut generated_pixels = 0usize;
    for target in targets {
        let Some(cel) = project.cel(target.layer_id, target.frame_id) else {
            continue;
        };
        let mut next_pixels = cel.pixels.clone();
        let mut transformed_pixels = Vec::new();
        for (&local_index, &color) in &cel.pixels {
            let world = world_index(local_index, cel.offset)?;
            if selection_set.is_empty() || selection_set.contains(&world) {
                next_pixels.remove(&local_index);
                for destination in transformed_positions(world, transform, bounds)? {
                    if !project.is_index_in_bounds(destination) {
                        return Err(format!(
                            "pixel transform moves content out of bounds: ({}, {})",
                            destination.x, destination.y
                        ));
                    }
                    transformed_pixels
                        .push((local_index_from_world(destination, cel.offset)?, color));
                    generated_pixels += 1;
                    if generated_pixels > MAX_GENERATED_PIXELS {
                        return Err("pixel transform generates too many pixels".to_string());
                    }
                }
            }
        }
        for (index, color) in transformed_pixels {
            next_pixels.insert(index, color);
        }
        updates.push((*target, next_pixels));
    }

    for (target, pixels) in updates {
        if let Some(cel) = project.cel_mut(target.layer_id, target.frame_id) {
            cel.pixels = pixels;
        }
    }
    Ok(())
}

pub fn crop_canvas(project: &mut Project, bounds: PixelBounds) -> Result<(), String> {
    if !matches!(project.grid_config, GridConfig::Square { .. }) {
        return Err("canvas crop currently requires a square grid".to_string());
    }
    let width = bounds.width()?;
    let height = bounds.height()?;
    if width == 0 || height == 0 {
        return Err("canvas crop bounds cannot be empty".to_string());
    }
    let canvas_max_x = i32::try_from(project.canvas_width)
        .map_err(|_| "canvas width exceeds supported crop coordinates".to_string())?
        - 1;
    let canvas_max_y = i32::try_from(project.canvas_height)
        .map_err(|_| "canvas height exceeds supported crop coordinates".to_string())?
        - 1;
    if bounds.min_x < 0
        || bounds.min_y < 0
        || bounds.max_x > canvas_max_x
        || bounds.max_y > canvas_max_y
    {
        return Err("canvas crop bounds must stay inside the current canvas".to_string());
    }

    materialize_all_links(project)?;
    for cel in &mut project.cels {
        let old_offset = cel.offset;
        cel.pixels.retain(|index, _| {
            world_index(*index, old_offset).is_ok_and(|world| bounds.contains(world))
        });
        cel.offset = GridIndex {
            x: old_offset.x - bounds.min_x,
            y: old_offset.y - bounds.min_y,
        };
    }
    project.canvas_width = width;
    project.canvas_height = height;
    project.symmetry_x.position =
        (project.symmetry_x.position - bounds.min_x as f32).clamp(0.0, width as f32);
    project.symmetry_y.position =
        (project.symmetry_y.position - bounds.min_y as f32).clamp(0.0, height as f32);
    Ok(())
}

pub fn trim_canvas(project: &mut Project) -> Result<PixelBounds, String> {
    if !matches!(project.grid_config, GridConfig::Square { .. }) {
        return Err("canvas trim currently requires a square grid".to_string());
    }
    let mut occupied = Vec::new();
    for cel in &project.cels {
        let resolved = project.resolved_cel(cel)?;
        for (&index, color) in &resolved.pixels {
            if color.a > 0.0 {
                occupied.push(world_index(index, cel.offset)?);
            }
        }
    }
    let bounds = PixelBounds::from_indices(occupied)
        .ok_or_else(|| "cannot trim a project without visible pixels".to_string())?;
    crop_canvas(project, bounds)?;
    Ok(bounds)
}

fn validate_transform(transform: PixelTransform) -> Result<(), String> {
    match transform {
        PixelTransform::Translate { dx: 0, dy: 0 } => {
            Err("pixel translation must move at least one cell".to_string())
        }
        PixelTransform::ScaleInteger { factor } if !(2..=8).contains(&factor) => {
            Err("pixel scale factor must be between 2 and 8".to_string())
        }
        _ => Ok(()),
    }
}

fn transformed_positions(
    index: GridIndex,
    transform: PixelTransform,
    bounds: PixelBounds,
) -> Result<Vec<GridIndex>, String> {
    let relative_x = index.x - bounds.min_x;
    let relative_y = index.y - bounds.min_y;
    let width = i32::try_from(bounds.width()?)
        .map_err(|_| "pixel transform width exceeds supported coordinates".to_string())?;
    let height = i32::try_from(bounds.height()?)
        .map_err(|_| "pixel transform height exceeds supported coordinates".to_string())?;
    let position = match transform {
        PixelTransform::Translate { dx, dy } => GridIndex {
            x: index
                .x
                .checked_add(dx)
                .ok_or_else(|| "pixel translation x coordinate overflowed".to_string())?,
            y: index
                .y
                .checked_add(dy)
                .ok_or_else(|| "pixel translation y coordinate overflowed".to_string())?,
        },
        PixelTransform::FlipHorizontal => GridIndex {
            x: bounds.max_x - relative_x,
            y: index.y,
        },
        PixelTransform::FlipVertical => GridIndex {
            x: index.x,
            y: bounds.max_y - relative_y,
        },
        PixelTransform::RotateClockwise => GridIndex {
            x: bounds.min_x + relative_y,
            y: bounds.min_y + width - 1 - relative_x,
        },
        PixelTransform::RotateCounterClockwise => GridIndex {
            x: bounds.min_x + height - 1 - relative_y,
            y: bounds.min_y + relative_x,
        },
        PixelTransform::ScaleInteger { factor } => {
            let factor = i32::from(factor);
            let base_x = bounds
                .min_x
                .checked_add(
                    relative_x
                        .checked_mul(factor)
                        .ok_or_else(|| "pixel scale x coordinate overflowed".to_string())?,
                )
                .ok_or_else(|| "pixel scale x coordinate overflowed".to_string())?;
            let base_y = bounds
                .min_y
                .checked_add(
                    relative_y
                        .checked_mul(factor)
                        .ok_or_else(|| "pixel scale y coordinate overflowed".to_string())?,
                )
                .ok_or_else(|| "pixel scale y coordinate overflowed".to_string())?;
            let mut positions = Vec::with_capacity((factor * factor) as usize);
            for offset_y in 0..factor {
                for offset_x in 0..factor {
                    positions.push(GridIndex {
                        x: base_x + offset_x,
                        y: base_y + offset_y,
                    });
                }
            }
            return Ok(positions);
        }
    };
    Ok(vec![position])
}

fn materialize_all_links(project: &mut Project) -> Result<(), String> {
    let updates = project
        .cels
        .iter()
        .filter(|cel| cel.linked_cel_id.is_some())
        .map(|cel| Ok((cel.id, project.resolved_cel(cel)?.pixels.clone())))
        .collect::<Result<Vec<_>, String>>()?;
    for (cel_id, pixels) in updates {
        let cel = project
            .cel_by_id_mut(cel_id)
            .expect("linked cel was collected from the project");
        cel.pixels = pixels;
        cel.linked_cel_id = None;
    }
    Ok(())
}

fn world_index(local: GridIndex, offset: GridIndex) -> Result<GridIndex, String> {
    Ok(GridIndex {
        x: local
            .x
            .checked_add(offset.x)
            .ok_or_else(|| "pixel world x coordinate overflowed".to_string())?,
        y: local
            .y
            .checked_add(offset.y)
            .ok_or_else(|| "pixel world y coordinate overflowed".to_string())?,
    })
}

fn local_index_from_world(world: GridIndex, offset: GridIndex) -> Result<GridIndex, String> {
    Ok(GridIndex {
        x: world
            .x
            .checked_sub(offset.x)
            .ok_or_else(|| "pixel local x coordinate overflowed".to_string())?,
        y: world
            .y
            .checked_sub(offset.y)
            .ok_or_else(|| "pixel local y coordinate overflowed".to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::{PixelBounds, PixelTransform, crop_canvas, transform_cel_pixels, trim_canvas};
    use crate::grid::GridIndex;
    use crate::model::{CelPosition, Project, Rgba};

    #[test]
    fn transform_uses_world_selection_and_preserves_unselected_pixels() {
        let mut project = Project::new_square(1.0, 6, 4);
        let layer_id = project.active_layer_id;
        let frame_id = project.active_frame_id;
        let cel = project.current_cel_mut().unwrap();
        cel.offset = GridIndex { x: 1, y: 0 };
        cel.pixels.insert(GridIndex { x: 0, y: 1 }, Rgba::WHITE);
        cel.pixels.insert(GridIndex { x: 1, y: 1 }, Rgba::BLACK);
        cel.pixels.insert(GridIndex { x: 4, y: 1 }, Rgba::WHITE);

        transform_cel_pixels(
            &mut project,
            &[CelPosition { layer_id, frame_id }],
            &[GridIndex { x: 1, y: 1 }, GridIndex { x: 2, y: 1 }],
            PixelTransform::FlipHorizontal,
        )
        .unwrap();

        let cel = project.current_cel().unwrap();
        assert_eq!(cel.pixels[&GridIndex { x: 0, y: 1 }], Rgba::BLACK);
        assert_eq!(cel.pixels[&GridIndex { x: 1, y: 1 }], Rgba::WHITE);
        assert!(cel.pixels.contains_key(&GridIndex { x: 4, y: 1 }));
    }

    #[test]
    fn rotate_and_integer_scale_are_deterministic() {
        let mut project = Project::new_square(1.0, 8, 8);
        let target = CelPosition {
            layer_id: project.active_layer_id,
            frame_id: project.active_frame_id,
        };
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 2 }, Rgba::WHITE);
        let selection = [
            GridIndex { x: 1, y: 1 },
            GridIndex { x: 2, y: 1 },
            GridIndex { x: 1, y: 2 },
            GridIndex { x: 2, y: 2 },
        ];

        transform_cel_pixels(
            &mut project,
            &[target],
            &selection,
            PixelTransform::RotateClockwise,
        )
        .unwrap();
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 2, y: 2 })
        );

        transform_cel_pixels(
            &mut project,
            &[target],
            &[],
            PixelTransform::ScaleInteger { factor: 2 },
        )
        .unwrap();
        assert_eq!(project.current_cel().unwrap().pixels.len(), 4);
    }

    #[test]
    fn crop_and_trim_update_all_cels_and_materialize_links() {
        let mut project = Project::new_square(1.0, 6, 5);
        let layer_id = project.active_layer_id;
        let first = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 1 }, Rgba::WHITE);
        let source_id = project.current_cel().unwrap().id;
        let second = project.add_frame(None, 100).unwrap();
        let linked = project.ensure_cel(layer_id, second).unwrap();
        linked.offset = GridIndex { x: 1, y: 1 };
        linked.linked_cel_id = Some(source_id);

        crop_canvas(
            &mut project,
            PixelBounds {
                min_x: 1,
                min_y: 1,
                max_x: 4,
                max_y: 3,
            },
        )
        .unwrap();
        assert_eq!((project.canvas_width, project.canvas_height), (4, 3));
        assert_eq!(
            project.cel(layer_id, first).unwrap().offset,
            GridIndex { x: -1, y: -1 }
        );
        assert_eq!(project.cel(layer_id, second).unwrap().linked_cel_id, None);

        trim_canvas(&mut project).unwrap();
        assert_eq!((project.canvas_width, project.canvas_height), (2, 2));
        project.validate().unwrap();
    }

    #[test]
    fn non_square_grids_reject_square_only_pixel_transforms() {
        let mut project =
            Project::new_with_grid(crate::model::GridConfig::Hexagon { cell_size: 1.0 }, 8, 8);
        let target = CelPosition {
            layer_id: project.active_layer_id,
            frame_id: project.active_frame_id,
        };
        let error = transform_cel_pixels(
            &mut project,
            &[target],
            &[],
            PixelTransform::RotateClockwise,
        )
        .unwrap_err();

        assert!(error.contains("square grid"));
    }
}
