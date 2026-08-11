use crate::grid::GridIndex;
use crate::model::{BlendMode, FrameId, LayerId, LayerKind, Project, Rgba};
use std::collections::HashMap;

pub type CompositedCells = HashMap<GridIndex, Rgba>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositePurpose {
    Editor,
    Export,
}

pub fn composite_frame_cells(
    project: &Project,
    frame_id: FrameId,
    purpose: CompositePurpose,
) -> Result<CompositedCells, String> {
    if project.frame(frame_id).is_none() {
        return Err(format!("unknown frame_id: {frame_id}"));
    }
    composite_children(project, frame_id, None, purpose)
}

pub fn composite_layer_cells(
    project: &Project,
    frame_id: FrameId,
    layer_id: LayerId,
    purpose: CompositePurpose,
) -> Result<CompositedCells, String> {
    let layer = project
        .layer(layer_id)
        .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
    if !project.layer_is_effectively_visible(layer_id)?
        || (purpose == CompositePurpose::Export && layer.kind == LayerKind::Reference)
    {
        return Ok(CompositedCells::new());
    }
    let source = if layer.kind == LayerKind::Group {
        composite_children(project, frame_id, Some(layer.id), purpose)?
    } else {
        raster_layer_cells(project, layer.id, frame_id)?
    };
    let mut target = CompositedCells::new();
    composite_image(&mut target, source, layer.blend_mode, layer.opacity);

    let mut parent_id = layer.parent_id;
    while let Some(id) = parent_id {
        let parent = project
            .layer(id)
            .ok_or_else(|| format!("layer {layer_id} has unknown parent {id}"))?;
        for color in target.values_mut() {
            color.a = (color.a * parent.opacity).clamp(0.0, 1.0);
        }
        parent_id = parent.parent_id;
    }
    Ok(target)
}

fn composite_children(
    project: &Project,
    frame_id: FrameId,
    parent_id: Option<LayerId>,
    purpose: CompositePurpose,
) -> Result<CompositedCells, String> {
    let mut target = CompositedCells::new();
    for layer in project
        .layers
        .iter()
        .filter(|layer| layer.parent_id == parent_id)
    {
        if !layer.visible {
            continue;
        }
        if purpose == CompositePurpose::Export && layer.kind == LayerKind::Reference {
            continue;
        }

        let source = if layer.kind == LayerKind::Group {
            composite_children(project, frame_id, Some(layer.id), purpose)?
        } else {
            raster_layer_cells(project, layer.id, frame_id)?
        };
        composite_image(&mut target, source, layer.blend_mode, layer.opacity);
    }
    Ok(target)
}

fn raster_layer_cells(
    project: &Project,
    layer_id: LayerId,
    frame_id: FrameId,
) -> Result<CompositedCells, String> {
    let Some(destination) = project.cel(layer_id, frame_id) else {
        return Ok(CompositedCells::new());
    };
    let source = project.resolved_cel(destination)?;
    Ok(source
        .pixels
        .iter()
        .map(|(index, color)| {
            (
                GridIndex {
                    x: index.x.saturating_add(destination.offset.x),
                    y: index.y.saturating_add(destination.offset.y),
                },
                *color,
            )
        })
        .collect())
}

fn composite_image(
    target: &mut CompositedCells,
    source: CompositedCells,
    blend_mode: BlendMode,
    opacity: f32,
) {
    for (index, source_color) in source {
        let output = blend_colors(
            target
                .get(&index)
                .copied()
                .unwrap_or(Rgba::new(0.0, 0.0, 0.0, 0.0)),
            source_color,
            blend_mode,
            opacity,
        );
        if output.a <= 0.0 {
            target.remove(&index);
        } else {
            target.insert(index, output);
        }
    }
}

