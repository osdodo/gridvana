use super::{ShapeDraft, ShapeKind};
use crate::i18n::tr;
use gridvana_core::grid::GridIndex;
use gridvana_core::model::{GridConfig, Project, Rgba};
use std::collections::{HashMap, HashSet};

const TOOL_SIZE_CURVE: [u8; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 16];

pub(super) fn min_selection_index(indices: &HashSet<GridIndex>) -> Option<GridIndex> {
    let mut indices = indices.iter().copied();
    let first = indices.next()?;
    Some(indices.fold(first, |minimum, index| GridIndex {
        x: minimum.x.min(index.x),
        y: minimum.y.min(index.y),
    }))
}

pub(super) fn selection_pixels_in_box(
    project: &Project,
    start: GridIndex,
    end: GridIndex,
) -> HashSet<GridIndex> {
    let grid = project.grid_config.create_system();
    let start_center = grid.cell_center(start);
    let end_center = grid.cell_center(end);

    let min_x = start_center.x.min(end_center.x);
    let max_x = start_center.x.max(end_center.x);
    let min_y = start_center.y.min(end_center.y);
    let max_y = start_center.y.max(end_center.y);

    project
        .canvas_grid_indices()
        .into_iter()
        .filter(|index| {
            let center = grid.cell_center(*index);
            center.x >= min_x && center.x <= max_x && center.y >= min_y && center.y <= max_y
        })
        .collect()
}

pub(super) fn magic_wand_indices(project: &Project, start: GridIndex) -> HashSet<GridIndex> {
    let pixels = current_cel_world_pixels(project);
    let target_color = pixels.get(&start).copied();
    let grid = project.grid_config.create_system();
    let mut selected = HashSet::new();
    let mut pending = vec![start];
    while let Some(index) = pending.pop() {
        if !selected.insert(index) {
            continue;
        }
        for neighbor in grid.neighbors(index) {
            if project.is_index_in_bounds(neighbor)
                && pixels.get(&neighbor).copied() == target_color
                && !selected.contains(&neighbor)
            {
                pending.push(neighbor);
            }
        }
    }
    selected
}

pub(super) fn same_color_indices(project: &Project, start: GridIndex) -> HashSet<GridIndex> {
    let pixels = current_cel_world_pixels(project);
    let target_color = pixels.get(&start).copied();
    project
        .canvas_grid_indices()
        .into_iter()
        .filter(|index| pixels.get(index).copied() == target_color)
        .collect()
}

pub(super) fn current_cel_world_pixels(project: &Project) -> HashMap<GridIndex, Rgba> {
    let Some(destination) = project.current_cel() else {
        return HashMap::new();
    };
    let Ok(source) = project.resolved_cel(destination) else {
        return HashMap::new();
    };
    source
        .pixels
        .iter()
        .filter_map(|(index, color)| {
            Some((
                GridIndex {
                    x: index.x.checked_add(destination.offset.x)?,
                    y: index.y.checked_add(destination.offset.y)?,
                },
                *color,
            ))
        })
        .collect()
}

pub(super) fn effective_tool_span(size: u8) -> u8 {
    let index = size.saturating_sub(1) as usize;
    TOOL_SIZE_CURVE
        .get(index)
        .copied()
        .unwrap_or(*TOOL_SIZE_CURVE.last().unwrap_or(&1))
}

pub(super) fn tool_size_display(config: GridConfig, size: u8) -> String {
    let span = effective_tool_span(size);
    match config {
        GridConfig::Square { .. } => format!(" · {span}×{span}"),
        GridConfig::Hexagon { .. } | GridConfig::Triangle { .. } => {
            format!(" · {} {span}", tr("span", "跨度"))
        }
    }
}

pub(super) fn radial_indices(project: &Project, start: GridIndex, size: u8) -> Vec<GridIndex> {
    if !project.is_index_in_bounds(start) {
        return Vec::new();
    }

    let size = effective_tool_span(size);

    if matches!(project.grid_config, GridConfig::Square { .. }) {
        let size = size.max(1) as i32;
        let start_x = start.x - ((size - 1) / 2);
        let start_y = start.y - ((size - 1) / 2);

        let mut indices = Vec::with_capacity((size * size) as usize);

        for x in start_x..(start_x + size) {
            for y in start_y..(start_y + size) {
                let index = GridIndex { x, y };
                if project.is_index_in_bounds(index) {
                    indices.push(index);
                }
            }
        }

        indices.sort_by_key(|index| (index.x, index.y));
        return indices;
    }

    let grid = project.grid_config.create_system();
    let center = grid.cell_center(start);
    let base_spacing = grid
        .neighbors(start)
        .into_iter()
        .filter(|index| project.is_index_in_bounds(*index))
        .map(|index| {
            let neighbor_center = grid.cell_center(index);
            let dx = neighbor_center.x - center.x;
            let dy = neighbor_center.y - center.y;
            (dx * dx + dy * dy).sqrt()
        })
        .filter(|distance| *distance > f32::EPSILON)
        .fold(f32::INFINITY, f32::min);

    let fallback_spacing = grid_cell_size(project.grid_config).max(1.0);
    let base_spacing = if base_spacing.is_finite() {
        base_spacing
    } else {
        fallback_spacing
    };
    let radius = (size as f32 - 0.5).max(0.0) * base_spacing;
    let radius_squared = radius * radius;

    let mut indices = project
        .canvas_grid_indices()
        .into_iter()
        .filter(|index| {
            let cell_center = grid.cell_center(*index);
            let dx = cell_center.x - center.x;
            let dy = cell_center.y - center.y;
            (dx * dx + dy * dy) <= radius_squared
        })
        .collect::<Vec<_>>();
    indices.sort_by_key(|index| (index.x, index.y));
    indices
}

