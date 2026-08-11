use crate::session::{AccessMode, CommittedProject, EditSessionStore};
use base64::Engine;
use gridvana_core::edit_ops::{EDIT_OP_JSON_SCHEMA, EditOp};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::VecDeque;

const PROTOCOL_VERSION: &str = "2025-03-26";

#[derive(Debug, Clone)]
pub enum ServerEvent {
    PreviewUpdated(Box<gridvana_core::model::Project>),
    SessionCommitted(Box<CommittedProject>),
    SessionRolledBack,
}

pub struct McpServer {
    store: EditSessionStore,
    events: VecDeque<ServerEvent>,
}

impl McpServer {
    pub fn new(store: EditSessionStore) -> Self {
        Self {
            store,
            events: VecDeque::new(),
        }
    }

    pub fn store(&self) -> &EditSessionStore {
        &self.store
    }

    pub fn set_selection<I>(&mut self, selection: I)
    where
        I: IntoIterator<Item = gridvana_core::grid::GridIndex>,
    {
        self.store.set_selection(selection);
    }

    pub fn set_timeline_selection<I>(&mut self, selection: I)
    where
        I: IntoIterator<Item = gridvana_core::model::CelPosition>,
    {
        self.store.set_timeline_selection(selection);
    }

    pub fn set_export_options(&mut self, options: gridvana_core::sprite_sheet::ExportOptions) {
        self.store.set_export_options(options);
    }

    pub fn replace_current_project(&mut self, project: gridvana_core::model::Project) {
        self.store.replace_current_project(project);
    }

    pub fn reset_edit_session(&mut self) {
        if self.store.reset_edit_session() {
            self.events.push_back(ServerEvent::SessionRolledBack);
        }
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = ServerEvent> + '_ {
        self.events.drain(..)
    }

    pub fn handle_str(&mut self, input: &str) -> Option<String> {
        let request = match serde_json::from_str::<Value>(input) {
            Ok(request) => request,
            Err(error) => {
                return Some(
                    serde_json::to_string(&error_response(
                        Value::Null,
                        -32700,
                        format!("parse error: {error}"),
                    ))
                    .expect("JSON-RPC error response should serialize"),
                );
            }
        };
        self.handle_value(request)
            .map(|response| serde_json::to_string(&response).expect("response should serialize"))
    }

    pub fn handle_value(&mut self, request: Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let response_id = id.clone().unwrap_or(Value::Null);
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return id.map(|_| error_response(response_id, -32600, "invalid request"));
        };
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

        let result = match method {
            "initialize" => Ok(self.initialize_result(&params)),
            "ping" => Ok(json!({})),
            "resources/list" => Ok(self.resources_list()),
            "resources/templates/list" => Ok(self.resource_templates_list()),
            "resources/read" => self.read_resource(params),
            "tools/list" => Ok(self.tools_list()),
            "tools/call" => self.call_tool(params),
            "notifications/initialized" | "notifications/cancelled" => Ok(json!({})),
            _ => Err((-32601, format!("method not found: {method}"))),
        };

        id.as_ref()?;

