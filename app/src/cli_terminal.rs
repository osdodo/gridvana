use crate::i18n::{Language, current_language, tr};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

const GRIDVANA_AGENT_INSTRUCTIONS: &str = r#"You are embedded inside Gridvana, a pixel-art editor.
Treat drawing, editing, animation, frame, layer, pixel, selection, and canvas requests as operations on the currently open Gridvana project.
For those requests, use only the gridvana MCP resources and tools. Read gridvana://project/summary, gridvana://selection/current, and gridvana://schema/edit-op first. Start one edit session, apply and preview the operations, then commit once when the result is complete.
If the project summary reports a zero-width or zero-height canvas, no canvas exists yet. For a drawing request, first call gridvana_apply_edit_ops with only one resize_canvas operation containing canvas_width and canvas_height. Do not use replace_project. Do not bundle pixel edits into that first call, because the editor must display the new canvas immediately. Apply pixel, layer, and frame operations in subsequent calls, then commit once through the same edit session.
Do not use image generation, filesystem image assets, or repository code changes for Gridvana project requests. Only perform software-development work when the user explicitly asks to change software or code."#;

pub(crate) fn mcp_agent_prompt(endpoint: &str) -> String {
    match current_language() {
        Language::English => format!(
            r#"Add the Streamable HTTP MCP service below as `gridvana` and connect to it, then use it for my next Gridvana canvas request.

MCP service URL: {endpoint}

After connecting, first read `gridvana://project/summary`, `gridvana://selection/current`, and `gridvana://schema/edit-op`. When editing the canvas, start an edit session, apply and preview the operations, and commit only after the result is complete.

This address works only on this device. Keep Gridvana open while using it."#
        ),
        Language::Chinese => format!(
            r#"请将下面的 Streamable HTTP MCP 服务添加为 `gridvana` 并连接，然后使用它处理我接下来的 Gridvana 画布请求。

MCP 服务地址：{endpoint}

连接后，请先读取 `gridvana://project/summary`、`gridvana://selection/current` 和 `gridvana://schema/edit-op`。需要编辑画布时，请启动一个编辑会话，应用并预览操作，确认结果完整后再提交。

该地址仅在本机有效。使用期间请保持 Gridvana 开启。"#
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliAgent {
    Codex,
    Claude,
}

impl CliAgent {
    pub const ALL: [Self; 2] = [Self::Codex, Self::Claude];
}

impl std::fmt::Display for CliAgent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexCliConfig {
    pub command: String,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub model: String,
}

impl Default for CodexCliConfig {
    fn default() -> Self {
        Self {
            command: "codex".to_string(),
            profile: String::new(),
            model: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaudeCliConfig {
    pub command: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_claude_effort")]
    pub effort: String,
}

impl Default for ClaudeCliConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            model: String::new(),
            effort: default_claude_effort(),
        }
    }
}

fn default_claude_effort() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliConfig {
    pub agent: CliAgent,
    #[serde(default)]
    pub ai_panel_enabled: bool,
    #[serde(default)]
    pub default_allow: bool,
    #[serde(default)]
    pub codex: CodexCliConfig,
    #[serde(default)]
    pub claude: ClaudeCliConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            agent: CliAgent::Codex,
            ai_panel_enabled: false,
            default_allow: false,
            codex: CodexCliConfig::default(),
            claude: ClaudeCliConfig::default(),
        }
    }
}

impl CliConfig {
    pub fn load() -> Result<Self, String> {
        load_from(&cli_config_path())
    }

    pub fn save(&self) -> Result<(), String> {
        save_to(self, &cli_config_path())
    }

    pub fn selected_command(&self) -> &str {
        match self.agent {
            CliAgent::Codex => &self.codex.command,
            CliAgent::Claude => &self.claude.command,
        }
    }

    pub fn should_show_ai_panel(&self) -> bool {
        self.ai_panel_enabled
    }

    fn validate(&self) -> Result<(), String> {
        if self.selected_command().trim().is_empty() {
            return Err(tr("CLI command cannot be empty", "CLI 命令不能为空").to_string());
        }
        if self.selected_command().contains('\0') {
            return Err(tr(
                "CLI command contains an invalid character",
                "CLI 命令包含非法字符",
            )
            .to_string());
        }
        if !matches!(
            self.claude.effort.as_str(),
            "default" | "low" | "medium" | "high" | "max"
        ) {
            return Err(tr(
                "Claude Effort must be default, low, medium, high, or max",
                "Claude Effort 必须是 default、low、medium、high 或 max",
            )
            .to_string());
        }
        Ok(())
    }
}

pub fn cli_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gridvana")
        .join("cli-agents.json")
}

