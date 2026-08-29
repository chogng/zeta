//! Product-neutral process host for the local App Server and backend management commands.
//!
//! Product clients may execute the `zeta-server` binary without importing or packaging another
//! product's command host. The crate owns only process arguments, environment binding, and stdio;
//! App Server and Remote domain behavior remain in their canonical shared crates.

mod app_server;
mod remote;

/// Runs one supported backend command from a product-neutral process entrypoint.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut arguments = arguments.into_iter();
    let Some(command) = arguments.next() else {
        return Err(usage().into());
    };
    match command.as_str() {
        "app-server" => run_app_server(arguments),
        "fast-regex-worker" => zeta_fast_regex_search::serve_worker_from_environment()
            .map_err(|error| error.to_string()),
        "remote" => run_remote(arguments),
        "remote-server" => zeta_remote_server::run_from_environment_with_product_services(
            arguments,
            app_server::product_services_path(),
        )
        .map_err(|error| error.to_string()),
        _ => Err(format!("unknown server command: {command}\n\n{}", usage())),
    }
}

/// Runs the local App Server compatibility command without selecting a Workspace implicitly.
pub fn run_app_server(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    app_server::run(arguments.into_iter().collect())
}

/// Runs product-neutral Remote catalog, runtime, and profile management commands.
pub fn run_remote(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    remote::run(arguments.into_iter().collect())
}

fn usage() -> &'static str {
    "usage: zeta-server app-server (--listen stdio:// | connect | daemon <start|restart|stop|version>) [--product-services PATH] | zeta-server remote <command> | zeta-server remote-server <command>"
}
