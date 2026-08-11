use crate::model::Project;
use crate::persistence::{deserialize_project, serialize_project};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

const RECOVERY_FORMAT: &str = "gridvana-recovery";
const RECOVERY_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryDocument {
    pub project: Project,
    pub project_path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
struct RecoveryFile {
    format: String,
    format_version: u32,
    project_path: Option<PathBuf>,
    project_data: serde_bytes::ByteBuf,
}

pub fn save_recovery_file(
    project: &Project,
    project_path: Option<&Path>,
    recovery_path: impl AsRef<Path>,
) -> Result<(), String> {
    let recovery_path = recovery_path.as_ref();
    let project_data = serialize_project(project)
        .map_err(|error| format!("cannot create recovery file: {error}"))?;
    let recovery = RecoveryFile {
        format: RECOVERY_FORMAT.to_string(),
        format_version: RECOVERY_FORMAT_VERSION,
        project_path: project_path.map(Path::to_path_buf),
        project_data: project_data.into(),
    };
    let data = rmp_serde::to_vec_named(&recovery)
        .map_err(|error| format!("failed to encode recovery file: {error}"))?;

    if let Some(parent) = recovery_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create recovery directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let temporary_path = temporary_recovery_path(recovery_path);
    let result = (|| {
        let mut file = File::create(&temporary_path).map_err(|error| {
            format!(
                "failed to create temporary recovery file {}: {error}",
                temporary_path.display()
            )
        })?;
        file.write_all(&data).map_err(|error| {
            format!(
                "failed to write temporary recovery file {}: {error}",
                temporary_path.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "failed to flush temporary recovery file {}: {error}",
                temporary_path.display()
            )
        })?;
        std::fs::rename(&temporary_path, recovery_path).map_err(|error| {
            format!(
                "failed to replace recovery file {}: {error}",
                recovery_path.display()
            )
        })
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&temporary_path);
    }
    result
}

pub fn load_recovery_file(path: impl AsRef<Path>) -> Result<RecoveryDocument, String> {
    let path = path.as_ref();
    let data = std::fs::read(path)
        .map_err(|error| format!("failed to read recovery file {}: {error}", path.display()))?;
    let recovery: RecoveryFile = rmp_serde::from_slice(&data)
        .map_err(|error| format!("invalid recovery file {}: {error}", path.display()))?;
    if recovery.format != RECOVERY_FORMAT {
        return Err(format!(
            "invalid recovery file {}: unexpected format {:?}",
            path.display(),
            recovery.format
        ));
    }
    if recovery.format_version != RECOVERY_FORMAT_VERSION {
        return Err(format!(
            "unsupported recovery format version {} in {}; expected {}",
            recovery.format_version,
            path.display(),
            RECOVERY_FORMAT_VERSION
        ));
    }
    let project = deserialize_project(&recovery.project_data).map_err(|error| {
        format!(
            "invalid project in recovery file {}: {error}",
            path.display()
        )
    })?;
    Ok(RecoveryDocument {
        project,
        project_path: recovery.project_path,
    })
}

pub fn remove_recovery_file(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "failed to remove recovery file {}: {error}",
            path.display()
        )),
    }
}

fn temporary_recovery_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "recovery".to_string());
    path.with_file_name(format!(".{file_name}.gridvana-tmp-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::{
        load_recovery_file, remove_recovery_file, save_recovery_file, temporary_recovery_path,
    };
    use crate::grid::GridIndex;
    use crate::io::{load_project, save_project};
    use crate::model::{Project, Rgba};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    fn test_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "gridvana-recovery-test-{}-{}",
            std::process::id(),
            NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn project_with_pixel(index: GridIndex, color: Rgba) -> Project {
        let mut project = Project::new_square(1.0, 4, 4);
        project
            .current_cel_mut()
            .unwrap()
            .pixels
            .insert(index, color);
        project
    }

    #[test]
    fn recovery_round_trips_without_overwriting_the_formal_project() {
        let directory = test_directory();
        std::fs::create_dir_all(&directory).unwrap();
        let formal_path = directory.join("drawing.gvn");
        let recovery_path = directory.join("autosave.recovery");
        let formal = project_with_pixel(GridIndex { x: 0, y: 0 }, Rgba::BLACK);
        let dirty = project_with_pixel(GridIndex { x: 1, y: 1 }, Rgba::WHITE);
        save_project(&formal, &formal_path).unwrap();

        save_recovery_file(&dirty, Some(&formal_path), &recovery_path).unwrap();
        let recovered = load_recovery_file(&recovery_path).unwrap();

        assert_eq!(recovered.project, dirty);
        assert_eq!(recovered.project_path, Some(formal_path.clone()));
        assert_eq!(load_project(&formal_path).unwrap(), formal);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_second_autosave_atomically_replaces_the_previous_snapshot() {
        let directory = test_directory();
        let recovery_path = directory.join("autosave.recovery");
        let first = project_with_pixel(GridIndex { x: 0, y: 0 }, Rgba::BLACK);
        let second = project_with_pixel(GridIndex { x: 2, y: 1 }, Rgba::WHITE);

        save_recovery_file(&first, None, &recovery_path).unwrap();
        save_recovery_file(&second, None, &recovery_path).unwrap();

        assert_eq!(load_recovery_file(&recovery_path).unwrap().project, second);
        assert!(!temporary_recovery_path(&recovery_path).exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn an_interrupted_temporary_write_does_not_damage_the_last_snapshot() {
        let directory = test_directory();
        let recovery_path = directory.join("autosave.recovery");
        let first = project_with_pixel(GridIndex { x: 0, y: 0 }, Rgba::BLACK);
        let second = project_with_pixel(GridIndex { x: 3, y: 3 }, Rgba::WHITE);
        save_recovery_file(&first, None, &recovery_path).unwrap();

        std::fs::write(
            temporary_recovery_path(&recovery_path),
            b"partial message pack",
        )
        .unwrap();
        assert_eq!(load_recovery_file(&recovery_path).unwrap().project, first);

        save_recovery_file(&second, None, &recovery_path).unwrap();
        assert_eq!(load_recovery_file(&recovery_path).unwrap().project, second);
        assert!(!temporary_recovery_path(&recovery_path).exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn corrupt_recovery_files_fail_clearly_and_can_be_discarded() {
        let directory = test_directory();
        std::fs::create_dir_all(&directory).unwrap();
        let recovery_path = directory.join("autosave.recovery");
        std::fs::write(&recovery_path, b"not a recovery file").unwrap();

        let error = load_recovery_file(&recovery_path).unwrap_err();
        assert!(error.contains("invalid recovery file"));

        remove_recovery_file(&recovery_path).unwrap();
        remove_recovery_file(&recovery_path).unwrap();
        assert!(!recovery_path.exists());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
