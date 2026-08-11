use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};

static CURRENT_LANGUAGE: AtomicU8 = AtomicU8::new(Language::English as u8);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English = 0,
    Chinese = 1,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Chinese];
}

impl std::fmt::Display for Language {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
        })
    }
}

/// Returns a localized static string using the language selected for the app.
pub fn tr(english: &'static str, chinese: &'static str) -> &'static str {
    match current_language() {
        Language::English => english,
        Language::Chinese => chinese,
    }
}

pub fn current_language() -> Language {
    match CURRENT_LANGUAGE.load(Ordering::Relaxed) {
        1 => Language::Chinese,
        _ => Language::English,
    }
}

pub fn set_current_language(language: Language) {
    CURRENT_LANGUAGE.store(language as u8, Ordering::Relaxed);
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppPreferences {
    #[serde(default)]
    pub language: Language,
}

impl AppPreferences {
    pub fn load() -> Result<Self, String> {
        load_from(&preferences_path())
    }

    pub fn save(&self) -> Result<(), String> {
        save_to(self, &preferences_path())
    }
}

pub fn preferences_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gridvana")
        .join("preferences.json")
}

fn load_from(path: &Path) -> Result<AppPreferences, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
            format!(
                "{}: {error}",
                tr("Invalid preferences file", "偏好设置文件无效")
            )
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppPreferences::default()),
        Err(error) => Err(format!(
            "{}: {error}",
            tr("Could not read preferences", "无法读取偏好设置")
        )),
    }
}

fn save_to(preferences: &AppPreferences, path: &Path) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        tr("Preferences directory is unavailable", "偏好设置目录不可用").to_string()
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "{}: {error}",
            tr(
                "Could not create preferences directory",
                "无法创建偏好设置目录"
            )
        )
    })?;
    let bytes = serde_json::to_vec_pretty(preferences).map_err(|error| {
        format!(
            "{}: {error}",
            tr("Could not encode preferences", "无法编码偏好设置")
        )
    })?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| {
        format!(
            "{}: {error}",
            tr("Could not write preferences", "无法写入偏好设置")
        )
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "{}: {error}",
            tr("Could not save preferences", "无法保存偏好设置")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{AppPreferences, Language, load_from, save_to};

    #[test]
    fn missing_preferences_default_to_english() {
        let path = std::env::temp_dir().join(format!(
            "gridvana-missing-preferences-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        assert_eq!(load_from(&path).unwrap().language, Language::English);
    }

    #[test]
    fn language_round_trips() {
        let directory =
            std::env::temp_dir().join(format!("gridvana-preferences-test-{}", std::process::id()));
        let path = directory.join("preferences.json");
        let preferences = AppPreferences {
            language: Language::Chinese,
        };

        save_to(&preferences, &path).unwrap();
        assert_eq!(load_from(&path).unwrap(), preferences);

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(directory);
    }
}
