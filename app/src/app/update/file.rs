use super::super::Gridvana;
use super::super::PendingRecovery;
use super::super::ui::normalize_project_save_path;
use crate::i18n::tr;
use crate::types::{
    InspectorPanel, Message, SpriteEmptyChoice, SpriteFrameRangeChoice, SpriteLayerRangeChoice,
    SpriteLayoutChoice, SpriteMetadataChoice, SpriteTrimChoice,
};
use gridvana_core::sprite_sheet::{
    EmptyFramePolicy, ExportOptions, FrameRange, LayerRange, MetadataFormat, SheetLayout, TrimMode,
};
use iced::Task;
use rfd::{FileDialog, MessageDialog, MessageLevel};
use std::path::{Path, PathBuf};

fn autosave_dirty_project(
    project: &gridvana_core::model::Project,
    project_path: Option<&Path>,
    has_canvas: bool,
    is_saved: bool,
    recovery_path: &Path,
) -> Result<bool, String> {
    if !has_canvas || is_saved {
        return Ok(false);
    }

    gridvana_core::recovery::save_recovery_file(project, project_path, recovery_path)?;
    Ok(true)
}

fn show_open_error(path: &Path, error: String) {
    let _ = MessageDialog::new()
        .set_title(tr("Open File Failed", "打开文件失败"))
        .set_level(MessageLevel::Error)
        .set_description(format!(
            "{}\n\n{}: {error}",
            path.display(),
            tr("Error", "错误")
        ))
        .show();
}

fn choose_export_path(filter_name: &str, extension: &str, file_name: &str) -> Option<PathBuf> {
    FileDialog::new()
        .add_filter(filter_name, &[extension])
        .set_file_name(file_name)
        .save_file()
        .map(|mut path| {
            if path
                .extension()
                .and_then(|path_extension| path_extension.to_str())
                .is_none_or(|path_extension| !path_extension.eq_ignore_ascii_case(extension))
            {
                path.set_extension(extension);
            }
            path
        })
}

fn choose_sprite_sheet_export_path(current_path: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = FileDialog::new().set_title(tr(
        "Choose sprite sheet export directory",
        "选择精灵表导出目录",
    ));
    if let Some(directory) = current_path.and_then(Path::parent) {
        dialog = dialog.set_directory(directory);
    }
    dialog
        .pick_folder()
        .map(|directory| directory.join("spritesheet.png"))
}

fn show_export_error(format: &str, error: String) {
    let _ = MessageDialog::new()
        .set_title(format!(
            "{} {format} {}",
            tr("Export", "导出"),
            tr("Failed", "失败")
        ))
        .set_level(MessageLevel::Error)
        .set_description(format!("{}: {error}", tr("Error", "错误")))
        .show();
}