fn load_from(path: &Path) -> Result<CliConfig, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let config: CliConfig = serde_json::from_slice(&bytes).map_err(|error| {
                format!(
                    "{}: {error}",
                    tr("Invalid CLI configuration", "CLI 配置格式无效")
                )
            })?;
            config.validate()?;
            Ok(config)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(CliConfig::default()),
        Err(error) => Err(format!(
            "{}: {error}",
            tr("Could not read CLI configuration", "无法读取 CLI 配置")
        )),
    }
}

fn save_to(config: &CliConfig, path: &Path) -> Result<(), String> {
    config.validate()?;
    let parent = path.parent().ok_or_else(|| {
        tr(
            "CLI configuration directory is unavailable",
            "CLI 配置目录不可用",
        )
        .to_string()
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "{}: {error}",
            tr(
                "Could not create CLI configuration directory",
                "无法创建 CLI 配置目录"
            )
        )
    })?;
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| {
        format!(
            "{}: {error}",
            tr("Could not encode CLI configuration", "无法编码 CLI 配置")
        )
    })?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| {
        format!(
            "{}: {error}",
            tr("Could not write CLI configuration", "无法写入 CLI 配置")
        )
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!(
            "{}: {error}",
            tr("Could not save CLI configuration", "无法保存 CLI 配置")
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchSpec {
    program: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    working_directory: PathBuf,
    temporary_files: Vec<PathBuf>,
}

impl LaunchSpec {
    fn for_config(
        config: &CliConfig,
        mcp_endpoint: &str,
        working_directory: PathBuf,
    ) -> Result<Self, String> {
        config.validate()?;
        match config.agent {
            CliAgent::Codex => Ok(codex_launch_spec(config, mcp_endpoint, working_directory)),
            CliAgent::Claude => claude_launch_spec(config, mcp_endpoint, working_directory),
        }
    }
}

pub(crate) fn cli_environment() -> HashMap<String, String> {
    let mut environment = HashMap::from([
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("COLORTERM".to_string(), "truecolor".to_string()),
    ]);
    let inherited_path = std::env::var_os("PATH");
    let search_paths = cli_search_paths(inherited_path.as_deref(), dirs::home_dir().as_deref());
    if let Ok(path) = std::env::join_paths(search_paths) {
        environment.insert("PATH".to_string(), path.to_string_lossy().into_owned());
    }
    environment
}

fn cli_search_paths(inherited_path: Option<&OsStr>, home: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = inherited_path
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if let Some(home) = home {
        for relative in [
            ".local/bin",
            "bin",
            ".cargo/bin",
            ".bun/bin",
            "Library/pnpm",
            ".volta/bin",
            ".asdf/shims",
            ".local/share/mise/shims",
        ] {
            push_unique_path(&mut paths, home.join(relative));
        }
        extend_versioned_bins(&mut paths, &home.join(".nvm/versions/node"), "bin");
        extend_versioned_bins(
            &mut paths,
            &home.join(".local/share/fnm/node-versions"),
            "installation/bin",
        );
        extend_versioned_bins(
            &mut paths,
            &home.join("Library/Application Support/fnm/node-versions"),
            "installation/bin",
        );
    }

    #[cfg(target_os = "macos")]
    extend_macos_paths(&mut paths, home);

    paths
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn extend_versioned_bins(paths: &mut Vec<PathBuf>, root: &Path, bin_suffix: &str) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut bins = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_dir())
                .map(|_| entry.path().join(bin_suffix))
        })
        .collect::<Vec<_>>();
    bins.sort_by(|left, right| right.cmp(left));
    for bin in bins {
        push_unique_path(paths, bin);
    }
}

