use crate::grid::{GridIndex, GridSystem, HexagonGrid, SquareGrid, TriangleGrid};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub const CURRENT_SCHEMA_VERSION: u32 = 6;
pub const DEFAULT_FRAME_DURATION_MS: u64 = 100;
pub const MAX_PALETTE_COLORS: usize = 256;
pub const MAX_PALETTE_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Rgba = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Rgba = Rgba {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xff) as f32 / 255.0;
        let g = ((hex >> 8) & 0xff) as f32 / 255.0;
        let b = (hex & 0xff) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: Vec<Rgba>,
}

impl Palette {
    pub fn pico8() -> Self {
        Self {
            name: "PICO-8".to_string(),
            colors: vec![
                Rgba::hex(0x000000),
                Rgba::hex(0x1D2B53),
                Rgba::hex(0x7E2553),
                Rgba::hex(0x008751),
                Rgba::hex(0xAB5236),
                Rgba::hex(0x5F574F),
                Rgba::hex(0xC2C3C7),
                Rgba::hex(0xFFF1E8),
                Rgba::hex(0xFF004D),
                Rgba::hex(0xFFA300),
                Rgba::hex(0xFFEC27),
                Rgba::hex(0x00E436),
                Rgba::hex(0x29ADFF),
                Rgba::hex(0x83769C),
                Rgba::hex(0xFF77A8),
                Rgba::hex(0xFFCCAA),
            ],
        }
    }

    pub fn deduplicate(&mut self) {
        let mut unique = Vec::with_capacity(self.colors.len());
        for color in self.colors.drain(..) {
            if !unique.contains(&color) {
                unique.push(color);
            }
        }
        self.colors = unique;
    }
}

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id!(LayerId);
stable_id!(FrameId);
stable_id!(CelId);
stable_id!(TagId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelPosition {
    pub layer_id: LayerId,
    pub frame_id: FrameId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl BlendMode {
    pub const ALL: [Self; 4] = [Self::Normal, Self::Multiply, Self::Screen, Self::Overlay];
}

impl fmt::Display for BlendMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Paint,
    Group,
    Background,
    Reference,
}

impl LayerKind {
    pub const ALL: [Self; 4] = [Self::Paint, Self::Group, Self::Background, Self::Reference];

    pub fn supports_cels(self) -> bool {
        self != Self::Group
    }
}

impl fmt::Display for LayerKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Paint => "Paint",
            Self::Group => "Group",
            Self::Background => "Background",
            Self::Reference => "Reference",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub kind: LayerKind,
    pub parent_id: Option<LayerId>,
}

