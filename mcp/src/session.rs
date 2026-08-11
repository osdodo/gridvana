use gridvana_core::edit_ops::{EditOp, ProjectSummary, SelectionSummary, apply_edit_ops};
use gridvana_core::grid::GridIndex;
use gridvana_core::model::{CelId, CelPosition, FrameId, LayerId, Project, TagId};
use gridvana_core::sprite_sheet::ExportOptions;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionStarted {
    pub session_id: String,
    pub base_revision: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditPreview {
    pub ok: bool,
    pub applied: usize,
    pub preview_revision: u64,
    pub project_summary: ProjectSummary,
    pub impact: EditImpact,
    pub message: String,
    #[serde(skip_serializing)]
    pub project: Project,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditImpact {
    pub layer_ids: Vec<LayerId>,
    pub frame_ids: Vec<FrameId>,
    pub cel_ids: Vec<CelId>,
    pub tag_ids: Vec<TagId>,
}

impl EditImpact {
    fn between(before: &Project, after: &Project) -> Self {
        let mut layer_ids = before
            .layers
            .iter()
            .map(|layer| layer.id)
            .chain(after.layers.iter().map(|layer| layer.id))
            .collect::<BTreeSet<_>>();
        layer_ids.retain(|id| before.layer(*id) != after.layer(*id));
        if before.active_layer_id != after.active_layer_id {
            layer_ids.insert(before.active_layer_id);
            layer_ids.insert(after.active_layer_id);
        }

        let mut frame_ids = before
            .frames
            .iter()
            .map(|frame| frame.id)
            .chain(after.frames.iter().map(|frame| frame.id))
            .collect::<BTreeSet<_>>();
        frame_ids.retain(|id| before.frame(*id) != after.frame(*id));
        if before.active_frame_id != after.active_frame_id {
            frame_ids.insert(before.active_frame_id);
            frame_ids.insert(after.active_frame_id);
        }

        let mut cel_ids = before
            .cels
            .iter()
            .map(|cel| cel.id)
            .chain(after.cels.iter().map(|cel| cel.id))
            .collect::<BTreeSet<_>>();
        cel_ids.retain(|id| before.cel_by_id(*id) != after.cel_by_id(*id));

        let mut tag_ids = before
            .tags
            .iter()
            .map(|tag| tag.id)
            .chain(after.tags.iter().map(|tag| tag.id))
            .collect::<BTreeSet<_>>();
        tag_ids.retain(|id| before.tag(*id) != after.tag(*id));
        if before.active_tag_id != after.active_tag_id {
            tag_ids.extend(before.active_tag_id);
            tag_ids.extend(after.active_tag_id);
        }

        Self {
            layer_ids: layer_ids.into_iter().collect(),
            frame_ids: frame_ids.into_iter().collect(),
            cel_ids: cel_ids.into_iter().collect(),
            tag_ids: tag_ids.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitResult {
    pub ok: bool,
    pub revision: u64,
    pub project_summary: ProjectSummary,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollbackResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CommittedProject {
    pub before: Project,
    pub after: Project,
    pub result: CommitResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    ReadOnly,
    SessionAlreadyActive,
    SessionNotFound(String),
    RevisionConflict { expected: u64, actual: u64 },
    InvalidEdit(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadOnly => write!(formatter, "service is read-only"),
            Self::SessionAlreadyActive => write!(formatter, "an edit session is already active"),
            Self::SessionNotFound(session_id) => {
                write!(formatter, "edit session not found: {session_id}")
            }
            Self::RevisionConflict { expected, actual } => write!(
                formatter,
                "project revision conflict: expected {expected}, current {actual}"
            ),
            Self::InvalidEdit(message) => write!(formatter, "invalid edit operations: {message}"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Debug, Clone)]
struct EditSession {
    id: String,
    base_revision: u64,
    preview_revision: u64,
    working_project: Project,
}

#[derive(Debug, Clone)]
pub struct EditSessionStore {
    project: Project,
    selection: Vec<GridIndex>,
    timeline_selection: Vec<CelPosition>,
    revision: u64,
    next_session_id: u64,
    access_mode: AccessMode,
    active_session: Option<EditSession>,
    export_options: ExportOptions,
}

impl EditSessionStore {
    pub fn new(project: Project, access_mode: AccessMode) -> Self {
        Self {
            project,
            selection: Vec::new(),
            timeline_selection: Vec::new(),
            revision: 0,
            next_session_id: 1,
            access_mode,
            active_session: None,
            export_options: ExportOptions::default(),
        }
    }

    pub fn access_mode(&self) -> AccessMode {
        self.access_mode
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn preview_project(&self) -> &Project {
        self.active_session
            .as_ref()
            .map_or(&self.project, |session| &session.working_project)
    }

    pub fn project_summary(&self) -> ProjectSummary {
        ProjectSummary::from_project(&self.project)
    }

    pub fn export_options(&self) -> &ExportOptions {
        &self.export_options
    }

    pub fn set_export_options(&mut self, options: ExportOptions) {
        self.export_options = options;
    }

    pub fn selection_summary(&self) -> SelectionSummary {
        SelectionSummary::from_project_selections(
            &self.project,
            self.project.active_layer_id,
            self.project.active_frame_id,
            self.selection.iter().copied(),
            self.timeline_selection.iter().copied(),
        )
    }

    pub fn set_selection<I>(&mut self, selection: I)
    where
        I: IntoIterator<Item = GridIndex>,
    {
        self.selection = selection.into_iter().collect();
    }

    pub fn set_timeline_selection<I>(&mut self, selection: I)
    where
        I: IntoIterator<Item = CelPosition>,
    {
        self.timeline_selection = selection.into_iter().collect();
    }

    pub fn replace_current_project(&mut self, project: Project) {
        self.project = project;
        self.selection.clear();
        self.timeline_selection.clear();
        self.revision = self.revision.saturating_add(1);
    }

    pub fn reset_edit_session(&mut self) -> bool {
        self.active_session.take().is_some()
    }

    pub fn start_edit_session(&mut self) -> Result<SessionStarted, SessionError> {
        self.ensure_writable()?;
        if self.active_session.is_some() {
            return Err(SessionError::SessionAlreadyActive);
        }

        let session_id = format!("s_{:016x}", self.next_session_id);
        self.next_session_id = self.next_session_id.saturating_add(1);
        self.active_session = Some(EditSession {
            id: session_id.clone(),
            base_revision: self.revision,
            preview_revision: self.revision,
            working_project: self.project.clone(),
        });

        Ok(SessionStarted {
            session_id,
            base_revision: self.revision,
        })
    }

    pub fn preview_edit_ops(
        &self,
        session_id: &str,
        base_revision: u64,
        ops: &[EditOp],
    ) -> Result<EditPreview, SessionError> {
        self.ensure_writable()?;
        let session = self.checked_session(session_id, base_revision)?;
        let preview =
            apply_edit_ops(&session.working_project, ops).map_err(SessionError::InvalidEdit)?;
        let impact = EditImpact::between(&session.working_project, &preview);

        Ok(EditPreview {
            ok: true,
            applied: ops.len(),
            preview_revision: session.preview_revision.saturating_add(1),
            project_summary: ProjectSummary::from_project(&preview),
            impact,
            message: "edit operations validated".to_string(),
            project: preview,
        })
    }

    pub fn apply_edit_ops(
        &mut self,
        session_id: &str,
        base_revision: u64,
        ops: &[EditOp],
    ) -> Result<EditPreview, SessionError> {
        self.ensure_writable()?;
        self.check_revision(base_revision)?;
        let session = self
            .active_session
            .as_mut()
            .filter(|session| session.id == session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        if session.base_revision != base_revision {
            return Err(SessionError::RevisionConflict {
                expected: session.base_revision,
                actual: base_revision,
            });
        }

        let previous = session.working_project.clone();
        session.working_project =
            apply_edit_ops(&previous, ops).map_err(SessionError::InvalidEdit)?;
        let impact = EditImpact::between(&previous, &session.working_project);
        session.preview_revision = session.preview_revision.saturating_add(1);

        Ok(EditPreview {
            ok: true,
            applied: ops.len(),
            preview_revision: session.preview_revision,
            project_summary: ProjectSummary::from_project(&session.working_project),
            impact,
            message: "preview updated".to_string(),
            project: session.working_project.clone(),
        })
    }

    pub fn commit_session(
        &mut self,
        session_id: &str,
        base_revision: u64,
    ) -> Result<CommittedProject, SessionError> {
        self.ensure_writable()?;
        self.check_revision(base_revision)?;
        let Some(session) = self.active_session.take() else {
            return Err(SessionError::SessionNotFound(session_id.to_string()));
        };
        if session.id != session_id {
            self.active_session = Some(session);
            return Err(SessionError::SessionNotFound(session_id.to_string()));
        }
        if session.base_revision != base_revision {
            let expected = session.base_revision;
            self.active_session = Some(session);
            return Err(SessionError::RevisionConflict {
                expected,
                actual: base_revision,
            });
        }

        let before = self.project.clone();
        self.project = session.working_project;
        self.revision = self.revision.saturating_add(1);
        let result = CommitResult {
            ok: true,
            revision: self.revision,
            project_summary: ProjectSummary::from_project(&self.project),
            message: "edit session committed".to_string(),
        };

        Ok(CommittedProject {
            before,
            after: self.project.clone(),
            result,
        })
    }

    pub fn rollback_session(&mut self, session_id: &str) -> Result<RollbackResult, SessionError> {
        self.ensure_writable()?;
        match self.active_session.as_ref() {
            Some(session) if session.id == session_id => {
                self.active_session = None;
                Ok(RollbackResult {
                    ok: true,
                    message: "edit session rolled back".to_string(),
                })
            }
            _ => Err(SessionError::SessionNotFound(session_id.to_string())),
        }
    }

    fn checked_session(
        &self,
        session_id: &str,
        base_revision: u64,
    ) -> Result<&EditSession, SessionError> {
        self.check_revision(base_revision)?;
        let session = self
            .active_session
            .as_ref()
            .filter(|session| session.id == session_id)
            .ok_or_else(|| SessionError::SessionNotFound(session_id.to_string()))?;
        if session.base_revision != base_revision {
            return Err(SessionError::RevisionConflict {
                expected: session.base_revision,
                actual: base_revision,
            });
        }
        Ok(session)
    }

    fn ensure_writable(&self) -> Result<(), SessionError> {
        match self.access_mode {
            AccessMode::ReadOnly => Err(SessionError::ReadOnly),
            AccessMode::ReadWrite => Ok(()),
        }
    }

    fn check_revision(&self, base_revision: u64) -> Result<(), SessionError> {
        if base_revision == self.revision {
            Ok(())
        } else {
            Err(SessionError::RevisionConflict {
                expected: base_revision,
                actual: self.revision,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AccessMode, EditSessionStore, SessionError};
    use gridvana_core::edit_ops::{EditOp, PixelChange};
    use gridvana_core::grid::GridIndex;
    use gridvana_core::model::{
        CURRENT_SCHEMA_VERSION, CelPosition, FrameId, LayerId, Project, Rgba, TagDirection,
    };
    use gridvana_core::persistence::deserialize_project;

    fn set_pixel_op(x: i32, y: i32) -> EditOp {
        EditOp::SetCelPixels {
            layer_id: LayerId(1),
            frame_id: FrameId(2),
            pixels: vec![PixelChange {
                index: GridIndex { x, y },
                color: Rgba::WHITE,
            }],
        }
    }

    #[test]
    fn preview_does_not_mutate_working_copy_but_apply_and_commit_do() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let started = store.start_edit_session().unwrap();

        store
            .preview_edit_ops(
                &started.session_id,
                started.base_revision,
                &[set_pixel_op(1, 2)],
            )
            .unwrap();
        assert_eq!(store.project_summary().total_colored_pixels, 0);

        store
            .apply_edit_ops(
                &started.session_id,
                started.base_revision,
                &[set_pixel_op(3, 4)],
            )
            .unwrap();
        assert_eq!(store.project_summary().total_colored_pixels, 0);

        let committed = store
            .commit_session(&started.session_id, started.base_revision)
            .unwrap();
        assert_eq!(committed.result.revision, 1);
        assert_eq!(store.project_summary().total_colored_pixels, 1);
    }

    #[test]
    fn timeline_selection_is_included_in_selection_summary() {
        let project = Project::new_square(20.0, 8, 8);
        let position = CelPosition {
            layer_id: project.active_layer_id,
            frame_id: project.active_frame_id,
        };
        let mut store = EditSessionStore::new(project, AccessMode::ReadWrite);
        store.set_timeline_selection([position]);

        let summary = store.selection_summary();
        assert!(summary.timeline.active);
        assert_eq!(summary.timeline.selected_cels, 1);
        assert_eq!(summary.timeline.cells[0].layer_id, position.layer_id);
    }

    #[test]
    fn tag_edits_appear_in_summary_impact_and_committed_revision() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first = project.active_frame_id;
        let last = project.add_frame(None, 100).unwrap();
        let mut store = EditSessionStore::new(project, AccessMode::ReadWrite);
        let started = store.start_edit_session().unwrap();

        let applied = store
            .apply_edit_ops(
                &started.session_id,
                started.base_revision,
                &[EditOp::AddTag {
                    name: "Run".to_string(),
                    from_frame_id: first,
                    to_frame_id: last,
                    direction: TagDirection::PingPong,
                }],
            )
            .unwrap();
        let tag_id = applied.project_summary.tags[0].tag_id;
        assert_eq!(applied.project_summary.active_tag_id, Some(tag_id));
        assert_eq!(applied.impact.tag_ids, vec![tag_id]);

        let committed = store
            .commit_session(&started.session_id, started.base_revision)
            .unwrap();
        assert_eq!(committed.result.revision, 1);
        assert_eq!(committed.result.project_summary.tags[0].tag_id, tag_id);
    }

    #[test]
    fn palette_edits_preview_and_commit_through_the_mcp_session() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let started = store.start_edit_session().unwrap();
        let color = Rgba::new(0.25, 0.5, 0.75, 0.5);
        let ops = [
            EditOp::SetForegroundColor { color },
            EditOp::ReplacePalette {
                name: "Session Colors".to_string(),
                colors: vec![Rgba::BLACK, color],
            },
        ];

        let preview = store
            .preview_edit_ops(&started.session_id, started.base_revision, &ops)
            .unwrap();
        assert_eq!(preview.project_summary.foreground_color, color);
        assert_eq!(preview.project_summary.palette.colors.len(), 2);

        store
            .apply_edit_ops(&started.session_id, started.base_revision, &ops)
            .unwrap();
        let committed = store
            .commit_session(&started.session_id, started.base_revision)
            .unwrap();
        assert_eq!(
            committed.result.project_summary.palette.name,
            "Session Colors"
        );
        assert_eq!(committed.result.project_summary.foreground_color, color);
    }

    #[test]
    fn manual_project_change_causes_revision_conflict() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let started = store.start_edit_session().unwrap();
        store.replace_current_project(Project::new_square(20.0, 16, 16));

        let error = store
            .apply_edit_ops(
                &started.session_id,
                started.base_revision,
                &[set_pixel_op(1, 1)],
            )
            .unwrap_err();
        assert_eq!(
            error,
            SessionError::RevisionConflict {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn rollback_discards_working_copy() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let started = store.start_edit_session().unwrap();
        store
            .apply_edit_ops(
                &started.session_id,
                started.base_revision,
                &[set_pixel_op(2, 2)],
            )
            .unwrap();

        store.rollback_session(&started.session_id).unwrap();

        assert_eq!(store.project_summary().total_colored_pixels, 0);
        assert_eq!(store.revision(), 0);
        assert!(store.start_edit_session().is_ok());
    }

    #[test]
    fn reset_discards_an_orphaned_terminal_session() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        store.start_edit_session().unwrap();

        assert!(store.reset_edit_session());
        assert!(!store.reset_edit_session());
        assert!(store.start_edit_session().is_ok());
    }

    #[test]
    fn read_only_store_rejects_edit_sessions() {
        let mut store =
            EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadOnly);
        assert!(matches!(
            store.start_edit_session(),
            Err(SessionError::ReadOnly)
        ));
    }

    #[test]
    fn v2_json_fixture_migrates_and_reports_the_current_schema() {
        let project = deserialize_project(include_bytes!("../tests/fixtures/empty.gvn")).unwrap();
        project.validate().unwrap();
        assert_eq!(project.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(project.active_layer_id, LayerId(1));
        assert_eq!(project.active_frame_id, FrameId(2));

        let store = EditSessionStore::new(project, AccessMode::ReadOnly);
        assert_eq!(
            store.project_summary().schema_version,
            CURRENT_SCHEMA_VERSION
        );
    }
}
