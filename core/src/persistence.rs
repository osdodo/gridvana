use crate::grid::GridIndex;
use crate::model::{
    AnimationTag, BlendMode, CURRENT_SCHEMA_VERSION, Cel, CelId, Frame, FrameId, GridConfig, Layer,
    LayerId, LayerKind, Palette, Project, Rgba, SymmetryLine, TagId,
};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

const SCHEMA_VERSION_V2: u32 = 2;
const SCHEMA_VERSION_V3: u32 = 3;
const SCHEMA_VERSION_V4: u32 = 4;
const SCHEMA_VERSION_V5: u32 = 5;
const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[
    SCHEMA_VERSION_V2,
    SCHEMA_VERSION_V3,
    SCHEMA_VERSION_V4,
    SCHEMA_VERSION_V5,
    CURRENT_SCHEMA_VERSION,
];

#[derive(Debug, Clone, Copy)]
enum Encoding {
    MessagePack,
    Json,
}

#[derive(Deserialize)]
struct SchemaHeader {
    schema_version: u32,
}

// Schemas V2 and V3 have the same persisted fields and are intentionally isolated
// here. Business modules only operate on the current Project type and never carry
// version-specific compatibility branches.
#[derive(Deserialize)]
struct ProjectBeforeTags {
    schema_version: u32,
    grid_config: GridConfigV2,
    canvas_width: u32,
    canvas_height: u32,
    layers: Vec<LayerV2>,
    frames: Vec<FrameV2>,
    cels: Vec<CelV2>,
    active_layer_id: u64,
    active_frame_id: u64,
    next_id: u64,
    symmetry_x: SymmetryLineV2,
    symmetry_y: SymmetryLineV2,
}

#[derive(Deserialize)]
struct ProjectV4 {
    schema_version: u32,
    grid_config: GridConfigV2,
    canvas_width: u32,
    canvas_height: u32,
    layers: Vec<LayerV2>,
    frames: Vec<FrameV2>,
    cels: Vec<CelV2>,
    tags: Vec<AnimationTag>,
    active_tag_id: Option<TagId>,
    active_layer_id: u64,
    active_frame_id: u64,
    next_id: u64,
    symmetry_x: SymmetryLineV2,
    symmetry_y: SymmetryLineV2,
}

#[derive(Deserialize)]
struct ProjectV5 {
    schema_version: u32,
    grid_config: GridConfig,
    canvas_width: u32,
    canvas_height: u32,
    layers: Vec<Layer>,
    frames: Vec<Frame>,
    cels: Vec<Cel>,
    tags: Vec<AnimationTag>,
    active_tag_id: Option<TagId>,
    active_layer_id: LayerId,
    active_frame_id: FrameId,
    next_id: u64,
    symmetry_x: SymmetryLine,
    symmetry_y: SymmetryLine,
}

#[derive(Deserialize)]
enum GridConfigV2 {
    Square { cell_size: f32 },
    Triangle { cell_size: f32 },
    Hexagon { cell_size: f32 },
}

impl GridConfigV2 {
    fn into_current(self) -> GridConfig {
        match self {
            Self::Square { cell_size } => GridConfig::Square { cell_size },
            Self::Triangle { cell_size } => GridConfig::Triangle { cell_size },
            Self::Hexagon { cell_size } => GridConfig::Hexagon { cell_size },
        }
    }
}

#[derive(Deserialize)]
struct LayerV2 {
    id: u64,
    name: String,
    visible: bool,
    locked: bool,
    opacity: f32,
}

impl LayerV2 {
    fn into_current(self) -> Layer {
        Layer {
            id: LayerId(self.id),
            name: self.name,
            visible: self.visible,
            locked: self.locked,
            opacity: self.opacity,
            blend_mode: BlendMode::Normal,
            kind: LayerKind::Paint,
            parent_id: None,
        }
    }
}

#[derive(Deserialize)]
struct FrameV2 {
    id: u64,
    duration_ms: u64,
}

impl FrameV2 {
    fn into_current(self) -> Frame {
        Frame {
            id: FrameId(self.id),
            duration_ms: self.duration_ms,
        }
    }
}

