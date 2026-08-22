use super::super::Gridvana;
use super::super::geometry::parse_canvas_size;
use crate::types::Message;
use gridvana_core::document::DocumentOp;
use gridvana_core::history::History;
use gridvana_core::model::Project;
use iced::Task;

impl Gridvana {
    pub(super) fn handle_project_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        match message {
            Message::ToggleAppMenu => {
                self.app_menu_open = !self.app_menu_open;
                Ok(Task::none())
            }
            Message::CloseAppMenu => {
                self.app_menu_open = false;
                Ok(Task::none())
            }
            Message::OpenAbout => {
                self.app_menu_open = false;
                self.cli_settings_open = false;
                self.new_project_dialog_open = false;
                self.pending_sprite_sheet_export_path = None;
                self.about_dialog_open = true;
                self.global_left_button_down = false;
                Ok(Task::none())
            }
            Message::CloseAbout => {
                self.about_dialog_open = false;
                self.global_left_button_down = false;
                Ok(Task::none())
            }
            Message::OpenNewProjectDialog => {
                self.app_menu_open = false;
                self.about_dialog_open = false;
                self.pending_sprite_sheet_export_path = None;
                self.new_project_dialog_open = true;
                self.global_left_button_down = false;
                Ok(Task::none())
            }
            Message::CloseNewProjectDialog => {
                self.new_project_dialog_open = false;
                self.global_left_button_down = false;
                Ok(Task::none())
            }
            Message::UpdateCanvasWidth(value) => {
                self.new_project_width = value;
                Ok(Task::none())
            }
            Message::UpdateCanvasHeight(value) => {
                self.new_project_height = value;
                Ok(Task::none())
            }
            Message::CreateNewProject => {
                self.app_menu_open = false;
                self.new_project_dialog_open = false;
                self.global_left_button_down = false;
                let width = parse_canvas_size(&self.new_project_width, 64);
                let height = parse_canvas_size(&self.new_project_height, 64);
                self.project = Project::new_square(20.0, width, height);
                self.history = History::new();
                self.current_stroke = None;
                self.current_shape = None;
                self.shape_preview_indices.clear();
                self.clear_selection_state();
                self.is_playing = false;
                self.playback_last_tick = None;
                self.playback_elapsed = std::time::Duration::ZERO;
                self.playback_sequence_index = 0;
                self.timeline_drag = None;
                self.clear_timeline_selection();
                self.has_canvas = true;
                self.ensure_mcp_service_started();
                self.project_path = None;
                self.normalize_project_state();
                self.is_saved = false;
                Ok(Task::none())
            }
            Message::Undo => {
                self.app_menu_open = false;
                if self.has_canvas && self.history.undo(&mut self.project) {
                    self.normalize_project_state();
                    self.is_saved = false;
                }
                Ok(Task::none())
            }
            Message::Redo => {
                self.app_menu_open = false;
                if self.has_canvas && self.history.redo(&mut self.project) {
                    self.normalize_project_state();
                    self.is_saved = false;
                }
                Ok(Task::none())
            }
            Message::ToggleSymmetryX => {
                if self.has_canvas {
                    let mut line = self.project.symmetry_x;
                    line.active = !line.active;
                    self.apply_document_transaction(vec![DocumentOp::SetSymmetryX { line }]);
                }
                Ok(Task::none())
            }
            Message::ToggleSymmetryY => {
                if self.has_canvas {
                    let mut line = self.project.symmetry_y;
                    line.active = !line.active;
                    self.apply_document_transaction(vec![DocumentOp::SetSymmetryY { line }]);
                }
                Ok(Task::none())
            }
            Message::UpdateSymmetryX(x) => {
                if self.has_canvas {
                    let mut line = self.project.symmetry_x;
                    line.position = x;
                    self.apply_document_transaction(vec![DocumentOp::SetSymmetryX { line }]);
                }
                Ok(Task::none())
            }
            Message::UpdateSymmetryY(y) => {
                if self.has_canvas {
                    let mut line = self.project.symmetry_y;
                    line.position = y;
                    self.apply_document_transaction(vec![DocumentOp::SetSymmetryY { line }]);
                }
                Ok(Task::none())
            }
            other => Err(other),
        }
    }
}
