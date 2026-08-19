#[cfg(target_os = "macos")]
use crate::i18n::tr;
use crate::types::Message;

#[cfg(target_os = "macos")]
use std::cell::Cell;

#[cfg(target_os = "macos")]
use muda::{
    Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};

pub(super) struct NativeMenuState {
    #[cfg(target_os = "macos")]
    menu: Menu,
    #[cfg(target_os = "macos")]
    window_menu: Submenu,
    #[cfg(target_os = "macos")]
    edit_menu: Submenu,
    #[cfg(target_os = "macos")]
    installed: Cell<bool>,
    #[cfg(target_os = "macos")]
    new_project_id: MenuId,
    #[cfg(target_os = "macos")]
    open_project_id: MenuId,
    #[cfg(target_os = "macos")]
    undo_id: MenuId,
    #[cfg(target_os = "macos")]
    redo_id: MenuId,
    #[cfg(target_os = "macos")]
    save_project_id: MenuId,
    #[cfg(target_os = "macos")]
    settings_id: MenuId,
    #[cfg(target_os = "macos")]
    about_id: MenuId,
    #[cfg(target_os = "macos")]
    new_project_item: MenuItem,
    #[cfg(target_os = "macos")]
    open_project_item: MenuItem,
    #[cfg(target_os = "macos")]
    undo_item: MenuItem,
    #[cfg(target_os = "macos")]
    redo_item: MenuItem,
    #[cfg(target_os = "macos")]
    save_project_item: MenuItem,
    #[cfg(target_os = "macos")]
    settings_item: MenuItem,
    #[cfg(target_os = "macos")]
    about_item: MenuItem,
}

impl NativeMenuState {
    pub(super) fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            let menu = Menu::new();

            let app_menu = Submenu::new("Gridvana", true);
            let edit_menu = Submenu::new(tr("Edit", "编辑"), true);
            let window_menu = Submenu::new(tr("Window", "窗口"), true);

            let new_project = MenuItem::with_id(
                "gridvana.file.new",
                tr("New Canvas", "创建画布"),
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyN)),
            );
            let open_project = MenuItem::with_id(
                "gridvana.file.open",
                tr("Open…", "打开…"),
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyO)),
            );
            let save_project = MenuItem::with_id(
                "gridvana.file.save",
                tr("Save", "保存"),
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyS)),
            );
            let settings = MenuItem::with_id(
                "gridvana.settings",
                tr("Settings…", "设置…"),
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::Comma)),
            );
            let about = MenuItem::with_id(
                "gridvana.about",
                tr("About Gridvana", "关于 Gridvana"),
                true,
                None,
            );
            let undo = MenuItem::with_id(
                "gridvana.edit.undo",
                tr("Undo", "撤销"),
                true,
                Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyZ)),
            );
            let redo = MenuItem::with_id(
                "gridvana.edit.redo",
                tr("Redo", "重做"),
                true,
                Some(Accelerator::new(
                    Some(Modifiers::SUPER | Modifiers::SHIFT),
                    Code::KeyZ,
                )),
            );
            app_menu
                .append_items(&[
                    &new_project,
                    &open_project,
                    &save_project,
                    &PredefinedMenuItem::separator(),
                    &settings,
                    &PredefinedMenuItem::separator(),
                    &about,
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::services(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::hide(None),
                    &PredefinedMenuItem::hide_others(None),
                    &PredefinedMenuItem::show_all(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::quit(None),
                ])
                .expect("failed to build macOS app menu");

            window_menu
                .append_items(&[
                    &PredefinedMenuItem::minimize(None),
                    &PredefinedMenuItem::maximize(None),
                    &PredefinedMenuItem::close_window(None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::bring_all_to_front(None),
                ])
                .expect("failed to build macOS window menu");

            // NOTE: Do not add PredefinedMenuItem::cut/copy/paste/select_all here.
            // On macOS those install system Cmd+X/C/V/A accelerators routed via
            // the AppKit `paste:`/`copy:` responder actions, which only work for
            // native NSText controls. They would swallow Cmd+V before it reaches
            // iced's text_input (which handles paste itself), breaking paste in
            // all our text fields.
            edit_menu
                .append_items(&[&undo, &redo])
                .expect("failed to build macOS edit menu");

            menu.append_items(&[&app_menu, &edit_menu, &window_menu])
                .expect("failed to attach macOS root menus");

            Self {
                menu,
                window_menu,
                edit_menu,
                installed: Cell::new(false),
                new_project_id: new_project.id().clone(),
                open_project_id: open_project.id().clone(),
                undo_id: undo.id().clone(),
                redo_id: redo.id().clone(),
                save_project_id: save_project.id().clone(),
                settings_id: settings.id().clone(),
                about_id: about.id().clone(),
                new_project_item: new_project,
                open_project_item: open_project,
                undo_item: undo,
                redo_item: redo,
                save_project_item: save_project,
                settings_item: settings,
                about_item: about,
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Self {}
        }
    }

    pub(super) fn poll_message(&self) -> Option<Message> {
        #[cfg(target_os = "macos")]
        {
            self.ensure_installed();

            let event = MenuEvent::receiver().try_recv().ok()?;

            if event.id == self.new_project_id {
                Some(Message::OpenNewProjectDialog)
            } else if event.id == self.open_project_id {
                Some(Message::OpenProject)
            } else if event.id == self.undo_id {
                Some(Message::Undo)
            } else if event.id == self.redo_id {
                Some(Message::Redo)
            } else if event.id == self.save_project_id {
                Some(Message::SaveProject)
            } else if event.id == self.settings_id {
                Some(Message::OpenCliSettings)
            } else if event.id == self.about_id {
                Some(Message::OpenAbout)
            } else {
                None
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub(super) fn set_language(&self) {
        #[cfg(target_os = "macos")]
        {
            self.edit_menu.set_text(tr("Edit", "编辑"));
            self.window_menu.set_text(tr("Window", "窗口"));
            self.new_project_item.set_text(tr("New Canvas", "创建画布"));
            self.open_project_item.set_text(tr("Open…", "打开…"));
            self.save_project_item.set_text(tr("Save", "保存"));
            self.settings_item.set_text(tr("Settings…", "设置…"));
            self.about_item
                .set_text(tr("About Gridvana", "关于 Gridvana"));
            self.undo_item.set_text(tr("Undo", "撤销"));
            self.redo_item.set_text(tr("Redo", "重做"));
        }
    }

    #[cfg(target_os = "macos")]
    fn ensure_installed(&self) {
        if self.installed.get() {
            return;
        }

        self.menu.init_for_nsapp();
        self.window_menu.set_as_windows_menu_for_nsapp();
        self.installed.set(true);
    }
}