#[cfg(target_os = "macos")]
fn extend_macos_paths(paths: &mut Vec<PathBuf>, home: Option<&Path>) {
    extend_paths_file(paths, Path::new("/etc/paths"), home);
    if let Ok(entries) = fs::read_dir("/etc/paths.d") {
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            extend_paths_file(paths, &file, home);
        }
    }
    for path in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/opt/local/bin",
    ] {
        push_unique_path(paths, PathBuf::from(path));
    }
}

#[cfg(target_os = "macos")]
fn extend_paths_file(paths: &mut Vec<PathBuf>, file: &Path, home: Option<&Path>) {
    let Ok(contents) = fs::read_to_string(file) else {
        return;
    };
    for line in contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let path = if let Some(relative) = line.strip_prefix("~/") {
            match home {
                Some(home) => home.join(relative),
                None => continue,
            }
        } else {
            PathBuf::from(line)
        };
        push_unique_path(paths, path);
    }
}

fn codex_launch_spec(
    config: &CliConfig,
    mcp_endpoint: &str,
    working_directory: PathBuf,
) -> LaunchSpec {
    codex_launch_spec_with_home(
        config,
        mcp_endpoint,
        working_directory,
        codex_home().as_deref(),
    )
}

fn codex_launch_spec_with_home(
    config: &CliConfig,
    mcp_endpoint: &str,
    working_directory: PathBuf,
    codex_home: Option<&Path>,
) -> LaunchSpec {
    let approval_policy = if config.default_allow {
        "never"
    } else {
        "on-request"
    };
    let mut args = vec![
        "--ask-for-approval".to_string(),
        approval_policy.to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "--no-alt-screen".to_string(),
        "--disable".to_string(),
        "plugins".to_string(),
        "--disable".to_string(),
        "image_generation".to_string(),
        "-C".to_string(),
        working_directory.display().to_string(),
        "-c".to_string(),
        codex_mcp_override(config, codex_home, mcp_endpoint),
        "-c".to_string(),
        format!(
            "developer_instructions={}",
            toml_command_line_string(GRIDVANA_AGENT_INSTRUCTIONS)
        ),
    ];
    if !config.codex.profile.trim().is_empty() {
        args.extend([
            "--profile".to_string(),
            config.codex.profile.trim().to_string(),
        ]);
    }
    if !config.codex.model.trim().is_empty() {
        args.extend(["--model".to_string(), config.codex.model.trim().to_string()]);
    }
    LaunchSpec {
        program: config.codex.command.trim().to_string(),
        args,
        env: cli_environment(),
        working_directory,
        temporary_files: Vec::new(),
    }
}

fn codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".codex")))
}

fn codex_mcp_override(config: &CliConfig, codex_home: Option<&Path>, mcp_endpoint: &str) -> String {
    let mut server_names = BTreeSet::new();
    if let Some(home) = codex_home {
        collect_codex_mcp_names(
            &home.join("config.toml"),
            config.codex.profile.trim(),
            &mut server_names,
        );
        if !config.codex.profile.trim().is_empty() {
            collect_codex_mcp_names(
                &home.join(format!("{}.config.toml", config.codex.profile.trim())),
                config.codex.profile.trim(),
                &mut server_names,
            );
        }
    }
    server_names.remove("gridvana");

    let mut entries = server_names
        .into_iter()
        .map(|name| format!("{}={{enabled=false}}", toml_string(&name)))
        .collect::<Vec<_>>();
    let gridvana_config = if config.default_allow {
        format!(
            "gridvana={{url={},default_tools_approval_mode={}}}",
            toml_string(mcp_endpoint),
            toml_string("approve"),
        )
    } else {
        format!("gridvana={{url={}}}", toml_string(mcp_endpoint))
    };
    entries.push(gridvana_config);
    format!("mcp_servers={{{}}}", entries.join(","))
}