impl Layer {
    pub fn new(id: LayerId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            kind: LayerKind::Paint,
            parent_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub id: FrameId,
    pub duration_ms: u64,
}

impl Frame {
    pub fn new(id: FrameId) -> Self {
        Self {
            id,
            duration_ms: DEFAULT_FRAME_DURATION_MS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagDirection {
    Forward,
    Reverse,
    PingPong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationTag {
    pub id: TagId,
    pub name: String,
    pub from_frame_id: FrameId,
    pub to_frame_id: FrameId,
    pub direction: TagDirection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cel {
    pub id: CelId,
    pub layer_id: LayerId,
    pub frame_id: FrameId,
    pub offset: GridIndex,
    #[serde(with = "grid_index_map")]
    pub pixels: HashMap<GridIndex, Rgba>,
    pub linked_cel_id: Option<CelId>,
}

impl Cel {
    pub fn new(id: CelId, layer_id: LayerId, frame_id: FrameId) -> Self {
        Self {
            id,
            layer_id,
            frame_id,
            offset: GridIndex { x: 0, y: 0 },
            pixels: HashMap::new(),
            linked_cel_id: None,
        }
    }
}

mod grid_index_map {
    use super::{GridIndex, Rgba};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct PixelEntry {
        index: GridIndex,
        color: Rgba,
    }

    pub fn serialize<S>(value: &HashMap<GridIndex, Rgba>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let pixels = value
            .iter()
            .map(|(index, color)| PixelEntry {
                index: *index,
                color: *color,
            })
            .collect::<Vec<_>>();
        pixels.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<GridIndex, Rgba>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let pixels = Vec::<PixelEntry>::deserialize(deserializer)?;
        Ok(pixels
            .into_iter()
            .map(|pixel| (pixel.index, pixel.color))
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GridConfig {
    Square { cell_size: f32 },
    Triangle { cell_size: f32 },
    Hexagon { cell_size: f32 },
}

impl GridConfig {
    pub fn create_system(&self) -> Box<dyn GridSystem> {
        match self {
            GridConfig::Square { cell_size } => Box::new(SquareGrid::new(*cell_size)),
            GridConfig::Triangle { cell_size } => Box::new(TriangleGrid::new(*cell_size)),
            GridConfig::Hexagon { cell_size } => Box::new(HexagonGrid::new(*cell_size)),
        }
    }

    pub fn create_system_with_cell_size(&self, cell_size: f32) -> Box<dyn GridSystem> {
        match self {
            GridConfig::Square { .. } => Box::new(SquareGrid::new(cell_size)),
            GridConfig::Triangle { .. } => Box::new(TriangleGrid::new(cell_size)),
            GridConfig::Hexagon { .. } => Box::new(HexagonGrid::new(cell_size)),
        }
    }

    pub fn with_cell_size(&self, cell_size: f32) -> Self {
        match self {
            GridConfig::Square { .. } => GridConfig::Square { cell_size },
            GridConfig::Triangle { .. } => GridConfig::Triangle { cell_size },
            GridConfig::Hexagon { .. } => GridConfig::Hexagon { cell_size },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub grid_config: GridConfig,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub layers: Vec<Layer>,
    pub frames: Vec<Frame>,
    pub cels: Vec<Cel>,
    pub tags: Vec<AnimationTag>,
    pub active_tag_id: Option<TagId>,
    pub palette: Palette,
    pub foreground_color: Rgba,
    pub background_color: Rgba,
    pub active_layer_id: LayerId,
    pub active_frame_id: FrameId,
    pub next_id: u64,
    pub symmetry_x: SymmetryLine,
    pub symmetry_y: SymmetryLine,
}

impl Project {
    pub fn new_square(cell_size: f32, canvas_width: u32, canvas_height: u32) -> Self {
        Self::new_with_grid(
            GridConfig::Square { cell_size },
            canvas_width,
            canvas_height,
        )
    }

    pub fn new_with_grid(grid_config: GridConfig, canvas_width: u32, canvas_height: u32) -> Self {
        let layer_id = LayerId(1);
        let frame_id = FrameId(2);
        let cel_id = CelId(3);
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            grid_config,
            canvas_width,
            canvas_height,
            layers: vec![Layer::new(layer_id, "Layer 1")],
            frames: vec![Frame::new(frame_id)],
            cels: vec![Cel::new(cel_id, layer_id, frame_id)],
            tags: Vec::new(),
            active_tag_id: None,
            palette: Palette::pico8(),
            foreground_color: Rgba::WHITE,
            background_color: Rgba::BLACK,
            active_layer_id: layer_id,
            active_frame_id: frame_id,
            next_id: 4,
            symmetry_x: SymmetryLine {
                active: false,
                position: (canvas_width as f32) / 2.0,
            },
            symmetry_y: SymmetryLine {
                active: false,
                position: (canvas_height as f32) / 2.0,
            },
        }
    }

    pub fn allocate_layer_id(&mut self) -> LayerId {
        LayerId(self.allocate_id())
    }

    pub fn allocate_frame_id(&mut self) -> FrameId {
        FrameId(self.allocate_id())
    }

    pub fn allocate_cel_id(&mut self) -> CelId {
        CelId(self.allocate_id())
    }

    pub fn allocate_tag_id(&mut self) -> TagId {
        TagId(self.allocate_id())
    }

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("Gridvana project ID space exhausted");
        id
    }

    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub fn layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|layer| layer.id == id)
    }

    pub fn frame(&self, id: FrameId) -> Option<&Frame> {
        self.frames.iter().find(|frame| frame.id == id)
    }

    pub fn frame_mut(&mut self, id: FrameId) -> Option<&mut Frame> {
        self.frames.iter_mut().find(|frame| frame.id == id)
    }

    pub fn tag(&self, id: TagId) -> Option<&AnimationTag> {
        self.tags.iter().find(|tag| tag.id == id)
    }

    pub fn tag_mut(&mut self, id: TagId) -> Option<&mut AnimationTag> {
        self.tags.iter_mut().find(|tag| tag.id == id)
    }

    pub fn cel(&self, layer_id: LayerId, frame_id: FrameId) -> Option<&Cel> {
        self.cels
            .iter()
            .find(|cel| cel.layer_id == layer_id && cel.frame_id == frame_id)
    }

    pub fn cel_mut(&mut self, layer_id: LayerId, frame_id: FrameId) -> Option<&mut Cel> {
        self.cels
            .iter_mut()
            .find(|cel| cel.layer_id == layer_id && cel.frame_id == frame_id)
    }

    pub fn cel_by_id(&self, id: CelId) -> Option<&Cel> {
        self.cels.iter().find(|cel| cel.id == id)
    }

    pub fn cel_by_id_mut(&mut self, id: CelId) -> Option<&mut Cel> {
        self.cels.iter_mut().find(|cel| cel.id == id)
    }

    pub fn ensure_cel(&mut self, layer_id: LayerId, frame_id: FrameId) -> Result<&mut Cel, String> {
        let layer = self
            .layer(layer_id)
            .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
        if !layer.kind.supports_cels() {
            return Err(format!("group layer {layer_id} cannot contain cels"));
        }
        if self.frame(frame_id).is_none() {
            return Err(format!("unknown frame_id: {frame_id}"));
        }
        if let Some(position) = self
            .cels
            .iter()
            .position(|cel| cel.layer_id == layer_id && cel.frame_id == frame_id)
        {
            return Ok(&mut self.cels[position]);
        }

        let id = self.allocate_cel_id();
        self.cels.push(Cel::new(id, layer_id, frame_id));
        Ok(self.cels.last_mut().expect("the new cel was just pushed"))
    }

    pub fn current_layer(&self) -> Option<&Layer> {
        self.layer(self.active_layer_id)
    }

    pub fn current_layer_mut(&mut self) -> Option<&mut Layer> {
        self.layer_mut(self.active_layer_id)
    }

    pub fn current_frame(&self) -> Option<&Frame> {
        self.frame(self.active_frame_id)
    }

    pub fn current_frame_mut(&mut self) -> Option<&mut Frame> {
        self.frame_mut(self.active_frame_id)
    }

    pub fn current_cel(&self) -> Option<&Cel> {
        self.cel(self.active_layer_id, self.active_frame_id)
    }

    pub fn current_cel_mut(&mut self) -> Option<&mut Cel> {
        self.cel_mut(self.active_layer_id, self.active_frame_id)
    }

    pub fn ensure_current_cel(&mut self) -> Result<&mut Cel, String> {
        self.ensure_cel(self.active_layer_id, self.active_frame_id)
    }

    pub fn resolved_cel(&self, cel: &Cel) -> Result<&Cel, String> {
        let mut current = self
            .cel_by_id(cel.id)
            .ok_or_else(|| format!("unknown cel_id: {}", cel.id))?;
        let mut visited = HashSet::new();
        while let Some(linked_id) = current.linked_cel_id {
            if !visited.insert(current.id) {
                return Err(format!("linked cel cycle includes cel_id {}", current.id));
            }
            current = self
                .cel_by_id(linked_id)
                .ok_or_else(|| format!("unknown linked_cel_id: {linked_id}"))?;
        }
        Ok(current)
    }

    pub fn active_layer_position(&self) -> Option<usize> {
        self.layers
            .iter()
            .position(|layer| layer.id == self.active_layer_id)
    }

    pub fn active_frame_position(&self) -> Option<usize> {
        self.frames
            .iter()
            .position(|frame| frame.id == self.active_frame_id)
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let id = self.allocate_layer_id();
        self.layers.push(Layer::new(id, name));
        self.active_layer_id = id;
        id
    }

    pub fn remove_layer_with_cels(&mut self, layer_id: LayerId) -> Result<(), String> {
        let position = self
            .layers
            .iter()
            .position(|layer| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
        let mut removed_layer_ids = self.descendant_layer_ids(layer_id)?;
        removed_layer_ids.insert(layer_id);
        let remaining_raster_layers = self
            .layers
            .iter()
            .filter(|layer| !removed_layer_ids.contains(&layer.id))
            .filter(|layer| layer.kind.supports_cels())
            .count();
        if remaining_raster_layers == 0 {
            return Err("a project must keep at least one raster layer".to_string());
        }
        let removed_cel_ids = self
            .cels
            .iter()
            .filter(|cel| removed_layer_ids.contains(&cel.layer_id))
            .map(|cel| cel.id)
            .collect::<HashSet<_>>();
        self.materialize_links_affected_by(&removed_cel_ids);
        self.layers
            .retain(|layer| !removed_layer_ids.contains(&layer.id));
        self.cels
            .retain(|cel| !removed_layer_ids.contains(&cel.layer_id));
        if removed_layer_ids.contains(&self.active_layer_id) {
            self.active_layer_id = self.layers[position.min(self.layers.len() - 1)].id;
        }
        Ok(())
    }

    pub fn descendant_layer_ids(&self, layer_id: LayerId) -> Result<HashSet<LayerId>, String> {
        if self.layer(layer_id).is_none() {
            return Err(format!("unknown layer_id: {layer_id}"));
        }
        let mut descendants = HashSet::new();
        let mut pending = vec![layer_id];
        while let Some(parent_id) = pending.pop() {
            for child in self
                .layers
                .iter()
                .filter(|layer| layer.parent_id == Some(parent_id))
            {
                if descendants.insert(child.id) {
                    pending.push(child.id);
                }
            }
        }
        Ok(descendants)
    }

    pub fn layer_depth(&self, layer_id: LayerId) -> Result<usize, String> {
        let mut current = self
            .layer(layer_id)
            .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
        let mut visited = HashSet::from([layer_id]);
        let mut depth = 0;
        while let Some(parent_id) = current.parent_id {
            if !visited.insert(parent_id) {
                return Err(format!("layer parent cycle includes layer {parent_id}"));
            }
            current = self
                .layer(parent_id)
                .ok_or_else(|| format!("layer {} has unknown parent {parent_id}", current.id))?;
            depth += 1;
        }
        Ok(depth)
    }

    pub fn layer_is_effectively_locked(&self, layer_id: LayerId) -> Result<bool, String> {
        let mut current = self
            .layer(layer_id)
            .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current.id) {
                return Err(format!("layer parent cycle includes layer {}", current.id));
            }
            if current.locked {
                return Ok(true);
            }
            let Some(parent_id) = current.parent_id else {
                return Ok(false);
            };
            current = self
                .layer(parent_id)
                .ok_or_else(|| format!("layer {} has unknown parent {parent_id}", current.id))?;
        }
    }

    pub fn layer_is_effectively_visible(&self, layer_id: LayerId) -> Result<bool, String> {
        let mut current = self
            .layer(layer_id)
            .ok_or_else(|| format!("unknown layer_id: {layer_id}"))?;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current.id) {
                return Err(format!("layer parent cycle includes layer {}", current.id));
            }
            if !current.visible {
                return Ok(false);
            }
            let Some(parent_id) = current.parent_id else {
                return Ok(true);
            };
            current = self
                .layer(parent_id)
                .ok_or_else(|| format!("layer {} has unknown parent {parent_id}", current.id))?;
        }
    }

    pub fn add_frame(
        &mut self,
        insert_at: Option<usize>,
        duration_ms: u64,
    ) -> Result<FrameId, String> {
        let position = insert_at.unwrap_or(self.frames.len());
        if position > self.frames.len() {
            return Err(format!(
                "frame insertion position out of bounds: {position}"
            ));
        }
        let id = self.allocate_frame_id();
        let mut frame = Frame::new(id);
        frame.duration_ms = duration_ms;
        self.frames.insert(position, frame);
        self.active_frame_id = id;
        Ok(id)
    }

    pub fn add_tag(
        &mut self,
        name: impl Into<String>,
        from_frame_id: FrameId,
        to_frame_id: FrameId,
        direction: TagDirection,
    ) -> Result<TagId, String> {
        let name = name.into();
        validate_tag_fields(self, &name, from_frame_id, to_frame_id)?;
        let id = self.allocate_tag_id();
        self.tags.push(AnimationTag {
            id,
            name,
            from_frame_id,
            to_frame_id,
            direction,
        });
        self.active_tag_id = Some(id);
        Ok(id)
    }

    pub fn remove_tag(&mut self, tag_id: TagId) -> Result<(), String> {
        let position = self
            .tags
            .iter()
            .position(|tag| tag.id == tag_id)
            .ok_or_else(|| format!("unknown tag_id: {tag_id}"))?;
        self.tags.remove(position);
        if self.active_tag_id == Some(tag_id) {
            self.active_tag_id = None;
        }
        Ok(())
    }

    pub fn rename_tag(&mut self, tag_id: TagId, name: impl Into<String>) -> Result<(), String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("tag name cannot be empty".to_string());
        }
        self.tag_mut(tag_id)
            .ok_or_else(|| format!("unknown tag_id: {tag_id}"))?
            .name = name;
        Ok(())
    }

    pub fn set_tag_range(
        &mut self,
        tag_id: TagId,
        from_frame_id: FrameId,
        to_frame_id: FrameId,
    ) -> Result<(), String> {
        let name = self
            .tag(tag_id)
            .ok_or_else(|| format!("unknown tag_id: {tag_id}"))?
            .name
            .clone();
        validate_tag_fields(self, &name, from_frame_id, to_frame_id)?;
        let tag = self
            .tag_mut(tag_id)
            .expect("the tag was checked immediately before mutation");
        tag.from_frame_id = from_frame_id;
        tag.to_frame_id = to_frame_id;
        Ok(())
    }

    pub fn set_tag_direction(
        &mut self,
        tag_id: TagId,
        direction: TagDirection,
    ) -> Result<(), String> {
        self.tag_mut(tag_id)
            .ok_or_else(|| format!("unknown tag_id: {tag_id}"))?
            .direction = direction;
        Ok(())
    }

    pub fn set_active_tag(&mut self, tag_id: Option<TagId>) -> Result<(), String> {
        if let Some(tag_id) = tag_id
            && self.tag(tag_id).is_none()
        {
            return Err(format!("unknown tag_id: {tag_id}"));
        }
        self.active_tag_id = tag_id;
        Ok(())
    }

    pub fn frame_ids_for_tag(&self, tag_id: TagId) -> Result<Vec<FrameId>, String> {
        let tag = self
            .tag(tag_id)
            .ok_or_else(|| format!("unknown tag_id: {tag_id}"))?;
        let from = self
            .frames
            .iter()
            .position(|frame| frame.id == tag.from_frame_id)
            .ok_or_else(|| format!("tag {tag_id} references an unknown start frame"))?;
        let to = self
            .frames
            .iter()
            .position(|frame| frame.id == tag.to_frame_id)
            .ok_or_else(|| format!("tag {tag_id} references an unknown end frame"))?;
        if from > to {
            return Err(format!("tag {tag_id} start frame follows its end frame"));
        }
        let range = self.frames[from..=to]
            .iter()
            .map(|frame| frame.id)
            .collect::<Vec<_>>();
        Ok(frame_ids_in_direction(&range, tag.direction))
    }

    pub fn duplicate_frame(&mut self, source_id: FrameId) -> Result<FrameId, String> {
        let source_position = self
            .frames
            .iter()
            .position(|frame| frame.id == source_id)
            .ok_or_else(|| format!("unknown frame_id: {source_id}"))?;
        let duration_ms = self.frames[source_position].duration_ms;
        let source_cels = self
            .cels
            .iter()
            .filter(|cel| cel.frame_id == source_id)
            .cloned()
            .collect::<Vec<_>>();
        let new_frame_id = self.add_frame(Some(source_position + 1), duration_ms)?;
        let mut duplicated_ids = HashMap::new();
        for source_cel in &source_cels {
            duplicated_ids.insert(source_cel.id, self.allocate_cel_id());
        }
        for source_cel in source_cels {
            let mut new_cel = source_cel;
            new_cel.id = duplicated_ids[&new_cel.id];
            new_cel.frame_id = new_frame_id;
            if let Some(linked_id) = new_cel.linked_cel_id
                && let Some(duplicated_link_id) = duplicated_ids.get(&linked_id)
            {
                new_cel.linked_cel_id = Some(*duplicated_link_id);
            }
            self.cels.push(new_cel);
        }
        Ok(new_frame_id)
    }

    pub fn remove_frame_with_cels(&mut self, frame_id: FrameId) -> Result<(), String> {
        if self.frames.len() <= 1 {
            return Err("a project must keep at least one frame".to_string());
        }
        let position = self
            .frames
            .iter()
            .position(|frame| frame.id == frame_id)
            .ok_or_else(|| format!("unknown frame_id: {frame_id}"))?;
        let removed_cel_ids = self
            .cels
            .iter()
            .filter(|cel| cel.frame_id == frame_id)
            .map(|cel| cel.id)
            .collect::<HashSet<_>>();
        self.materialize_links_affected_by(&removed_cel_ids);
        self.frames.remove(position);
        self.cels.retain(|cel| frame_id != cel.frame_id);
        let removed_tag_ids = self
            .tags
            .iter()
            .filter(|tag| tag.from_frame_id == frame_id && tag.to_frame_id == frame_id)
            .map(|tag| tag.id)
            .collect::<HashSet<_>>();
        self.tags.retain(|tag| !removed_tag_ids.contains(&tag.id));
        for tag in &mut self.tags {
            if tag.from_frame_id == frame_id {
                tag.from_frame_id = self.frames[position.min(self.frames.len() - 1)].id;
            }
            if tag.to_frame_id == frame_id {
                tag.to_frame_id = self.frames[position.saturating_sub(1)].id;
            }
        }
        if self
            .active_tag_id
            .is_some_and(|tag_id| removed_tag_ids.contains(&tag_id))
        {
            self.active_tag_id = None;
        }
        if self.active_frame_id == frame_id {
            self.active_frame_id = self.frames[position.min(self.frames.len() - 1)].id;
        }
        Ok(())
    }

    pub fn remove_cel_preserving_links(&mut self, layer_id: LayerId, frame_id: FrameId) -> bool {
        let Some(cel_id) = self.cel(layer_id, frame_id).map(|cel| cel.id) else {
            return false;
        };
        let removed = HashSet::from([cel_id]);
        self.materialize_links_affected_by(&removed);
        self.cels.retain(|cel| cel.id != cel_id);
        true
    }

    fn materialize_links_affected_by(&mut self, removed_cel_ids: &HashSet<CelId>) {
        let updates = self
            .cels
            .iter()
            .filter(|cel| !removed_cel_ids.contains(&cel.id))
            .filter(|cel| self.link_chain_intersects(cel, removed_cel_ids))
            .filter_map(|cel| {
                self.resolved_cel(cel)
                    .ok()
                    .map(|resolved| (cel.id, resolved.pixels.clone()))
            })
            .collect::<Vec<_>>();
        for (cel_id, pixels) in updates {
            if let Some(cel) = self.cel_by_id_mut(cel_id) {
                cel.pixels = pixels;
                cel.linked_cel_id = None;
            }
        }
    }

    fn link_chain_intersects(&self, cel: &Cel, targets: &HashSet<CelId>) -> bool {
        let mut current = cel;
        let mut visited = HashSet::new();
        while let Some(linked_id) = current.linked_cel_id {
            if targets.contains(&linked_id) {
                return true;
            }
            if !visited.insert(current.id) {
                return false;
            }
            let Some(linked) = self.cel_by_id(linked_id) else {
                return false;
            };
            current = linked;
        }
        false
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "unsupported project schema_version {}; expected {CURRENT_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if self.layers.is_empty() {
            return Err("a project must contain at least one layer".to_string());
        }
        if !self.layers.iter().any(|layer| layer.kind.supports_cels()) {
            return Err("a project must contain at least one raster layer".to_string());
        }
        if self.frames.is_empty() {
            return Err("a project must contain at least one frame".to_string());
        }
        if self.palette.name.trim().is_empty() {
            return Err("project palette name cannot be empty".to_string());
        }
        if self.palette.name.chars().count() > MAX_PALETTE_NAME_LENGTH {
            return Err(format!(
                "project palette name is too long: {} > {MAX_PALETTE_NAME_LENGTH}",
                self.palette.name.chars().count()
            ));
        }
        if self.palette.colors.len() > MAX_PALETTE_COLORS {
            return Err(format!(
                "project palette contains too many colors: {} > {MAX_PALETTE_COLORS}",
                self.palette.colors.len()
            ));
        }
        validate_rgba(self.foreground_color, "foreground color")?;
        validate_rgba(self.background_color, "background color")?;
        for (index, color) in self.palette.colors.iter().copied().enumerate() {
            validate_rgba(color, &format!("palette color {index}"))?;
        }
        let cell_size = match self.grid_config {
            GridConfig::Square { cell_size }
            | GridConfig::Triangle { cell_size }
            | GridConfig::Hexagon { cell_size } => cell_size,
        };
        if !cell_size.is_finite() || cell_size <= 0.0 {
            return Err("grid cell_size must be finite and greater than zero".to_string());
        }
        if !self.symmetry_x.position.is_finite()
            || !(0.0..=self.canvas_width as f32).contains(&self.symmetry_x.position)
        {
            return Err("symmetry_x position is outside the canvas".to_string());
        }
        if !self.symmetry_y.position.is_finite()
            || !(0.0..=self.canvas_height as f32).contains(&self.symmetry_y.position)
        {
            return Err("symmetry_y position is outside the canvas".to_string());
        }
        if self.layer(self.active_layer_id).is_none() {
            return Err(format!(
                "active_layer_id {} does not exist",
                self.active_layer_id
            ));
        }
        if self.frame(self.active_frame_id).is_none() {
            return Err(format!(
                "active_frame_id {} does not exist",
                self.active_frame_id
            ));
        }

        let mut all_ids = HashSet::new();
        let mut background_layer = None;
        for (position, layer) in self.layers.iter().enumerate() {
            if layer.id.0 == 0 {
                return Err("object IDs must be greater than zero".to_string());
            }
            if !all_ids.insert(layer.id.0) {
                return Err(format!("duplicate object ID: {}", layer.id));
            }
            if layer.name.trim().is_empty() {
                return Err(format!("layer {} has an empty name", layer.id));
            }
            if !layer.opacity.is_finite() || !(0.0..=1.0).contains(&layer.opacity) {
                return Err(format!("layer {} has invalid opacity", layer.id));
            }
            if let Some(parent_id) = layer.parent_id {
                let parent = self.layer(parent_id).ok_or_else(|| {
                    format!("layer {} references unknown parent {parent_id}", layer.id)
                })?;
                if parent.kind != LayerKind::Group {
                    return Err(format!(
                        "layer {} parent {parent_id} is not a group",
                        layer.id
                    ));
                }
            }
            self.layer_depth(layer.id)?;
            if layer.kind == LayerKind::Background {
                if background_layer.replace(layer.id).is_some() {
                    return Err("a project can contain at most one background layer".to_string());
                }
                if position != 0 {
                    return Err(format!("background layer {} must be bottommost", layer.id));
                }
                if layer.parent_id.is_some() {
                    return Err(format!(
                        "background layer {} cannot have a parent",
                        layer.id
                    ));
                }
                if layer.blend_mode != BlendMode::Normal {
                    return Err(format!(
                        "background layer {} must use normal blend mode",
                        layer.id
                    ));
                }
            }
        }
        for frame in &self.frames {
            if frame.id.0 == 0 {
                return Err("object IDs must be greater than zero".to_string());
            }
            if !all_ids.insert(frame.id.0) {
                return Err(format!("duplicate object ID: {}", frame.id));
            }
            if frame.duration_ms == 0 {
                return Err(format!("frame {} has zero duration", frame.id));
            }
        }

        if let Some(active_tag_id) = self.active_tag_id
            && self.tag(active_tag_id).is_none()
        {
            return Err(format!("active_tag_id {active_tag_id} does not exist"));
        }
        for tag in &self.tags {
            if tag.id.0 == 0 {
                return Err("object IDs must be greater than zero".to_string());
            }
            if !all_ids.insert(tag.id.0) {
                return Err(format!("duplicate object ID: {}", tag.id));
            }
            validate_tag_fields(self, &tag.name, tag.from_frame_id, tag.to_frame_id)
                .map_err(|error| format!("tag {} is invalid: {error}", tag.id))?;
        }

        let mut intersections = HashSet::new();
        for cel in &self.cels {
            if cel.id.0 == 0 {
                return Err("object IDs must be greater than zero".to_string());
            }
            if !all_ids.insert(cel.id.0) {
                return Err(format!("duplicate object ID: {}", cel.id));
            }
            if self.layer(cel.layer_id).is_none() {
                return Err(format!(
                    "cel {} references unknown layer {}",
                    cel.id, cel.layer_id
                ));
            }
            if self
                .layer(cel.layer_id)
                .is_some_and(|layer| !layer.kind.supports_cels())
            {
                return Err(format!("group layer {} cannot contain cels", cel.layer_id));
            }
            if self.frame(cel.frame_id).is_none() {
                return Err(format!(
                    "cel {} references unknown frame {}",
                    cel.id, cel.frame_id
                ));
            }
            if !intersections.insert((cel.layer_id, cel.frame_id)) {
                return Err(format!(
                    "multiple cels occupy layer {} and frame {}",
                    cel.layer_id, cel.frame_id
                ));
            }
            let resolved = self.resolved_cel(cel)?;
            for index in resolved.pixels.keys() {
                let positioned = GridIndex {
                    x: index.x.saturating_add(cel.offset.x),
                    y: index.y.saturating_add(cel.offset.y),
                };
                if !self.is_index_in_bounds(positioned) {
                    return Err(format!(
                        "cel {} contains out-of-bounds pixel ({}, {})",
                        cel.id, positioned.x, positioned.y
                    ));
                }
            }
        }

        if all_ids.iter().any(|id| *id >= self.next_id) {
            return Err("next_id must be greater than every allocated ID".to_string());
        }
        Ok(())
    }

