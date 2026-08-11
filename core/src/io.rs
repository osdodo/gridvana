use crate::composite::{CompositePurpose, composite_frame_cells};
use crate::model::{Project, Rgba};
use crate::persistence::{deserialize_project, serialize_project};
use image::codecs::gif::{GifEncoder, Repeat};
use image::codecs::png::PngEncoder;
use image::{ColorType, Delay, Frame, ImageEncoder};
use image::{Rgba as ImageRgba, RgbaImage};
use std::fs::File;
use std::path::Path;

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

#[derive(Clone, Copy)]
struct Point2 {
    x: f32,
    y: f32,
}

pub fn save_project(project: &Project, path: impl AsRef<Path>) -> Result<(), String> {
    let data = serialize_project(project)?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

pub fn load_project(path: impl AsRef<Path>) -> Result<Project, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    deserialize_project(&data)
}

pub fn export_png(project: &Project, path: impl AsRef<Path>) -> Result<(), String> {
    let frame_position = project
        .active_frame_position()
        .ok_or_else(|| "Active frame does not exist".to_string())?;
    export_frame_png(project, frame_position, path)
}

pub fn export_png_sequence(project: &Project, path: impl AsRef<Path>) -> Result<(), String> {
    if project.frames.is_empty() {
        return Err("Cannot export an image sequence without frames".to_string());
    }

    let path = path.as_ref();
    if project.frames.len() == 1 {
        return export_frame_png(project, 0, path);
    }

    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let bounds = compute_bounds_for_frames(
        project,
        grid.as_ref(),
        render_config,
        0..project.frames.len(),
        CompositePurpose::Export,
    )
    .unwrap_or(default_bounds());
    let output_paths = png_sequence_paths(path, project.frames.len());
    let mut written_paths = Vec::with_capacity(project.frames.len());

    for (frame_position, output_path) in output_paths.into_iter().enumerate() {
        let image = render_frame_with_bounds(
            project,
            frame_position,
            grid.as_ref(),
            render_config,
            bounds,
            CompositePurpose::Export,
        )?;
        if let Err(error) = image.save(&output_path) {
            for written_path in written_paths {
                let _ = std::fs::remove_file(written_path);
            }
            let _ = std::fs::remove_file(&output_path);
            return Err(error.to_string());
        }
        written_paths.push(output_path);
    }

    Ok(())
}

fn png_sequence_paths(path: &Path, frame_count: usize) -> Vec<std::path::PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| std::ffi::OsStr::new("frame"));
    let number_width = frame_count.to_string().len().max(3);

    (1..=frame_count)
        .map(|frame_number| {
            let file_name = format!(
                "{}_{frame_number:0number_width$}.png",
                stem.to_string_lossy()
            );
            parent.join(file_name)
        })
        .collect()
}

pub fn render_frame_rgba(
    project: &Project,
    frame_position: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let bounds = compute_canvas_bounds(project, grid.as_ref(), render_config);
    let image = render_frame_with_bounds(
        project,
        frame_position,
        grid.as_ref(),
        render_config,
        bounds,
        CompositePurpose::Editor,
    )?;
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

pub fn render_frame_rgba_for_export(
    project: &Project,
    frame_position: usize,
) -> Result<(u32, u32, Vec<u8>), String> {
    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let bounds = compute_canvas_bounds(project, grid.as_ref(), render_config);
    let image = render_frame_with_bounds(
        project,
        frame_position,
        grid.as_ref(),
        render_config,
        bounds,
        CompositePurpose::Export,
    )?;
    let (width, height) = image.dimensions();
    Ok((width, height, image.into_raw()))
}

pub fn render_frame_png_bytes(project: &Project, frame_position: usize) -> Result<Vec<u8>, String> {
    let (width, height, rgba) = render_frame_rgba(project, frame_position)?;
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgba, width, height, ColorType::Rgba8.into())
        .map_err(|error| error.to_string())?;
    Ok(png)
}