        Some(match result {
            Ok(result) => success_response(response_id, result),
            Err((code, message)) => error_response(response_id, code, message),
        })
    }

    fn initialize_result(&self, params: &Value) -> Value {
        let requested_version = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(PROTOCOL_VERSION);
        let protocol_version = if requested_version == PROTOCOL_VERSION {
            requested_version
        } else {
            PROTOCOL_VERSION
        };

        json!({
            "protocolVersion": protocol_version,
            "capabilities": {
                "resources": {},
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "gridvana-mcp-service",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Read gridvana://project/summary, gridvana://selection/current, gridvana://export/sprite-sheet, and gridvana://schema/edit-op before editing. Export configuration is read-only. If canvas_width or canvas_height is 0, use resize_canvas before pixel edits; replace_project is not supported. Apply changes in an edit session and commit once."
        })
    }

    fn resources_list(&self) -> Value {
        json!({
            "resources": [
                {
                    "uri": "gridvana://project/summary",
                    "name": "Current project summary",
                    "description": "Compact summary of the current Gridvana project",
                    "mimeType": "application/json"
                },
                {
                    "uri": "gridvana://selection/current",
                    "name": "Current selection",
                    "description": "Current frame, layer, selection, and nearby pixel context",
                    "mimeType": "application/json"
                },
                {
                    "uri": "gridvana://schema/edit-op",
                    "name": "Edit operation schema",
                    "description": "JSON Schema for Gridvana EditOp arrays",
                    "mimeType": "application/schema+json"
                },
                {
                    "uri": "gridvana://export/sprite-sheet",
                    "name": "Sprite sheet export configuration",
                    "description": "Read-only summary of the current editor sprite sheet export options",
                    "mimeType": "application/json"
                },
                {
                    "uri": "gridvana://frame/active.png",
                    "name": "Active frame composite",
                    "description": "Rendered PNG composite of the active edit-session frame, or the committed project when no session is active",
                    "mimeType": "image/png"
                }
            ]
        })
    }

    fn resource_templates_list(&self) -> Value {
        json!({ "resourceTemplates": [] })
    }

    fn read_resource(&self, params: Value) -> RpcResult {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| (-32602, "resources/read requires a uri".to_string()))?;

        let content = match uri {
            "gridvana://project/summary" => json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&self.store.project_summary())
                    .map_err(internal_serialization_error)?
            }),
            "gridvana://selection/current" => json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&self.store.selection_summary())
                    .map_err(internal_serialization_error)?
            }),
            "gridvana://schema/edit-op" => json!({
                "uri": uri,
                "mimeType": "application/schema+json",
                "text": EDIT_OP_JSON_SCHEMA
            }),
            "gridvana://export/sprite-sheet" => json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(self.store.export_options())
                    .map_err(internal_serialization_error)?
            }),
            "gridvana://frame/active.png" => {
                let project = self.store.preview_project();
                let position = project
                    .active_frame_position()
                    .ok_or_else(|| (-32002, "active frame does not exist".to_string()))?;
                let png = gridvana_core::io::render_frame_png_bytes(project, position)
                    .map_err(|error| (-32603, format!("frame rendering failed: {error}")))?;
                json!({
                    "uri": uri,
                    "mimeType": "image/png",
                    "blob": base64::engine::general_purpose::STANDARD.encode(png)
                })
            }
            _ => return Err((-32002, format!("resource not found: {uri}"))),
        };

        Ok(json!({
            "contents": [content]
        }))
    }

    fn tools_list(&self) -> Value {
        let mut tools = Vec::new();
        if self.store.access_mode() != AccessMode::ReadOnly {
            tools.extend([
                tool(
                    "gridvana_start_edit_session",
                    "Start an isolated edit session against the current project revision",
                    json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                ),
                tool(
                    "gridvana_preview_edit_ops",
                    "Validate EditOps and return their projected summary without mutating the session",
                    edit_ops_tool_schema(),
                ),
                tool(
                    "gridvana_apply_edit_ops",
                    "Apply validated EditOps to the session working copy",
                    edit_ops_tool_schema(),
                ),
                tool(
                    "gridvana_commit_session",
                    "Commit the session working copy as one project change",
                    session_revision_schema(),
                ),
                tool(
                    "gridvana_rollback_session",
                    "Discard the session working copy",
                    json!({
                        "type": "object",
                        "properties": {
                            "session_id": { "type": "string", "minLength": 1 }
                        },
                        "required": ["session_id"],
                        "additionalProperties": false
                    }),
                ),
            ]);
        }
        json!({ "tools": tools })
    }

    fn call_tool(&mut self, params: Value) -> RpcResult {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| (-32602, "tools/call requires a name".to_string()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = match name {
            "gridvana_start_edit_session" => {
                self.store.start_edit_session().and_then(to_json_value)
            }
            "gridvana_preview_edit_ops" => parse_arguments::<EditOpsArguments>(arguments)
                .and_then(|input| {
                    self.store
                        .preview_edit_ops(&input.session_id, input.base_revision, &input.ops)
                })
                .and_then(|preview| {
                    let result = to_json_value(&preview)?;
                    self.events
                        .push_back(ServerEvent::PreviewUpdated(Box::new(preview.project)));
                    Ok(result)
                }),
            "gridvana_apply_edit_ops" => parse_arguments::<EditOpsArguments>(arguments)
                .and_then(|input| {
                    self.store
                        .apply_edit_ops(&input.session_id, input.base_revision, &input.ops)
                })
                .and_then(|preview| {
                    let result = to_json_value(&preview)?;
                    self.events
                        .push_back(ServerEvent::PreviewUpdated(Box::new(preview.project)));
                    Ok(result)
                }),
            "gridvana_commit_session" => parse_arguments::<SessionRevisionArguments>(arguments)
                .and_then(|input| {
                    self.store
                        .commit_session(&input.session_id, input.base_revision)
                })
                .and_then(|commit| {
                    let result = to_json_value(commit.result.clone())?;
                    self.events
                        .push_back(ServerEvent::SessionCommitted(Box::new(commit)));
                    Ok(result)
                }),
            "gridvana_rollback_session" => parse_arguments::<SessionArguments>(arguments)
                .and_then(|input| self.store.rollback_session(&input.session_id))
                .and_then(|result| {
                    let result = to_json_value(result)?;
                    self.events.push_back(ServerEvent::SessionRolledBack);
                    Ok(result)
                }),
            _ => return Err((-32602, format!("unknown tool: {name}"))),
        };

        Ok(match result {
            Ok(value) => tool_result(value, false),
            Err(error) => tool_result(json!({ "ok": false, "error": error.to_string() }), true),
        })
    }
}