pub(super) fn parse_canvas_size(value: &str, fallback: u32) -> u32 {
    let parsed = value.trim().parse::<u32>().unwrap_or(fallback);
    parsed.clamp(1, 4096)
}

pub(super) fn grid_cell_size(config: GridConfig) -> f32 {
    match config {
        GridConfig::Square { cell_size } => cell_size,
        GridConfig::Hexagon { cell_size } => cell_size,
        GridConfig::Triangle { cell_size } => cell_size,
    }
}

pub(super) fn constrained_shape_end_index(
    project: &Project,
    shape: ShapeDraft,
    shift_pressed: bool,
) -> GridIndex {
    if !shift_pressed {
        return shape.current;
    }

    let grid = project.grid_config.create_system();
    let start_center = grid.cell_center(shape.start);
    let current_center = grid.cell_center(shape.current);

    let dx = current_center.x - start_center.x;
    let dy = current_center.y - start_center.y;

    if dx.abs() <= f32::EPSILON && dy.abs() <= f32::EPSILON {
        return shape.current;
    }

    let target = match shape.kind {
        ShapeKind::Rectangle
        | ShapeKind::RectangleHollow
        | ShapeKind::Circle
        | ShapeKind::CircleHollow => {
            let side = dx.abs().max(dy.abs());
            let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
            let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };

            gridvana_core::grid::Point::new(
                start_center.x + sign_x * side,
                start_center.y + sign_y * side,
            )
        }
        ShapeKind::Line => {
            let angle = dy.atan2(dx);
            let snap_step = std::f32::consts::FRAC_PI_4;
            let snapped_angle = (angle / snap_step).round() * snap_step;
            let length = (dx * dx + dy * dy).sqrt();

            gridvana_core::grid::Point::new(
                start_center.x + length * snapped_angle.cos(),
                start_center.y + length * snapped_angle.sin(),
            )
        }
    };

    if let Some(index) = grid.world_to_grid(target)
        && project.is_index_in_bounds(index)
    {
        index
    } else {
        shape.current
    }
}

pub(super) fn rectangle_shape_indices(
    project: &Project,
    start: GridIndex,
    end: GridIndex,
    filled: bool,
) -> Vec<GridIndex> {
    let grid = project.grid_config.create_system();
    let start_center = grid.cell_center(start);
    let end_center = grid.cell_center(end);

    let min_x = start_center.x.min(end_center.x);
    let max_x = start_center.x.max(end_center.x);
    let min_y = start_center.y.min(end_center.y);
    let max_y = start_center.y.max(end_center.y);

    let border_thickness = (grid_cell_size(project.grid_config) * 0.5).max(0.5);
    let inner_min_x = min_x + border_thickness;
    let inner_max_x = max_x - border_thickness;
    let inner_min_y = min_y + border_thickness;
    let inner_max_y = max_y - border_thickness;

    canvas_grid_indices(project)
        .into_iter()
        .filter(|index| {
            let center = grid.cell_center(*index);
            let is_inside_outer =
                center.x >= min_x && center.x <= max_x && center.y >= min_y && center.y <= max_y;

            if filled || !is_inside_outer {
                return is_inside_outer;
            }

            if inner_min_x > inner_max_x || inner_min_y > inner_max_y {
                return true;
            }

            let is_inside_inner = center.x >= inner_min_x
                && center.x <= inner_max_x
                && center.y >= inner_min_y
                && center.y <= inner_max_y;

            !is_inside_inner
        })
        .collect()
}

