use std::env;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::InProcessClientOptions;
use zeta_app_server_client::InProcessTransport;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_client::start_in_process_client;
use zeta_app_server_protocol::common::ClientInfo;
use zeta_app_server_protocol::json_schema_v1;
use zeta_app_server_protocol::typescript_v1;
use zeta_app_server_protocol::v1::thread::ThreadStartParams;
use zeta_app_server_protocol::v1::turn::InputItem;
use zeta_app_server_protocol::v1::turn::InputItemKind;
use zeta_app_server_protocol::v1::turn::TurnStartParams;

fn main() {
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        usage();
        return;
    };
    let outcome = match command.as_str() {
        "ask" => ask(arguments.collect::<Vec<_>>().join(" ")),
        "exec" => execute(arguments.collect()),
        "app-server" => app_server_command(arguments.collect()),
        _ => Err(format!("unknown command: {command}")),
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
    println!("{}", zeta_tui::render_response(&response));
    Ok(())
}

fn run_app_server(arguments: Vec<String>) -> Result<(), String> {
    if arguments.as_slice() != ["--listen", "stdio://"] {
        return Err("usage: zeta app-server --listen stdio://".into());
    }
    open_local_app_server(LocalAppServerOptions::new(state_root()))
        .map_err(|error| error.to_string())?
        .serve_stdio()
        .map_err(|error| error.to_string())
}

fn app_server_command(arguments: Vec<String>) -> Result<(), String> {
    match arguments.first().map(String::as_str) {
        Some("generate-ts") => generate_protocol(arguments, ProtocolArtifact::TypeScript),
        Some("generate-json-schema") => generate_protocol(arguments, ProtocolArtifact::JsonSchema),
        _ => run_app_server(arguments),
    }
}

enum ProtocolArtifact {
    TypeScript,
    JsonSchema,
}

fn generate_protocol(arguments: Vec<String>, artifact: ProtocolArtifact) -> Result<(), String> {
    if arguments.get(1).map(String::as_str) != Some("--protocol-version")
        || arguments.get(2).map(String::as_str) != Some("1")
        || arguments.get(3).map(String::as_str) != Some("--out")
        || arguments.get(4).is_none()
        || arguments.len() != 5
    {
        return Err(
            "usage: zeta app-server generate-(ts|json-schema) --protocol-version 1 --out <path>"
                .into(),
        );
    }
    let output = PathBuf::from(&arguments[4]);
    let (path, contents) = match artifact {
        ProtocolArtifact::TypeScript => (output.join("types.ts"), typescript_v1()),
        ProtocolArtifact::JsonSchema => (output, json_schema_v1()),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, contents).map_err(|error| error.to_string())
}

fn state_root() -> PathBuf {
    env::var_os("ZETA_STATE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".zeta"))
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
    let thread = client
        .start_thread(ThreadStartParams {
            idempotency_key: request_key("thread"),
            title: title.into(),
        })
        .map_err(|error| error.to_string())?;
    client
        .start_turn(TurnStartParams {
            idempotency_key: request_key("turn"),
            thread_id: thread.thread_id,
            input: vec![InputItem {
                kind: InputItemKind::Text,
                text: prompt,
            }],
        })
        .map_err(|error| error.to_string())?;
    client
        .drain_notifications()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find_map(|notification| match notification {
            ServerNotification::AgentMessageCompleted(message) => Some(message.text),
            _ => None,
        })
        .ok_or_else(|| "app server completed without an agent message".into())
}

fn in_process_client() -> Result<AppServerClient<InProcessTransport>, String> {
    start_in_process_client(InProcessClientOptions::new(
        state_root(),
        ClientInfo {
            name: "zeta-cli".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
    ))
    .map_err(|error| error.to_string())
}

fn request_key(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{timestamp}", std::process::id())
}

fn usage() {
    eprintln!("usage: zeta <ask|exec|app-server> ...");
}
