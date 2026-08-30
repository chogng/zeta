use signal_hook::SigId;
use signal_hook::consts::SIGINT;
use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_client::local_profile_root;
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_exec::AppServerTarget;
use zeta_exec::DiscardExecEventSink;
use zeta_exec::EmbeddedAppServerOptions;
use zeta_exec::ExecEntry;
use zeta_exec::ExecError;
use zeta_exec::ExecFailure;
use zeta_exec::ExecOutcome;
use zeta_exec::ExecRunRequest;
use zeta_exec::ExecRunner;
use zeta_exec::HeadlessApprovalMode;
use zeta_exec::JsonLinesExecEventSink;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

mod remote;

fn main() {
    let mut arguments = env::args().skip(1);
    let outcome = match arguments.next() {
        None => interactive().map_err(CliError::failure),
        Some(command) => match command.as_str() {
            "ask" => ask(arguments.collect::<Vec<_>>().join(" ")),
            "exec" => execute(arguments.collect()),
            "app-server" => app_server_command(arguments.collect()).map_err(CliError::failure),
            "mcp-server" => mcp_server_command(arguments.collect()).map_err(CliError::failure),
            "remote" => remote::run(arguments.collect()).map_err(CliError::failure),
            "remote-server" => {
                zeta_server_host::run(std::iter::once("remote-server".to_owned()).chain(arguments))
                    .map_err(CliError::failure)
            }
            _ => Err(CliError::usage(format!("unknown command: {command}"))),
        },
    };
    if let Err(error) = outcome {
        eprintln!("zeta: {}", error.message);
        std::process::exit(error.exit_code);
    }
}

fn ask(prompt: String) -> Result<(), CliError> {
    if prompt.is_empty() {
        return Err(CliError::usage("ask requires a prompt"));
    }
    run_headless(HeadlessCliOptions {
        entry: HeadlessEntry::New,
        title: "CLI conversation".into(),
        prompt,
        output: ExecOutputMode::Human,
        approval: HeadlessApprovalMode::DenyInteractiveRequests,
    })
}

fn interactive() -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(
            "interactive mode requires a TTY; use `zeta ask` or `zeta exec` instead".into(),
        );
    }
    let profile_root = local_profile_root();
    let executable = env::current_exe()
        .map_err(|error| format!("could not resolve zeta executable: {error}"))?;
    let dir_root = configured_dir()?;
    let command = StdioAppServerCommand::new(executable)
        .with_argument("app-server")
        .with_argument("connect")
        .with_environment_variable("ZETA_PROFILE_ROOT", profile_root.clone().into_os_string())
        .with_environment_variable("ZETA_WORKSPACE_ROOT", dir_root.clone().into_os_string());
    let session = AppServerSession::start_stdio(
        command,
        ClientInfo {
            name: "zeta-cli".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        zeta_tui::client_capabilities(),
    )
    .map_err(|error| error.to_string())?;
    let options = zeta_tui::TuiOptions::new("TUI conversation")
        .with_dir_root(&dir_root)
        .with_profile_root(&profile_root);
    match zeta_tui::run(session, options).map_err(|error| error.to_string())? {
        zeta_tui::TuiExit::UserRequested | zeta_tui::TuiExit::TerminationRequested => Ok(()),
        zeta_tui::TuiExit::ConnectionLost { reason, .. } => Err(reason),
    }
}

fn run_app_server(arguments: Vec<String>) -> Result<(), String> {
    zeta_server_host::run_app_server(arguments)
}

fn app_server_command(arguments: Vec<String>) -> Result<(), String> {
    run_app_server(arguments)
}

