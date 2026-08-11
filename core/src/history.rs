use crate::model::Project;

pub trait EditCommand: Send + Sync {
    fn apply(&mut self, project: &mut Project);
    fn undo(&mut self, project: &mut Project);
}

pub struct History {
    undo_stack: Vec<Box<dyn EditCommand>>,
    redo_stack: Vec<Box<dyn EditCommand>>,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, mut cmd: Box<dyn EditCommand>, project: &mut Project) {
        cmd.apply(project);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, project: &mut Project) -> bool {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(project);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, project: &mut Project) -> bool {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.apply(project);
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }
}
