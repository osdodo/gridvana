use super::super::Gridvana;
use super::super::McpCopyFeedback;
use crate::cli_terminal::{TerminalSession, cli_environment, mcp_agent_prompt};
use crate::i18n::{set_current_language, tr};
use crate::types::{InspectorPanel, Message};
use crate::web_terminal::{self, WebTerminalEvent};
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
                if self.terminal_session.is_none() {
                    return Ok(self.start_cli_terminal());
                }
                self.sync_terminal_webview_visibility();
                self.focus_terminal();
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
                self.sync_terminal_webview_visibility();
                Ok(Task::none())
            }
            Message::CloseCliSettings => {
                self.cli_settings_open = false;
                self.sync_terminal_webview_visibility();
                self.focus_terminal();
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
                self.cli_status = self.terminal_session.as_ref().map_or_else(
                    || {
                        format!(
                            "{} · {}",
                            tr("Current terminal", "当前终端"),
                            self.cli_config.agent
                        )
                    },
                    |session| {
                        format!(
                            "{} {}",
                            session.agent,
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
            Message::TerminalHostWindow(Some(window_id)) => Ok(self
                .terminal_webview
                .create_task(window_id, Message::TerminalWebViewReady)),
            Message::TerminalHostWindow(None) => {
                self.cli_status = tr(
                    "Could not access the Gridvana window; the web terminal did not start",
                    "无法获取 Gridvana 窗口，Web 终端未启动",
                )
                .to_string();
                Ok(Task::none())
            }
            Message::TerminalWebViewReady(Ok(())) => {
                self.terminal_webview.take_staged();
                self.terminal_webview_ready = true;
                self.sync_terminal_webview_visibility();
                Ok(Task::none())
            }
            Message::TerminalWebViewReady(Err(error)) => {
                self.cli_status = format!(
                    "{}: {error}",
                    tr("Web terminal failed to start", "Web 终端启动失败")
                );
                Ok(Task::none())
            }
            Message::TerminalWebViewIpc(message) => {
                self.handle_terminal_ipc(&message.body);
                Ok(Task::none())
            }
            Message::PollCliTerminal => {
                self.poll_cli_terminal();
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
        if self.terminal_session.is_some() {
            self.sync_terminal_webview_visibility();
            self.focus_terminal();
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
        match TerminalSession::start(&self.cli_config, &endpoint, working_directory) {
            Ok(session) => {
                self.cli_status = format!(
                    "{} {}",
                    session.agent,
                    tr("terminal connected", "终端已连接")
                );
                if let Some((cols, rows)) = self.terminal_size
                    && let Err(error) = session.resize(cols, rows)
                {
                    self.cli_status = error;
                }
                self.terminal_session = Some(session);
                self.terminal_webview
                    .evaluate_script(web_terminal::CLEAR_SCRIPT);
                self.sync_terminal_webview_visibility();
                self.focus_terminal();
            }
            Err(error) => {
                self.cli_status = error;
                self.sync_terminal_webview_visibility();
            }
        }
        Task::none()
    }

    fn handle_terminal_ipc(&mut self, body: &str) {
        let event = match web_terminal::parse_ipc(body) {
            Ok(event) => event,
            Err(error) => {
                self.cli_status = error;
                return;
            }
        };
        match event {
            WebTerminalEvent::Ready { cols, rows } => {
                self.terminal_page_ready = true;
                self.terminal_size = Some((cols, rows));
                if let Some(session) = self.terminal_session.as_ref()
                    && let Err(error) = session.resize(cols, rows)
                {
                    self.cli_status = error;
                }
                self.terminal_webview
                    .evaluate_script(web_terminal::CLEAR_SCRIPT);
                self.flush_terminal_output();
                self.focus_terminal();
            }
            WebTerminalEvent::Input { data } => {
                if let Some(session) = self.terminal_session.as_mut()
                    && let Err(error) = session.write(data.as_bytes())
                {
                    self.cli_status = error;
                }
            }
            WebTerminalEvent::Resize { cols, rows } => {
                self.terminal_size = Some((cols, rows));
                if let Some(session) = self.terminal_session.as_ref()
                    && let Err(error) = session.resize(cols, rows)
                {
                    self.cli_status = error;
                }
            }
        }
    }

    fn poll_cli_terminal(&mut self) {
        if self.terminal_page_ready {
            self.flush_terminal_output();
        }
        let exit = self
            .terminal_session
            .as_mut()
            .and_then(|session| match session.try_wait() {
                Ok(status) => status.map(|status| (session.agent, status)),
                Err(error) => {
                    self.cli_status = error;
                    None
                }
            });
        if let Some((agent, status)) = exit {
            self.flush_terminal_output();
            self.terminal_session = None;
            self.terminal_webview.set_visible(false);
            self.reset_agent_edit_session();
            if self.inspector_panel == InspectorPanel::AiAgent && !self.ai_agent_panel_available() {
                self.inspector_panel = InspectorPanel::Layers;
            }
            self.cli_status = format!("{agent} {} · {status}", tr("terminal exited", "终端已退出"));
        }
    }

    fn flush_terminal_output(&self) {
        let Some(session) = self.terminal_session.as_ref() else {
            return;
        };
        let chunks = session.drain_output();
        if chunks.is_empty() {
            return;
        }
        let mut output = Vec::with_capacity(chunks.iter().map(Vec::len).sum());
        for chunk in chunks {
            output.extend_from_slice(&chunk);
        }
        self.terminal_webview
            .evaluate_script(&web_terminal::output_script(&output));
    }

    fn focus_terminal(&self) {
        if self.terminal_webview_ready
            && self.inspector_panel == InspectorPanel::AiAgent
            && !self.cli_settings_open
            && self.terminal_session.is_some()
        {
            self.terminal_webview
                .evaluate_script(web_terminal::FOCUS_SCRIPT);
        }
    }

    pub(in crate::app) fn sync_terminal_webview_visibility(&self) {
        self.terminal_webview.set_visible(
            self.terminal_webview_ready
                && self.inspector_panel == InspectorPanel::AiAgent
                && !self.cli_settings_open
                && !self.new_project_dialog_open
                && !self.about_dialog_open
                && self.terminal_session.is_some(),
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
                    self.sync_terminal_webview_visibility();
                }
                self.cli_status = if self.terminal_session.is_some() {
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