fn compute_canvas_bounds(
    project: &Project,
    grid: &dyn crate::grid::GridSystem,
    _render_config: crate::model::GridConfig,
) -> Bounds {
    let indices = project.canvas_grid_indices();
    if indices.is_empty() {
        return default_bounds();
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for index in indices {
        for point in grid.cell_geometry(index) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
    }

    if !min_x.is_finite() {
        return default_bounds();
    }

    Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

pub fn export_frame_png(
    project: &Project,
    frame_position: usize,
    path: impl AsRef<Path>,
) -> Result<(), String> {
    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let bounds = compute_bounds_for_frames(
        project,
        grid.as_ref(),
        render_config,
        [frame_position],
        CompositePurpose::Export,
    )
    .unwrap_or(default_bounds());
    let image = render_frame_with_bounds(
        project,
        frame_position,
        grid.as_ref(),
        render_config,
        bounds,
        CompositePurpose::Export,
    )?;
    image.save(path).map_err(|e| e.to_string())
}

pub fn export_sprite_sheet(project: &Project, path: impl AsRef<Path>) -> Result<(), String> {
    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let frame_positions = active_animation_frame_positions(project)?;
    let bounds = compute_bounds_for_frames(
        project,
        grid.as_ref(),
        render_config,
        frame_positions.iter().copied(),
        CompositePurpose::Export,
    )
    .unwrap_or(default_bounds());
    let (frame_width, frame_height) = bounds_dimensions(bounds);
    let frame_count = frame_positions.len().max(1);
    let mut sheet = RgbaImage::new(frame_width * frame_count as u32, frame_height);

    for (sheet_position, frame_position) in frame_positions.into_iter().enumerate() {
        let frame = render_frame_with_bounds(
            project,
            frame_position,
            grid.as_ref(),
            render_config,
            bounds,
            CompositePurpose::Export,
        )?;
        blit(&mut sheet, &frame, sheet_position as u32 * frame_width, 0);
    }

    sheet.save(path).map_err(|e| e.to_string())
}

pub fn export_gif(project: &Project, path: impl AsRef<Path>) -> Result<(), String> {
    if project.frames.is_empty() {
        return Err("Cannot export an animation without frames".to_string());
    }

    let cell_size = raster_cell_size(project);
    let render_config = project.grid_config.with_cell_size(cell_size);
    let grid = project.grid_config.create_system_with_cell_size(cell_size);
    let frame_positions = active_animation_frame_positions(project)?;
    let bounds = compute_bounds_for_frames(
        project,
        grid.as_ref(),
        render_config,
        frame_positions.iter().copied(),
        CompositePurpose::Export,
    )
    .unwrap_or(default_bounds());
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = GifEncoder::new_with_speed(file, 10);
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| e.to_string())?;

    for frame_position in frame_positions {
        let animation_frame = &project.frames[frame_position];
        let image = render_frame_with_bounds(
            project,
            frame_position,
            grid.as_ref(),
            render_config,
            bounds,
            CompositePurpose::Export,
        )?;
        let delay = gif_delay(animation_frame.duration_ms);
        encoder
            .encode_frame(Frame::from_parts(image, 0, 0, delay))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn active_animation_frame_positions(project: &Project) -> Result<Vec<usize>, String> {
    let frame_ids = match project.active_tag_id {
        Some(tag_id) => project.frame_ids_for_tag(tag_id)?,
        None => project.frames.iter().map(|frame| frame.id).collect(),
    };
    frame_ids
        .into_iter()
        .map(|frame_id| {
            project
                .frames
                .iter()
                .position(|frame| frame.id == frame_id)
                .ok_or_else(|| format!("animation references unknown frame_id {frame_id}"))
        })
        .collect()
}

fn gif_delay(duration_ms: u64) -> Delay {
    // GIF stores delays in centiseconds. Round to the nearest representable value
    // and keep every frame visible for at least one centisecond.
    let centiseconds = (duration_ms.saturating_add(5) / 10).clamp(1, u16::MAX as u64) as u32;
    Delay::from_numer_denom_ms(centiseconds * 10, 1)
}

fn default_bounds() -> Bounds {
    Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 1.0,
    }
}

fn raster_cell_size(project: &Project) -> f32 {
    match project.grid_config {
        crate::model::GridConfig::Triangle { .. } => 2.0,
        crate::model::GridConfig::Square { .. } | crate::model::GridConfig::Hexagon { .. } => 1.0,
    }
}

fn bounds_dimensions(bounds: Bounds) -> (u32, u32) {
    let width = (bounds.max_x - bounds.min_x).ceil().max(1.0) as u32;
    let height = (bounds.max_y - bounds.min_y).ceil().max(1.0) as u32;
    (width, height)
}

fn compute_bounds_for_frames<I>(
    project: &Project,
    grid: &dyn crate::grid::GridSystem,
    _render_config: crate::model::GridConfig,
    frame_positions: I,
    purpose: CompositePurpose,
) -> Option<Bounds>
where
    I: IntoIterator<Item = usize>,
{
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for frame_position in frame_positions {
        if let Some(frame) = project.frames.get(frame_position) {
            let cells = composite_frame_cells(project, frame.id, purpose).ok()?;
            for index in cells.keys() {
                let top_face = grid.cell_geometry(*index);
                if top_face.is_empty() {
                    continue;
                }
                for point in top_face {
                    min_x = min_x.min(point.x);
                    min_y = min_y.min(point.y);
                    max_x = max_x.max(point.x);
                    max_y = max_y.max(point.y);
                }
            }
        }
    }

    if !min_x.is_finite() {
        return None;
    }

    let padding = 1.0;
    Some(Bounds {
        min_x: min_x - padding,
        min_y: min_y - padding,
        max_x: max_x + padding,
        max_y: max_y + padding,
    })
}

fn render_frame_with_bounds(
    project: &Project,
    frame_position: usize,
    grid: &dyn crate::grid::GridSystem,
    _render_config: crate::model::GridConfig,
    bounds: Bounds,
    purpose: CompositePurpose,
) -> Result<RgbaImage, String> {
    let (width, height) = bounds_dimensions(bounds);
    let mut image = RgbaImage::new(width, height);

    let frame = project
        .frames
        .get(frame_position)
        .ok_or_else(|| "Invalid frame position".to_string())?;

    let cells = composite_frame_cells(project, frame.id, purpose)?;
    for (index, color) in cells {
        let polygon = grid
            .cell_geometry(index)
            .into_iter()
            .map(|p| Point2 {
                x: p.x - bounds.min_x,
                y: p.y - bounds.min_y,
            })
            .collect::<Vec<_>>();
        if polygon.is_empty() {
            continue;
        }
        draw_polygon(&mut image, &polygon, color);
    }

    Ok(image)
}

fn draw_polygon(image: &mut RgbaImage, polygon: &[Point2], color: Rgba) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    let width = image.width() as i32;
    let height = image.height() as i32;
    let start_x = min_x.floor().max(0.0) as i32;
    let start_y = min_y.floor().max(0.0) as i32;
    let end_x = max_x.ceil().min(width as f32) as i32;
    let end_y = max_y.ceil().min(height as f32) as i32;

    for y in start_y..end_y {
        for x in start_x..end_x {
            let point = Point2 {
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
            };
            if point_in_polygon(point, polygon) {
                let pixel = image.get_pixel_mut(x as u32, y as u32);
                blend_pixel(pixel, color);
            }
        }
    }
}

fn point_in_polygon(point: Point2, polygon: &[Point2]) -> bool {
    let mut inside = false;
    let mut j = polygon.len().wrapping_sub(1);

    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let intersects = (pi.y > point.y) != (pj.y > point.y)
            && point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y + f32::EPSILON) + pi.x;
        if intersects {
            inside = !inside;
        }
        j = i;
    }

    inside
}

