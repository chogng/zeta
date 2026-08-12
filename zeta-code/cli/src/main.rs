use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::InProcessClientOptions;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_client::local_profile_root;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionRequest, SessionRequestParams, SessionRequestResult,
    SessionThreadReadParams, SessionThreadSubscribeParams,
};
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_protocol::{CommandId, ThreadItem, TurnStatus};

fn main() {
    let mut arguments = env::args().skip(1);
    let outcome = match arguments.next() {
        None => interactive(),
        Some(command) => match command.as_str() {
            "ask" => ask(arguments.collect::<Vec<_>>().join(" ")),
            "exec" => execute(arguments.collect()),
            "app-server" => app_server_command(arguments.collect()),
            "mcp-server" => mcp_server_command(arguments.collect()),
            _ => Err(format!("unknown command: {command}")),
        },
    };
    if let Err(message) = outcome {
        eprintln!("zeta: {message}");
        std::process::exit(1);
    }
}

fn ask(prompt: String) -> Result<(), String> {
    if prompt.is_empty() {
        return Err("ask requires a prompt".into());
    }
    let response = run_prompt(prompt, "CLI conversation")?;
    println!("{response}");
    Ok(())
}

fn interactive() -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(
            "interactive mode requires a TTY; use `zeta ask` or `zeta exec` instead".into(),
        );
    }
    let session = AppServerSession::start_embedded(
        InProcessClientOptions::new(
            local_profile_root(),
            ClientInfo {
                name: "zeta-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_capabilities(zeta_tui::client_capabilities())
        .with_workspace_root(configured_workspace()?),
    )
    .map_err(|error| error.to_string())?;
    zeta_tui::run(session, zeta_tui::TuiOptions::new("TUI conversation"))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn run_app_server(arguments: Vec<String>) -> Result<(), String> {
    if arguments.as_slice() != ["--listen", "stdio://"] {
        return Err("usage: zeta app-server --listen stdio://".into());
    }
    let options = LocalAppServerOptions::new(local_profile_root())
        .with_workspace_root(configured_workspace()?);
    open_local_app_server(options)
        .map_err(|error| error.to_string())?
        .serve_stdio()
        .map_err(|error| error.to_string())
}

fn app_server_command(arguments: Vec<String>) -> Result<(), String> {
    run_app_server(arguments)
}

fn mcp_server_command(arguments: Vec<String>) -> Result<(), String> {
    let options =
        zeta_mcp_server::McpServerOptions::new(local_profile_root(), configured_workspace()?);
    match arguments.as_slice() {
        [] => zeta_mcp_server::run_stdio(options).map_err(|error| error.to_string()),
        [listen, address] if listen == "--listen" && address == "stdio://" => {
            zeta_mcp_server::run_stdio(options).map_err(|error| error.to_string())
        }
        [listen, address] if listen == "--listen" => {
            let (socket, path) = parse_mcp_http_address(address)?;
            let token = env::var("ZETA_MCP_BEARER_TOKEN")
                .map_err(|_| "ZETA_MCP_BEARER_TOKEN is required for Streamable HTTP".to_string())?;
            let mut http_options = zeta_mcp_server::HttpServerOptions::new(socket, path, token);
            if let Ok(origin) = env::var("ZETA_MCP_ALLOWED_ORIGIN") {
                http_options = http_options.with_allowed_origin(origin);
            }
            zeta_mcp_server::run_http(options, http_options).map_err(|error| error.to_string())
        }
        _ => Err("usage: zeta mcp-server [--listen stdio://|http://IP:PORT/PATH]".into()),
    }
}

fn parse_mcp_http_address(address: &str) -> Result<(std::net::SocketAddr, String), String> {
    let remainder = address.strip_prefix("http://").ok_or_else(|| {
        "MCP HTTP listener must use http:// behind a TLS reverse proxy".to_string()
    })?;
    let (authority, path) = remainder
        .split_once('/')
        .ok_or_else(|| "MCP HTTP listener must include an endpoint path".to_string())?;
    let socket = authority
        .parse()
        .map_err(|_| "MCP HTTP listener authority must be an IP:PORT pair".to_string())?;
    Ok((socket, format!("/{path}")))
}

fn current_workspace() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("could not resolve current workspace: {error}"))
}

