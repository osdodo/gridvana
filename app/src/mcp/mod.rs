use gridvana_core::grid::GridIndex;
use gridvana_core::model::{CelPosition, Project};
use gridvana_mcp::McpServer;
use gridvana_mcp::protocol::ServerEvent;
use gridvana_mcp::session::{AccessMode, EditSessionStore};
use gridvana_mcp::transport::{EventHandler, run_http_until_shutdown};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct EmbeddedMcpService {
    endpoint: String,
    server: Arc<Mutex<McpServer>>,
    events: Arc<Mutex<VecDeque<ServerEvent>>>,
    running: Arc<AtomicBool>,
    project_snapshot: Vec<u8>,
}

impl EmbeddedMcpService {
    pub fn start(
        project: &Project,
        selection: impl IntoIterator<Item = GridIndex>,
    ) -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("failed to bind embedded MCP server: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("failed to read embedded MCP address: {error}"))?;
        ensure_loopback(address)?;

        let mut store = EditSessionStore::new(project.clone(), AccessMode::ReadWrite);
        store.set_selection(selection);
        let server = Arc::new(Mutex::new(McpServer::new(store)));
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let running = Arc::new(AtomicBool::new(true));
        let event_queue = Arc::clone(&events);
        let event_handler: EventHandler = Arc::new(move |event| {
            event_queue
                .lock()
                .map_err(|_| "embedded MCP event queue was poisoned".to_string())?
                .push_back(event);
            Ok(())
        });

        let thread_server = Arc::clone(&server);
        let thread_running = Arc::clone(&running);
        std::thread::spawn(move || {
            if let Err(error) =
                run_http_until_shutdown(listener, thread_server, event_handler, thread_running)
            {
                eprintln!("embedded MCP server stopped: {error}");
            }
        });

        Ok(Self {
            endpoint: format!("http://{address}/mcp"),
            server,
            events,
            running,
            project_snapshot: project_snapshot(project)?,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn sync_editor_state(
        &mut self,
        project: &Project,
        selection: impl IntoIterator<Item = GridIndex>,
    ) -> Result<(), String> {
        let snapshot = project_snapshot(project)?;
        let mut server = self
            .server
            .lock()
            .map_err(|_| "embedded MCP server lock was poisoned".to_string())?;
        if snapshot != self.project_snapshot {
            server.replace_current_project(project.clone());
            self.project_snapshot = snapshot;
        }
        server.set_selection(selection);
        let local_events = server.drain_events().collect::<Vec<_>>();
        drop(server);
        if !local_events.is_empty() {
            self.events
                .lock()
                .map_err(|_| "embedded MCP event queue was poisoned".to_string())?
                .extend(local_events);
        }
        Ok(())
    }

    pub fn set_timeline_selection(
        &self,
        selection: impl IntoIterator<Item = CelPosition>,
    ) -> Result<(), String> {
        self.server
            .lock()
            .map_err(|_| "embedded MCP server lock was poisoned".to_string())?
            .set_timeline_selection(selection);
        Ok(())
    }

    pub fn drain_events(&self) -> Result<Vec<ServerEvent>, String> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| "embedded MCP event queue was poisoned".to_string())?;
        Ok(events.drain(..).collect())
    }

    pub fn accept_server_project(&mut self, project: &Project) -> Result<(), String> {
        self.project_snapshot = project_snapshot(project)?;
        Ok(())
    }

    pub fn replace_editor_project(&mut self, project: &Project) -> Result<(), String> {
        self.reset_edit_session()?;
        self.sync_editor_state(project, std::iter::empty())
    }

    pub fn set_export_options(
        &self,
        options: gridvana_core::sprite_sheet::ExportOptions,
    ) -> Result<(), String> {
        self.server
            .lock()
            .map_err(|_| "embedded MCP server lock was poisoned".to_string())?
            .set_export_options(options);
        Ok(())
    }

    pub fn reset_edit_session(&self) -> Result<(), String> {
        self.server
            .lock()
            .map_err(|_| "embedded MCP server lock was poisoned".to_string())?
            .reset_edit_session();
        Ok(())
    }
}

impl Drop for EmbeddedMcpService {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
    }
}

fn ensure_loopback(address: SocketAddr) -> Result<(), String> {
    if address.ip().is_loopback() {
        Ok(())
    } else {
        Err(format!("embedded MCP address is not loopback: {address}"))
    }
}

fn project_snapshot(project: &Project) -> Result<Vec<u8>, String> {
    serde_json::to_vec(project).map_err(|error| format!("failed to snapshot project: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedMcpService, project_snapshot};
    use gridvana_core::grid::GridIndex;
    use gridvana_core::model::{CelPosition, Project};
    use gridvana_mcp::McpServer;
    use gridvana_mcp::protocol::ServerEvent;
    use gridvana_mcp::session::{AccessMode, EditSessionStore};
    use std::collections::VecDeque;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    #[test]
    fn replacing_editor_project_updates_snapshot_and_clears_selection() {
        let original = Project::new_square(20.0, 2, 2);
        let mut store = EditSessionStore::new(original.clone(), AccessMode::ReadWrite);
        store.set_selection([GridIndex { x: 0, y: 0 }, GridIndex { x: 1, y: 1 }]);
        store.set_timeline_selection([CelPosition {
            layer_id: original.active_layer_id,
            frame_id: original.active_frame_id,
        }]);
        let mut service = EmbeddedMcpService {
            endpoint: String::new(),
            server: Arc::new(Mutex::new(McpServer::new(store))),
            events: Arc::new(Mutex::new(VecDeque::<ServerEvent>::new())),
            running: Arc::new(AtomicBool::new(false)),
            project_snapshot: project_snapshot(&original).unwrap(),
        };
        let replacement = Project::new_square(20.0, 7, 5);

        service.replace_editor_project(&replacement).unwrap();

        let server = service.server.lock().unwrap();
        assert_eq!(server.store().project(), &replacement);
        assert_eq!(server.store().selection_summary().selected_cells, 0);
        assert!(!server.store().selection_summary().timeline.active);
    }
}