fn blend_pixel(dst: &mut ImageRgba<u8>, src: Rgba) {
    let src_a = src.a.clamp(0.0, 1.0);
    let dst_a = dst[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        dst[0] = 0;
        dst[1] = 0;
        dst[2] = 0;
        dst[3] = 0;
        return;
    }

    let src_r = src.r.clamp(0.0, 1.0);
    let src_g = src.g.clamp(0.0, 1.0);
    let src_b = src.b.clamp(0.0, 1.0);

    let dst_r = dst[0] as f32 / 255.0;
    let dst_g = dst[1] as f32 / 255.0;
    let dst_b = dst[2] as f32 / 255.0;

    let out_r = (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a;
    let out_g = (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a;
    let out_b = (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a;

    dst[0] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
    dst[1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
    dst[2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
    dst[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn blit(target: &mut RgbaImage, source: &RgbaImage, offset_x: u32, offset_y: u32) {
    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = source.get_pixel(x, y);
            target.put_pixel(offset_x + x, offset_y + y, *pixel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        export_frame_png, export_gif, export_png_sequence, export_sprite_sheet, load_project,
        render_frame_rgba,
    };
    use crate::grid::GridIndex;
    use crate::model::{GridConfig, LayerKind, Project, Rgba, TagDirection};
    use image::codecs::gif::GifDecoder;
    use image::{AnimationDecoder, GenericImageView};
    use std::fs::File;
    use std::io::BufReader;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn hex_canvas_preview_uses_only_in_bounds_cells() {
        let project = Project::new_with_grid(GridConfig::Hexagon { cell_size: 1.0 }, 4, 4);

        let indices = project.canvas_grid_indices();

        assert!(indices.contains(&GridIndex { x: -1, y: 2 }));
        assert!(!indices.contains(&GridIndex { x: 3, y: 3 }));

        let (width, height, _) = render_frame_rgba(&project, 0).expect("preview should render");
        assert_eq!((width, height), (6, 5));
    }

    #[test]
    fn triangle_canvas_preview_renders_at_higher_resolution() {
        let project = Project::new_with_grid(GridConfig::Triangle { cell_size: 1.0 }, 4, 4);

        let (width, height, _) = render_frame_rgba(&project, 0).expect("preview should render");
        assert_eq!((width, height), (9, 9));
    }

    #[test]
    fn gif_export_preserves_frames_delays_and_canvas_size() {
        let mut project = Project::new_square(1.0, 4, 4);
        project.frames[0].duration_ms = 120;
        project
            .ensure_current_cel()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);

        let second_frame_id = project.add_frame(None, 340).unwrap();
        project
            .ensure_cel(project.layers[0].id, second_frame_id)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 0 }, Rgba::WHITE);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gridvana-gif-export-{}-{unique}.gif",
            std::process::id()
        ));

        export_gif(&project, &path).expect("GIF should export");
        let decoder = GifDecoder::new(BufReader::new(File::open(&path).unwrap())).unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].delay().numer_denom_ms(), (120, 1));
        assert_eq!(frames[1].delay().numer_denom_ms(), (340, 1));
        assert_eq!(frames[0].buffer().dimensions(), (5, 3));
        assert_eq!(frames[1].buffer().dimensions(), (5, 3));
    }

    #[test]
    fn gif_export_uses_the_active_tag_range_and_direction() {
        let mut project = Project::new_square(1.0, 2, 2);
        let first = project.active_frame_id;
        project.frame_mut(first).unwrap().duration_ms = 110;
        let second = project.add_frame(None, 220).unwrap();
        let third = project.add_frame(None, 330).unwrap();
        project
            .add_tag("Reverse", second, third, TagDirection::Reverse)
            .unwrap();

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gridvana-tagged-gif-export-{}-{unique}.gif",
            std::process::id()
        ));

        export_gif(&project, &path).expect("tagged GIF should export");
        let decoder = GifDecoder::new(BufReader::new(File::open(&path).unwrap())).unwrap();
        let frames = decoder.into_frames().collect_frames().unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].delay().numer_denom_ms(), (330, 1));
        assert_eq!(frames[1].delay().numer_denom_ms(), (220, 1));
    }

    #[test]
    fn png_sequence_exports_every_frame_with_shared_canvas_size() {
        let mut project = Project::new_square(1.0, 4, 4);
        project
            .ensure_current_cel()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);

        let second_frame_id = project.add_frame(None, 100).unwrap();
        project
            .ensure_cel(project.layers[0].id, second_frame_id)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 0 }, Rgba::WHITE);

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "gridvana-png-sequence-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let selected_path = directory.join("walk.png");

        export_png_sequence(&project, &selected_path).expect("PNG sequence should export");
        let first_path = directory.join("walk_001.png");
        let second_path = directory.join("walk_002.png");
        let first = image::open(&first_path).unwrap();
        let second = image::open(&second_path).unwrap();
        let sprite_sheet_path = directory.join("walk-sheet.png");
        export_sprite_sheet(&project, &sprite_sheet_path).expect("sprite sheet should export");
        let sprite_sheet = image::open(&sprite_sheet_path).unwrap();
        project
            .add_tag(
                "Second",
                second_frame_id,
                second_frame_id,
                TagDirection::Forward,
            )
            .unwrap();
        let tagged_sheet_path = directory.join("walk-tagged-sheet.png");
        export_sprite_sheet(&project, &tagged_sheet_path)
            .expect("tagged sprite sheet should export");
        let tagged_sheet = image::open(&tagged_sheet_path).unwrap();

        assert!(!selected_path.exists());
        assert_eq!(first.dimensions(), (5, 3));
        assert_eq!(second.dimensions(), (5, 3));
        assert_eq!(sprite_sheet.dimensions(), (10, 3));
        assert_eq!(tagged_sheet.dimensions(), (3, 3));

        std::fs::remove_file(first_path).unwrap();
        std::fs::remove_file(second_path).unwrap();
        std::fs::remove_file(sprite_sheet_path).unwrap();
        std::fs::remove_file(tagged_sheet_path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn frame_render_composites_global_layers_offsets_links_and_opacity() {
        let mut project = Project::new_square(1.0, 4, 4);
        let base_layer = project.active_layer_id;
        let source_frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let top_layer = project.add_layer("Top");
        project.layer_mut(top_layer).unwrap().opacity = 0.5;
        project
            .ensure_cel(top_layer, source_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::BLACK);
        let linked_frame = project.add_frame(None, 100).unwrap();
        let source_cel = project.cel(base_layer, source_frame).unwrap().id;
        let linked = project.ensure_cel(base_layer, linked_frame).unwrap();
        linked.offset = GridIndex { x: 1, y: 0 };
        linked.linked_cel_id = Some(source_cel);

        let (_, _, first) = render_frame_rgba(&project, 0).unwrap();
        let (_, _, second) = render_frame_rgba(&project, 1).unwrap();
        assert!(first.chunks_exact(4).any(|pixel| pixel[3] > 0));
        assert!(second.chunks_exact(4).any(|pixel| pixel[3] > 0));
        project.validate().unwrap();
    }

    #[test]
    fn reference_layers_are_excluded_from_png_and_gif_exports() {
        let mut project = Project::new_square(1.0, 2, 1);
        let frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let reference = project.add_layer("Reference");
        project.layer_mut(reference).unwrap().kind = LayerKind::Reference;
        project
            .ensure_cel(reference, frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 0 }, Rgba::new(0.0, 1.0, 0.0, 1.0));
        let mut without_reference = project.clone();
        without_reference.layer_mut(reference).unwrap().visible = false;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "gridvana-reference-export-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let png = directory.join("actual.png");
        let expected_png = directory.join("expected.png");
        let gif = directory.join("actual.gif");
        let expected_gif = directory.join("expected.gif");

        export_frame_png(&project, 0, &png).unwrap();
        export_frame_png(&without_reference, 0, &expected_png).unwrap();
        assert_eq!(
            image::open(&png).unwrap().to_rgba8(),
            image::open(&expected_png).unwrap().to_rgba8()
        );

        export_gif(&project, &gif).unwrap();
        export_gif(&without_reference, &expected_gif).unwrap();
        let actual_frames = GifDecoder::new(BufReader::new(File::open(&gif).unwrap()))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        let expected_frames = GifDecoder::new(BufReader::new(File::open(&expected_gif).unwrap()))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(actual_frames[0].buffer(), expected_frames[0].buffer());

        for path in [png, expected_png, gif, expected_gif] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn legacy_project_files_are_rejected_instead_of_migrated() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "gridvana-legacy-project-{}-{unique}.gvn",
            std::process::id()
        ));
        std::fs::write(
            &path,
            br#"{
                "grid_config":{"Square":{"cell_size":20.0}},
                "canvas_width":8,"canvas_height":8,
                "frames":[{"layers":[],"duration_ms":100}],
                "active_frame_index":0,"active_layer_index":0,
                "symmetry_x":{"active":false,"position":4.0},
                "symmetry_y":{"active":false,"position":4.0}
            }"#,
        )
        .unwrap();
        let result = load_project(&path);
        std::fs::remove_file(path).unwrap();
        assert!(result.is_err());
    }
}