fn configured_workspace() -> Result<PathBuf, String> {
    env::var_os("ZETA_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(current_workspace)
}

fn execute(arguments: Vec<String>) -> Result<(), String> {
    let prompt = arguments.join(" ");
    if prompt.is_empty() {
        return Err("exec requires a prompt".into());
    }
    println!("{}", run_prompt(prompt, "CLI execution")?);
    Ok(())
}

fn run_prompt(prompt: String, title: &str) -> Result<String, String> {
    let mut app_server = in_process_session()?;
    let result = run_prompt_in_session(&mut app_server, prompt, title);
    let shutdown = app_server.shutdown().map_err(|error| error.to_string());
    match (result, shutdown) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(response), Ok(())) => Ok(response),
    }
}

fn run_prompt_in_session(
    app_server: &mut AppServerSession,
    prompt: String,
    title: &str,
) -> Result<String, String> {
    let mut client = app_server.client();
    let session = client
        .create_session(SessionCreateParams {
            command_id: CommandId::new(request_key("session"))
                .expect("generated command ID is non-empty"),
            title: title.into(),
        })
        .map_err(|error| error.to_string())?;
    let thread = client
        .request_session(SessionRequestParams {
            command_id: CommandId::new(request_key("thread"))
                .expect("generated command ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: session.session.sequence,
            request: SessionRequest::CreateThread {
                title: title.into(),
            },
        })
        .map_err(|error| error.to_string())?;
    let SessionRequestResult::Thread(thread) = thread else {
        return Err("app server returned a non-Thread result for CreateThread".into());
    };
    client
        .subscribe_session_thread(SessionThreadSubscribeParams {
            session_id: session.session.session_id.clone(),
            thread_id: thread.thread_id.clone(),
            after_sequence: 0,
            history: None,
        })
        .map_err(|error| error.to_string())?;
    let events = app_server
        .take_events()
        .map_err(|error| error.to_string())?;
    let SessionRequestResult::Turn(_) = client
        .request_session(SessionRequestParams {
            command_id: CommandId::new(request_key("turn"))
                .expect("generated command ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: 1,
            request: SessionRequest::StartTurn {
                thread_id: thread.thread_id.clone(),
                input: vec![InputItem::Text { text: prompt }],
            },
        })
        .map_err(|error| error.to_string())?
    else {
        return Err("app server returned a non-Turn result for StartTurn".into());
    };
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("timed out waiting for the Turn to complete".into());
        }
        let event = events
            .recv_timeout(remaining)
            .map_err(|error| format!("could not receive App Server progress: {error}"))?;
        match event {
            AppServerEvent::Notification(ServerNotification::SessionThreadUpdate(update))
                if update.thread_id == thread.thread_id => {}
            AppServerEvent::ConnectionClosed(reason) => {
                return Err(format!("App Server connection closed: {reason:?}"));
            }
            AppServerEvent::Notification(_) => continue,
        }
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: session.session.session_id.clone(),
                thread_id: thread.thread_id.clone(),
                history: None,
            })
            .map_err(|error| error.to_string())?;
        let turn = snapshot
            .thread
            .turns
            .last()
            .ok_or_else(|| "app server did not create a Turn".to_string())?;
        match turn.status {
            TurnStatus::Completed => {
                return turn
                    .items
                    .iter()
                    .find_map(|item| match item {
                        ThreadItem::AgentMessage { text, .. } => Some(text.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| "app server completed without an agent message".into());
            }
            TurnStatus::Failed => return Err("app server failed the Turn".into()),
            TurnStatus::Interrupted => return Err("app server interrupted the Turn".into()),
            TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
            | TurnStatus::Cancelling => {}
        }
    }
}

fn in_process_session() -> Result<AppServerSession, String> {
    AppServerSession::start_embedded(
        InProcessClientOptions::new(
            local_profile_root(),
            ClientInfo {
                name: "zeta-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_workspace_root(configured_workspace()?),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

fn request_key(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{timestamp}", std::process::id())
}