    pub fn is_index_in_bounds(&self, index: GridIndex) -> bool {
        match self.grid_config {
            GridConfig::Hexagon { cell_size } => {
                if cell_size <= 0.0 {
                    return false;
                }
                let q = index.x as f32;
                let r = index.y as f32;
                let sqrt3 = 3.0f32.sqrt();
                let center_x = cell_size * (sqrt3 * q + sqrt3 / 2.0 * r);
                let center_y = cell_size * (3.0 / 2.0 * r);
                center_x >= 0.0
                    && center_y >= 0.0
                    && center_x < self.canvas_width as f32 * cell_size
                    && center_y < self.canvas_height as f32 * cell_size
            }
            GridConfig::Triangle { cell_size } => {
                if cell_size <= 0.0 {
                    return false;
                }
                let w = cell_size / 2.0;
                let h = cell_size * 3.0f32.sqrt() / 2.0;
                let is_up = (index.x + index.y) % 2 != 0;
                let center_x = index.x as f32 * w + w;
                let center_y = if is_up {
                    index.y as f32 * h + 2.0 * h / 3.0
                } else {
                    index.y as f32 * h + h / 3.0
                };
                center_x >= 0.0
                    && center_y >= 0.0
                    && center_x < self.canvas_width as f32 * cell_size
                    && center_y < self.canvas_height as f32 * cell_size
            }
            GridConfig::Square { .. } => {
                index.x >= 0
                    && index.y >= 0
                    && (index.x as u32) < self.canvas_width
                    && (index.y as u32) < self.canvas_height
            }
        }
    }

