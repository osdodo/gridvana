use crate::io::render_frame_rgba_for_export;
use crate::model::{FrameId, LayerId, Project, TagDirection, TagId};
use image::{ColorType, ImageEncoder, RgbaImage, codecs::png::PngEncoder};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_SCALE: u32 = 16;
const MAX_SHEET_DIMENSION: u32 = 16_384;
const MAX_SHEET_PIXELS: u64 = 67_108_864;
static NEXT_TEMPORARY_OUTPUT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameRange {
    All,
    ActiveTag,
    Explicit(Vec<FrameId>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerRange {
    All,
    Visible,
    Single(LayerId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetLayout {
    Horizontal,
    Vertical,
    FixedRows(u32),
    FixedColumns(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrimMode {
    None,
    Sprite,
    PerFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmptyFramePolicy {
    Include,
    Skip,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataFormat {
    Array,
    Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportOptions {
    pub frames: FrameRange,
    pub layers: LayerRange,
    pub layout: SheetLayout,
    pub scale: u32,
    pub padding: u32,
    pub spacing: u32,
    pub border: u32,
    pub trim: TrimMode,
    pub empty_frames: EmptyFramePolicy,
    pub extrude: u32,
    pub metadata_format: MetadataFormat,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            frames: FrameRange::All,
            layers: LayerRange::Visible,
            layout: SheetLayout::Horizontal,
            scale: 1,
            padding: 0,
            spacing: 0,
            border: 0,
            trim: TrimMode::None,
            empty_frames: EmptyFramePolicy::Include,
            extrude: 0,
            metadata_format: MetadataFormat::Array,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportedFrame {
    pub key: String,
    pub frame_id: FrameId,
    pub duration_ms: u64,
    pub frame: ExportRect,
    pub source_size: ExportSize,
    pub trim_offset: ExportPoint,
    pub trimmed_source_size: ExportSize,
    pub empty: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportPoint {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportedTag {
    pub tag_id: TagId,
    pub name: String,
    pub from_frame_id: FrameId,
    pub to_frame_id: FrameId,
    pub direction: TagDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportMetadata {
    pub image_width: u32,
    pub image_height: u32,
    pub scale: u32,
    pub frames: Vec<ExportedFrame>,
    pub tags: Vec<ExportedTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetExport {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub metadata: ExportMetadata,
}

impl SpriteSheetExport {
    pub fn metadata_json(&self, format: MetadataFormat) -> Result<String, String> {
        #[derive(Serialize)]
        struct ArrayDocument<'a> {
            frames: &'a [ExportedFrame],
            meta: MetadataMeta<'a>,
        }
        #[derive(Serialize)]
        struct HashDocument<'a> {
            frames: BTreeMap<&'a str, &'a ExportedFrame>,
            meta: MetadataMeta<'a>,
        }
        #[derive(Serialize)]
        struct MetadataMeta<'a> {
            image_width: u32,
            image_height: u32,
            scale: u32,
            tags: &'a [ExportedTag],
        }

        let meta = || MetadataMeta {
            image_width: self.metadata.image_width,
            image_height: self.metadata.image_height,
            scale: self.metadata.scale,
            tags: &self.metadata.tags,
        };
        match format {
            MetadataFormat::Array => serde_json::to_string_pretty(&ArrayDocument {
                frames: &self.metadata.frames,
                meta: meta(),
            }),
            MetadataFormat::Hash => serde_json::to_string_pretty(&HashDocument {
                frames: self
                    .metadata
                    .frames
                    .iter()
                    .map(|frame| (frame.key.as_str(), frame))
                    .collect(),
                meta: meta(),
            }),
        }
        .map_err(|error| format!("failed to encode sprite sheet metadata: {error}"))
    }
}

struct RenderedFrame {
    frame_id: FrameId,
    duration_ms: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    content_bounds: Option<PixelBounds>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct PixelBounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

pub fn build_sprite_sheet(
    project: &Project,
    options: &ExportOptions,
) -> Result<SpriteSheetExport, String> {
    validate_options(options)?;
    project.validate()?;
    let selected_frames = selected_frame_positions(project, &options.frames)?;
    let render_project = project_for_layer_range(project, options.layers)?;
    let mut rendered = Vec::with_capacity(selected_frames.len());
    for frame_position in selected_frames {
        let frame = &project.frames[frame_position];
        let (width, height, rgba) = render_frame_rgba_for_export(&render_project, frame_position)?;
        let content_bounds = alpha_bounds(width, height, &rgba);
        if content_bounds.is_none() {
            match options.empty_frames {
                EmptyFramePolicy::Error => {
                    return Err(format!("frame {} is fully transparent", frame.id));
                }
                EmptyFramePolicy::Skip => continue,
                EmptyFramePolicy::Include => {}
            }
        }
        rendered.push(RenderedFrame {
            frame_id: frame.id,
            duration_ms: frame.duration_ms,
            width,
            height,
            rgba,
            content_bounds,
            tags: tags_for_frame(project, frame_position),
        });
    }
    if rendered.is_empty() {
        return Err("sprite sheet contains no frames after applying the empty-frame policy".into());
    }

    let sprite_bounds = match options.trim {
        TrimMode::Sprite => union_bounds(rendered.iter().filter_map(|frame| frame.content_bounds)),
        _ => None,
    };
    let mut images = Vec::with_capacity(rendered.len());
    for frame in &rendered {
        let bounds = match options.trim {
            TrimMode::None => PixelBounds {
                x: 0,
                y: 0,
                width: frame.width,
                height: frame.height,
            },
            TrimMode::Sprite => sprite_bounds.unwrap_or(PixelBounds {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }),
            TrimMode::PerFrame => frame.content_bounds.unwrap_or(PixelBounds {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }),
        };
        let cropped = crop_rgba(frame.width, frame.height, &frame.rgba, bounds)?;
        images.push((
            bounds,
            scale_rgba(&cropped, bounds.width, bounds.height, options.scale)?,
        ));
    }

    let max_width = images
        .iter()
        .map(|(_, image)| image.width())
        .max()
        .unwrap_or(1);
    let max_height = images
        .iter()
        .map(|(_, image)| image.height())
        .max()
        .unwrap_or(1);
    let slot_width = checked_add(max_width, checked_mul(options.padding, 2)?)?;
    let slot_height = checked_add(max_height, checked_mul(options.padding, 2)?)?;
    let (columns, rows) = layout_grid(options.layout, images.len())?;
    let width = sheet_axis_size(columns, slot_width, options.spacing, options.border)?;
    let height = sheet_axis_size(rows, slot_height, options.spacing, options.border)?;
    validate_sheet_size(width, height)?;
    let mut sheet = RgbaImage::new(width, height);
    let mut metadata_frames = Vec::with_capacity(rendered.len());

    for (index, ((bounds, image), source)) in images.iter().zip(&rendered).enumerate() {
        let column =
            u32::try_from(index).map_err(|_| "frame index overflowed".to_string())? % columns;
        let row = u32::try_from(index).map_err(|_| "frame index overflowed".to_string())? / columns;
        let x = options.border + column * (slot_width + options.spacing) + options.padding;
        let y = options.border + row * (slot_height + options.spacing) + options.padding;
        blit(&mut sheet, image, x, y);
        if options.extrude > 0 {
            extrude_edges(&mut sheet, image, x, y, options.extrude);
        }
        metadata_frames.push(ExportedFrame {
            key: format!("frame_{}_{index}", source.frame_id.0),
            frame_id: source.frame_id,
            duration_ms: source.duration_ms,
            frame: ExportRect {
                x,
                y,
                width: image.width(),
                height: image.height(),
            },
            source_size: ExportSize {
                width: source.width,
                height: source.height,
            },
            trim_offset: ExportPoint {
                x: bounds.x,
                y: bounds.y,
            },
            trimmed_source_size: ExportSize {
                width: bounds.width,
                height: bounds.height,
            },
            empty: source.content_bounds.is_none(),
            tags: source.tags.clone(),
        });
    }

    Ok(SpriteSheetExport {
        width,
        height,
        rgba: sheet.into_raw(),
        metadata: ExportMetadata {
            image_width: width,
            image_height: height,
            scale: options.scale,
            frames: metadata_frames,
            tags: project
                .tags
                .iter()
                .map(|tag| ExportedTag {
                    tag_id: tag.id,
                    name: tag.name.clone(),
                    from_frame_id: tag.from_frame_id,
                    to_frame_id: tag.to_frame_id,
                    direction: tag.direction,
                })
                .collect(),
        },
    })
}

pub fn export_sprite_sheet_files(
    project: &Project,
    png_path: impl AsRef<Path>,
    json_path: impl AsRef<Path>,
    options: &ExportOptions,
) -> Result<(), String> {
    let export = build_sprite_sheet(project, options)?;
    let json = export.metadata_json(options.metadata_format)?;
    let png_path = png_path.as_ref();
    let json_path = json_path.as_ref();
    let png_temp = temporary_output_path(png_path);
    let json_temp = temporary_output_path(json_path);

    let result = (|| {
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(
                &export.rgba,
                export.width,
                export.height,
                ColorType::Rgba8.into(),
            )
            .map_err(|error| format!("failed to encode sprite sheet PNG: {error}"))?;
        std::fs::write(&png_temp, png)
            .map_err(|error| format!("failed to write {}: {error}", png_temp.display()))?;
        std::fs::write(&json_temp, json)
            .map_err(|error| format!("failed to write {}: {error}", json_temp.display()))?;
        std::fs::rename(&png_temp, png_path)
            .map_err(|error| format!("failed to finalize {}: {error}", png_path.display()))?;
        if let Err(error) = std::fs::rename(&json_temp, json_path) {
            let _ = std::fs::remove_file(png_path);
            return Err(format!(
                "failed to finalize {}: {error}",
                json_path.display()
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&png_temp);
        let _ = std::fs::remove_file(&json_temp);
    }
    result
}

fn validate_options(options: &ExportOptions) -> Result<(), String> {
    if !(1..=MAX_SCALE).contains(&options.scale) {
        return Err(format!(
            "sprite sheet scale must be between 1 and {MAX_SCALE}"
        ));
    }
    if options.extrude > options.padding {
        return Err("extrude must not exceed padding".to_string());
    }
    match options.layout {
        SheetLayout::FixedRows(0) | SheetLayout::FixedColumns(0) => {
            Err("fixed row and column counts must be greater than zero".to_string())
        }
        _ => Ok(()),
    }
}

fn selected_frame_positions(project: &Project, range: &FrameRange) -> Result<Vec<usize>, String> {
    let ids = match range {
        FrameRange::All => project.frames.iter().map(|frame| frame.id).collect(),
        FrameRange::ActiveTag => {
            let tag_id = project.active_tag_id.ok_or_else(|| {
                "active tag frame range requested but no tag is active".to_string()
            })?;
            project.frame_ids_for_tag(tag_id)?
        }
        FrameRange::Explicit(ids) if ids.is_empty() => {
            return Err("explicit frame range cannot be empty".to_string());
        }
        FrameRange::Explicit(ids) => ids.clone(),
    };
    ids.into_iter()
        .map(|id| {
            project
                .frames
                .iter()
                .position(|frame| frame.id == id)
                .ok_or_else(|| format!("unknown frame_id in export range: {id}"))
        })
        .collect()
}

fn project_for_layer_range(project: &Project, range: LayerRange) -> Result<Project, String> {
    let mut render_project = project.clone();
    match range {
        LayerRange::All => {
            for layer in &mut render_project.layers {
                layer.visible = true;
            }
        }
        LayerRange::Visible => {}
        LayerRange::Single(layer_id) => {
            let selected = render_project
                .layer(layer_id)
                .ok_or_else(|| format!("unknown layer_id in export range: {layer_id}"))?;
            let mut included = render_project.descendant_layer_ids(layer_id)?;
            included.insert(layer_id);
            let mut parent_id = selected.parent_id;
            while let Some(id) = parent_id {
                let parent = render_project.layer(id).ok_or_else(|| {
                    format!("layer {layer_id} has unknown parent in export range: {id}")
                })?;
                included.insert(id);
                parent_id = parent.parent_id;
            }
            for layer in &mut render_project.layers {
                layer.visible = included.contains(&layer.id);
            }
        }
    }
    Ok(render_project)
}

fn tags_for_frame(project: &Project, frame_position: usize) -> Vec<String> {
    project
        .tags
        .iter()
        .filter(|tag| {
            let from = project
                .frames
                .iter()
                .position(|frame| frame.id == tag.from_frame_id);
            let to = project
                .frames
                .iter()
                .position(|frame| frame.id == tag.to_frame_id);
            matches!((from, to), (Some(from), Some(to)) if (from..=to).contains(&frame_position))
        })
        .map(|tag| tag.name.clone())
        .collect()
}

fn alpha_bounds(width: u32, height: u32, rgba: &[u8]) -> Option<PixelBounds> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for (position, pixel) in rgba.chunks_exact(4).enumerate() {
        if pixel[3] == 0 {
            continue;
        }
        let position = u32::try_from(position).ok()?;
        let x = position % width;
        let y = position / width;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        found = true;
    }
    if found {
        Some(PixelBounds {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        })
    } else {
        None
    }
}

fn union_bounds(bounds: impl IntoIterator<Item = PixelBounds>) -> Option<PixelBounds> {
    bounds.into_iter().reduce(|left, right| {
        let min_x = left.x.min(right.x);
        let min_y = left.y.min(right.y);
        let max_x = (left.x + left.width).max(right.x + right.width);
        let max_y = (left.y + left.height).max(right.y + right.height);
        PixelBounds {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    })
}

fn crop_rgba(
    source_width: u32,
    source_height: u32,
    rgba: &[u8],
    bounds: PixelBounds,
) -> Result<RgbaImage, String> {
    if bounds.x + bounds.width > source_width || bounds.y + bounds.height > source_height {
        return Err("trim bounds exceed the rendered frame".to_string());
    }
    let source = RgbaImage::from_raw(source_width, source_height, rgba.to_vec())
        .ok_or_else(|| "rendered RGBA byte length does not match its dimensions".to_string())?;
    Ok(
        image::imageops::crop_imm(&source, bounds.x, bounds.y, bounds.width, bounds.height)
            .to_image(),
    )
}

fn scale_rgba(rgba: &RgbaImage, width: u32, height: u32, scale: u32) -> Result<RgbaImage, String> {
    let scaled_width = checked_mul(width, scale)?;
    let scaled_height = checked_mul(height, scale)?;
    Ok(image::imageops::resize(
        rgba,
        scaled_width,
        scaled_height,
        image::imageops::FilterType::Nearest,
    ))
}

fn layout_grid(layout: SheetLayout, count: usize) -> Result<(u32, u32), String> {
    let count = u32::try_from(count).map_err(|_| "frame count exceeds u32".to_string())?;
    let (columns, rows) = match layout {
        SheetLayout::Horizontal => (count, 1),
        SheetLayout::Vertical => (1, count),
        SheetLayout::FixedColumns(columns) => {
            let columns = columns.min(count);
            (columns, count.div_ceil(columns))
        }
        SheetLayout::FixedRows(rows) => {
            let rows = rows.min(count);
            (count.div_ceil(rows), rows)
        }
    };
    Ok((columns.max(1), rows.max(1)))
}

fn sheet_axis_size(count: u32, slot: u32, spacing: u32, border: u32) -> Result<u32, String> {
    checked_add(
        checked_mul(border, 2)?,
        checked_add(
            checked_mul(count, slot)?,
            checked_mul(count.saturating_sub(1), spacing)?,
        )?,
    )
}

fn validate_sheet_size(width: u32, height: u32) -> Result<(), String> {
    if width > MAX_SHEET_DIMENSION || height > MAX_SHEET_DIMENSION {
        return Err(format!(
            "sprite sheet dimensions {width}x{height} exceed {MAX_SHEET_DIMENSION}x{MAX_SHEET_DIMENSION}"
        ));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_SHEET_PIXELS {
        return Err(format!(
            "sprite sheet contains {pixels} pixels; maximum is {MAX_SHEET_PIXELS}"
        ));
    }
    Ok(())
}

fn blit(target: &mut RgbaImage, source: &RgbaImage, x: u32, y: u32) {
    for source_y in 0..source.height() {
        for source_x in 0..source.width() {
            target.put_pixel(
                x + source_x,
                y + source_y,
                *source.get_pixel(source_x, source_y),
            );
        }
    }
}

fn extrude_edges(target: &mut RgbaImage, source: &RgbaImage, x: u32, y: u32, amount: u32) {
    let amount = i64::from(amount);
    let width = i64::from(source.width());
    let height = i64::from(source.height());
    for offset_y in -amount..height + amount {
        for offset_x in -amount..width + amount {
            if (0..width).contains(&offset_x) && (0..height).contains(&offset_y) {
                continue;
            }
            let source_x = offset_x.clamp(0, width - 1) as u32;
            let source_y = offset_y.clamp(0, height - 1) as u32;
            let target_x = (i64::from(x) + offset_x) as u32;
            let target_y = (i64::from(y) + offset_y) as u32;
            target.put_pixel(target_x, target_y, *source.get_pixel(source_x, source_y));
        }
    }
}

fn checked_add(left: u32, right: u32) -> Result<u32, String> {
    left.checked_add(right)
        .ok_or_else(|| "sprite sheet dimension overflowed".to_string())
}

fn checked_mul(left: u32, right: u32) -> Result<u32, String> {
    left.checked_mul(right)
        .ok_or_else(|| "sprite sheet dimension overflowed".to_string())
}

fn temporary_output_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sprite-sheet".to_string());
    let temporary_id = NEXT_TEMPORARY_OUTPUT_ID.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        "{file_name}.gridvana-tmp-{}-{temporary_id}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        EmptyFramePolicy, ExportOptions, FrameRange, LayerRange, MetadataFormat, SheetLayout,
        TrimMode, build_sprite_sheet, export_sprite_sheet_files,
    };
    use crate::grid::GridIndex;
    use crate::model::{Layer, LayerKind, Project, Rgba, TagDirection};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn project_with_frames() -> (
        Project,
        crate::model::FrameId,
        crate::model::FrameId,
        crate::model::FrameId,
    ) {
        let mut project = Project::new_square(1.0, 3, 2);
        let first = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::new(1.0, 0.0, 0.0, 1.0));
        let second = project.add_frame(None, 220).unwrap();
        project
            .ensure_cel(project.active_layer_id, second)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 1 }, Rgba::new(0.0, 0.0, 1.0, 1.0));
        let empty = project.add_frame(None, 330).unwrap();
        (project, first, second, empty)
    }

    #[test]
    fn layouts_scale_padding_spacing_and_border_have_exact_dimensions() {
        let (project, first, second, _) = project_with_frames();
        let options = ExportOptions {
            frames: FrameRange::Explicit(vec![first, second]),
            layout: SheetLayout::Horizontal,
            scale: 2,
            padding: 1,
            spacing: 3,
            border: 4,
            ..ExportOptions::default()
        };
        let horizontal = build_sprite_sheet(&project, &options).unwrap();
        assert_eq!((horizontal.width, horizontal.height), (27, 14));
        assert_eq!(horizontal.metadata.frames[0].frame.x, 5);
        assert_eq!(horizontal.metadata.frames[1].frame.x, 16);
        assert_eq!(horizontal.metadata.frames[0].frame.width, 6);

        let vertical = build_sprite_sheet(
            &project,
            &ExportOptions {
                layout: SheetLayout::Vertical,
                frames: FrameRange::Explicit(vec![first, second]),
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!((vertical.width, vertical.height), (3, 4));

        let grid = build_sprite_sheet(
            &project,
            &ExportOptions {
                layout: SheetLayout::FixedColumns(2),
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!((grid.width, grid.height), (6, 4));
    }

    #[test]
    fn trim_modes_and_empty_frame_policies_are_precise() {
        let (project, first, second, empty) = project_with_frames();
        let per_frame = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first, second]),
                trim: TrimMode::PerFrame,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!((per_frame.width, per_frame.height), (2, 1));
        assert_eq!(per_frame.metadata.frames[0].trim_offset.x, 0);
        assert_eq!(per_frame.metadata.frames[1].trim_offset.x, 2);
        assert_eq!(per_frame.metadata.frames[1].trim_offset.y, 1);

        let sprite_trim = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first, second]),
                trim: TrimMode::Sprite,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!((sprite_trim.width, sprite_trim.height), (6, 2));

        let skipped = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first, empty, second]),
                empty_frames: EmptyFramePolicy::Skip,
                trim: TrimMode::PerFrame,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!(skipped.metadata.frames.len(), 2);
        let error = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![empty]),
                empty_frames: EmptyFramePolicy::Error,
                ..ExportOptions::default()
            },
        )
        .unwrap_err();
        assert!(error.contains("fully transparent"));
    }

    #[test]
    fn layer_ranges_and_extrude_copy_expected_pixels() {
        let (mut project, first, _, _) = project_with_frames();
        let hidden = project.add_layer("Hidden");
        project.layer_mut(hidden).unwrap().visible = false;
        project
            .ensure_cel(hidden, first)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 0 }, Rgba::new(0.0, 1.0, 0.0, 1.0));

        let visible = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first]),
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!(&visible.rgba[4..8], &[0, 0, 0, 0]);
        let all = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first]),
                layers: LayerRange::All,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!(&all.rgba[4..8], &[0, 255, 0, 255]);
        let single = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first]),
                layers: LayerRange::Single(hidden),
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!(&single.rgba[4..8], &[0, 255, 0, 255]);

        let extruded = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::Explicit(vec![first]),
                trim: TrimMode::PerFrame,
                padding: 1,
                extrude: 1,
                ..ExportOptions::default()
            },
        )
        .unwrap();
        assert_eq!((extruded.width, extruded.height), (3, 3));
        assert!(
            extruded
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel == [255, 0, 0, 255])
        );
    }

    #[test]
    fn single_group_range_keeps_its_subtree_and_required_ancestors() {
        let mut project = Project::new_square(1.0, 3, 1);
        let frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let group_id = project.allocate_layer_id();
        let mut group = Layer::new(group_id, "Group");
        group.kind = LayerKind::Group;
        project.layers.push(group);
        let child_id = project.add_layer("Child");
        project.layer_mut(child_id).unwrap().parent_id = Some(group_id);
        project
            .ensure_cel(child_id, frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 0 }, Rgba::new(0.0, 1.0, 0.0, 1.0));
        let sibling_id = project.add_layer("Sibling");
        project
            .ensure_cel(sibling_id, frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 0 }, Rgba::new(0.0, 0.0, 1.0, 1.0));

        for selected in [group_id, child_id] {
            let export = build_sprite_sheet(
                &project,
                &ExportOptions {
                    frames: FrameRange::Explicit(vec![frame]),
                    layers: LayerRange::Single(selected),
                    ..ExportOptions::default()
                },
            )
            .unwrap();
            assert_eq!(&export.rgba[0..4], &[0, 0, 0, 0]);
            assert_eq!(&export.rgba[4..8], &[0, 255, 0, 255]);
            assert_eq!(&export.rgba[8..12], &[0, 0, 0, 0]);
        }
    }

    #[test]
    fn active_tag_order_and_array_hash_metadata_match_atlas_rectangles() {
        let (mut project, first, second, _) = project_with_frames();
        project
            .add_tag("Reverse", first, second, TagDirection::Reverse)
            .unwrap();
        let export = build_sprite_sheet(
            &project,
            &ExportOptions {
                frames: FrameRange::ActiveTag,
                trim: TrimMode::PerFrame,
                ..ExportOptions::default()
            },
        )
        .unwrap();

        assert_eq!(export.metadata.frames[0].frame_id, second);
        assert_eq!(export.metadata.frames[1].frame_id, first);
        assert_eq!(export.metadata.frames[0].tags, vec!["Reverse"]);
        assert_eq!(export.metadata.frames[0].frame.x, 0);
        assert_eq!(export.metadata.frames[1].frame.x, 1);
        assert_eq!(&export.rgba[0..4], &[0, 0, 255, 255]);
        assert_eq!(&export.rgba[4..8], &[255, 0, 0, 255]);
        let array: serde_json::Value =
            serde_json::from_str(&export.metadata_json(MetadataFormat::Array).unwrap()).unwrap();
        let hash: serde_json::Value =
            serde_json::from_str(&export.metadata_json(MetadataFormat::Hash).unwrap()).unwrap();
        assert!(array["frames"].is_array());
        assert!(hash["frames"].is_object());
        assert_eq!(array["meta"]["tags"][0]["name"], "Reverse");
    }

    #[test]
    fn paired_file_export_cleans_up_when_json_cannot_be_written() {
        let (project, _, _, _) = project_with_frames();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "gridvana-sprite-sheet-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let png_path = directory.join("sheet.png");
        let json_path = directory.join("missing").join("sheet.json");

        let error =
            export_sprite_sheet_files(&project, &png_path, &json_path, &ExportOptions::default())
                .unwrap_err();
        assert!(error.contains("failed to write"));
        assert!(!png_path.exists());
        assert!(std::fs::read_dir(&directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("gridvana-tmp")
        }));

        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn paired_file_export_can_replace_previous_outputs_without_temp_files() {
        let (project, _, _, _) = project_with_frames();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "gridvana-sprite-sheet-repeat-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let png_path = directory.join("sheet.png");
        let json_path = directory.join("sheet.json");

        export_sprite_sheet_files(&project, &png_path, &json_path, &ExportOptions::default())
            .unwrap();
        export_sprite_sheet_files(&project, &png_path, &json_path, &ExportOptions::default())
            .unwrap();

        assert!(png_path.exists());
        assert!(json_path.exists());
        assert!(std::fs::read_dir(&directory).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("gridvana-tmp")
        }));

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn temporary_output_paths_are_unique_and_not_hidden() {
        let path = std::path::Path::new("sheet.png");
        let first = super::temporary_output_path(path);
        let second = super::temporary_output_path(path);

        assert_ne!(first, second);
        assert!(
            !first
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with('.')
        );
    }
}