#[derive(Deserialize)]
struct CelV2 {
    id: u64,
    layer_id: u64,
    frame_id: u64,
    offset: GridIndexV2,
    pixels: Vec<PixelEntryV2>,
    linked_cel_id: Option<u64>,
}

impl CelV2 {
    fn into_current(self) -> Cel {
        Cel {
            id: CelId(self.id),
            layer_id: LayerId(self.layer_id),
            frame_id: FrameId(self.frame_id),
            offset: self.offset.into_current(),
            pixels: self
                .pixels
                .into_iter()
                .map(|pixel| (pixel.index.into_current(), pixel.color.into_current()))
                .collect::<HashMap<_, _>>(),
            linked_cel_id: self.linked_cel_id.map(CelId),
        }
    }
}

#[derive(Deserialize)]
struct PixelEntryV2 {
    index: GridIndexV2,
    color: RgbaV2,
}

#[derive(Deserialize)]
struct GridIndexV2 {
    x: i32,
    y: i32,
}

impl GridIndexV2 {
    fn into_current(self) -> GridIndex {
        GridIndex {
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Deserialize)]
struct RgbaV2 {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl RgbaV2 {
    fn into_current(self) -> Rgba {
        Rgba::new(self.r, self.g, self.b, self.a)
    }
}

#[derive(Deserialize)]
struct SymmetryLineV2 {
    active: bool,
    position: f32,
}

impl SymmetryLineV2 {
    fn into_current(self) -> SymmetryLine {
        SymmetryLine {
            active: self.active,
            position: self.position,
        }
    }
}

pub fn serialize_project(project: &Project) -> Result<Vec<u8>, String> {
    project.validate()?;
    rmp_serde::to_vec_named(project).map_err(|error| {
        format!("failed to encode project schema V{CURRENT_SCHEMA_VERSION}: {error}")
    })
}

pub fn deserialize_project(data: &[u8]) -> Result<Project, String> {
    let (encoding, schema_version) = detect_schema_version(data)?;
    let project = match schema_version {
        SCHEMA_VERSION_V2 => migrate_before_tags_to_v6(
            decode(data, encoding, "schema V2 project")?,
            SCHEMA_VERSION_V2,
        )?,
        SCHEMA_VERSION_V3 => migrate_before_tags_to_v6(
            decode(data, encoding, "schema V3 project")?,
            SCHEMA_VERSION_V3,
        )?,
        SCHEMA_VERSION_V4 => migrate_v4_to_v6(decode(data, encoding, "schema V4 project")?)?,
        SCHEMA_VERSION_V5 => migrate_v5_to_v6(decode(data, encoding, "schema V5 project")?)?,
        CURRENT_SCHEMA_VERSION => decode(data, encoding, "current project")?,
        version => return Err(unsupported_version_error(version)),
    };
    project
        .validate()
        .map_err(|error| format!("invalid project after schema migration: {error}"))?;
    Ok(project)
}

fn detect_schema_version(data: &[u8]) -> Result<(Encoding, u32), String> {
    match rmp_serde::from_slice::<SchemaHeader>(data) {
        Ok(header) => Ok((Encoding::MessagePack, header.schema_version)),
        Err(message_pack_error) => match serde_json::from_slice::<SchemaHeader>(data) {
            Ok(header) => Ok((Encoding::Json, header.schema_version)),
            Err(json_error) => Err(format!(
                "invalid project file: could not read schema_version as MessagePack ({message_pack_error}) or JSON ({json_error})"
            )),
        },
    }
}

fn decode<T: DeserializeOwned>(
    data: &[u8],
    encoding: Encoding,
    description: &str,
) -> Result<T, String> {
    match encoding {
        Encoding::MessagePack => rmp_serde::from_slice(data)
            .map_err(|error| format!("failed to decode {description}: {error}")),
        Encoding::Json => serde_json::from_slice(data)
            .map_err(|error| format!("failed to decode {description}: {error}")),
    }
}

fn migrate_before_tags_to_v6(
    project: ProjectBeforeTags,
    source_schema_version: u32,
) -> Result<Project, String> {
    if project.schema_version != source_schema_version {
        return Err(format!(
            "V{source_schema_version} migration received schema_version {}",
            project.schema_version,
        ));
    }

    // V4 adds animation tags. Older projects deterministically start with no tags
    // and no active tag; every pre-existing field and stable ID is copied unchanged.
    Ok(Project {
        schema_version: CURRENT_SCHEMA_VERSION,
        grid_config: project.grid_config.into_current(),
        canvas_width: project.canvas_width,
        canvas_height: project.canvas_height,
        layers: project
            .layers
            .into_iter()
            .map(LayerV2::into_current)
            .collect(),
        frames: project
            .frames
            .into_iter()
            .map(FrameV2::into_current)
            .collect(),
        cels: project.cels.into_iter().map(CelV2::into_current).collect(),
        tags: Vec::new(),
        active_tag_id: None,
        palette: Palette::pico8(),
        foreground_color: Rgba::WHITE,
        background_color: Rgba::BLACK,
        active_layer_id: LayerId(project.active_layer_id),
        active_frame_id: FrameId(project.active_frame_id),
        next_id: project.next_id,
        symmetry_x: project.symmetry_x.into_current(),
        symmetry_y: project.symmetry_y.into_current(),
    })
}

fn migrate_v4_to_v6(project: ProjectV4) -> Result<Project, String> {
    if project.schema_version != SCHEMA_VERSION_V4 {
        return Err(format!(
            "V4 migration received schema_version {}",
            project.schema_version
        ));
    }
    Ok(Project {
        schema_version: CURRENT_SCHEMA_VERSION,
        grid_config: project.grid_config.into_current(),
        canvas_width: project.canvas_width,
        canvas_height: project.canvas_height,
        layers: project
            .layers
            .into_iter()
            .map(LayerV2::into_current)
            .collect(),
        frames: project
            .frames
            .into_iter()
            .map(FrameV2::into_current)
            .collect(),
        cels: project.cels.into_iter().map(CelV2::into_current).collect(),
        tags: project.tags,
        active_tag_id: project.active_tag_id,
        palette: Palette::pico8(),
        foreground_color: Rgba::WHITE,
        background_color: Rgba::BLACK,
        active_layer_id: LayerId(project.active_layer_id),
        active_frame_id: FrameId(project.active_frame_id),
        next_id: project.next_id,
        symmetry_x: project.symmetry_x.into_current(),
        symmetry_y: project.symmetry_y.into_current(),
    })
}

fn migrate_v5_to_v6(project: ProjectV5) -> Result<Project, String> {
    if project.schema_version != SCHEMA_VERSION_V5 {
        return Err(format!(
            "V5 migration received schema_version {}",
            project.schema_version
        ));
    }
    Ok(Project {
        schema_version: CURRENT_SCHEMA_VERSION,
        grid_config: project.grid_config,
        canvas_width: project.canvas_width,
        canvas_height: project.canvas_height,
        layers: project.layers,
        frames: project.frames,
        cels: project.cels,
        tags: project.tags,
        active_tag_id: project.active_tag_id,
        palette: Palette::pico8(),
        foreground_color: Rgba::WHITE,
        background_color: Rgba::BLACK,
        active_layer_id: project.active_layer_id,
        active_frame_id: project.active_frame_id,
        next_id: project.next_id,
        symmetry_x: project.symmetry_x,
        symmetry_y: project.symmetry_y,
    })
}

fn unsupported_version_error(version: u32) -> String {
    let supported = SUPPORTED_SCHEMA_VERSIONS
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "unsupported project schema_version {version}; supported versions are {supported} (current: {CURRENT_SCHEMA_VERSION})"
    )
}

#[cfg(test)]
mod tests {
    use super::{deserialize_project, serialize_project};
    use crate::grid::GridIndex;
    use crate::model::{
        BlendMode, CURRENT_SCHEMA_VERSION, CelId, FrameId, LayerId, LayerKind, Project, Rgba,
        TagDirection, TagId,
    };

