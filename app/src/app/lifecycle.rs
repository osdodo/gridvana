use super::ui::color_to_hex;
use super::{
    DEFAULT_AI_INSPECTOR_WIDTH, DEFAULT_INSPECTOR_WIDTH, Gridvana, MIN_BRUSH_SIZE, MIN_ERASER_SIZE,
    PendingRecovery, SpriteSheetExportForm,
};
use crate::cli_terminal::CliConfig;
use crate::i18n::{AppPreferences, set_current_language, tr};
use crate::types::{ColorSlot, InspectorPanel, Message, Tool};
use gridvana_core::grid::GridIndex;
use gridvana_core::history::History;
use gridvana_core::model::{Project, Rgba};
use gridvana_core::transform::PixelTransform;
use iced::keyboard::{
    self, Modifiers,
    key::{Key, Named},
};
use iced::{Subscription, Task, Theme};
use std::collections::HashSet;
use std::path::PathBuf;

const AUTOSAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
// Batch PTY output so xterm does not render partial frames from a single TUI redraw.
#[cfg(target_os = "windows")]
const TERMINAL_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
#[cfg(not(target_os = "windows"))]
const TERMINAL_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

fn recovery_file_path() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("gridvana")
        .join("autosave.recovery")
}

impl Gridvana {
    pub fn new() -> (Self, Task<Message>) {
        let (preferences, language_save_error) = match AppPreferences::load() {
            Ok(preferences) => (preferences, None),
            Err(error) => (AppPreferences::default(), Some(error)),
        };
        let language = preferences.language;
        set_current_language(language);
        let project = Project::new_square(20.0, 0, 0);
        let (mcp_service, mcp_status) =
            match crate::mcp::EmbeddedMcpService::start(&project, std::iter::empty()) {
                Ok(service) => {
                    let status = format!(
                        "{} · {}",
                        tr("MCP connected", "MCP 已连接"),
                        service.endpoint()
                    );
                    (Some(service), status)
                }
                Err(error) => (
                    None,
                    format!("{} · {error}", tr("MCP did not start", "MCP 未启动")),
                ),
            };
        let (cli_config, cli_status) = match CliConfig::load() {
            Ok(config) => {
                let status = format!("{} · {}", tr("Current terminal", "当前终端"), config.agent);
                (config, status)
            }
            Err(error) => (CliConfig::default(), error),
        };
        let cli_config_draft = cli_config.clone();
        let terminal_webview =
            iced_wry::WebViewController::new(crate::web_terminal::webview_config());
        let recovery_file_path = recovery_file_path();
        let pending_recovery = match recovery_file_path.try_exists() {
            Ok(true) => Some(
                gridvana_core::recovery::load_recovery_file(&recovery_file_path)
                    .map(Box::new)
                    .map(PendingRecovery::Available)
                    .unwrap_or_else(PendingRecovery::Damaged),
            ),
            Ok(false) => None,
            Err(error) => Some(PendingRecovery::Damaged(format!(
                "{} {}: {error}",
                tr("Could not inspect recovery file", "无法检查恢复文件"),
                recovery_file_path.display()
            ))),
        };

        (
            Self {
                project,
                ai_preview_project: None,
                mcp_service,
                mcp_status,
                terminal_session: None,
                terminal_webview,
                terminal_webview_ready: false,
                terminal_page_ready: false,
                terminal_size: None,
                cli_settings_open: false,
                settings_section: crate::types::SettingsSection::General,
                preferences,
                language,
                language_save_error,
                about_dialog_open: false,
                cli_config,
                cli_config_draft,
                cli_status,
                cli_save_error: None,
                cli_test_in_flight: false,
                mcp_copy_feedback: None,
                inspector_panel: InspectorPanel::Layers,
                inspector_width: DEFAULT_INSPECTOR_WIDTH,
                ai_inspector_width: DEFAULT_AI_INSPECTOR_WIDTH,
                inspector_resize: None,
                native_menu: super::native_menu::NativeMenuState::new(),
                history: History::new(),
                current_tool: Tool::Brush,
                brush_size: MIN_BRUSH_SIZE,
                eraser_size: MIN_ERASER_SIZE,
                active_color_slot: ColorSlot::Foreground,
                current_color_hex_input: color_to_hex(Rgba::WHITE),
                recent_colors: Vec::new(),
                is_playing: false,
                onion_skin_enabled: true,
                onion_skin_settings: crate::canvas::OnionSkinSettings::default(),
                playback_last_tick: None,
                playback_elapsed: std::time::Duration::ZERO,
                playback_sequence_index: 0,
                timeline_drag: None,
                timeline_selection: HashSet::new(),
                timeline_selection_anchor: None,
                timeline_cel_clipboard: None,
                cel_context_menu: None,
                selection_context_menu: None,
                canvas_context_menu: None,
                canvas_size_popover_open: false,
                app_menu_open: false,
                current_stroke: None,
                current_shape: None,
                shape_preview_indices: Vec::new(),
                has_canvas: false,
                new_project_dialog_open: false,
                pending_sprite_sheet_export_path: None,
                sprite_sheet_export_form: SpriteSheetExportForm::default(),
                sprite_sheet_export_estimate: Err(tr(
                    "No export path selected",
                    "尚未选择导出路径",
                )
                .to_string()),
                last_export_summary: None,
                new_project_width: "64".to_string(),
                new_project_height: "64".to_string(),
                project_path: None,
                is_saved: false,
                recovery_file_path,
                pending_recovery,
                autosave_error: None,
                cursor_position: None,
                global_left_button_down: false,
                space_pressed: false,
                shift_pressed: false,
                alt_pressed: false,
                zoom_modifier_pressed: false,
                hovered_grid_index: None,
                preview_visible: true,
                preview_offset: iced::Point::ORIGIN,
                preview_drag: None,
                move_mode_active: false,
                selection_indices: HashSet::new(),
                selection_box_draft: None,
                selection_move_draft: None,
                selection_clipboard: None,
                floating_selection: None,
                paste_offset: GridIndex { x: 1, y: 1 },
                resize_canvas_width: "0".to_string(),
                resize_canvas_height: "0".to_string(),
            },
            iced::window::oldest().map(Message::TerminalHostWindow),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_events = iced::event::listen_with(|event, status, _id| {
            if let iced::Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
                return Some(Message::UpdateKeyboardModifiers {
                    shift_pressed: modifiers.shift(),
                    alt_pressed: modifiers.alt(),
                    zoom_modifier_pressed: modifiers.contains(Modifiers::COMMAND),
                });
            }

            if let iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            )) = event
            {
                return Some(Message::GlobalLeftPressed);
            }

            if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) = event {
                return Some(Message::UpdateCursorPosition(position));
            }

