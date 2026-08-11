use super::super::ui::{
    ACCENT, APP_BACKGROUND, BORDER_SUBTLE, CONTROL_BACKGROUND, PANEL_BACKGROUND, SUCCESS,
    SURFACE_BACKGROUND, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, compact_action_button_style,
    pick_list_menu_style, pick_list_style, text_input_style,
};
use super::super::{Gridvana, McpCopyFeedback};
use crate::cli_terminal::{CliAgent, cli_config_path};
use crate::icons::{Icon, icon_button};
use crate::types::{Message, SettingsSection};
use iced::{Background, Color, Element, Length, Theme, widget};

const CARD_WIDTH: f32 = 760.0;
const CARD_HEIGHT: f32 = 620.0;
const NAV_WIDTH: f32 = 172.0;
const LABEL_WIDTH: f32 = 186.0;

const SECTIONS: [(SettingsSection, &str); 2] = [
    (SettingsSection::Agent, "AI 代理"),
    (SettingsSection::Mcp, "MCP 服务"),
];

impl Gridvana {
    pub(super) fn settings_overlay(&self) -> Option<Element<'_, Message>> {
        if !self.cli_settings_open {
            return None;
        }

        let card = widget::container(
            widget::column![
                self.settings_header(),
                widget::rule::horizontal(1),
                widget::row![
                    self.settings_nav(),
                    widget::rule::vertical(1),
                    widget::container(
                        widget::scrollable(
                            widget::container(self.settings_section_body())
                                .padding(iced::Padding::new(18.0).right(22))
                        )
                        .height(Length::Fill),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill),
                ]
                .height(Length::Fill),
                widget::rule::horizontal(1),
                self.settings_footer(),
            ]
            .spacing(0),
        )
        .width(Length::Fixed(CARD_WIDTH))
        .height(Length::Fixed(CARD_HEIGHT))
        .style(|_: &Theme| {
            widget::container::Style::default()
                .background(SURFACE_BACKGROUND)
                .border(iced::Border {
                    color: BORDER_SUBTLE,
                    width: 1.0,
                    radius: 10.0.into(),
                })
                .shadow(iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.34),
                    offset: iced::Vector::new(0.0, 12.0),
                    blur_radius: 32.0,
                })
        });

        Some(widget::opaque(
            widget::container(widget::center(card))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(24)
                .style(|_| {
                    widget::container::Style::default().background(Color::from_rgba(
                        APP_BACKGROUND.r,
                        APP_BACKGROUND.g,
                        APP_BACKGROUND.b,
                        0.76,
                    ))
                }),
        ))
    }

    fn settings_header(&self) -> Element<'_, Message> {
        widget::container(
            widget::row![
                widget::column![
                    widget::text("设置").size(15).color(TEXT_PRIMARY),
                    widget::text("Gridvana 偏好设置").size(10).color(TEXT_MUTED),
                ]
                .spacing(3)
                .width(Length::Fill),
                icon_button(Icon::CloseCircle, 13.0, 26.0, false, true)
                    .on_press(Message::CloseCliSettings),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .into()
    }

    fn settings_nav(&self) -> Element<'_, Message> {
        let items = SECTIONS
            .into_iter()
            .map(|(section, label)| {
                let active = self.settings_section == section;
                widget::button(
                    widget::row![
                        widget::container(widget::Space::new())
                            .width(Length::Fixed(2.0))
                            .height(Length::Fixed(15.0))
                            .style(move |_| {
                                widget::container::Style::default().background(if active {
                                    ACCENT
                                } else {
                                    Color::TRANSPARENT
                                })
                            }),
                        widget::text(label).size(11).color(if active {
                            TEXT_PRIMARY
                        } else {
                            TEXT_SECONDARY
                        }),
                    ]
                    .spacing(9)
                    .align_y(iced::Alignment::Center),
                )
                .on_press(Message::SelectSettingsSection(section))
                .padding([7, 10])
                .width(Length::Fill)
                .style(move |theme: &Theme, status| {
                    let mut style = widget::button::text(theme, status);
                    style.background = if active {
                        Some(Background::Color(CONTROL_BACKGROUND))
                    } else {
                        match status {
                            widget::button::Status::Hovered | widget::button::Status::Pressed => {
                                Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04)))
                            }
                            _ => None,
                        }
                    };
                    style
                })
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>();

        widget::container(widget::column(items).spacing(2))
            .padding([12, 10])
            .width(Length::Fixed(NAV_WIDTH))
            .height(Length::Fill)
            .style(|_| widget::container::Style::default().background(PANEL_BACKGROUND))
            .into()
    }

    fn settings_section_body(&self) -> Element<'_, Message> {
        match self.settings_section {
            SettingsSection::Agent => self.settings_agent_section(),
            SettingsSection::Mcp => self.settings_mcp_section(),
        }
    }

    fn settings_agent_section(&self) -> Element<'_, Message> {
        let draft = &self.cli_config_draft;
        let visibility_rows = vec![setting_row(
            "启用 AI Agent 面板",
            Some("即使尚未自定义 Agent 配置，也在检查器中显示该面板"),
            widget::row![
                widget::Space::new().width(Length::Fill),
                widget::toggler(draft.ai_panel_enabled).on_toggle(Message::SetAiPanelEnabled),
            ]
            .align_y(iced::Alignment::Center)
            .into(),
        )];
        let agent_choice = widget::row![
            agent_option(CliAgent::Codex, draft.agent),
            agent_option(CliAgent::Claude, draft.agent),
        ]
        .spacing(6);

        let test_label = if self.cli_test_in_flight {
            "检测中"
        } else {
            "检测"
        };
        let command_value = match draft.agent {
            CliAgent::Codex => &draft.codex.command,
            CliAgent::Claude => &draft.claude.command,
        };
        let command_placeholder = match draft.agent {
            CliAgent::Codex => "codex",
            CliAgent::Claude => "claude",
        };
        let on_command: fn(String) -> Message = match draft.agent {
            CliAgent::Codex => Message::UpdateCodexCommand,
            CliAgent::Claude => Message::UpdateClaudeCommand,
        };
        let command_control = widget::row![
            widget::text_input(command_placeholder, command_value)
                .on_input(on_command)
                .padding([6, 8])
                .size(11)
                .style(text_input_style)
                .width(Length::Fill),
            widget::button(widget::text(test_label).size(10).line_height(1.0))
                .padding([7, 10])
                .style(|theme: &Theme, status| compact_action_button_style(theme, status, false))
                .on_press_maybe((!self.cli_test_in_flight).then_some(Message::TestCliConnection)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        let mut rows = vec![
            setting_row(
                "代理",
                Some("启动画布 AI 会话所使用的 CLI"),
                agent_choice.into(),
            ),
            setting_row(
                "可执行命令",
                Some("PATH 中的命令名或绝对路径"),
                command_control.into(),
            ),
        ];

        match draft.agent {
            CliAgent::Codex => {
                rows.push(setting_row(
                    "Profile",
                    Some("留空则使用 Codex 默认 profile"),
                    text_field("默认", &draft.codex.profile, Message::UpdateCodexProfile),
                ));
                rows.push(setting_row(
                    "模型",
                    Some("留空则使用 Codex 默认模型"),
                    text_field("默认", &draft.codex.model, Message::UpdateCodexModel),
                ));
            }
            CliAgent::Claude => {
                rows.push(setting_row(
                    "模型",
                    Some("留空则使用 Claude 默认模型"),
                    text_field("默认", &draft.claude.model, Message::UpdateClaudeModel),
                ));
                rows.push(setting_row(
                    "Effort",
                    Some("推理强度"),
                    widget::pick_list(
                        ["default", "low", "medium", "high", "max"]
                            .map(str::to_string)
                            .to_vec(),
                        Some(draft.claude.effort.clone()),
                        Message::UpdateClaudeEffort,
                    )
                    .text_size(11)
                    .padding([6, 8])
                    .width(Length::Fill)
                    .style(pick_list_style)
                    .menu_style(pick_list_menu_style)
                    .into(),
                ));
            }
        }

        let permission_hint = match draft.agent {
            CliAgent::Codex => {
                "跳过操作许可询问，并继续使用只读沙箱；画布编辑经由 Gridvana MCP 完成。"
            }
            CliAgent::Claude => "跳过权限确认；仅在信任当前项目和请求时开启。",
        };
        let permission_rows = vec![setting_row(
            "默认允许",
            Some(permission_hint),
            widget::row![
                widget::Space::new().width(Length::Fill),
                widget::toggler(draft.default_allow).on_toggle(Message::SetCliDefaultAllow),
            ]
            .align_y(iced::Alignment::Center)
            .into(),
        )];

        let launch_hint = match draft.agent {
            CliAgent::Codex => "Codex 以交互 TUI 启动，MCP 工具授权直接在终端中完成。",
            CliAgent::Claude => "Claude 以交互模式启动，并通过临时 MCP 配置连接当前画布。",
        };

        widget::column![
            group("界面", visibility_rows),
            group("代理配置", rows),
            group("权限", permission_rows),
            widget::column![
                widget::text(launch_hint).size(10).color(TEXT_MUTED),
                widget::text(format!("配置文件：{}", cli_config_path().display()))
                    .size(9)
                    .font(iced::Font::MONOSPACE)
                    .color(TEXT_MUTED)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            ]
            .spacing(5),
        ]
        .spacing(16)
        .into()
    }

    fn settings_mcp_section(&self) -> Element<'_, Message> {
        let connected = self.mcp_status.starts_with("MCP 已连接");
        let endpoint = self
            .mcp_service
            .as_ref()
            .map(|service| service.endpoint().to_string());
        let endpoint_value = endpoint.clone().unwrap_or_else(|| "未启动".to_string());
        let endpoint_copy_label = if self.mcp_copy_feedback == Some(McpCopyFeedback::Endpoint) {
            "已复制"
        } else {
            "复制链接"
        };
        let prompt_copy_label = if self.mcp_copy_feedback == Some(McpCopyFeedback::AgentPrompt) {
            "已复制"
        } else {
            "复制提示词"
        };
        let endpoint_control = widget::row![
            widget::container(mono_value(endpoint_value)).width(Length::Fill),
            widget::button(widget::text(endpoint_copy_label).size(10).line_height(1.0))
                .padding([7, 10])
                .style(|theme: &Theme, status| compact_action_button_style(theme, status, false))
                .on_press_maybe(endpoint.is_some().then_some(Message::CopyMcpEndpoint)),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let prompt_control = widget::row![
            widget::Space::new().width(Length::Fill),
            widget::button(widget::text(prompt_copy_label).size(10).line_height(1.0))
                .padding([7, 10])
                .style(|theme: &Theme, status| compact_action_button_style(theme, status, false))
                .on_press_maybe(endpoint.is_some().then_some(Message::CopyMcpAgentPrompt)),
        ]
        .align_y(iced::Alignment::Center);
        let session = self
            .terminal_session
            .as_ref()
            .map(|session| format!("{} 会话运行中", session.agent))
            .unwrap_or_else(|| "无运行中的会话".to_string());

        let rows = vec![
            setting_row(
                "服务状态",
                None,
                widget::row![
                    status_dot(connected),
                    widget::text(self.mcp_status.clone())
                        .size(11)
                        .color(if connected { SUCCESS } else { TEXT_SECONDARY })
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                ]
                .spacing(7)
                .align_y(iced::Alignment::Center)
                .into(),
            ),
            setting_row("端点", None, endpoint_control.into()),
            setting_row(
                "Agent 提示词",
                Some("粘贴给 Codex 或其它支持 MCP 的代理"),
                prompt_control.into(),
            ),
            setting_row("终端会话", None, value_text(session)),
        ];

        widget::column![
            group("嵌入式 MCP", rows),
            widget::text(
                "MCP 服务随 Gridvana 启动，仅监听本机回环地址，代理通过该端点读取并编辑当前画布。"
            )
            .size(10)
            .color(TEXT_MUTED),
        ]
        .spacing(16)
        .into()
    }

    fn settings_footer(&self) -> Element<'_, Message> {
        let dirty = self.cli_config_draft != self.cli_config;
        let status: Element<'_, Message> = if self.settings_section == SettingsSection::Agent
            && let Some(error) = self.cli_save_error.as_ref()
        {
            widget::text(error)
                .size(10)
                .color(ACCENT)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .into()
        } else if dirty {
            widget::text("有未保存的更改").size(10).color(ACCENT).into()
        } else if self.settings_section == SettingsSection::Mcp
            && let Some(feedback) = self.mcp_copy_feedback
        {
            let message = match feedback {
                McpCopyFeedback::Endpoint => "MCP 服务链接已复制",
                McpCopyFeedback::AgentPrompt => "Agent 提示词已复制，可直接粘贴使用",
            };
            widget::text(message).size(10).color(SUCCESS).into()
        } else if self.settings_section == SettingsSection::Mcp {
            widget::text("链接仅在本机有效；使用时请保持 Gridvana 开启")
                .size(10)
                .color(TEXT_MUTED)
                .into()
        } else {
            widget::text(self.cli_status.clone())
                .size(10)
                .color(if self.cli_status.starts_with("CLI 配置已保存") {
                    SUCCESS
                } else {
                    TEXT_MUTED
                })
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .into()
        };

        widget::container(
            widget::row![
                widget::container(status).width(Length::Fill),
                widget::button(widget::text("关闭").size(11).line_height(1.0))
                    .padding([7, 12])
                    .style(|theme: &Theme, status| compact_action_button_style(
                        theme, status, false
                    ))
                    .on_press(Message::CloseCliSettings),
                widget::button(widget::text("保存").size(11).line_height(1.0))
                    .padding([7, 12])
                    .style(|theme: &Theme, status| compact_action_button_style(theme, status, true))
                    .on_press(Message::SaveCliConfig),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .into()
    }
}

fn group<'a>(title: &'static str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut body = widget::column![].spacing(0);
    for (index, row) in rows.into_iter().enumerate() {
        if index > 0 {
            body = body.push(widget::rule::horizontal(1));
        }
        body = body.push(row);
    }

    widget::column![
        widget::text(title).size(10).color(TEXT_MUTED),
        widget::container(body)
            .width(Length::Fill)
            .style(|_: &Theme| {
                widget::container::Style::default()
                    .background(PANEL_BACKGROUND)
                    .border(iced::Border {
                        color: BORDER_SUBTLE,
                        width: 1.0,
                        radius: 4.0.into(),
                    })
            }),
    ]
    .spacing(7)
    .into()
}