fn collect_codex_mcp_names(path: &Path, profile: &str, names: &mut BTreeSet<String>) {
    let Ok(source) = fs::read_to_string(path) else {
        return;
    };
    let Ok(config) = source.parse::<toml::Value>() else {
        return;
    };
    collect_table_keys(config.get("mcp_servers"), names);
    if !profile.is_empty() {
        collect_table_keys(
            config
                .get("profiles")
                .and_then(|profiles| profiles.get(profile))
                .and_then(|profile| profile.get("mcp_servers")),
            names,
        );
    }
}

fn collect_table_keys(value: Option<&toml::Value>, names: &mut BTreeSet<String>) {
    if let Some(table) = value.and_then(toml::Value::as_table) {
        names.extend(table.keys().cloned());
    }
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

fn toml_command_line_string(value: &str) -> String {
    serde_json::to_string(value).expect("strings always serialize")
}

fn claude_launch_spec(
    config: &CliConfig,
    mcp_endpoint: &str,
    working_directory: PathBuf,
) -> Result<LaunchSpec, String> {
    let permission_mode = if config.default_allow {
        "bypassPermissions"
    } else {
        "default"
    };
    let config_path = std::env::temp_dir().join(format!(
        "gridvana-claude-mcp-{}-{}.json",
        std::process::id(),
        NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mcp_config = json!({
        "mcpServers": {
            "gridvana": {
                "type": "http",
                "url": mcp_endpoint
            }
        }
    });
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&mcp_config).map_err(|error| {
            format!(
                "{}: {error}",
                tr(
                    "Could not encode Claude MCP configuration",
                    "无法编码 Claude MCP 配置"
                )
            )
        })?,
    )
    .map_err(|error| {
        format!(
            "{}: {error}",
            tr(
                "Could not create Claude MCP configuration",
                "无法创建 Claude MCP 配置"
            )
        )
    })?;

    let mut args = vec![
        "--strict-mcp-config".to_string(),
        "--mcp-config".to_string(),
        config_path.display().to_string(),
        "--disable-slash-commands".to_string(),
        "--append-system-prompt".to_string(),
        GRIDVANA_AGENT_INSTRUCTIONS.to_string(),
        "--permission-mode".to_string(),
        permission_mode.to_string(),
    ];
    if !config.claude.model.trim().is_empty() {
        args.extend([
            "--model".to_string(),
            config.claude.model.trim().to_string(),
        ]);
    }
    if config.claude.effort != "default" {
        args.extend(["--effort".to_string(), config.claude.effort.clone()]);
    }
    Ok(LaunchSpec {
        program: config.claude.command.trim().to_string(),
        args,
        env: cli_environment(),
        working_directory,
        temporary_files: vec![config_path],
    })
}

/// Builds the command/environment used by the native Iced terminal widget.
/// The widget owns the PTY, while this module remains responsible for the
/// agent-specific MCP and permission configuration.
pub(crate) struct IcedTerminalLaunch {
    pub(crate) id: u64,
    pub(crate) agent: CliAgent,
    pub(crate) backend: iced_term::settings::BackendSettings,
    pub(crate) temporary_files: Vec<PathBuf>,
}

pub(crate) fn iced_terminal_settings(
    config: &CliConfig,
    mcp_endpoint: &str,
    working_directory: PathBuf,
) -> Result<IcedTerminalLaunch, String> {
    let spec = LaunchSpec::for_config(config, mcp_endpoint, working_directory)?;
    let (program, args) = windows_launch_command(&spec);
    Ok(IcedTerminalLaunch {
        id: NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed),
        agent: config.agent,
        backend: iced_term::settings::BackendSettings {
            program,
            args,
            env: spec.env,
            working_directory: Some(spec.working_directory),
            #[cfg(target_os = "windows")]
            escape_args: true,
        },
        temporary_files: spec.temporary_files,
    })
}

#[cfg(not(windows))]
fn windows_launch_command(spec: &LaunchSpec) -> (String, Vec<String>) {
    (spec.program.clone(), spec.args.clone())
}

