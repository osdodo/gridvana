use crate::model::{CURRENT_SCHEMA_VERSION, Project};
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy)]
enum Encoding {
    MessagePack,
    Json,
}

#[derive(Deserialize)]
struct SchemaHeader {
    schema_version: u32,
}

pub fn serialize_project(project: &Project) -> Result<Vec<u8>, String> {
    project.validate()?;
    rmp_serde::to_vec_named(project).map_err(|error| {
        format!("failed to encode project schema V{CURRENT_SCHEMA_VERSION}: {error}")
    })
}

pub fn deserialize_project(data: &[u8]) -> Result<Project, String> {
    let (encoding, schema_version) = detect_schema_version(data)?;
    if schema_version != CURRENT_SCHEMA_VERSION {
        return Err(unsupported_version_error(schema_version));
    }
    let project: Project = decode(data, encoding, "current project")?;
    project
        .validate()
        .map_err(|error| format!("invalid project: {error}"))?;
    Ok(project)
}

fn detect_schema_version(data: &[u8]) -> Result<(Encoding, u32), String> {
    match rmp_serde::from_slice::<SchemaHeader>(data) {
        Ok(header) => Ok((Encoding::MessagePack, header.schema_version)),
        Err(message_pack_error) => match serde_json::from_slice::<SchemaHeader>(data) {
            Ok(header) => Ok((Encoding::Json, header.schema_version)),
            Err(json_error) => Err(format!(
                "invalid project file: could not read schema_version as MessagePack ({message_pack_error}) or JSON ({json_error})"
            )),
        },
    }
}

fn decode<T: DeserializeOwned>(
    data: &[u8],
    encoding: Encoding,
    description: &str,
) -> Result<T, String> {
    match encoding {
        Encoding::MessagePack => rmp_serde::from_slice(data)
            .map_err(|error| format!("failed to decode {description}: {error}")),
        Encoding::Json => serde_json::from_slice(data)
            .map_err(|error| format!("failed to decode {description}: {error}")),
    }
}

fn unsupported_version_error(version: u32) -> String {
    format!(
        "unsupported project schema_version {version}; only schema version {CURRENT_SCHEMA_VERSION} is supported"
    )
}

#[cfg(test)]
mod tests {
    use super::{deserialize_project, serialize_project};
    use crate::model::{CURRENT_SCHEMA_VERSION, Project, Rgba, TagDirection};

    #[test]
    fn current_schema_round_trips_overlapping_animation_tags_and_active_id() {
        let mut project = Project::new_square(20.0, 8, 8);
        let first = project.active_frame_id;
        let second = project.add_frame(None, 90).unwrap();
        let third = project.add_frame(None, 120).unwrap();
        project
            .add_tag("Walk", first, second, TagDirection::Forward)
            .unwrap();
        project
            .add_tag("Reverse", second, third, TagDirection::Reverse)
            .unwrap();
        let active = project
            .add_tag("All", first, third, TagDirection::PingPong)
            .unwrap();
        project.palette.name = "Project Colors".to_string();
        project.palette.colors = vec![Rgba::new(0.2, 0.4, 0.6, 0.8)];
        project.foreground_color = Rgba::new(0.1, 0.2, 0.3, 0.4);
        project.background_color = Rgba::new(0.9, 0.8, 0.7, 0.6);

        let encoded = serialize_project(&project).unwrap();
        let decoded = deserialize_project(&encoded).unwrap();

        assert_eq!(decoded, project);
        assert_eq!(decoded.tags.len(), 3);
        assert_eq!(decoded.active_tag_id, Some(active));
    }

    #[test]
    fn serializer_refuses_to_write_a_non_current_schema() {
        let mut project = Project::new_square(20.0, 8, 8);
        project.schema_version = CURRENT_SCHEMA_VERSION - 1;

        let error = serialize_project(&project).unwrap_err();
        assert!(error.contains(&format!("schema_version {}", CURRENT_SCHEMA_VERSION - 1)));
        assert!(error.contains(&format!("expected {CURRENT_SCHEMA_VERSION}")));
    }

    #[test]
    fn non_current_versions_are_rejected_clearly() {
        for version in [
            1,
            CURRENT_SCHEMA_VERSION - 1,
            CURRENT_SCHEMA_VERSION + 1,
            u32::MAX,
        ] {
            let data = format!(r#"{{"schema_version":{version}}}"#);
            let error = deserialize_project(data.as_bytes()).unwrap_err();
            assert!(error.contains(&format!("schema_version {version}")));
            assert!(error.contains(&format!(
                "only schema version {CURRENT_SCHEMA_VERSION} is supported"
            )));
        }
    }

    #[test]
    fn missing_version_has_an_explicit_schema_error() {
        let error = deserialize_project(br#"{"canvas_width":8}"#).unwrap_err();
        assert!(error.contains("schema_version"));
    }
}