pub(super) fn ellipse_shape_indices(
    project: &Project,
    start: GridIndex,
    end: GridIndex,
    filled: bool,
) -> Vec<GridIndex> {
    let grid = project.grid_config.create_system();
    let start_center = grid.cell_center(start);
    let end_center = grid.cell_center(end);

    let center_x = (start_center.x + end_center.x) * 0.5;
    let center_y = (start_center.y + end_center.y) * 0.5;

    let mut radius_x = (start_center.x - end_center.x).abs() * 0.5;
    let mut radius_y = (start_center.y - end_center.y).abs() * 0.5;

    let fallback_radius = (grid_cell_size(project.grid_config) * 0.25).max(0.5);
    if radius_x <= f32::EPSILON {
        radius_x = fallback_radius;
    }
    if radius_y <= f32::EPSILON {
        radius_y = fallback_radius;
    }

    let border_thickness = (grid_cell_size(project.grid_config) * 0.5).max(0.5);
    let inner_radius_x = (radius_x - border_thickness).max(0.0);
    let inner_radius_y = (radius_y - border_thickness).max(0.0);

    canvas_grid_indices(project)
        .into_iter()
        .filter(|index| {
            let center = grid.cell_center(*index);
            let nx = (center.x - center_x) / radius_x;
            let ny = (center.y - center_y) / radius_y;
            let is_inside_outer = (nx * nx + ny * ny) <= 1.0;

            if filled || !is_inside_outer {
                return is_inside_outer;
            }

            if inner_radius_x <= f32::EPSILON || inner_radius_y <= f32::EPSILON {
                return true;
            }

            let inner_nx = (center.x - center_x) / inner_radius_x;
            let inner_ny = (center.y - center_y) / inner_radius_y;
            let is_inside_inner = (inner_nx * inner_nx + inner_ny * inner_ny) <= 1.0;
            !is_inside_inner
        })
        .collect()
}

pub(super) fn line_shape_indices(
    project: &Project,
    start: GridIndex,
    end: GridIndex,
) -> Vec<GridIndex> {
    let grid = project.grid_config.create_system();
    let start_center = grid.cell_center(start);
    let end_center = grid.cell_center(end);

    let dx = end_center.x - start_center.x;
    let dy = end_center.y - start_center.y;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance <= f32::EPSILON {
        return vec![start];
    }

    let step = (grid_cell_size(project.grid_config) * 0.2).max(0.5);
    let step_count = ((distance / step).ceil() as usize).max(1);

    let mut visited = HashSet::new();
    let mut indices = Vec::new();

    for i in 0..=step_count {
        let t = i as f32 / step_count as f32;
        let point =
            gridvana_core::grid::Point::new(start_center.x + dx * t, start_center.y + dy * t);

        if let Some(index) = grid.world_to_grid(point)
            && project.is_index_in_bounds(index)
            && visited.insert(index)
        {
            indices.push(index);
        }
    }

    for endpoint in [start, end] {
        if project.is_index_in_bounds(endpoint) && visited.insert(endpoint) {
            indices.push(endpoint);
        }
    }

    indices
}

fn canvas_grid_indices(project: &Project) -> Vec<GridIndex> {
    project.canvas_grid_indices()
}

#[cfg(test)]
mod tests {
    use super::{
        current_cel_world_pixels, magic_wand_indices, min_selection_index, same_color_indices,
    };
    use gridvana_core::grid::GridIndex;
    use gridvana_core::model::{Project, Rgba};
    use std::collections::HashSet;

    #[test]
    fn selection_anchor_uses_the_independent_minimum_coordinates() {
        let selection = HashSet::from([
            GridIndex { x: 5, y: 1 },
            GridIndex { x: 2, y: 4 },
            GridIndex { x: 3, y: 3 },
        ]);

        assert_eq!(
            min_selection_index(&selection),
            Some(GridIndex { x: 2, y: 1 })
        );
    }

    #[test]
    fn magic_wand_and_color_selection_include_transparent_cells() {
        let mut project = Project::new_square(1.0, 3, 2);
        let cel = project.current_cel_mut().unwrap();
        cel.pixels.insert(GridIndex { x: 1, y: 0 }, Rgba::WHITE);
        cel.pixels.insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);

        let contiguous = magic_wand_indices(&project, GridIndex { x: 0, y: 0 });
        assert_eq!(
            contiguous,
            HashSet::from([GridIndex { x: 0, y: 0 }, GridIndex { x: 0, y: 1 }])
        );

        let transparent = same_color_indices(&project, GridIndex { x: 2, y: 0 });
        assert_eq!(transparent.len(), 4);
        assert!(!transparent.contains(&GridIndex { x: 1, y: 0 }));
    }

    #[test]
    fn selection_reads_resolved_pixels_at_the_destination_offset() {
        let mut project = Project::new_square(1.0, 4, 3);
        let cel = project.current_cel_mut().unwrap();
        cel.offset = GridIndex { x: 2, y: 1 };
        cel.pixels.insert(GridIndex { x: 0, y: 0 }, Rgba::BLACK);

        let pixels = current_cel_world_pixels(&project);
        assert_eq!(pixels.get(&GridIndex { x: 2, y: 1 }), Some(&Rgba::BLACK));
        assert_eq!(pixels.len(), 1);
    }
}
