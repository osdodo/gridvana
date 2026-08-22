use super::super::Gridvana;
use crate::types::Message;
use iced::{Element, Length, widget};

impl Gridvana {
    pub fn view(&self) -> Element<'_, Message> {
        let base: Element<'_, Message> = if self.has_visible_canvas() {
            let shape_preview_hint = self.shape_preview_hint();
            let selection_display_indices = self.selection_display_indices();

            let tool_rail = self.editor_tool_rail();
            let canvas_with_preview = self.editor_canvas_with_preview(selection_display_indices);
            let inspector = self.editor_inspector();
            let main_workspace =
                self.editor_main_workspace(tool_rail, canvas_with_preview, inspector);
            let bottom_panel = self.editor_bottom_panel();
            let status_bar = self.editor_status_bar();
            let main_content = self.editor_main_content(main_workspace, bottom_panel, status_bar);
            let floating_hint_layer =
                self.editor_floating_hint_layer(shape_preview_hint.as_deref());

            widget::stack(vec![main_content, floating_hint_layer])
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .into()
        } else {
            let tool_rail = self.editor_tool_rail();
            let empty_canvas = self.editor_empty_canvas();
            let inspector = self.editor_inspector();
            let main_workspace = self.editor_main_workspace(tool_rail, empty_canvas, inspector);
            let bottom_panel = self.editor_empty_bottom_panel();
            let status_bar = self.editor_status_bar();

            self.editor_main_content(main_workspace, bottom_panel, status_bar)
        };

        #[cfg(windows)]
        let base = widget::column![self.window_title_bar(), base]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let mut layers = vec![base];

        #[cfg(any(windows, target_os = "linux"))]
        if let Some(app_menu) = self.app_menu_overlay() {
            layers.push(app_menu);
        }

        if let Some(cel_context_menu) = self.cel_context_menu_overlay() {
            layers.push(cel_context_menu);
        }

        if let Some(selection_context_menu) = self.selection_context_menu_overlay() {
            layers.push(selection_context_menu);
        }

        if let Some(canvas_context_menu) = self.canvas_context_menu_overlay() {
            layers.push(canvas_context_menu);
        }

        if let Some(settings) = self.settings_overlay() {
            layers.push(settings);
        }

        if let Some(new_project_overlay) = self.new_project_overlay() {
            layers.push(new_project_overlay);
        }

        if let Some(sprite_sheet_overlay) = self.sprite_sheet_export_overlay() {
            layers.push(sprite_sheet_overlay);
        }

        if let Some(recovery_overlay) = self.recovery_overlay() {
            layers.push(recovery_overlay);
        }

        if layers.len() == 1 {
            layers.pop().unwrap()
        } else {
            widget::stack(layers)
                .width(Length::Fill)
                .height(Length::Fill)
                .clip(true)
                .into()
        }
    }
}
