use super::super::Gridvana;
use super::super::McpCopyFeedback;
use crate::cli_terminal::{cli_environment, iced_terminal_settings, mcp_agent_prompt};
use crate::i18n::{set_current_language, tr};
use crate::types::{InspectorPanel, Message};
use iced::Task;

impl Gridvana {
    pub(super) fn handle_terminal_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        match message {
            Message::SetInspectorPanel(InspectorPanel::AiAgent) => {
                if !self.ai_agent_panel_available() {
                    return Ok(Task::none());
                }
                self.inspector_panel = InspectorPanel::AiAgent;
                self.pending_sprite_sheet_export_path = None;
                if self.terminal.is_none() {
                    return Ok(self.start_cli_terminal());
                }
                Ok(Task::none())
            }
            Message::OpenCliSettings => {
                self.app_menu_open = false;
                self.about_dialog_open = false;
                self.global_left_button_down = false;
                self.cli_config_draft = self.cli_config.clone();
                self.mcp_copy_feedback = None;
                self.cli_save_error = None;
                self.cli_settings_open = true;
                Ok(Task::none())
            }
            Message::CloseCliSettings => {
                self.cli_settings_open = false;
                Ok(Task::none())
            }
            Message::SelectCliAgent(agent) => {
                self.cli_config_draft.agent = agent;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::SetAiPanelEnabled(enabled) => {
                self.cli_config_draft.ai_panel_enabled = enabled;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::SelectSettingsSection(section) => {
                self.settings_section = section;
                Ok(Task::none())
            }
            Message::SetLanguage(language) => {
                self.language = language;
                self.preferences.language = language;
                set_current_language(language);
                self.language_save_error = self.preferences.save().err();
                self.native_menu.set_language();
                self.mcp_status = self.mcp_service.as_ref().map_or_else(
                    || tr("MCP is not running", "MCP 未启动").to_string(),
                    |service| {
                        format!(
                            "{} · {}",
                            tr("MCP connected", "MCP 已连接"),
                            service.endpoint()
                        )
                    },
                );
                self.cli_status = self.terminal.as_ref().map_or_else(
                    || {
                        format!(
                            "{} · {}",
                            tr("Current terminal", "当前终端"),
                            self.cli_config.agent
                        )
                    },
                    |_terminal| {
                        format!(
                            "{} {}",
                            self.cli_config.agent,
                            tr("terminal connected", "终端已连接")
                        )
                    },
                );
                if self.pending_sprite_sheet_export_path.is_some() {
                    self.refresh_sprite_sheet_export_estimate();
                } else {
                    self.sprite_sheet_export_estimate =
                        Err(tr("No export path selected", "尚未选择导出路径").to_string());
                }
                Ok(Task::none())
            }
            Message::CopyMcpEndpoint => {
                let Some(endpoint) = self
                    .mcp_service
                    .as_ref()
                    .map(|service| service.endpoint().to_string())
                else {
                    return Ok(Task::none());
                };
                self.mcp_copy_feedback = Some(McpCopyFeedback::Endpoint);
                Ok(iced::clipboard::write(endpoint))
            }
            Message::CopyMcpAgentPrompt => {
                let Some(endpoint) = self
                    .mcp_service
                    .as_ref()
                    .map(|service| service.endpoint().to_string())
                else {
                    return Ok(Task::none());
                };
                self.mcp_copy_feedback = Some(McpCopyFeedback::AgentPrompt);
                Ok(iced::clipboard::write(mcp_agent_prompt(&endpoint)))
            }
            Message::SetCliDefaultAllow(default_allow) => {
                self.cli_config_draft.default_allow = default_allow;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateCodexCommand(value) => {
                self.cli_config_draft.codex.command = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateCodexProfile(value) => {
                self.cli_config_draft.codex.profile = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateCodexModel(value) => {
                self.cli_config_draft.codex.model = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateClaudeCommand(value) => {
                self.cli_config_draft.claude.command = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateClaudeModel(value) => {
                self.cli_config_draft.claude.model = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::UpdateClaudeEffort(value) => {
                self.cli_config_draft.claude.effort = value;
                self.cli_save_error = None;
                Ok(Task::none())
            }
            Message::SaveCliConfig => {
                self.save_cli_config();
                Ok(Task::none())
            }
            Message::TestCliConnection => self.test_cli_connection(),
            Message::CliConnectionTestFinished(result) => {
                self.cli_test_in_flight = false;
                self.cli_status = result.unwrap_or_else(|error| error);
                Ok(Task::none())
            }
            Message::StartCliTerminal => Ok(self.start_cli_terminal()),
            Message::IcedTerminal(iced_term::Event::BackendCall(_, command)) => {
                let Some(terminal) = self.terminal.as_mut() else {
                    return Ok(Task::none());
                };
                if matches!(
                    terminal.handle(iced_term::Command::ProxyToBackend(command)),
                    iced_term::actions::Action::Shutdown
                ) {
                    self.finish_terminal();
                }
                Ok(Task::none())
            }
            other => Err(other),
        }
    }

    fn test_cli_connection(&mut self) -> Result<Task<Message>, Message> {
        if self.cli_test_in_flight {
            return Ok(Task::none());
        }
        let command = self.cli_config_draft.selected_command().trim().to_string();
        if command.is_empty() {
            self.cli_status = tr("CLI command cannot be empty", "CLI 命令不能为空").to_string();
            return Ok(Task::none());
        }
        self.cli_test_in_flight = true;
        self.cli_status = format!(
            "{} {}…",
            tr("Testing", "正在检测"),
            self.cli_config_draft.agent
        );
        Ok(Task::perform(
            async move {
                std::process::Command::new(&command)
                    .arg("--version")
                    .envs(cli_environment())
                    .output()
                    .map_err(|error| {
                        format!("{} {command}: {error}", tr("Could not run", "无法运行"))
                    })
                    .and_then(|output| {
                        let text = if output.stdout.is_empty() {
                            String::from_utf8_lossy(&output.stderr).trim().to_string()
                        } else {
                            String::from_utf8_lossy(&output.stdout).trim().to_string()
                        };
                        if output.status.success() {
                            Ok(if text.is_empty() {
                                tr("CLI connection succeeded", "CLI 连接成功").to_string()
                            } else {
                                format!(
                                    "{} · {text}",
                                    tr("CLI connection succeeded", "CLI 连接成功")
                                )
                            })
                        } else {
                            Err(format!(
                                "{} · {text}",
                                tr("CLI test failed", "CLI 检测失败")
                            ))
                        }
                    })
            },
            Message::CliConnectionTestFinished,
        ))
    }

    fn start_cli_terminal(&mut self) -> Task<Message> {
        if self.terminal.is_some() {
            return Task::none();
        }
        let Some(endpoint) = self
            .mcp_service
            .as_ref()
            .map(|service| service.endpoint().to_string())
        else {
            self.cli_status = tr(
                "The MCP service is not running, so the CLI cannot connect",
                "MCP 服务未启动，无法连接 CLI",
            )
            .to_string();
            return Task::none();
        };
        let working_directory = std::env::current_dir().unwrap_or_else(|_| ".".into());
        match iced_terminal_settings(&self.cli_config, &endpoint, working_directory) {
            Ok(launch) => {
                let mut palette = iced_term::ColorPalette::default();
                palette.background = "#101318".to_string();
                palette.black = "#101318".to_string();
                palette.foreground = "#ebedf2".to_string();
                palette.white = "#c4c7d2".to_string();
                palette.bright_white = "#ffffff".to_string();
                let settings = iced_term::settings::Settings {
                    backend: launch.backend,
                    font: iced_term::settings::FontSettings {
                        size: 12.0,
                        scale_factor: 1.25,
                        #[cfg(target_os = "windows")]
                        font_type: iced::Font::with_name("NSimSun"),
                        ..Default::default()
                    },
                    theme: iced_term::settings::ThemeSettings::new(Box::new(palette)),
                    ..Default::default()
                };
                match iced_term::Terminal::new(launch.id, settings) {
                    Ok(terminal) => {
                        self.cli_status = format!(
                            "{} {}",
                            launch.agent,
                            tr("terminal connected", "终端已连接")
                        );
                        self.terminal = Some(terminal);
                        self.terminal_temp_files = launch.temporary_files;
                    }
                    Err(error) => {
                        for path in launch.temporary_files {
                            let _ = std::fs::remove_file(path);
                        }
                        self.cli_status = format!(
                            "{}: {error}",
                            tr("Could not create terminal", "无法创建终端")
                        );
                    }
                }
            }
            Err(error) => {
                self.cli_status = error;
            }
        }
        Task::none()
    }

    fn finish_terminal(&mut self) {
        self.terminal = None;
        for path in self.terminal_temp_files.drain(..) {
            let _ = std::fs::remove_file(path);
        }
        self.reset_agent_edit_session();
        self.cli_status = format!(
            "{} · {}",
            self.cli_config.agent,
            tr("terminal exited", "终端已退出")
        );
    }

    fn save_cli_config(&mut self) {
        let config = self.cli_config_draft.clone();
        match config.save() {
            Ok(()) => {
                self.cli_config = config.clone();
                self.cli_config_draft = config;
                self.cli_save_error = None;
                if self.inspector_panel == InspectorPanel::AiAgent
                    && !self.ai_agent_panel_available()
                {
                    self.inspector_panel = InspectorPanel::Layers;
                }
                self.cli_status = if self.terminal.is_some() {
                    tr(
                        "CLI configuration saved; exit the terminal and restart it to apply the changes",
                        "CLI 配置已保存；请在终端内退出，重新启动后生效",
                    ).to_string()
                } else {
                    format!(
                        "{} · {}",
                        tr("CLI configuration saved", "CLI 配置已保存"),
                        self.cli_config.agent
                    )
                };
            }
            Err(error) => {
                self.cli_status = error.clone();
                self.cli_save_error = Some(error);
            }
        }
    }

    fn reset_agent_edit_session(&mut self) {
        self.ai_preview_project = None;
        if let Some(service) = self.mcp_service.as_ref()
            && let Err(error) = service.reset_edit_session()
        {
            self.mcp_status = format!(
                "{} · {error}",
                tr("MCP session cleanup failed", "MCP 会话清理失败")
            );
        }
    }
}