    const V2_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/project-v2.gvn");
    const V3_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/project-v3.gvn");
    const V4_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/project-v4.json");
    const V5_FIXTURE: &[u8] = include_bytes!("../tests/fixtures/project-v5.json");

    #[test]
    fn v2_fixture_migrates_without_changing_project_data_or_ids() {
        let project = deserialize_project(V2_FIXTURE).expect("V2 fixture should migrate");

        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!((project.canvas_width, project.canvas_height), (4, 3));
        assert_eq!(
            project
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>(),
            vec![LayerId(11), LayerId(12)]
        );
        assert_eq!(project.layers[0].name, "Base");
        assert!(project.layers[0].locked);
        assert_eq!(project.layers[0].opacity, 0.75);
        assert!(!project.layers[1].visible);
        assert_eq!(
            project
                .frames
                .iter()
                .map(|frame| frame.id)
                .collect::<Vec<_>>(),
            vec![FrameId(21), FrameId(22)]
        );
        assert_eq!(project.frames[0].duration_ms, 80);
        assert_eq!(project.frames[1].duration_ms, 140);
        assert_eq!(
            project.cels.iter().map(|cel| cel.id).collect::<Vec<_>>(),
            vec![CelId(31), CelId(32)]
        );
        assert_eq!(project.cels[1].linked_cel_id, Some(CelId(31)));
        assert_eq!(project.cels[1].offset, GridIndex { x: 1, y: 0 });
        assert_eq!(
            project.cels[0].pixels[&GridIndex { x: 1, y: 1 }],
            Rgba::new(0.1, 0.2, 0.3, 0.5)
        );
        assert_eq!(project.active_layer_id, LayerId(12));
        assert_eq!(project.active_frame_id, FrameId(22));
        assert_eq!(project.next_id, 33);
        assert!(project.tags.is_empty());
        assert_eq!(project.active_tag_id, None);
        assert_eq!(project.palette, crate::model::Palette::pico8());
        assert_eq!(project.foreground_color, Rgba::WHITE);
        assert_eq!(project.background_color, Rgba::BLACK);
    }

