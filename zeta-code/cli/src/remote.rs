#[path = "remote_connect.rs"]
mod connect_command;

pub(crate) fn run(arguments: Vec<String>) -> Result<(), String> {
    match arguments.split_first() {
        Some((command, arguments)) if command == "connect" => {
            connect_command::parse(arguments).and_then(connect_command::run)
        }
        _ => zeta_server_host::run_remote(arguments),
    }
}