pub fn blend_colors(dst: Rgba, src: Rgba, blend_mode: BlendMode, opacity: f32) -> Rgba {
    let source_alpha = (src.a * opacity).clamp(0.0, 1.0);
    let backdrop_alpha = dst.a.clamp(0.0, 1.0);
    let output_alpha = source_alpha + backdrop_alpha * (1.0 - source_alpha);
    if output_alpha <= 0.0 {
        return Rgba::new(0.0, 0.0, 0.0, 0.0);
    }

    let source = [
        src.r.clamp(0.0, 1.0),
        src.g.clamp(0.0, 1.0),
        src.b.clamp(0.0, 1.0),
    ];
    let backdrop = [
        dst.r.clamp(0.0, 1.0),
        dst.g.clamp(0.0, 1.0),
        dst.b.clamp(0.0, 1.0),
    ];
    let mut output = [0.0; 3];
    for channel in 0..3 {
        let blended = blend_channel(backdrop[channel], source[channel], blend_mode);
        let premultiplied = source_alpha
            * ((1.0 - backdrop_alpha) * source[channel] + backdrop_alpha * blended)
            + backdrop_alpha * (1.0 - source_alpha) * backdrop[channel];
        output[channel] = (premultiplied / output_alpha).clamp(0.0, 1.0);
    }
    Rgba::new(output[0], output[1], output[2], output_alpha)
}

fn blend_channel(backdrop: f32, source: f32, blend_mode: BlendMode) -> f32 {
    match blend_mode {
        BlendMode::Normal => source,
        BlendMode::Multiply => backdrop * source,
        BlendMode::Screen => backdrop + source - backdrop * source,
        BlendMode::Overlay if backdrop <= 0.5 => 2.0 * backdrop * source,
        BlendMode::Overlay => 1.0 - 2.0 * (1.0 - backdrop) * (1.0 - source),
    }
}

#[cfg(test)]
mod tests {
    use super::{CompositePurpose, blend_colors, composite_frame_cells};
    use crate::grid::GridIndex;
    use crate::model::{BlendMode, Layer, LayerKind, Project, Rgba};

    fn assert_color_close(actual: Rgba, expected: Rgba) {
        assert!((actual.r - expected.r).abs() < 0.0001, "{actual:?}");
        assert!((actual.g - expected.g).abs() < 0.0001, "{actual:?}");
        assert!((actual.b - expected.b).abs() < 0.0001, "{actual:?}");
        assert!((actual.a - expected.a).abs() < 0.0001, "{actual:?}");
    }

    #[test]
    fn initial_blend_modes_use_standard_source_over_math() {
        let backdrop = Rgba::new(0.25, 0.5, 0.75, 1.0);
        let source = Rgba::new(0.8, 0.4, 0.2, 1.0);
        assert_color_close(
            blend_colors(backdrop, source, BlendMode::Normal, 1.0),
            source,
        );
        assert_color_close(
            blend_colors(backdrop, source, BlendMode::Multiply, 1.0),
            Rgba::new(0.2, 0.2, 0.15, 1.0),
        );
        assert_color_close(
            blend_colors(backdrop, source, BlendMode::Screen, 1.0),
            Rgba::new(0.85, 0.7, 0.8, 1.0),
        );
        assert_color_close(
            blend_colors(backdrop, source, BlendMode::Overlay, 1.0),
            Rgba::new(0.4, 0.4, 0.6, 1.0),
        );
    }

    #[test]
    fn group_opacity_is_applied_after_children_are_flattened() {
        let mut project = Project::new_square(1.0, 2, 2);
        let frame_id = project.active_frame_id;
        let first_layer = project.active_layer_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::BLACK);
        let group_id = project.allocate_layer_id();
        let mut group = Layer::new(group_id, "Group");
        group.kind = LayerKind::Group;
        group.opacity = 0.5;
        project.layers.push(group);
        let child_id = project.add_layer("Child");
        project.layer_mut(child_id).unwrap().parent_id = Some(group_id);
        project
            .ensure_cel(child_id, frame_id)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        project.active_layer_id = first_layer;
        project.validate().unwrap();

        let cells = composite_frame_cells(&project, frame_id, CompositePurpose::Editor).unwrap();
        assert_color_close(
            cells[&GridIndex { x: 0, y: 0 }],
            Rgba::new(0.5, 0.5, 0.5, 1.0),
        );
    }

    #[test]
    fn reference_layers_are_editor_only() {
        let mut project = Project::new_square(1.0, 2, 2);
        let frame_id = project.active_frame_id;
        project.layers[0].kind = LayerKind::Reference;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);

        let editor = composite_frame_cells(&project, frame_id, CompositePurpose::Editor).unwrap();
        let export = composite_frame_cells(&project, frame_id, CompositePurpose::Export).unwrap();

        assert!(editor.contains_key(&GridIndex { x: 1, y: 1 }));
        assert!(export.is_empty());
    }
}