            if let iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )) = event
            {
                return Some(Message::GlobalLeftReleased);
            }

            if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Space),
                modifiers,
                ..
            }) = event
                && status == iced::event::Status::Ignored
                && !modifiers.contains(Modifiers::COMMAND)
            {
                return Some(Message::SetSpacePressed(true));
            }

            if let iced::Event::Keyboard(keyboard::Event::KeyReleased {
                key: Key::Named(Named::Space),
                ..
            }) = event
            {
                return Some(Message::SetSpacePressed(false));
            }

            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
                && status == iced::event::Status::Ignored
            {
                match key {
                    Key::Named(Named::Escape) => return Some(Message::DeactivateMoveSelection),
                    #[cfg(not(target_os = "macos"))]
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("n")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::OpenNewProjectDialog);
                    }
                    #[cfg(not(target_os = "macos"))]
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("o")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::OpenProject);
                    }
                    #[cfg(not(target_os = "macos"))]
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("s")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::SaveProject);
                    }
                    #[cfg(not(target_os = "macos"))]
                    Key::Character(c)
                        if c.as_str() == "," && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::OpenCliSettings);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("a")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::SelectAllPixels);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("i")
                            && modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) =>
                    {
                        return Some(Message::InvertPixelSelection);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("x")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::CutPixelSelection);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("c")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::CopySelection);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("d")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::DuplicateSelection);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("v")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::PasteSelection);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("z")
                            && modifiers.contains(Modifiers::COMMAND | Modifiers::SHIFT) =>
                    {
                        return Some(Message::Redo);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("y")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::Redo);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("z")
                            && modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::Undo);
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("h")
                            && modifiers.contains(Modifiers::SHIFT)
                            && !modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::TransformPixelSelectionSequence(vec![
                            PixelTransform::FlipHorizontal,
                        ]));
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("v")
                            && modifiers.contains(Modifiers::SHIFT)
                            && !modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::TransformPixelSelectionSequence(vec![
                            PixelTransform::FlipVertical,
                        ]));
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("r")
                            && modifiers.contains(Modifiers::SHIFT)
                            && !modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::TransformPixelSelectionSequence(vec![
                            PixelTransform::RotateClockwise,
                        ]));
                    }
                    Key::Character(c)
                        if c.as_str().eq_ignore_ascii_case("l")
                            && modifiers.contains(Modifiers::SHIFT)
                            && !modifiers.contains(Modifiers::COMMAND) =>
                    {
                        return Some(Message::TransformPixelSelectionSequence(vec![
                            PixelTransform::RotateCounterClockwise,
                        ]));
                    }
                    Key::Named(Named::Backspace) | Key::Named(Named::Delete) => {
                        return Some(Message::DeletePixelSelection);
                    }
                    Key::Named(Named::ArrowLeft) => return Some(Message::MoveSelectionBy(-1, 0)),
                    Key::Named(Named::ArrowRight) => return Some(Message::MoveSelectionBy(1, 0)),
                    Key::Named(Named::ArrowUp) => return Some(Message::MoveSelectionBy(0, -1)),
                    Key::Named(Named::ArrowDown) => return Some(Message::MoveSelectionBy(0, 1)),
                    _ => {}
                }
            }
            None
        });

        let mut subscriptions = vec![keyboard_events];

        subscriptions.push(iced::time::every(AUTOSAVE_INTERVAL).map(|_| Message::AutosaveTick));

        if self.mcp_service.is_some() {
            subscriptions.push(
                iced::time::every(std::time::Duration::from_millis(50))
                    .map(|_| Message::PollMcpServer),
            );
        }

        subscriptions.push(
            self.terminal_webview
                .ipc_subscription()
                .map(Message::TerminalWebViewIpc),
        );

        if self.terminal_session.is_some() {
            subscriptions
                .push(iced::time::every(TERMINAL_POLL_INTERVAL).map(|_| Message::PollCliTerminal));
        }

        #[cfg(target_os = "macos")]
        subscriptions.push(
            iced::time::every(std::time::Duration::from_millis(50))
                .map(|_| Message::PollNativeMenu),
        );

        if self.is_playing {
            subscriptions
                .push(iced::time::every(std::time::Duration::from_millis(16)).map(Message::Tick));
        }

        Subscription::batch(subscriptions)
    }

    pub fn theme(&self) -> Theme {
        super::ui::app_theme()
    }
}