fn setting_row<'a>(
    label: &'static str,
    description: Option<&'static str>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    let mut label_column = widget::column![widget::text(label).size(11).color(TEXT_PRIMARY)]
        .spacing(3)
        .width(Length::Fixed(LABEL_WIDTH));
    if let Some(description) = description {
        label_column = label_column.push(
            widget::text(description)
                .size(9)
                .color(TEXT_MUTED)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        );
    }

    widget::container(
        widget::row![label_column, widget::container(control).width(Length::Fill)]
            .spacing(14)
            .align_y(iced::Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .into()
}

fn agent_option(agent: CliAgent, selected: CliAgent) -> Element<'static, Message> {
    let active = agent == selected;
    widget::button(
        widget::container(widget::text(agent.to_string()).size(10))
            .width(Length::Fill)
            .align_x(iced::Alignment::Center),
    )
    .on_press(Message::SelectCliAgent(agent))
    .padding([6, 10])
    .width(Length::FillPortion(1))
    .style(move |theme: &Theme, status| compact_action_button_style(theme, status, active))
    .into()
}

fn text_field<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    widget::text_input(placeholder, value)
        .on_input(on_input)
        .padding([6, 8])
        .size(11)
        .style(text_input_style)
        .width(Length::Fill)
        .into()
}

fn value_text(value: String) -> Element<'static, Message> {
    widget::text(value)
        .size(11)
        .color(TEXT_SECONDARY)
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
        .into()
}

fn mono_value(value: String) -> Element<'static, Message> {
    widget::text(value)
        .size(10)
        .font(iced::Font::MONOSPACE)
        .color(TEXT_SECONDARY)
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
        .into()
}

fn status_dot(active: bool) -> Element<'static, Message> {
    widget::container(widget::Space::new())
        .width(Length::Fixed(7.0))
        .height(Length::Fixed(7.0))
        .style(move |_| {
            widget::container::Style::default()
                .background(if active { SUCCESS } else { TEXT_MUTED })
                .border(iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 4.0.into(),
                })
        })
        .into()
}