fn mcp_server_command(arguments: Vec<String>) -> Result<(), String> {
    let options = zeta_mcp_server::McpServerOptions::new(local_profile_root(), configured_dir()?);
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

fn current_dir() -> Result<PathBuf, String> {
    env::current_dir().map_err(|error| format!("could not resolve current directory: {error}"))
}

fn configured_dir() -> Result<PathBuf, String> {
    env::var_os("ZETA_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(current_dir)
}

fn execute(arguments: Vec<String>) -> Result<(), CliError> {
    run_headless(parse_exec_arguments(arguments)?)
}

fn parse_exec_arguments(arguments: Vec<String>) -> Result<HeadlessCliOptions, CliError> {
    let mut index = 0;
    let mut output = ExecOutputMode::Human;
    let mut approval = None;
    let mut entry = None;
    let mut title = "CLI execution".to_string();
    let mut prompt = Vec::new();
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--" => {
                prompt.extend(arguments[index + 1..].iter().cloned());
                break;
            }
            "--jsonl" => output = ExecOutputMode::JsonLines,
            "--auto-review" => {
                select_approval(&mut approval, HeadlessApprovalMode::AutomaticReview)?
            }
            "--dangerously-bypass-permissions" => {
                select_approval(&mut approval, HeadlessApprovalMode::BypassPermissions)?
            }
            "--title" => {
                index += 1;
                title = required_argument(&arguments, index, "--title")?.clone();
            }
            "--resume" => {
                index += 1;
                let session_id = parse_session_id(required_argument(
                    &arguments,
                    index,
                    "--resume SESSION_ID THREAD_ID",
                )?)?;
                index += 1;
                let thread_id = parse_thread_id(required_argument(
                    &arguments,
                    index,
                    "--resume SESSION_ID THREAD_ID",
                )?)?;
                select_entry(
                    &mut entry,
                    HeadlessEntry::Resume {
                        session_id,
                        thread_id,
                    },
                )?;
            }
            "--fork" => {
                index += 1;
                let session_id = parse_session_id(required_argument(
                    &arguments,
                    index,
                    "--fork SESSION_ID PARENT_THREAD_ID",
                )?)?;
                index += 1;
                let parent_thread_id = parse_thread_id(required_argument(
                    &arguments,
                    index,
                    "--fork SESSION_ID PARENT_THREAD_ID",
                )?)?;
                select_entry(
                    &mut entry,
                    HeadlessEntry::Fork {
                        session_id,
                        parent_thread_id,
                    },
                )?;
            }
            argument if argument.starts_with("--") => {
                return Err(CliError::usage(format!("unknown exec option: {argument}")));
            }
            _ => prompt.push(arguments[index].clone()),
        }
        index += 1;
    }
    let prompt = prompt.join(" ");
    if prompt.trim().is_empty() {
        return Err(CliError::usage("exec requires a prompt"));
    }
    Ok(HeadlessCliOptions {
        entry: entry.unwrap_or(HeadlessEntry::New),
        title,
        prompt,
        output,
        approval: approval.unwrap_or(HeadlessApprovalMode::DenyInteractiveRequests),
    })
}

fn run_headless(options: HeadlessCliOptions) -> Result<(), CliError> {
    let entry = match options.entry {
        HeadlessEntry::New => ExecEntry::New {
            title: options.title,
            input: prompt_input(options.prompt),
        },
        HeadlessEntry::Resume {
            session_id,
            thread_id,
        } => ExecEntry::Resume {
            session_id,
            thread_id,
            input: prompt_input(options.prompt),
        },
        HeadlessEntry::Fork {
            session_id,
            parent_thread_id,
        } => ExecEntry::Fork {
            session_id,
            parent_thread_id,
            title: options.title,
            input: prompt_input(options.prompt),
        },
    };
    let request = ExecRunRequest::new(entry).with_approval_mode(options.approval);
    let runner = headless_runner()?;
    let interrupt = InterruptSignal::register()?;
    let outcome = match options.output {
        ExecOutputMode::Human => {
            runner.run(request, DiscardExecEventSink, interrupt.cancellation())
        }
        ExecOutputMode::JsonLines => {
            let stdout = std::io::stdout();
            let sink = JsonLinesExecEventSink::new(stdout.lock());
            runner.run(request, sink, interrupt.cancellation())
        }
    }
    .map_err(exec_error)?;
    finish_headless_outcome(outcome, options.output)
}

