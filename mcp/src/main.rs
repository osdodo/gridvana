use gridvana_core::io::{load_project, save_project};
use gridvana_mcp::McpServer;
use gridvana_mcp::protocol::ServerEvent;
use gridvana_mcp::session::{AccessMode, EditSessionStore};
use gridvana_mcp::transport::{EventHandler, run_http, run_stdio, stdin_reader};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

enum Transport {
    Stdio,
    Http(String),
}

struct Args {
    transport: Transport,
    project_path: PathBuf,
    access_mode: AccessMode,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("gridvana-mcp-service: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args().skip(1))?;
    let project = load_project(&args.project_path)
        .map_err(|error| format!("failed to load {}: {error}", args.project_path.display()))?;
    let store = EditSessionStore::new(project, args.access_mode);
    let server = McpServer::new(store);
    let project_path = args.project_path.clone();
    let event_handler: EventHandler = Arc::new(move |event| {
        if let ServerEvent::SessionCommitted(commit) = event {
            save_project(&commit.after, &project_path)
                .map_err(|error| format!("failed to save {}: {error}", project_path.display()))?;
        }
        Ok(())
    });

    match args.transport {
        Transport::Stdio => {
            let mut server = server;
            run_stdio(
                &mut server,
                stdin_reader(),
                std::io::stdout(),
                &event_handler,
            )
        }
        Transport::Http(address) => {
            let address = parse_loopback_address(&address)?;
            let listener = TcpListener::bind(address)
                .map_err(|error| format!("failed to bind {address}: {error}"))?;
            let address = listener
                .local_addr()
                .map_err(|error| format!("failed to read listener address: {error}"))?;
            eprintln!("Gridvana MCP listening on http://{address}/mcp");
            run_http(listener, Arc::new(Mutex::new(server)), event_handler)
        }
    }
}

fn parse_loopback_address(address: &str) -> Result<SocketAddr, String> {
    let address = address
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid HTTP address {address}: {error}"))?;
    if address.ip().is_loopback() {
        Ok(address)
    } else {
        Err(format!("HTTP address must be loopback, got {address}"))
    }
}

fn parse_args<I>(arguments: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut arguments = arguments.into_iter();
    let mut transport = None;
    let mut project_path = None;
    let mut access_mode = AccessMode::ReadOnly;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--stdio" => set_transport(&mut transport, Transport::Stdio)?,
            "--http" => {
                let address = arguments
                    .next()
                    .ok_or_else(|| "--http requires an address".to_string())?;
                set_transport(&mut transport, Transport::Http(address))?;
            }
            "--project" => {
                project_path = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--project requires a path".to_string())?,
                ));
            }
            "--readonly" => access_mode = AccessMode::ReadOnly,
            "--write" => access_mode = AccessMode::ReadWrite,
            "--help" | "-h" => return Err(usage().to_string()),
            _ => return Err(format!("unknown argument: {argument}\n{}", usage())),
        }
    }

    Ok(Args {
        transport: transport.ok_or_else(|| format!("transport is required\n{}", usage()))?,
        project_path: project_path.ok_or_else(|| format!("--project is required\n{}", usage()))?,
        access_mode,
    })
}

fn set_transport(current: &mut Option<Transport>, next: Transport) -> Result<(), String> {
    if current.is_some() {
        Err("choose exactly one of --stdio or --http".to_string())
    } else {
        *current = Some(next);
        Ok(())
    }
}

fn usage() -> &'static str {
    "usage: gridvana-mcp-service (--stdio | --http 127.0.0.1:17321) --project FILE [--readonly | --write]"
}

#[cfg(test)]
mod tests {
    use super::{AccessMode, Transport, parse_args, parse_loopback_address};

    #[test]
    fn defaults_to_read_only() {
        let args = parse_args([
            "--stdio".to_string(),
            "--project".to_string(),
            "demo.gvn".to_string(),
        ])
        .unwrap();
        assert!(matches!(args.transport, Transport::Stdio));
        assert_eq!(args.access_mode, AccessMode::ReadOnly);
    }

    #[test]
    fn write_flag_enables_writes() {
        let args = parse_args([
            "--http".to_string(),
            "127.0.0.1:0".to_string(),
            "--project".to_string(),
            "demo.gvn".to_string(),
            "--write".to_string(),
        ])
        .unwrap();
        assert_eq!(args.access_mode, AccessMode::ReadWrite);
    }

    #[test]
    fn http_address_must_be_loopback() {
        assert!(parse_loopback_address("127.0.0.1:17321").is_ok());
        assert!(parse_loopback_address("0.0.0.0:17321").is_err());
    }
}