    #[test]
    fn v3_fixture_adds_empty_tag_defaults_without_changing_existing_data() {
        let project = deserialize_project(V3_FIXTURE).expect("V3 fixture should migrate");

        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(
            project
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>(),
            vec![LayerId(11), LayerId(12)]
        );
        assert_eq!(
            project
                .frames
                .iter()
                .map(|frame| (frame.id, frame.duration_ms))
                .collect::<Vec<_>>(),
            vec![(FrameId(21), 80), (FrameId(22), 140)]
        );
        assert_eq!(
            project.cels.iter().map(|cel| cel.id).collect::<Vec<_>>(),
            vec![CelId(31), CelId(32)]
        );
        assert_eq!(
            project.cels[0].pixels[&GridIndex { x: 1, y: 1 }],
            Rgba::new(0.1, 0.2, 0.3, 0.5)
        );
        assert_eq!(project.next_id, 33);
        assert!(project.tags.is_empty());
        assert_eq!(project.active_tag_id, None);
    }

    #[test]
    fn v4_fixture_adds_layer_defaults_without_changing_tags_data_or_ids() {
        let project = deserialize_project(V4_FIXTURE).expect("V4 fixture should migrate");

        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(
            project
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>(),
            vec![LayerId(11), LayerId(12)]
        );
        assert!(project.layers.iter().all(|layer| {
            layer.kind == LayerKind::Paint
                && layer.blend_mode == BlendMode::Normal
                && layer.parent_id.is_none()
        }));
        assert_eq!(project.layers[0].opacity, 0.75);
        assert!(project.layers[0].locked);
        assert!(!project.layers[1].visible);
        assert_eq!(project.tags.len(), 1);
        assert_eq!(project.tags[0].id, TagId(40));
        assert_eq!(project.tags[0].direction, TagDirection::PingPong);
        assert_eq!(project.active_tag_id, Some(TagId(40)));
        assert_eq!(project.active_layer_id, LayerId(12));
        assert_eq!(project.active_frame_id, FrameId(22));
        assert_eq!(project.cels[1].linked_cel_id, Some(CelId(31)));
        assert_eq!(project.cels[1].offset, GridIndex { x: 1, y: 0 });
        assert_eq!(
            project.cels[0].pixels[&GridIndex { x: 1, y: 1 }],
            Rgba::new(0.1, 0.2, 0.3, 0.5)
        );
        assert_eq!(project.next_id, 41);
        assert_eq!(project.palette, crate::model::Palette::pico8());
        assert_eq!(project.foreground_color, Rgba::WHITE);
        assert_eq!(project.background_color, Rgba::BLACK);
    }