    pub fn canvas_grid_indices(&self) -> Vec<GridIndex> {
        match self.grid_config {
            GridConfig::Square { .. } => {
                let mut indices = Vec::new();
                for x in 0..self.canvas_width as i32 {
                    for y in 0..self.canvas_height as i32 {
                        indices.push(GridIndex { x, y });
                    }
                }
                indices
            }
            GridConfig::Hexagon { cell_size } => {
                if cell_size <= 0.0 {
                    return Vec::new();
                }
                let sqrt3 = 3.0f32.sqrt();
                let width_world = self.canvas_width as f32 * cell_size;
                let height_world = self.canvas_height as f32 * cell_size;
                let r_max = (height_world / (1.5 * cell_size)).ceil() as i32 + 1;
                let mut indices = Vec::new();
                for r in 0..=r_max {
                    let q_min = (-r as f32 / 2.0).floor() as i32 - 1;
                    let q_max =
                        ((width_world / (sqrt3 * cell_size)) - r as f32 / 2.0).ceil() as i32 + 1;
                    for q in q_min..=q_max {
                        let index = GridIndex { x: q, y: r };
                        if self.is_index_in_bounds(index) {
                            indices.push(index);
                        }
                    }
                }
                indices
            }
            GridConfig::Triangle { cell_size } => {
                if cell_size <= 0.0 {
                    return Vec::new();
                }
                let max_x_world = self.canvas_width as i32;
                let max_y_world = self.canvas_height as f32;
                let max_y = ((max_y_world * 2.0 / 3.0f32.sqrt()).ceil() as i32) + 2;
                let mut indices = Vec::new();
                for y in 0..=max_y {
                    for x in -1..=max_x_world * 2 {
                        let index = GridIndex { x, y };
                        if self.is_index_in_bounds(index) {
                            indices.push(index);
                        }
                    }
                }
                indices
            }
        }
    }
}

fn validate_rgba(color: Rgba, description: &str) -> Result<(), String> {
    let channels = [color.r, color.g, color.b, color.a];
    if channels
        .iter()
        .any(|channel| !channel.is_finite() || !(0.0..=1.0).contains(channel))
    {
        return Err(format!("{description} has an invalid RGBA channel"));
    }
    Ok(())
}

pub fn frame_ids_in_direction(frames: &[FrameId], direction: TagDirection) -> Vec<FrameId> {
    match direction {
        TagDirection::Forward => frames.to_vec(),
        TagDirection::Reverse => frames.iter().rev().copied().collect(),
        TagDirection::PingPong => {
            let mut ordered = frames.to_vec();
            if frames.len() > 2 {
                ordered.extend(frames[1..frames.len() - 1].iter().rev().copied());
            }
            ordered
        }
    }
}

fn validate_tag_fields(
    project: &Project,
    name: &str,
    from_frame_id: FrameId,
    to_frame_id: FrameId,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("tag name cannot be empty".to_string());
    }
    let from = project
        .frames
        .iter()
        .position(|frame| frame.id == from_frame_id)
        .ok_or_else(|| format!("unknown from_frame_id: {from_frame_id}"))?;
    let to = project
        .frames
        .iter()
        .position(|frame| frame.id == to_frame_id)
        .ok_or_else(|| format!("unknown to_frame_id: {to_frame_id}"))?;
    if from > to {
        return Err("tag start frame must not follow its end frame".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SymmetryLine {
    pub active: bool,
    pub position: f32,
}

impl Default for SymmetryLine {
    fn default() -> Self {
        Self {
            active: false,
            position: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlendMode, Cel, CelId, FrameId, Layer, LayerId, LayerKind, MAX_PALETTE_COLORS,
        MAX_PALETTE_NAME_LENGTH, Project, Rgba, TagDirection, TagId,
    };
    use crate::grid::GridIndex;

    #[test]
    fn new_project_is_valid_and_round_trips() {
        let project = Project::new_square(16.0, 8, 8);
        project.validate().unwrap();
        let encoded = rmp_serde::to_vec_named(&project).unwrap();
        let decoded: Project = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(decoded, project);
        decoded.validate().unwrap();
    }

    #[test]
    fn stable_ids_survive_insert_delete_and_reorder() {
        let mut project = Project::new_square(16.0, 8, 8);
        let original_layer = project.active_layer_id;
        let original_frame = project.active_frame_id;
        let second_layer = project.add_layer("Ink");
        let second_frame = project.add_frame(Some(0), 140).unwrap();
        project.layers.swap(0, 1);
        project.frames.swap(0, 1);
        assert_eq!(project.layer(original_layer).unwrap().name, "Layer 1");
        assert_eq!(project.frame(original_frame).unwrap().duration_ms, 100);
        project.remove_layer_with_cels(second_layer).unwrap();
        project.remove_frame_with_cels(second_frame).unwrap();
        assert_eq!(project.layers[0].id, original_layer);
        assert_eq!(project.frames[0].id, original_frame);
        project.validate().unwrap();
    }

    #[test]
    fn animation_tag_directions_produce_stable_frame_id_sequences() {
        let mut project = Project::new_square(16.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 90).unwrap();
        let third = project.add_frame(None, 110).unwrap();
        let forward = project
            .add_tag("Forward", first, third, TagDirection::Forward)
            .unwrap();
        let reverse = project
            .add_tag("Reverse", first, third, TagDirection::Reverse)
            .unwrap();
        let ping_pong = project
            .add_tag("Ping Pong", first, third, TagDirection::PingPong)
            .unwrap();

        assert_eq!(
            project.frame_ids_for_tag(forward).unwrap(),
            vec![first, second, third]
        );
        assert_eq!(
            project.frame_ids_for_tag(reverse).unwrap(),
            vec![third, second, first]
        );
        assert_eq!(
            project.frame_ids_for_tag(ping_pong).unwrap(),
            vec![first, second, third, second]
        );
        project.validate().unwrap();
    }

    #[test]
    fn deleting_tag_endpoint_frames_repairs_ranges_and_removes_single_frame_tags() {
        let mut project = Project::new_square(16.0, 8, 8);
        let first = project.active_frame_id;
        let middle = project.add_frame(None, 90).unwrap();
        let last = project.add_frame(None, 110).unwrap();
        let range = project
            .add_tag("Range", first, last, TagDirection::Forward)
            .unwrap();
        let single = project
            .add_tag("Single", middle, middle, TagDirection::Forward)
            .unwrap();
        project.active_tag_id = Some(single);

        project.remove_frame_with_cels(middle).unwrap();
        assert!(project.tag(single).is_none());
        assert_eq!(project.active_tag_id, None);
        assert_eq!(project.frame_ids_for_tag(range).unwrap(), vec![first, last]);

        project.remove_frame_with_cels(first).unwrap();
        let repaired = project.tag(range).unwrap();
        assert_eq!((repaired.from_frame_id, repaired.to_frame_id), (last, last));
        project.validate().unwrap();
    }

    #[test]
    fn validate_rejects_invalid_tag_fields_order_and_active_id() {
        let mut project = Project::new_square(16.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 100).unwrap();
        let tag = project
            .add_tag("Walk", first, second, TagDirection::Forward)
            .unwrap();

        project.tag_mut(tag).unwrap().name = "  ".to_string();
        assert!(project.validate().unwrap_err().contains("name"));
        project.tag_mut(tag).unwrap().name = "Walk".to_string();
        project.frames.swap(0, 1);
        assert!(project.validate().unwrap_err().contains("start frame"));
        project.frames.swap(0, 1);
        project.active_tag_id = Some(TagId(999));
        assert!(project.validate().unwrap_err().contains("active_tag_id"));
    }

    #[test]
    fn validate_rejects_invalid_project_palette_and_active_colors() {
        let mut project = Project::new_square(16.0, 8, 8);
        project.palette.name = " ".to_string();
        assert!(project.validate().unwrap_err().contains("palette name"));

        project.palette.name = "x".repeat(MAX_PALETTE_NAME_LENGTH + 1);
        assert!(project.validate().unwrap_err().contains("too long"));

        project.palette.name = "Colors".to_string();
        project.palette.colors = vec![Rgba::WHITE; MAX_PALETTE_COLORS + 1];
        assert!(project.validate().unwrap_err().contains("too many colors"));

        project.palette.colors.clear();
        project.foreground_color.a = f32::NAN;
        assert!(project.validate().unwrap_err().contains("foreground"));
    }

    #[test]
    fn cels_are_sparse_and_deletions_cascade() {
        let mut project = Project::new_square(16.0, 8, 8);
        let first_layer = project.active_layer_id;
        let first_frame = project.active_frame_id;
        let second_layer = project.add_layer("Ink");
        let second_frame = project.add_frame(None, 100).unwrap();
        assert!(project.cel(second_layer, second_frame).is_none());
        project
            .ensure_cel(second_layer, second_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 2 }, Rgba::WHITE);
        project.remove_layer_with_cels(first_layer).unwrap();
        assert!(project.cel(first_layer, first_frame).is_none());
        project.remove_frame_with_cels(first_frame).unwrap();
        assert!(project.cels.iter().all(|cel| cel.frame_id != first_frame));
        project.validate().unwrap();
    }

    #[test]
    fn validate_rejects_duplicate_intersections_and_dangling_references() {
        let mut project = Project::new_square(16.0, 8, 8);
        let cel = project.cels[0].clone();
        let duplicate_id = project.allocate_cel_id();
        project.cels.push(Cel {
            id: duplicate_id,
            ..cel
        });
        assert!(project.validate().unwrap_err().contains("multiple cels"));
        project.cels.pop();
        project.cels[0].layer_id = LayerId(999);
        assert!(project.validate().unwrap_err().contains("unknown layer"));
    }

    #[test]
    fn validate_rejects_link_cycles_and_out_of_bounds_pixels() {
        let mut project = Project::new_square(16.0, 8, 8);
        let layer = project.active_layer_id;
        let first_frame = project.active_frame_id;
        let second_frame = project.add_frame(None, 100).unwrap();
        let first_cel = project.cel(layer, first_frame).unwrap().id;
        let second_cel = project.ensure_cel(layer, second_frame).unwrap().id;
        project.cel_by_id_mut(first_cel).unwrap().linked_cel_id = Some(second_cel);
        project.cel_by_id_mut(second_cel).unwrap().linked_cel_id = Some(first_cel);
        assert!(project.validate().unwrap_err().contains("cycle"));
        project.cel_by_id_mut(first_cel).unwrap().linked_cel_id = None;
        project.cel_by_id_mut(second_cel).unwrap().linked_cel_id = None;
        project
            .cel_by_id_mut(first_cel)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 8, y: 0 }, Rgba::WHITE);
        assert!(project.validate().unwrap_err().contains("out-of-bounds"));
    }

    #[test]
    fn validate_rejects_nonexistent_active_ids_and_non_monotonic_allocator() {
        let mut project = Project::new_square(16.0, 8, 8);
        project.active_frame_id = FrameId(99);
        assert!(project.validate().unwrap_err().contains("active_frame_id"));
        project.active_frame_id = project.frames[0].id;
        project.next_id = CelId(3).0;
        assert!(project.validate().unwrap_err().contains("next_id"));
    }

    #[test]
    fn deleting_a_link_source_materializes_dependents() {
        let mut project = Project::new_square(16.0, 8, 8);
        let layer_id = project.active_layer_id;
        let source_frame = project.active_frame_id;
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 3, y: 2 }, Rgba::WHITE);
        let linked_frame = project.add_frame(None, 100).unwrap();
        let source_cel_id = project.cel(layer_id, source_frame).unwrap().id;
        project
            .ensure_cel(layer_id, linked_frame)
            .unwrap()
            .linked_cel_id = Some(source_cel_id);

