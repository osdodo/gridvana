use crate::document::{DocumentOp, DocumentPixel, apply_document_op};
use crate::grid::GridIndex;
use crate::history::EditCommand;
use crate::model::{FrameId, LayerId, Project, Rgba};

#[derive(Debug)]
pub struct StrokeCommand {
    pixels: Vec<StrokePixel>,
    layer_id: LayerId,
    frame_id: FrameId,
    cel_existed_before: bool,
}

#[derive(Debug)]
pub struct StrokePixel {
    pub index: GridIndex,
    pub old_color: Option<Rgba>,
    pub new_color: Option<Rgba>,
}

impl StrokeCommand {
    pub fn new(
        frame_id: FrameId,
        layer_id: LayerId,
        pixels: Vec<StrokePixel>,
        cel_existed_before: bool,
    ) -> Self {
        Self {
            pixels,
            layer_id,
            frame_id,
            cel_existed_before,
        }
    }

    fn apply_colors<F>(&self, project: &mut Project, color: F)
    where
        F: Fn(&StrokePixel) -> Option<Rgba>,
    {
        let mut set_pixels = Vec::new();
        let mut erase_indices = Vec::new();
        for pixel in &self.pixels {
            match color(pixel) {
                Some(color) => set_pixels.push(DocumentPixel {
                    index: pixel.index,
                    color,
                }),
                None => erase_indices.push(pixel.index),
            }
        }
        if !set_pixels.is_empty() {
            let result = apply_document_op(
                project,
                &DocumentOp::SetCelPixels {
                    layer_id: self.layer_id,
                    frame_id: self.frame_id,
                    pixels: set_pixels,
                },
            );
            debug_assert!(result.is_ok(), "stroke set operation failed: {result:?}");
        }
        if !erase_indices.is_empty() {
            let result = apply_document_op(
                project,
                &DocumentOp::EraseCelPixels {
                    layer_id: self.layer_id,
                    frame_id: self.frame_id,
                    indices: erase_indices,
                },
            );
            debug_assert!(result.is_ok(), "stroke erase operation failed: {result:?}");
        }
    }
}

impl EditCommand for StrokeCommand {
    fn apply(&mut self, project: &mut Project) {
        self.apply_colors(project, |pixel| pixel.new_color);
    }

    fn undo(&mut self, project: &mut Project) {
        self.apply_colors(project, |pixel| pixel.old_color);
        if !self.cel_existed_before {
            project.remove_cel_preserving_links(self.layer_id, self.frame_id);
        }
    }
}

#[derive(Debug)]
pub struct ReplaceProjectCommand {
    before: Project,
    after: Project,
}

impl ReplaceProjectCommand {
    pub fn new(before: Project, after: Project) -> Self {
        Self { before, after }
    }
}

impl EditCommand for ReplaceProjectCommand {
    fn apply(&mut self, project: &mut Project) {
        *project = self.after.clone();
    }

    fn undo(&mut self, project: &mut Project) {
        *project = self.before.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::{ReplaceProjectCommand, StrokeCommand, StrokePixel};
    use crate::grid::GridIndex;
    use crate::history::History;
    use crate::model::{Project, Rgba};

    #[test]
    fn replace_project_command_supports_undo_and_redo() {
        let mut before = Project::new_square(20.0, 8, 8);
        before
            .ensure_current_cel()
            .unwrap()
            .pixels
            .insert(GridIndex { x: 1, y: 1 }, Rgba::WHITE);
        let mut after = before.clone();
        let cel = after.ensure_current_cel().unwrap();
        cel.pixels.remove(&GridIndex { x: 1, y: 1 });
        cel.pixels.insert(GridIndex { x: 3, y: 4 }, Rgba::BLACK);

        let mut project = before.clone();
        let mut history = History::new();
        history.push(
            Box::new(ReplaceProjectCommand::new(before, after)),
            &mut project,
        );
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 3, y: 4 })
        );
        history.undo(&mut project);
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 1, y: 1 })
        );
        history.redo(&mut project);
        assert!(
            project
                .current_cel()
                .unwrap()
                .pixels
                .contains_key(&GridIndex { x: 3, y: 4 })
        );
    }

    #[test]
    fn undoing_a_stroke_restores_an_empty_intersection() {
        let mut project = Project::new_square(20.0, 8, 8);
        let layer_id = project.add_layer("Ink");
        let frame_id = project.active_frame_id;
        let mut history = History::new();
        history.push(
            Box::new(StrokeCommand::new(
                frame_id,
                layer_id,
                vec![StrokePixel {
                    index: GridIndex { x: 1, y: 1 },
                    old_color: None,
                    new_color: Some(Rgba::WHITE),
                }],
                false,
            )),
            &mut project,
        );
        assert!(project.cel(layer_id, frame_id).is_some());
        history.undo(&mut project);
        assert!(project.cel(layer_id, frame_id).is_none());
        history.redo(&mut project);
        assert!(project.cel(layer_id, frame_id).is_some());
    }
}