    #[test]
    fn v5_fixture_adds_palette_defaults_without_changing_project_data() {
        let project = deserialize_project(V5_FIXTURE).expect("V5 fixture should migrate");

        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(project.layers[0].id, LayerId(11));
        assert_eq!(project.layers[1].blend_mode, BlendMode::Multiply);
        assert_eq!(project.frames[1].duration_ms, 140);
        assert_eq!(project.cels[1].linked_cel_id, Some(CelId(31)));
        assert_eq!(project.tags[0].id, TagId(40));
        assert_eq!(project.active_tag_id, Some(TagId(40)));
        assert_eq!(project.next_id, 41);
        assert_eq!(project.palette, crate::model::Palette::pico8());
        assert_eq!(project.foreground_color, Rgba::WHITE);
        assert_eq!(project.background_color, Rgba::BLACK);
    }

    #[test]
    fn migrated_project_saves_and_reloads_as_current_schema() {
        let project = deserialize_project(V2_FIXTURE).unwrap();
        let encoded = serialize_project(&project).unwrap();
        let decoded = deserialize_project(&encoded).unwrap();

        assert_eq!(decoded, project);
        assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn current_schema_round_trips_overlapping_animation_tags_and_active_id() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 90).unwrap();
        let third = project.add_frame(None, 120).unwrap();
        project
            .add_tag("Walk", first, second, TagDirection::Forward)
            .unwrap();
        project
            .add_tag("Reverse", second, third, TagDirection::Reverse)
            .unwrap();
        let active = project
            .add_tag("All", first, third, TagDirection::PingPong)
            .unwrap();
        project.palette.name = "Project Colors".to_string();
        project.palette.colors = vec![Rgba::new(0.2, 0.4, 0.6, 0.8)];
        project.foreground_color = Rgba::new(0.1, 0.2, 0.3, 0.4);
        project.background_color = Rgba::new(0.9, 0.8, 0.7, 0.6);

        let encoded = serialize_project(&project).unwrap();
        let decoded = deserialize_project(&encoded).unwrap();

        assert_eq!(decoded, project);
        assert_eq!(decoded.tags.len(), 3);
        assert_eq!(decoded.active_tag_id, Some(active));
    }

    #[test]
    fn serializer_refuses_to_write_a_non_current_schema() {
        let mut project = deserialize_project(V2_FIXTURE).unwrap();
        project.schema_version = 2;

        let error = serialize_project(&project).unwrap_err();
        assert!(error.contains("schema_version 2"));
        assert!(error.contains(&format!("expected {CURRENT_SCHEMA_VERSION}")));
    }

    #[test]
    fn unknown_past_and_future_versions_are_rejected_clearly() {
        for version in [1, CURRENT_SCHEMA_VERSION + 1, u32::MAX] {
            let data = format!(r#"{{"schema_version":{version}}}"#);
            let error = deserialize_project(data.as_bytes()).unwrap_err();
            assert!(error.contains(&format!("schema_version {version}")));
            assert!(error.contains("supported versions are 2, 3, 4, 5, 6"));
        }
    }

    #[test]
    fn missing_version_has_an_explicit_schema_error() {
        let error = deserialize_project(br#"{"canvas_width":8}"#).unwrap_err();
        assert!(error.contains("schema_version"));
    }
}