#[cfg(windows)]
fn windows_launch_command(spec: &LaunchSpec) -> (String, Vec<String>) {
    let configured = spec.program.trim();
    let configured_path = Path::new(configured);
    let script = if matches!(
        configured_path.extension().and_then(OsStr::to_str),
        Some("cmd" | "bat" | "ps1")
    ) {
        Some(configured_path.to_path_buf())
    } else {
        find_windows_script(configured, spec.env.get("PATH").map(OsStr::new))
    };

    let Some(script) = script else {
        return (configured.to_string(), spec.args.clone());
    };

    let script = script.to_string_lossy().into_owned();
    let extension = Path::new(&script)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("ps1") {
        let mut args = vec![
            "-NoProfile".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-File".to_string(),
            script,
        ];
        args.extend(spec.args.clone());
        ("powershell.exe".to_string(), args)
    } else {
        let mut args = vec!["/d".to_string(), "/c".to_string(), script];
        args.extend(spec.args.clone());
        let command_processor = std::env::var_os("ComSpec")
            .filter(|path| !path.is_empty())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "cmd.exe".to_string());
        (command_processor, args)
    }
}

#[cfg(windows)]
fn find_windows_script(program: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    let configured = Path::new(program);
    let candidates = if configured.is_absolute() || configured.components().count() > 1 {
        vec![configured.to_path_buf()]
    } else {
        path.map(std::env::split_paths)
            .into_iter()
            .flatten()
            .map(|directory| directory.join(configured))
            .collect()
    };

    candidates.into_iter().find_map(|candidate| {
        ["cmd", "bat", "ps1"].into_iter().find_map(|extension| {
            let script = candidate.with_extension(extension);
            script.is_file().then_some(script)
        })
    })
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::windows_launch_command;
    use super::{
        CliAgent, CliConfig, GRIDVANA_AGENT_INSTRUCTIONS, LaunchSpec, cli_search_paths,
        codex_launch_spec_with_home, load_from, mcp_agent_prompt, save_to,
    };
    #[test]
    fn cli_config_round_trips() {
        let root =
            std::env::temp_dir().join(format!("gridvana-cli-config-test-{}", std::process::id()));
        let path = root.join("cli.json");
        let mut config = CliConfig {
            agent: CliAgent::Claude,
            ai_panel_enabled: true,
            ..CliConfig::default()
        };
        config.claude.command = "/usr/local/bin/claude".to_string();
        config.claude.effort = "high".to_string();
        config.default_allow = true;

        save_to(&config, &path).unwrap();
        assert_eq!(load_from(&path).unwrap(), config);

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn ai_panel_is_hidden_for_the_untouched_default_config() {
        assert!(!CliConfig::default().should_show_ai_panel());
    }

    #[test]
    fn ai_panel_is_shown_when_explicitly_enabled() {
        let config = CliConfig {
            ai_panel_enabled: true,
            ..CliConfig::default()
        };

        assert!(config.should_show_ai_panel());
    }

    #[test]
    fn ai_panel_stays_hidden_when_only_an_agent_is_configured() {
        let mut config = CliConfig::default();
        config.codex.model = "gpt-5".to_string();

        assert!(!config.should_show_ai_panel());

        let config = CliConfig {
            agent: CliAgent::Claude,
            ..CliConfig::default()
        };
        assert!(!config.should_show_ai_panel());
    }

    #[test]
    fn legacy_cli_config_defaults_to_ai_panel_disabled() {
        let config: CliConfig = serde_json::from_str(
            r#"{
                "agent": "codex",
                "default_allow": false,
                "codex": { "command": "codex", "profile": "", "model": "" },
                "claude": { "command": "claude", "model": "", "effort": "default" }
            }"#,
        )
        .unwrap();

        assert!(!config.ai_panel_enabled);
        assert!(!config.should_show_ai_panel());
    }

    #[test]
    fn mcp_agent_prompt_contains_endpoint_and_workflow() {
        let prompt = mcp_agent_prompt("http://127.0.0.1:51109/mcp");

        assert!(prompt.contains("http://127.0.0.1:51109/mcp"));
        assert!(prompt.contains("gridvana://project/summary"));
        assert!(prompt.contains("gridvana://selection/current"));
        assert!(prompt.contains("gridvana://schema/edit-op"));
        assert!(prompt.contains("apply and preview the operations"));
    }

    #[test]
    fn cli_path_keeps_inherited_entries_and_adds_user_install_locations() {
        let home = std::path::Path::new("/Users/gridvana-test");
        let inherited = std::env::join_paths(["/usr/bin", "/bin"]).unwrap();
        let paths = cli_search_paths(Some(&inherited), Some(home));

        assert_eq!(paths[0], std::path::PathBuf::from("/usr/bin"));
        assert_eq!(paths[1], std::path::PathBuf::from("/bin"));
        assert!(paths.contains(&home.join(".local/bin")));
        assert!(paths.contains(&home.join("Library/pnpm")));
        assert!(paths.contains(&home.join(".volta/bin")));
        #[cfg(target_os = "macos")]
        assert!(paths.contains(&std::path::PathBuf::from("/opt/homebrew/bin")));
    }

    #[cfg(windows)]
    #[test]
    fn windows_npm_command_shim_uses_cmd_instead_of_extensionless_script() {
        let root = std::env::temp_dir().join(format!(
            "gridvana-windows-command-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("codex"), "#!/bin/sh\n").unwrap();
        let shim = root.join("codex.cmd");
        std::fs::write(&shim, "@echo off\r\n").unwrap();
        let spec = LaunchSpec {
            program: "codex".to_string(),
            args: vec!["--version".to_string()],
            env: std::collections::HashMap::from([(
                "PATH".to_string(),
                root.to_string_lossy().into_owned(),
            )]),
            working_directory: root.clone(),
            temporary_files: Vec::new(),
        };

        let (program, args) = windows_launch_command(&spec);

        assert!(program.to_ascii_lowercase().ends_with("cmd.exe"));
        assert_eq!(args[0..2], ["/d", "/c"]);
        assert_eq!(std::path::Path::new(&args[2]), shim);
        assert_eq!(args[3], "--version");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn codex_launches_interactively_with_gridvana_mcp() {
        let config = CliConfig::default();
        let workdir = std::env::temp_dir();
        let spec = LaunchSpec::for_config(&config, "http://127.0.0.1:51109/mcp", workdir).unwrap();

        assert_eq!(spec.program, "codex");
        assert!(!spec.args.iter().any(|argument| argument == "exec"));
        assert!(
            spec.args
                .iter()
                .any(|argument| argument == "--no-alt-screen")
        );
        assert!(spec.args.iter().any(|argument| {
            argument.contains("gridvana={url=\"http://127.0.0.1:51109/mcp\"}")
        }));
        let instructions_override = spec
            .args
            .iter()
            .find(|argument| argument.starts_with("developer_instructions="))
            .unwrap();
        assert!(instructions_override.contains("currently open Gridvana project"));
        assert!(instructions_override.contains("resize_canvas"));
        assert!(!instructions_override.contains("one replace_project operation"));
        assert!(!instructions_override.contains(['\r', '\n']));
        assert!(instructions_override.contains("\\n"));

        let parsed: toml::Value = instructions_override.parse().unwrap();
        assert_eq!(
            parsed["developer_instructions"].as_str(),
            Some(GRIDVANA_AGENT_INSTRUCTIONS)
        );
        assert!(
            spec.args
                .windows(2)
                .any(|arguments| { arguments == ["--disable".to_string(), "plugins".to_string()] })
        );
        assert!(spec.args.windows(2).any(|arguments| {
            arguments == ["--disable".to_string(), "image_generation".to_string()]
        }));
        assert!(spec.args.windows(2).any(|arguments| {
            arguments == ["--ask-for-approval".to_string(), "on-request".to_string()]
        }));
    }

    #[test]
    fn default_allow_skips_codex_approvals_but_keeps_read_only_sandbox() {
        let config = CliConfig {
            default_allow: true,
            ..CliConfig::default()
        };
        let spec =
            LaunchSpec::for_config(&config, "http://127.0.0.1:51109/mcp", std::env::temp_dir())
                .unwrap();

        assert!(spec.args.windows(2).any(|arguments| {
            arguments == ["--ask-for-approval".to_string(), "never".to_string()]
        }));
        assert!(
            spec.args.windows(2).any(|arguments| {
                arguments == ["--sandbox".to_string(), "read-only".to_string()]
            })
        );
        let mcp_override = spec
            .args
            .iter()
            .find(|argument| argument.starts_with("mcp_servers="))
            .unwrap();
        assert!(mcp_override.contains("default_tools_approval_mode=\"approve\""));
        assert!(
            !spec
                .args
                .iter()
                .any(|argument| argument == "--dangerously-bypass-approvals-and-sandbox")
        );
    }

    #[test]
    fn codex_disables_non_gridvana_mcp_servers_from_base_and_profile() {
        let root = std::env::temp_dir().join(format!(
            "gridvana-codex-isolation-test-{}-{}",
            std::process::id(),
            super::NEXT_TERMINAL_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("config.toml"),
            r#"
[mcp_servers.unrelated]
command = "false"

[mcp_servers.gridvana]
url = "http://stale.invalid/mcp"
"#,
        )
        .unwrap();
        std::fs::write(
            root.join("drawing.config.toml"),
            r#"
[mcp_servers."profile.server"]
command = "false"
"#,
        )
        .unwrap();
        let mut config = CliConfig::default();
        config.codex.profile = "drawing".to_string();

        let spec = codex_launch_spec_with_home(
            &config,
            "http://127.0.0.1:51109/mcp",
            std::env::temp_dir(),
            Some(&root),
        );
        let mcp_override = spec
            .args
            .iter()
            .find(|argument| argument.starts_with("mcp_servers="))
            .unwrap();

        assert!(mcp_override.contains("\"unrelated\"={enabled=false}"));
        assert!(mcp_override.contains("\"profile.server\"={enabled=false}"));
        assert!(!mcp_override.contains("\"gridvana\"={enabled=false}"));
        assert!(mcp_override.contains("gridvana={url=\"http://127.0.0.1:51109/mcp\"}"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn claude_launches_interactively_with_terminal_permissions() {
        let config = CliConfig {
            agent: CliAgent::Claude,
            ..CliConfig::default()
        };
        let spec =
            LaunchSpec::for_config(&config, "http://127.0.0.1:51109/mcp", std::env::temp_dir())
                .unwrap();

        assert_eq!(spec.program, "claude");
        assert!(!spec.args.iter().any(|argument| argument == "--print"));
        assert!(spec.args.windows(2).any(|arguments| {
            arguments == ["--permission-mode".to_string(), "default".to_string()]
        }));
        assert!(
            spec.args
                .iter()
                .any(|argument| argument == "--disable-slash-commands")
        );
        assert!(spec.args.windows(2).any(|arguments| {
            arguments
                == [
                    "--append-system-prompt".to_string(),
                    GRIDVANA_AGENT_INSTRUCTIONS.to_string(),
                ]
        }));
        let mcp_config = std::fs::read_to_string(&spec.temporary_files[0]).unwrap();
        assert!(mcp_config.contains("http://127.0.0.1:51109/mcp"));

        for path in spec.temporary_files {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn default_allow_uses_claude_bypass_permissions_mode() {
        let config = CliConfig {
            agent: CliAgent::Claude,
            default_allow: true,
            ..CliConfig::default()
        };
        let spec =
            LaunchSpec::for_config(&config, "http://127.0.0.1:51109/mcp", std::env::temp_dir())
                .unwrap();

        assert!(spec.args.windows(2).any(|arguments| {
            arguments
                == [
                    "--permission-mode".to_string(),
                    "bypassPermissions".to_string(),
                ]
        }));

        for path in spec.temporary_files {
            let _ = std::fs::remove_file(path);
        }
    }
}
