mod canvas;
mod file;
mod mcp;
mod project;
mod terminal;
mod timeline;

use super::Gridvana;
use crate::types::Message;
use iced::Task;

impl Gridvana {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        if let Message::BeginWindowDrag = message {
            self.app_menu_open = false;
            return iced::window::oldest().and_then(iced::window::drag);
        }

        if let Message::ToggleWindowMaximized = message {
            self.app_menu_open = false;
            return iced::window::oldest().and_then(iced::window::toggle_maximize);
        }

        if let Message::MinimizeWindow = message {
            self.app_menu_open = false;
            return iced::window::oldest()
                .and_then(|window_id| iced::window::minimize(window_id, true));
        }

        if let Message::ExitApplication = message {
            return iced::exit();
        }

        if let Message::PollNativeMenu = message {
            if let Some(message) = self.native_menu.poll_message() {
                return self.update(message);
            }

            return Task::none();
        }

        let message = match self.handle_mcp_message(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.handle_terminal_message(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.handle_canvas_message(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.handle_project_message(message) {
            Ok(task) => return task,
            Err(message) => message,
        };
        let message = match self.handle_timeline_message(message) {
            Ok(task) => return task,
            Err(message) => message,
        };

        match self.handle_file_message(message) {
            Ok(task) => task,
            Err(_message) => Task::none(),
        }
    }
}
