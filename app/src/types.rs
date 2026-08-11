use crate::cli_terminal::CliAgent;
use gridvana_core::grid::GridIndex;
use gridvana_core::model::{BlendMode, FrameId, LayerId, LayerKind, Rgba, TagId};
use gridvana_core::transform::PixelTransform;
use iced::Point;

macro_rules! export_choice {
    ($name:ident, [$($variant:ident => $label:literal),+ $(,)?]) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: [Self; export_choice!(@count $($variant),+)] = [$(Self::$variant),+];
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let label = match self { $(Self::$variant => $label),+ };
                formatter.write_str(label)
            }
        }
    };
    (@count $($variant:ident),+) => { <[()]>::len(&[$(export_choice!(@unit $variant)),+]) };
    (@unit $variant:ident) => { () };
}

export_choice!(SpriteFrameRangeChoice, [All => "全部帧", ActiveTag => "活动标签"]);
export_choice!(SpriteLayerRangeChoice, [Visible => "可见图层", All => "全部图层", Active => "活动图层"]);
export_choice!(SpriteLayoutChoice, [Horizontal => "横向", Vertical => "纵向", FixedRows => "固定行数", FixedColumns => "固定列数"]);
export_choice!(SpriteTrimChoice, [None => "不裁切", Sprite => "整体裁切", PerFrame => "逐帧裁切"]);
export_choice!(SpriteEmptyChoice, [Include => "保留空帧", Skip => "跳过空帧", Error => "遇空帧报错"]);
export_choice!(SpriteMetadataChoice, [Array => "JSON Array", Hash => "JSON Hash"]);
export_choice!(SelectionCombineMode, [Replace => "替换", Add => "相加", Subtract => "相减", Intersect => "相交"]);
export_choice!(TransformTargetChoice, [CurrentCel => "当前 Cel", SelectedCels => "选中 Cel", CompositedFrame => "合成帧"]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSlot {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPanel {
    Layers,
    AiAgent,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Agent,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    HandPoint,
    MagicWand,
    ColorSelect,
    Brush,
    Eraser,
    PaintBucket,
    Picker,
    Rectangle,
    RectangleHollow,
    Circle,
    CircleHollow,
    Line,
}

#[derive(Debug, Clone)]
pub enum Message {
    CanvasEvent,
    SetInspectorPanel(InspectorPanel),
    OpenCliSettings,
    CloseCliSettings,
    SelectSettingsSection(SettingsSection),
    OpenAbout,
    CloseAbout,
    SelectCliAgent(CliAgent),
    SetAiPanelEnabled(bool),
    SetCliDefaultAllow(bool),
    UpdateCodexCommand(String),
    UpdateCodexProfile(String),
    UpdateCodexModel(String),
    UpdateClaudeCommand(String),
    UpdateClaudeModel(String),
    UpdateClaudeEffort(String),
    CopyMcpEndpoint,
    CopyMcpAgentPrompt,
    SaveCliConfig,
    TestCliConnection,
    CliConnectionTestFinished(Result<String, String>),
    StartCliTerminal,
    TerminalHostWindow(Option<iced::window::Id>),
    TerminalWebViewReady(Result<(), String>),
    TerminalWebViewIpc(iced_wry::IpcMessage),
    PollCliTerminal,
    PollMcpServer,
    StrokeStart(GridIndex, ColorSlot),
    StrokeAdd(GridIndex),
    StrokeEnd,
    DeactivateMoveSelection,
    SetSpacePressed(bool),
    CopySelection,
    PasteSelection,
    DuplicateSelection,
    SelectAllPixels,
    InvertPixelSelection,
    ClearPixelSelection,
    DeletePixelSelection,
    CutPixelSelection,
    SetSelectionCombineMode(SelectionCombineMode),
    SetTransformTarget(TransformTargetChoice),
    SetTransformScale(u8),
    TransformPixelSelection(PixelTransform),
    CropCanvasToSelection,
    TrimCanvas,
    UpdateResizeCanvasWidth(String),
    UpdateResizeCanvasHeight(String),
    ResizeCurrentCanvas,
    MoveSelectionBy(i32, i32),
    UpdateHoveredGridIndex(Option<GridIndex>),
    PickColor(GridIndex, ColorSlot),
    SelectTool(Tool),
    UpdateBrushSize(u8),
    UpdateEraserSize(u8),
    SelectColor(Rgba),
    SetActiveColorSlot(ColorSlot),
    SwapForegroundBackground,
    UpdateColorHexInput(String),
    SubmitColorHexInput,
    UpdateColorValue(u8),
    UpdateColorAlpha(u8),
    UpdateCanvasWidth(String),
    UpdateCanvasHeight(String),
    BeginInspectorResize,
    EndInspectorResize,
    ToggleAppMenu,
    PollNativeMenu,
    OpenNewProjectDialog,
    CloseNewProjectDialog,
    CreateNewProject,
    OpenProject,
    Undo,
    Redo,
    AddLayer,
    AddLayerGroup,
    RemoveLayer(LayerId),
    SelectLayer(LayerId),
    ToggleLayerVisibility(LayerId),
    ToggleLayerLocked(LayerId),
    SetLayerOpacity(LayerId, u8),
    SetLayerBlendMode(LayerId, BlendMode),
    SetLayerKind(LayerId, LayerKind),
    SetLayerParent(LayerId, Option<LayerId>),
    RenameLayer(LayerId, String),
    AddFrame,
    DuplicateFrame,
    RemoveFrame(FrameId),
    SelectFrame(FrameId),
    SetFrameDuration(FrameId, String),
    BeginFrameDrag(FrameId),
    BeginLayerDrag(LayerId),
    HoverTimelineDrag(usize),
    FinishTimelineDrag,
    BeginCelDrag(LayerId, FrameId),
    HoverCelDrag(LayerId, FrameId),
    OpenCelContextMenu(LayerId, FrameId),
    CloseCelContextMenu,
    CopyTimelineCels,
    PasteTimelineCels,
    DeleteTimelineCels,
    LinkTimelineCels,
    UnlinkTimelineCels,
    AddAnimationTag,
    RemoveAnimationTag(TagId),
    SelectAnimationTag(Option<TagId>),
    ToggleAnimationTagSelection(TagId),
    RenameAnimationTag(TagId, String),
    BeginTagRangeDrag(TagId, FrameId),
    HoverTagRangeDrag(FrameId),
    CycleAnimationTagDirection(TagId),
    TogglePlayback,
    ToggleOnionSkin,
    SetOnionPreviousFrames(u8),
    SetOnionNextFrames(u8),
    SetOnionOpacity(u8),
    ToggleOnionPreviousTint,
    ToggleOnionNextTint,
    ToggleOnionActiveLayerOnly,
    Tick(std::time::Instant),
    AutosaveTick,
    RecoverAutosave,
    DiscardAutosave,
    SaveProject,
    ExportGif,
    ExportPngSequence,
    ExportSpriteSheet,
    SelectSpriteFrameRange(SpriteFrameRangeChoice),
    SelectSpriteLayerRange(SpriteLayerRangeChoice),
    SelectSpriteLayout(SpriteLayoutChoice),
    SelectSpriteTrim(SpriteTrimChoice),
    SelectSpriteEmpty(SpriteEmptyChoice),
    SelectSpriteMetadata(SpriteMetadataChoice),
    SetSpriteTrimPerFrame(bool),
    SetSpriteExtrudeEnabled(bool),
    SetSpriteFixedCount(u8),
    SetSpriteScale(u8),
    SetSpritePadding(u8),
    SetSpriteSpacing(u8),
    SetSpriteBorder(u8),
    SetSpriteExtrude(u8),
    ConfirmSpriteSheetExport,
    CancelSpriteSheetExport,
    ToggleSymmetryX,
    ToggleSymmetryY,
    UpdateSymmetryX(f32),
    UpdateSymmetryY(f32),
    SelectCel(LayerId, FrameId),
    UpdateCursorPosition(Point),
    GlobalLeftPressed,
    GlobalLeftReleased,
    UpdateKeyboardModifiers {
        shift_pressed: bool,
        zoom_modifier_pressed: bool,
    },
}