fn headless_runner() -> Result<ExecRunner, CliError> {
    let target = AppServerTarget::Embedded(
        EmbeddedAppServerOptions::new(
            local_profile_root(),
            ClientInfo {
                name: "zeta-cli-exec".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_dir_root(configured_dir().map_err(CliError::failure)?),
    );
    Ok(ExecRunner::new(target))
}

fn finish_headless_outcome(outcome: ExecOutcome, output: ExecOutputMode) -> Result<(), CliError> {
    if let ExecOutcome::Completed { .. } = &outcome {
        if output == ExecOutputMode::Human
            && let Some(message) = outcome.final_message()
        {
            println!("{message}");
        }
        return Ok(());
    }
    Err(CliError {
        message: outcome_message(&outcome),
        exit_code: outcome.exit_code().get(),
    })
}

fn outcome_message(outcome: &ExecOutcome) -> String {
    match outcome {
        ExecOutcome::Completed { .. } => "headless run completed".into(),
        ExecOutcome::Failed {
            failure: ExecFailure::Reported { error },
            ..
        } => format!("Turn failed: {}", error.message),
        ExecOutcome::Failed {
            failure: ExecFailure::Unspecified,
            ..
        } => "Turn failed without a stable error".into(),
        ExecOutcome::Interrupted { reason, .. } => {
            format!("Turn was interrupted: {reason:?}")
        }
        ExecOutcome::RequiresInteraction { interaction, .. } => format!(
            "headless run requires an unsupported {:?} interaction",
            interaction.kind
        ),
        ExecOutcome::OutcomeUnknown { reason, .. } => {
            format!("Turn outcome is unknown: {reason:?}")
        }
    }
}

fn exec_error(error: ExecError) -> CliError {
    let exit_code = if matches!(error, ExecError::CancelledBeforeStart) {
        130
    } else {
        1
    };
    CliError {
        message: error.to_string(),
        exit_code,
    }
}

fn prompt_input(prompt: String) -> Vec<InputItem> {
    vec![InputItem::Text { text: prompt }]
}

fn select_approval(
    selected: &mut Option<HeadlessApprovalMode>,
    approval: HeadlessApprovalMode,
) -> Result<(), CliError> {
    if selected.replace(approval).is_some() {
        return Err(CliError::usage("select only one headless approval option"));
    }
    Ok(())
}

fn select_entry(
    selected: &mut Option<HeadlessEntry>,
    entry: HeadlessEntry,
) -> Result<(), CliError> {
    if selected.replace(entry).is_some() {
        return Err(CliError::usage("select only one of --resume or --fork"));
    }
    Ok(())
}

fn required_argument<'a>(
    arguments: &'a [String],
    index: usize,
    option: &str,
) -> Result<&'a String, CliError> {
    arguments
        .get(index)
        .ok_or_else(|| CliError::usage(format!("{option} requires another argument")))
}

fn parse_session_id(value: &str) -> Result<SessionId, CliError> {
    SessionId::new(value).map_err(|error| CliError::usage(error.to_string()))
}

fn parse_thread_id(value: &str) -> Result<ThreadId, CliError> {
    ThreadId::new(value).map_err(|error| CliError::usage(error.to_string()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum HeadlessEntry {
    New,
    Resume {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    Fork {
        session_id: SessionId,
        parent_thread_id: ThreadId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExecOutputMode {
    Human,
    JsonLines,
}

struct HeadlessCliOptions {
    entry: HeadlessEntry,
    title: String,
    prompt: String,
    output: ExecOutputMode,
    approval: HeadlessApprovalMode,
}

#[derive(Debug)]
struct CliError {
    message: String,
    exit_code: i32,
}

impl CliError {
    fn failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 1,
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }
}

struct InterruptSignal {
    requested: Arc<AtomicBool>,
    registration: SigId,
}

impl InterruptSignal {
    fn register() -> Result<Self, CliError> {
        let requested = Arc::new(AtomicBool::new(false));
        let registration = signal_hook::flag::register(SIGINT, Arc::clone(&requested))
            .map_err(|error| CliError::failure(format!("could not register Ctrl-C: {error}")))?;
        Ok(Self {
            requested,
            registration,
        })
    }

    fn cancellation(&self) -> &Arc<AtomicBool> {
        &self.requested
    }
}

impl Drop for InterruptSignal {
    fn drop(&mut self) {
        signal_hook::low_level::unregister(self.registration);
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