impl Gridvana {
    pub(super) fn handle_file_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        match message {
            Message::AutosaveTick => {
                match autosave_dirty_project(
                    &self.project,
                    self.project_path.as_deref(),
                    self.has_canvas,
                    self.is_saved,
                    &self.recovery_file_path,
                ) {
                    Ok(true) => self.autosave_error = None,
                    Ok(false) => {}
                    Err(error) => self.autosave_error = Some(error),
                }
                Ok(Task::none())
            }
            Message::RecoverAutosave => {
                match self.pending_recovery.take() {
                    Some(PendingRecovery::Available(recovery)) => {
                        let recovery = *recovery;
                        self.load_project_into_editor(
                            recovery.project,
                            recovery.project_path,
                            false,
                        );
                        self.autosave_error = None;
                    }
                    pending => self.pending_recovery = pending,
                }
                Ok(Task::none())
            }
            Message::DiscardAutosave => {
                match gridvana_core::recovery::remove_recovery_file(&self.recovery_file_path) {
                    Ok(()) => {
                        self.pending_recovery = None;
                        self.autosave_error = None;
                    }
                    Err(error) => {
                        self.pending_recovery = Some(PendingRecovery::Damaged(error.clone()));
                        self.autosave_error = Some(error);
                    }
                }
                Ok(Task::none())
            }
            Message::OpenProject => {
                self.app_menu_open = false;
                self.new_project_dialog_open = false;
                self.pending_sprite_sheet_export_path = None;
                if let Some(path) = FileDialog::new()
                    .add_filter("Gridvana Project", &["gvn"])
                    .pick_file()
                {
                    match gridvana_core::io::load_project(&path) {
                        Ok(project) => {
                            self.load_project_into_editor(project, Some(path), true);
                        }
                        Err(error) => show_open_error(&path, error),
                    }
                }
                Ok(Task::none())
            }
            Message::SaveProject => {
                self.app_menu_open = false;
                if !self.has_canvas {
                    return Ok(Task::none());
                }
                let save_path = match &self.project_path {
                    Some(path) => Some(path.clone()),
                    None => {
                        let dialog = FileDialog::new().add_filter("Gridvana Project", &["gvn"]);
                        dialog
                            .set_file_name("project.gvn")
                            .save_file()
                            .map(normalize_project_save_path)
                    }
                };

                if let Some(path) = save_path {
                    match gridvana_core::io::save_project(&self.project, &path) {
                        Ok(_) => {
                            self.project_path = Some(path);
                            self.is_saved = true;
                            match gridvana_core::recovery::remove_recovery_file(
                                &self.recovery_file_path,
                            ) {
                                Ok(()) => self.autosave_error = None,
                                Err(error) => self.autosave_error = Some(error),
                            }
                        }
                        Err(error) => {
                            let _ = MessageDialog::new()
                                .set_title(tr("Save Project Failed", "保存项目失败"))
                                .set_level(MessageLevel::Error)
                                .set_description(format!("{}: {}", tr("Error", "错误"), error))
                                .show();
                        }
                    }
                }
                Ok(Task::none())
            }
            Message::ExportGif => {
                self.app_menu_open = false;
                if !self.has_canvas {
                    return Ok(Task::none());
                }
                if let Some(path) = choose_export_path("GIF Animation", "gif", "animation.gif") {
                    match gridvana_core::io::export_gif(&self.project, &path) {
                        Ok(()) => {
                            let name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            self.last_export_summary =
                                Some(format!("{} {name}", tr("Exported", "已导出")));
                        }
                        Err(error) => show_export_error("GIF", error),
                    }
                }
                Ok(Task::none())
            }
            Message::ExportPngSequence => {
                self.app_menu_open = false;
                if !self.has_canvas {
                    return Ok(Task::none());
                }
                if let Some(path) = choose_export_path("PNG Image", "png", "frame.png") {
                    match gridvana_core::io::export_png_sequence(&self.project, &path) {
                        Ok(()) => {
                            let stem = path
                                .file_stem()
                                .map(|stem| stem.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let count = self.project.frames.len();
                            self.last_export_summary = Some(format!(
                                "{} {count} {} {stem}_*.png",
                                tr("Exported", "已导出"),
                                tr("files", "张")
                            ));
                        }
                        Err(error) => show_export_error("PNG Sequence", error),
                    }
                }
                Ok(Task::none())
            }
            Message::ExportSpriteSheet => {
                self.app_menu_open = false;
                self.inspector_panel = InspectorPanel::Export;
                self.sync_terminal_webview_visibility();
                if !self.has_canvas {
                    return Ok(Task::none());
                }
                if let Some(path) = choose_sprite_sheet_export_path(
                    self.pending_sprite_sheet_export_path.as_deref(),
                ) {
                    self.pending_sprite_sheet_export_path = Some(path);
                    self.refresh_sprite_sheet_export_estimate();
                }
                Ok(Task::none())
            }
            Message::SelectSpriteFrameRange(value) => {
                self.sprite_sheet_export_form.frame_range = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SelectSpriteLayerRange(value) => {
                self.sprite_sheet_export_form.layer_range = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SelectSpriteLayout(value) => {
                self.sprite_sheet_export_form.layout = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SelectSpriteTrim(value) => {
                self.sprite_sheet_export_form.trim = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SelectSpriteEmpty(value) => {
                self.sprite_sheet_export_form.empty = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SelectSpriteMetadata(value) => {
                self.sprite_sheet_export_form.metadata = value;
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteTrimPerFrame(enabled) => {
                self.sprite_sheet_export_form.trim = if enabled {
                    SpriteTrimChoice::PerFrame
                } else {
                    SpriteTrimChoice::None
                };
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteExtrudeEnabled(enabled) => {
                if enabled {
                    self.sprite_sheet_export_form.padding =
                        self.sprite_sheet_export_form.padding.max(1);
                    self.sprite_sheet_export_form.extrude = 1;
                } else {
                    self.sprite_sheet_export_form.extrude = 0;
                }
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteFixedCount(value) => {
                self.sprite_sheet_export_form.fixed_count = value.clamp(1, 16);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteScale(value) => {
                self.sprite_sheet_export_form.scale = value.clamp(1, 16);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpritePadding(value) => {
                self.sprite_sheet_export_form.padding = value.min(32);
                self.sprite_sheet_export_form.extrude = self
                    .sprite_sheet_export_form
                    .extrude
                    .min(self.sprite_sheet_export_form.padding);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteSpacing(value) => {
                self.sprite_sheet_export_form.spacing = value.min(32);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteBorder(value) => {
                self.sprite_sheet_export_form.border = value.min(32);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::SetSpriteExtrude(value) => {
                self.sprite_sheet_export_form.extrude =
                    value.min(self.sprite_sheet_export_form.padding);
                self.refresh_sprite_sheet_export_estimate();
                Ok(Task::none())
            }
            Message::CancelSpriteSheetExport => {
                self.pending_sprite_sheet_export_path = None;
                Ok(Task::none())
            }
            Message::ConfirmSpriteSheetExport => {
                if let Some(png_path) = self.pending_sprite_sheet_export_path.clone() {
                    let mut json_path = png_path.clone();
                    json_path.set_extension("json");
                    let options = self.sprite_sheet_export_options();
                    match gridvana_core::sprite_sheet::export_sprite_sheet_files(
                        &self.project,
                        &png_path,
                        &json_path,
                        &options,
                    ) {
                        Ok(()) => {
                            let png_name = png_path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            let json_name = json_path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            self.last_export_summary = Some(format!(
                                "{} {png_name} {} {json_name}",
                                tr("Exported", "已导出"),
                                tr("and", "与")
                            ));
                        }
                        Err(error) => show_export_error("Sprite Sheet + JSON", error),
                    }
                }
                Ok(Task::none())
            }
            other => Err(other),
        }
    }

    pub(in crate::app) fn sprite_sheet_export_options(&self) -> ExportOptions {
        let form = self.sprite_sheet_export_form;
        ExportOptions {
            frames: match form.frame_range {
                SpriteFrameRangeChoice::All => FrameRange::All,
                SpriteFrameRangeChoice::ActiveTag => FrameRange::ActiveTag,
            },
            layers: match form.layer_range {
                SpriteLayerRangeChoice::Visible => LayerRange::Visible,
                SpriteLayerRangeChoice::All => LayerRange::All,
                SpriteLayerRangeChoice::Active => LayerRange::Single(self.project.active_layer_id),
            },
            layout: match form.layout {
                SpriteLayoutChoice::Horizontal => SheetLayout::Horizontal,
                SpriteLayoutChoice::Vertical => SheetLayout::Vertical,
                SpriteLayoutChoice::FixedRows => {
                    SheetLayout::FixedRows(u32::from(form.fixed_count))
                }
                SpriteLayoutChoice::FixedColumns => {
                    SheetLayout::FixedColumns(u32::from(form.fixed_count))
                }
            },
            scale: u32::from(form.scale),
            padding: u32::from(form.padding),
            spacing: u32::from(form.spacing),
            border: u32::from(form.border),
            trim: match form.trim {
                SpriteTrimChoice::None => TrimMode::None,
                SpriteTrimChoice::Sprite => TrimMode::Sprite,
                SpriteTrimChoice::PerFrame => TrimMode::PerFrame,
            },
            empty_frames: match form.empty {
                SpriteEmptyChoice::Include => EmptyFramePolicy::Include,
                SpriteEmptyChoice::Skip => EmptyFramePolicy::Skip,
                SpriteEmptyChoice::Error => EmptyFramePolicy::Error,
            },
            extrude: u32::from(form.extrude),
            metadata_format: match form.metadata {
                SpriteMetadataChoice::Array => MetadataFormat::Array,
                SpriteMetadataChoice::Hash => MetadataFormat::Hash,
            },
        }
    }

    pub(super) fn refresh_sprite_sheet_export_estimate(&mut self) {
        self.last_export_summary = None;
        self.sprite_sheet_export_estimate = gridvana_core::sprite_sheet::build_sprite_sheet(
            &self.project,
            &self.sprite_sheet_export_options(),
        )
        .map(|export| (export.width, export.height));
    }
}

#[cfg(test)]
mod tests {
    use super::autosave_dirty_project;
    use gridvana_core::grid::GridIndex;
    use gridvana_core::model::{Project, Rgba};
    use gridvana_core::recovery::load_recovery_file;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_path(extension: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "gridvana-open-file-{}-{unique}.{extension}",
            std::process::id()
        ))
    }

    #[test]
    fn autosave_writes_only_a_dirty_committed_project() {
        let recovery_path = temporary_path("recovery");
        let formal_path = temporary_path("gvn");
        let mut committed = Project::new_square(1.0, 3, 3);
        committed
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 0, y: 0 }, Rgba::WHITE);
        let mut uncommitted_preview = committed.clone();
        uncommitted_preview
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 2, y: 2 }, Rgba::BLACK);

        assert!(
            !autosave_dirty_project(&committed, Some(&formal_path), true, true, &recovery_path,)
                .unwrap()
        );
        assert!(!recovery_path.exists());
        assert!(
            !autosave_dirty_project(&committed, Some(&formal_path), false, false, &recovery_path,)
                .unwrap()
        );
        assert!(!recovery_path.exists());

        assert!(
            autosave_dirty_project(&committed, Some(&formal_path), true, false, &recovery_path,)
                .unwrap()
        );
        let recovered = load_recovery_file(&recovery_path).unwrap();
        assert_eq!(recovered.project, committed);
        assert_ne!(recovered.project, uncommitted_preview);
        assert_eq!(recovered.project_path, Some(formal_path));

        std::fs::remove_file(recovery_path).unwrap();
    }
}