type RpcResult = Result<Value, (i64, String)>;

#[derive(Deserialize)]
struct EditOpsArguments {
    session_id: String,
    base_revision: u64,
    ops: Vec<EditOp>,
}

#[derive(Deserialize)]
struct SessionRevisionArguments {
    session_id: String,
    base_revision: u64,
}

#[derive(Deserialize)]
struct SessionArguments {
    session_id: String,
}

fn parse_arguments<T>(arguments: Value) -> Result<T, crate::session::SessionError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(arguments).map_err(|error| {
        let message = error.to_string();
        let message = if message.contains("replace_project") {
            "replace_project is not supported. If the canvas is empty, use resize_canvas with canvas_width and canvas_height, then apply stable-ID pixel/layer/frame operations."
                .to_string()
        } else {
            message
        };
        crate::session::SessionError::InvalidEdit(message)
    })
}

fn to_json_value<T>(value: T) -> Result<Value, crate::session::SessionError>
where
    T: serde::Serialize,
{
    serde_json::to_value(value)
        .map_err(|error| crate::session::SessionError::InvalidEdit(error.to_string()))
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

fn edit_ops_tool_schema() -> Value {
    let edit_schema: Value =
        serde_json::from_str(EDIT_OP_JSON_SCHEMA).expect("embedded EditOp schema should be valid");
    json!({
        "type": "object",
        "properties": {
            "session_id": { "type": "string", "minLength": 1 },
            "base_revision": { "type": "integer", "minimum": 0 },
            "ops": {
                "type": "array",
                "minItems": 1,
                "maxItems": 128,
                "items": edit_schema["items"].clone()
            },
        },
        "required": ["session_id", "base_revision", "ops"],
        "additionalProperties": false,
        "$defs": edit_schema["$defs"].clone()
    })
}

fn session_revision_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "session_id": { "type": "string", "minLength": 1 },
            "base_revision": { "type": "integer", "minimum": 0 }
        },
        "required": ["session_id", "base_revision"],
        "additionalProperties": false
    })
}