        project.remove_frame_with_cels(source_frame).unwrap();

        let dependent = project.cel(layer_id, linked_frame).unwrap();
        assert_eq!(dependent.linked_cel_id, None);
        assert!(dependent.pixels.contains_key(&GridIndex { x: 3, y: 2 }));
        project.validate().unwrap();
    }

    #[test]
    fn validate_rejects_invalid_layer_parents_and_parent_cycles() {
        let mut dangling = Project::new_square(16.0, 8, 8);
        dangling.layers[0].parent_id = Some(LayerId(999));
        assert!(dangling.validate().unwrap_err().contains("unknown parent"));

        let mut non_group = Project::new_square(16.0, 8, 8);
        let parent = non_group.add_layer("Paint Parent");
        non_group.layers[0].parent_id = Some(parent);
        assert!(non_group.validate().unwrap_err().contains("not a group"));

        let mut cycle = Project::new_square(16.0, 8, 8);
        let first_id = cycle.allocate_layer_id();
        let second_id = cycle.allocate_layer_id();
        let mut first = Layer::new(first_id, "First Group");
        first.kind = LayerKind::Group;
        first.parent_id = Some(second_id);
        let mut second = Layer::new(second_id, "Second Group");
        second.kind = LayerKind::Group;
        second.parent_id = Some(first_id);
        cycle.layers.extend([first, second]);
        assert!(cycle.validate().unwrap_err().contains("parent cycle"));
    }

    #[test]
    fn validate_rejects_group_cels_and_invalid_background_layers() {
        let mut group_with_cel = Project::new_square(16.0, 8, 8);
        group_with_cel.add_layer("Raster");
        group_with_cel.layers[0].kind = LayerKind::Group;
        assert!(
            group_with_cel
                .validate()
                .unwrap_err()
                .contains("cannot contain cels")
        );

        let mut misplaced = Project::new_square(16.0, 8, 8);
        let background = misplaced.add_layer("Background");
        misplaced.layer_mut(background).unwrap().kind = LayerKind::Background;
        assert!(misplaced.validate().unwrap_err().contains("bottommost"));

        let mut blended = Project::new_square(16.0, 8, 8);
        blended.layers[0].kind = LayerKind::Background;
        blended.layers[0].blend_mode = BlendMode::Multiply;
        assert!(
            blended
                .validate()
                .unwrap_err()
                .contains("normal blend mode")
        );

        let mut duplicate = Project::new_square(16.0, 8, 8);
        duplicate.layers[0].kind = LayerKind::Background;
        let second = duplicate.add_layer("Second Background");
        duplicate.layer_mut(second).unwrap().kind = LayerKind::Background;
        duplicate.layers.swap(0, 1);
        assert!(
            duplicate
                .validate()
                .unwrap_err()
                .contains("at most one background")
        );
    }

    #[test]
    fn group_state_is_inherited_and_group_deletion_cascades_safely() {
        let mut project = Project::new_square(16.0, 8, 8);
        let surviving_layer = project.active_layer_id;
        let source_frame = project.active_frame_id;
        let linked_frame = project.add_frame(None, 100).unwrap();

        let group_id = project.allocate_layer_id();
        let mut group = Layer::new(group_id, "Group");
        group.kind = LayerKind::Group;
        group.visible = false;
        group.locked = true;
        project.layers.push(group);
        let child_id = project.add_layer("Child");
        project.layer_mut(child_id).unwrap().parent_id = Some(group_id);
        project
            .ensure_cel(child_id, source_frame)
            .unwrap()
            .pixels
            .insert(GridIndex { x: 3, y: 2 }, Rgba::WHITE);
        let source_cel_id = project.cel(child_id, source_frame).unwrap().id;
        project
            .ensure_cel(surviving_layer, linked_frame)
            .unwrap()
            .linked_cel_id = Some(source_cel_id);

        assert!(project.layer_is_effectively_locked(child_id).unwrap());
        assert!(!project.layer_is_effectively_visible(child_id).unwrap());
        project.remove_layer_with_cels(group_id).unwrap();

        assert!(project.layer(group_id).is_none());
        assert!(project.layer(child_id).is_none());
        let dependent = project.cel(surviving_layer, linked_frame).unwrap();
        assert_eq!(dependent.linked_cel_id, None);
        assert!(dependent.pixels.contains_key(&GridIndex { x: 3, y: 2 }));
        project.validate().unwrap();
    }
}
