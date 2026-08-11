use crate::McpServer;
use crate::protocol::ServerEvent;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_HTTP_BODY_BYTES: usize = 4 * 1024 * 1024;
pub type EventHandler = Arc<dyn Fn(ServerEvent) -> Result<(), String> + Send + Sync>;

pub fn run_stdio<R, W>(
    server: &mut McpServer,
    reader: R,
    mut writer: W,
    event_handler: &EventHandler,
) -> Result<(), String>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.map_err(|error| format!("failed to read stdin: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }

        let response = server.handle_str(&line);
        dispatch_events(server, event_handler)?;
        if let Some(response) = response {
            writeln!(writer, "{response}")
                .map_err(|error| format!("failed to write stdout: {error}"))?;
            writer
                .flush()
                .map_err(|error| format!("failed to flush stdout: {error}"))?;
        }
    }
    Ok(())
}

pub fn run_http(
    listener: TcpListener,
    server: Arc<Mutex<McpServer>>,
    event_handler: EventHandler,
) -> Result<(), String> {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = Arc::clone(&server);
                let event_handler = Arc::clone(&event_handler);
                std::thread::spawn(move || {
                    if let Err(error) = handle_http_connection(stream, &server, &event_handler) {
                        eprintln!("MCP HTTP connection failed: {error}");
                    }
                });
            }
            Err(error) => return Err(format!("failed to accept HTTP connection: {error}")),
        }
    }
    Ok(())
}

pub fn run_http_until_shutdown(
    listener: TcpListener,
    server: Arc<Mutex<McpServer>>,
    event_handler: EventHandler,
    running: Arc<AtomicBool>,
) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to configure HTTP listener: {error}"))?;
    while running.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, _address)) => {
                let server = Arc::clone(&server);
                let event_handler = Arc::clone(&event_handler);
                std::thread::spawn(move || {
                    if let Err(error) = handle_http_connection(stream, &server, &event_handler) {
                        eprintln!("MCP HTTP connection failed: {error}");
                    }
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(format!("failed to accept HTTP connection: {error}")),
        }
    }
    Ok(())
}

fn handle_http_connection(
    mut stream: TcpStream,
    server: &Arc<Mutex<McpServer>>,
    event_handler: &EventHandler,
) -> Result<(), String> {
    let request = read_http_request(&mut stream)?;
    if request.method == "GET" && request.path == "/health" {
        return write_http_response(&mut stream, 200, Some(r#"{"ok":true}"#));
    }
    if request.method != "POST" || request.path != "/mcp" {
        return write_http_response(
            &mut stream,
            404,
            Some(r#"{"error":"POST /mcp is required"}"#),
        );
    }

    let input = std::str::from_utf8(&request.body)
        .map_err(|error| format!("request body is not UTF-8: {error}"))?;
    let response = handle_http_mcp_request(input, server, event_handler)?;

    match response {
        Some(response) => write_http_response(&mut stream, 200, Some(&response)),
        None => write_http_response(&mut stream, 202, None),
    }
}

fn handle_http_mcp_request(
    input: &str,
    server: &Arc<Mutex<McpServer>>,
    event_handler: &EventHandler,
) -> Result<Option<String>, String> {
    let mut server = server
        .lock()
        .map_err(|_| "MCP server lock was poisoned".to_string())?;
    let response = server.handle_str(input);
    dispatch_events(&mut server, event_handler)?;
    Ok(response)
}

struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_http_request<R>(stream: &mut R) -> Result<HttpRequest, String>
where
    R: Read,
{
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| format!("failed to read request line: {error}"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "missing HTTP method".to_string())?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| "missing HTTP path".to_string())?
        .to_string();

    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|error| format!("failed to read HTTP header: {error}"))?;
        if header == "\r\n" || header == "\n" || header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value
                .trim()
                .parse()
                .map_err(|_| "invalid Content-Length".to_string())?;
        }
    }

    if content_length > MAX_HTTP_BODY_BYTES {
        return Err(format!(
            "HTTP request body exceeds {MAX_HTTP_BODY_BYTES} bytes"
        ));
    }
    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("failed to read HTTP body: {error}"))?;

    Ok(HttpRequest { method, path, body })
}

fn write_http_response<W>(stream: &mut W, status: u16, body: Option<&str>) -> Result<(), String>
where
    W: Write,
{
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        404 => "Not Found",
        _ => "Error",
    };
    let body = body.unwrap_or_default();
    let content_type = if body.is_empty() {
        "text/plain"
    } else {
        "application/json"
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(|error| format!("failed to write HTTP response: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush HTTP response: {error}"))
}

fn dispatch_events(server: &mut McpServer, event_handler: &EventHandler) -> Result<(), String> {
    for event in server.drain_events() {
        event_handler(event)?;
    }
    Ok(())
}

pub fn stdin_reader() -> BufReader<io::Stdin> {
    BufReader::new(io::stdin())
}

#[cfg(test)]
mod tests {
    use super::{EventHandler, read_http_request, run_stdio, write_http_response};
    use crate::McpServer;
    use crate::session::{AccessMode, EditSessionStore};
    use gridvana_core::model::Project;
    use serde_json::Value;
    use std::io::Cursor;
    use std::sync::Arc;

    #[test]
    fn stdio_transport_handles_fake_client_request() {
        let store = EditSessionStore::new(Project::new_square(20.0, 8, 8), AccessMode::ReadOnly);
        let mut server = McpServer::new(store);
        let input = Cursor::new(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n",
        );
        let mut output = Vec::new();
        let handler: EventHandler = Arc::new(|_| Ok(()));

        run_stdio(&mut server, input, &mut output, &handler).unwrap();

        let response: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(
            response["result"]["serverInfo"]["name"],
            "gridvana-mcp-service"
        );
    }

    #[test]
    fn http_transport_parses_post_body_and_writes_json_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let request = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut input = Cursor::new(request.into_bytes());
        let request = read_http_request(&mut input).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/mcp");
        assert_eq!(request.body, body.as_bytes());

        let mut output = Vec::new();
        write_http_response(&mut output, 200, Some(r#"{"ok":true}"#)).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(output.ends_with(r#"{"ok":true}"#));
    }
}
