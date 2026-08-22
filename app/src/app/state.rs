use super::native_menu::NativeMenuState;
use crate::canvas::OnionSkinSettings;
use crate::cli_terminal::CliConfig;
use crate::i18n::{AppPreferences, Language};
use crate::types::{
    ColorSlot, InspectorPanel, SelectionCombineMode, SettingsSection, SpriteEmptyChoice,
    SpriteFrameRangeChoice, SpriteLayerRangeChoice, SpriteLayoutChoice, SpriteMetadataChoice,
    SpriteTrimChoice, Tool,
};
use gridvana_core::grid::GridIndex;
use gridvana_core::history::History;
use gridvana_core::model::{CelPosition, FrameId, LayerId, Project, Rgba, TagId};
use gridvana_core::recovery::RecoveryDocument;
use iced::Point;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub(super) enum StrokeKind {
    Brush { color: Rgba, size: u8 },
    Eraser { size: u8 },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ShapeKind {
    Rectangle,
    RectangleHollow,
    Circle,
    CircleHollow,
    Line,
}

pub(super) const MAX_RECENT_COLORS: usize = 16;
pub(super) const MIN_BRUSH_SIZE: u8 = 1;
pub(super) const MAX_BRUSH_SIZE: u8 = 12;
pub(super) const MIN_ERASER_SIZE: u8 = 1;
pub(super) const MAX_ERASER_SIZE: u8 = 12;
pub(super) const DEFAULT_INSPECTOR_WIDTH: f32 = 304.0;
pub(super) const DEFAULT_AI_INSPECTOR_WIDTH: f32 = 520.0;
pub(super) const MIN_INSPECTOR_WIDTH: f32 = 260.0;
pub(super) const MIN_AI_INSPECTOR_WIDTH: f32 = 360.0;
pub(super) const MAX_INSPECTOR_WIDTH: f32 = 720.0;

#[derive(Debug)]
pub(super) enum PendingRecovery {
    Available(Box<RecoveryDocument>),
    Damaged(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum McpCopyFeedback {
    Endpoint,
    AgentPrompt,
}

#[derive(Debug)]
pub(super) struct StrokeBuilder {
    pub(super) kind: StrokeKind,
    pub(super) changes: HashMap<GridIndex, Option<Rgba>>,
    pub(super) layer_id: LayerId,
    pub(super) frame_id: FrameId,
    pub(super) cel_existed_before: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ShapeDraft {
    pub(super) kind: ShapeKind,
    pub(super) start: GridIndex,
    pub(super) current: GridIndex,
    pub(super) color: Rgba,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SelectionMoveDraft {
    pub(super) start: GridIndex,
    pub(super) current: GridIndex,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SelectionBoxDraft {
    pub(super) start: GridIndex,
    pub(super) current: GridIndex,
    pub(super) combine_mode: SelectionCombineMode,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct InspectorResize {
    pub(super) start_cursor_x: f32,
    pub(super) start_width: f32,
    pub(super) target: InspectorPanel,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PreviewDrag {
    pub(super) start_cursor: Point,
    pub(super) start_offset: Point,
}

#[derive(Debug, Clone)]
pub(super) struct ClipboardPixel {
    pub(super) offset_x: i32,
    pub(super) offset_y: i32,
    pub(super) color: Rgba,
}

#[derive(Debug, Clone)]
pub(super) struct SelectionClipboard {
    pub(super) anchor: GridIndex,
    pub(super) source_layer_id: LayerId,
    pub(super) source_frame_id: FrameId,
    pub(super) selected_offsets: Vec<(i32, i32)>,
    pub(super) pixels: Vec<ClipboardPixel>,
}

/// Pasted content that hovers above the cel until it is committed, so moving
/// or transforming it never disturbs the pixels underneath.
#[derive(Debug, Clone)]
pub(super) struct FloatingSelection {
    pub(super) pixels: HashMap<GridIndex, Rgba>,
    pub(super) layer_id: LayerId,
    pub(super) frame_id: FrameId,
}

#[derive(Debug, Clone)]
pub(super) struct TimelineClipboardCel {
    pub(super) layer_offset: isize,
    pub(super) frame_offset: isize,
    pub(super) cel_offset: GridIndex,
    pub(super) pixels: Vec<(GridIndex, Rgba)>,
    pub(super) populated: bool,
}

#[derive(Debug, Clone)]
pub(super) struct TimelineCelClipboard {
    pub(super) cells: Vec<TimelineClipboardCel>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SpriteSheetExportForm {
    pub(super) frame_range: SpriteFrameRangeChoice,
    pub(super) layer_range: SpriteLayerRangeChoice,
    pub(super) layout: SpriteLayoutChoice,
    pub(super) trim: SpriteTrimChoice,
    pub(super) empty: SpriteEmptyChoice,
    pub(super) metadata: SpriteMetadataChoice,
    pub(super) fixed_count: u8,
    pub(super) scale: u8,
    pub(super) padding: u8,
    pub(super) spacing: u8,
    pub(super) border: u8,
    pub(super) extrude: u8,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TimelineDrag {
    Frame {
        frame_id: FrameId,
        target: usize,
    },
    Layer {
        layer_id: LayerId,
        target: usize,
    },
    Cel {
        origin: CelPosition,
        target: CelPosition,
    },
    TagRange {
        tag_id: TagId,
        anchor: FrameId,
        target: FrameId,
    },
}

impl Default for SpriteSheetExportForm {
    fn default() -> Self {
        Self {
            frame_range: SpriteFrameRangeChoice::All,
            layer_range: SpriteLayerRangeChoice::Visible,
            layout: SpriteLayoutChoice::FixedColumns,
            trim: SpriteTrimChoice::PerFrame,
            empty: SpriteEmptyChoice::Include,
            metadata: SpriteMetadataChoice::Array,
            fixed_count: 4,
            scale: 4,
            padding: 1,
            spacing: 0,
            border: 0,
            extrude: 1,
        }
    }
}

pub struct Gridvana {
    pub(super) project: Project,
    pub(super) ai_preview_project: Option<Project>,
    pub(super) mcp_service: Option<crate::mcp::EmbeddedMcpService>,
    pub(super) mcp_status: String,
    pub(super) terminal: Option<iced_term::Terminal>,
    pub(super) terminal_temp_files: Vec<std::path::PathBuf>,
    pub(super) cli_settings_open: bool,
    pub(super) settings_section: SettingsSection,
    pub(super) preferences: AppPreferences,
    pub(super) language: Language,
    pub(super) language_save_error: Option<String>,
    pub(super) about_dialog_open: bool,
    pub(super) cli_config: CliConfig,
    pub(super) cli_config_draft: CliConfig,
    pub(super) cli_status: String,
    pub(super) cli_save_error: Option<String>,
    pub(super) cli_test_in_flight: bool,
    pub(super) mcp_copy_feedback: Option<McpCopyFeedback>,
    pub(super) inspector_panel: InspectorPanel,
    pub(super) inspector_width: f32,
    pub(super) ai_inspector_width: f32,
    pub(super) inspector_resize: Option<InspectorResize>,
    pub(super) native_menu: NativeMenuState,
    pub(super) history: History,
    pub(super) current_tool: Tool,
    pub(super) brush_size: u8,
    pub(super) eraser_size: u8,
    pub(super) active_color_slot: ColorSlot,
    pub(super) current_color_hex_input: String,
    pub(super) recent_colors: Vec<Rgba>,
    pub(super) is_playing: bool,
    pub(super) onion_skin_enabled: bool,
    pub(super) onion_skin_settings: OnionSkinSettings,
    pub(super) playback_last_tick: Option<std::time::Instant>,
    pub(super) playback_elapsed: std::time::Duration,
    pub(super) playback_sequence_index: usize,
    pub(super) timeline_drag: Option<TimelineDrag>,
    pub(super) timeline_selection: HashSet<CelPosition>,
    pub(super) timeline_selection_anchor: Option<CelPosition>,
    pub(super) timeline_cel_clipboard: Option<TimelineCelClipboard>,
    pub(super) cel_context_menu: Option<CelPosition>,
    pub(super) selection_context_menu: Option<Point>,
    pub(super) canvas_context_menu: Option<Point>,
    pub(super) canvas_size_popover_open: bool,
    pub(super) app_menu_open: bool,
    pub(super) current_stroke: Option<StrokeBuilder>,
    pub(super) current_shape: Option<ShapeDraft>,
    pub(super) shape_preview_indices: Vec<GridIndex>,
    pub(super) has_canvas: bool,
    pub(super) new_project_dialog_open: bool,
    pub(super) pending_sprite_sheet_export_path: Option<PathBuf>,
    pub(super) sprite_sheet_export_form: SpriteSheetExportForm,
    pub(super) sprite_sheet_export_estimate: Result<(u32, u32), String>,
    pub(super) last_export_summary: Option<String>,
    pub(super) new_project_width: String,
    pub(super) new_project_height: String,
    pub(super) project_path: Option<PathBuf>,
    pub(super) is_saved: bool,
    pub(super) recovery_file_path: PathBuf,
    pub(super) pending_recovery: Option<PendingRecovery>,
    pub(super) autosave_error: Option<String>,
    pub(super) cursor_position: Option<Point>,
    pub(super) global_left_button_down: bool,
    pub(super) space_pressed: bool,
    pub(super) shift_pressed: bool,
    pub(super) alt_pressed: bool,
    pub(super) zoom_modifier_pressed: bool,
    pub(super) hovered_grid_index: Option<GridIndex>,
    pub(super) preview_visible: bool,
    pub(super) preview_offset: Point,
    pub(super) preview_drag: Option<PreviewDrag>,
    pub(super) move_mode_active: bool,
    pub(super) selection_indices: HashSet<GridIndex>,
    pub(super) selection_box_draft: Option<SelectionBoxDraft>,
    pub(super) selection_move_draft: Option<SelectionMoveDraft>,
    pub(super) selection_clipboard: Option<SelectionClipboard>,
    pub(super) floating_selection: Option<FloatingSelection>,
    pub(super) paste_offset: GridIndex,
    pub(super) resize_canvas_width: String,
    pub(super) resize_canvas_height: String,
}

impl Gridvana {
    pub(super) fn ai_agent_panel_available(&self) -> bool {
        self.cli_config.should_show_ai_panel()
    }

    pub(super) fn active_inspector_width(&self) -> f32 {
        if self.inspector_panel == InspectorPanel::AiAgent && self.ai_agent_panel_available() {
            self.ai_inspector_width
        } else {
            self.inspector_width
        }
    }

    pub(super) fn displayed_project(&self) -> &Project {
        self.ai_preview_project.as_ref().unwrap_or(&self.project)
    }

    pub(super) fn has_visible_canvas(&self) -> bool {
        self.has_canvas
            || self
                .ai_preview_project
                .as_ref()
                .is_some_and(|project| project.canvas_width > 0 && project.canvas_height > 0)
    }
}

impl Drop for Gridvana {
    fn drop(&mut self) {
        self.terminal.take();
        for path in self.terminal_temp_files.drain(..) {
            let _ = std::fs::remove_file(path);
        }
    }
}
