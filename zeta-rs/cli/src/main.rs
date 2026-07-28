use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::InProcessClientOptions;
use zeta_app_server_client::InProcessTransport;
use zeta_app_server_client::start_in_process_client;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_app_server_protocol::protocol::turn::{InputItem, TurnStartParams};
use zeta_protocol::{CommandId, ThreadItem, TurnStatus};

fn main() {
    let mut arguments = env::args().skip(1);
    let outcome = match arguments.next() {
        None => interactive(),
        Some(command) => match command.as_str() {
            "ask" => ask(arguments.collect::<Vec<_>>().join(" ")),
            "exec" => execute(arguments.collect()),
            "app-server" => app_server_command(arguments.collect()),
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
    let client = in_process_client()?;
    zeta_tui::run(client, zeta_tui::TuiOptions::new("TUI conversation"))
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn run_app_server(arguments: Vec<String>) -> Result<(), String> {
    if arguments.as_slice() != ["--listen", "stdio://"] {
        return Err("usage: zeta app-server --listen stdio://".into());
    }
    let mut options = LocalAppServerOptions::new(state_root());
    if let Some(workspace_root) = env::var_os("ZETA_WORKSPACE_ROOT") {
        options = options.with_workspace_root(workspace_root);
    }
    options = options.with_tool_workspace(current_workspace()?);
    open_local_app_server(options)
        .map_err(|error| error.to_string())?
        .serve_stdio()
        .map_err(|error| error.to_string())
}

fn app_server_command(arguments: Vec<String>) -> Result<(), String> {
    run_app_server(arguments)
}

fn state_root() -> PathBuf {
    env::var_os("ZETA_STATE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".zeta"))
}

fn current_workspace() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("could not resolve current workspace: {error}"))
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
    let mut client = in_process_client()?;
    let session = client
        .create_session(SessionCreateParams {
            command_id: CommandId::new(request_key("session"))
                .expect("generated command ID is non-empty"),
            title: title.into(),
        })
        .map_err(|error| error.to_string())?;
    let thread = client
        .create_session_thread(SessionThreadCreateParams {
            command_id: CommandId::new(request_key("thread"))
                .expect("generated command ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: session.session.sequence,
            title: title.into(),
        })
        .map_err(|error| error.to_string())?;
    client
        .start_turn(TurnStartParams {
            command_id: CommandId::new(request_key("turn"))
                .expect("generated command ID is non-empty"),
            session_id: session.session.session_id,
            thread_id: thread.thread_id.clone(),
            expected_sequence: 1,
            input: vec![InputItem::Text { text: prompt }],
        })
        .map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let snapshot = client
            .read_thread(ThreadReadParams {
                thread_id: thread.thread_id.clone(),
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
            | TurnStatus::Cancelling => {
                if Instant::now() >= deadline {
                    return Err("timed out waiting for the Turn to complete".into());
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn in_process_client() -> Result<AppServerClient<InProcessTransport>, String> {
    start_in_process_client(
        InProcessClientOptions::new(
            state_root(),
            ClientInfo {
                name: "zeta-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_tool_workspace(current_workspace()?),
    )
    .map_err(|error| error.to_string())
}

fn request_key(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{timestamp}", std::process::id())
}