fn tool_result(value: Value, is_error: bool) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&value).expect("tool result should serialize")
        }],
        "structuredContent": value,
        "isError": is_error
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() }
    })
}

fn internal_serialization_error(error: serde_json::Error) -> (i64, String) {
    (-32603, format!("serialization failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::McpServer;
    use crate::session::{AccessMode, EditSessionStore};
    use gridvana_core::model::Project;
    use serde_json::{Value, json};

    fn request(id: u64, method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    fn call(server: &mut McpServer, id: u64, method: &str, params: Value) -> Value {
        server
            .handle_value(request(id, method, params))
            .expect("request should produce a response")["result"]
            .clone()
    }

    #[test]
    fn fake_client_reads_project_summary_and_edit_schema() {
        let store = EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let mut server = McpServer::new(store);

        let resources = call(&mut server, 1, "resources/list", json!({}));
        assert_eq!(resources["resources"].as_array().unwrap().len(), 5);

        let templates = call(&mut server, 2, "resources/templates/list", json!({}));
        assert_eq!(templates["resourceTemplates"], json!([]));

        let summary = call(
            &mut server,
            3,
            "resources/read",
            json!({ "uri": "gridvana://project/summary" }),
        );
        let summary_text = summary["contents"][0]["text"].as_str().unwrap();
        assert!(summary_text.contains("\"canvas_width\": 8"));

        let schema = call(
            &mut server,
            4,
            "resources/read",
            json!({ "uri": "gridvana://schema/edit-op" }),
        );
        assert!(
            schema["contents"][0]["text"]
                .as_str()
                .unwrap()
                .contains("set_cel_pixels")
        );

        let export = call(
            &mut server,
            6,
            "resources/read",
            json!({ "uri": "gridvana://export/sprite-sheet" }),
        );
        let export_text = export["contents"][0]["text"].as_str().unwrap();
        assert!(export_text.contains("\"layout\": \"horizontal\""));
        assert!(export_text.contains("\"metadata_format\": \"array\""));

        let frame = call(
            &mut server,
            5,
            "resources/read",
            json!({ "uri": "gridvana://frame/active.png" }),
        );
        assert_eq!(frame["contents"][0]["mimeType"], "image/png");
        assert!(
            frame["contents"][0]["blob"]
                .as_str()
                .unwrap()
                .starts_with("iVBOR")
        );
    }

    #[test]
    fn fake_client_applies_and_commits_edit_ops() {
        let store = EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadWrite);
        let mut server = McpServer::new(store);
        let committed_frame = call(
            &mut server,
            10,
            "resources/read",
            json!({ "uri": "gridvana://frame/active.png" }),
        );
        let committed_blob = committed_frame["contents"][0]["blob"]
            .as_str()
            .unwrap()
            .to_string();

        let started = call(
            &mut server,
            1,
            "tools/call",
            json!({ "name": "gridvana_start_edit_session", "arguments": {} }),
        );
        let session_id = started["structuredContent"]["session_id"].as_str().unwrap();
        let base_revision = started["structuredContent"]["base_revision"]
            .as_u64()
            .unwrap();
        let edit = json!({
            "session_id": session_id,
            "base_revision": base_revision,
            "ops": [{
                "type": "set_cel_pixels",
                "frame_id": 2,
                "layer_id": 1,
                "pixels": [{
                    "index": { "x": 2, "y": 3 },
                    "color": { "r": 1.0, "g": 0.5, "b": 0.0, "a": 1.0 }
                }]
            }]
        });

        let applied = call(
            &mut server,
            2,
            "tools/call",
            json!({ "name": "gridvana_apply_edit_ops", "arguments": edit }),
        );
        assert!(!applied["isError"].as_bool().unwrap());
        assert_eq!(
            applied["structuredContent"]["impact"]["cel_ids"],
            json!([3])
        );
        assert_eq!(server.store().project_summary().total_colored_pixels, 0);
        let working_frame = call(
            &mut server,
            11,
            "resources/read",
            json!({ "uri": "gridvana://frame/active.png" }),
        );
        assert_ne!(
            working_frame["contents"][0]["blob"].as_str().unwrap(),
            committed_blob
        );

        let committed = call(
            &mut server,
            3,
            "tools/call",
            json!({
                "name": "gridvana_commit_session",
                "arguments": {
                    "session_id": session_id,
                    "base_revision": base_revision
                }
            }),
        );
        assert_eq!(committed["structuredContent"]["revision"], 1);
        assert_eq!(server.store().project_summary().total_colored_pixels, 1);
        let events = server.drain_events().collect::<Vec<_>>();
        assert!(matches!(events[0], super::ServerEvent::PreviewUpdated(_)));
        assert!(matches!(events[1], super::ServerEvent::SessionCommitted(_)));
    }

    #[test]
    fn fake_client_can_create_canvas_and_draw_from_empty_editor_state() {
        let store = EditSessionStore::new(Project::new_square(20.0, 0, 0), AccessMode::ReadWrite);
        let mut server = McpServer::new(store);
        let started = call(
            &mut server,
            1,
            "tools/call",
            json!({ "name": "gridvana_start_edit_session", "arguments": {} }),
        );
        let session_id = started["structuredContent"]["session_id"].as_str().unwrap();
        let base_revision = started["structuredContent"]["base_revision"]
            .as_u64()
            .unwrap();
        let rejected = call(
            &mut server,
            2,
            "tools/call",
            json!({
                "name": "gridvana_apply_edit_ops",
                "arguments": {
                    "session_id": session_id,
                    "base_revision": base_revision,
                    "ops": [{ "type": "replace_project", "project": {} }]
                }
            }),
        );
        assert!(rejected["isError"].as_bool().unwrap());
        assert!(
            rejected["structuredContent"]["error"]
                .as_str()
                .unwrap()
                .contains("resize_canvas")
        );
        let applied = call(
            &mut server,
            3,
            "tools/call",
            json!({
                "name": "gridvana_apply_edit_ops",
                "arguments": {
                    "session_id": session_id,
                    "base_revision": base_revision,
                    "ops": [
                        { "type": "resize_canvas", "canvas_width": 16, "canvas_height": 16 },
                        { "type": "rename_layer", "layer_id": 1, "name": "Ball" },
                        {
                            "type": "set_cel_pixels",
                            "layer_id": 1,
                            "frame_id": 2,
                            "pixels": [{
                                "index": { "x": 8, "y": 8 },
                                "color": { "r": 0.2, "g": 0.5, "b": 1.0, "a": 1.0 }
                            }]
                        }
                    ]
                }
            }),
        );
        assert!(!applied["isError"].as_bool().unwrap());
        assert_eq!(
            applied["structuredContent"]["project_summary"]["canvas_width"],
            16
        );
        let committed = call(
            &mut server,
            4,
            "tools/call",
            json!({
                "name": "gridvana_commit_session",
                "arguments": {
                    "session_id": session_id,
                    "base_revision": base_revision
                }
            }),
        );
        assert!(!committed["isError"].as_bool().unwrap());
        assert_eq!(server.store().project().canvas_width, 16);
        assert_eq!(server.store().project().canvas_height, 16);
        assert_eq!(server.store().project_summary().total_colored_pixels, 1);
    }

    #[test]
    fn read_only_server_does_not_advertise_write_tools() {
        let store = EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadOnly);
        let mut server = McpServer::new(store);
        let tools = call(&mut server, 1, "tools/list", json!({}));
        assert!(tools["tools"].as_array().unwrap().is_empty());
    }
}
