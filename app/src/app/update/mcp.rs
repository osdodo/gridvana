use super::super::Gridvana;
use crate::i18n::tr;
use crate::types::Message;
use gridvana_core::commands::ReplaceProjectCommand;
use gridvana_core::history::EditCommand;
use gridvana_mcp::protocol::ServerEvent;
use iced::Task;

impl Gridvana {
    pub(super) fn handle_mcp_message(
        &mut self,
        message: Message,
    ) -> Result<Task<Message>, Message> {
        match message {
            Message::PollMcpServer => {}
            other => return Err(other),
        }

        let mut selection = self.selection_indices.iter().copied().collect::<Vec<_>>();
        selection.sort_by_key(|index| (index.y, index.x));
        let export_options = self.sprite_sheet_export_options();
        let events = {
            let Some(service) = self.mcp_service.as_mut() else {
                return Ok(Task::none());
            };
            if let Err(error) = service.sync_editor_state(&self.project, selection) {
                self.mcp_status = format!("{} · {error}", tr("MCP sync failed", "MCP 同步失败"));
                return Ok(Task::none());
            }
            if let Err(error) =
                service.set_timeline_selection(self.timeline_selection.iter().copied())
            {
                self.mcp_status = format!(
                    "{} · {error}",
                    tr(
                        "MCP timeline selection sync failed",
                        "MCP 时间轴选择同步失败"
                    )
                );
                return Ok(Task::none());
            }
            if let Err(error) = service.set_export_options(export_options) {
                self.mcp_status = format!(
                    "{} · {error}",
                    tr(
                        "MCP export configuration sync failed",
                        "MCP 导出配置同步失败"
                    )
                );
                return Ok(Task::none());
            }
            match service.drain_events() {
                Ok(events) => events,
                Err(error) => {
                    self.mcp_status =
                        format!("{} · {error}", tr("MCP event failed", "MCP 事件失败"));
                    return Ok(Task::none());
                }
            }
        };

        for event in events {
            match event {
                ServerEvent::PreviewUpdated(project) => {
                    self.ai_preview_project = Some(*project);
                }
                ServerEvent::SessionRolledBack => {
                    self.ai_preview_project = None;
                }
                ServerEvent::SessionCommitted(commit) => {
                    self.ai_preview_project = None;
                    if let Some(service) = self.mcp_service.as_mut()
                        && let Err(error) = service.accept_server_project(&commit.after)
                    {
                        self.mcp_status =
                            format!("{} · {error}", tr("MCP snapshot failed", "MCP 快照失败"));
                    }
                    let command: Box<dyn EditCommand> =
                        Box::new(ReplaceProjectCommand::new(commit.before, commit.after));
                    self.history.push(command, &mut self.project);
                    self.sync_editor_after_external_edit();
                }
            }
        }

        Ok(Task::none())
    }
}
