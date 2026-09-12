use std::path::Path;
use std::path::PathBuf;

use crate::ConnectionOptions;
use crate::GrantSource;
use crate::LifecycleCommand;
use zeta_app_server::discovered_product_services_path;
use zeta_app_server::local_profile_root;

/// Runs daemon connection and lifecycle commands using the explicit host environment.
pub fn run_command(
    arguments: impl IntoIterator<Item = String>,
    daemon_executable: &Path,
) -> Result<(), String> {
    if !daemon_executable.is_absolute() {
        return Err("daemon executable must be an absolute path".into());
    }
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    let (command, product_services) = parse(&arguments)?;
    let grant_source = match std::env::var("ZETA_DIR_GRANT_SOURCE").as_deref() {
        Ok("userConfig") => GrantSource::UserConfig,
        Ok("hostConfiguration") | Err(std::env::VarError::NotPresent) => {
            GrantSource::HostConfiguration
        }
        _ => return Err("ZETA_DIR_GRANT_SOURCE must be userConfig or hostConfiguration".into()),
    };
    let options = ConnectionOptions::new(
        local_profile_root(),
        std::env::var_os("ZETA_WORKSPACE_ROOT").map(PathBuf::from),
        grant_source,
        product_services.or_else(discovered_product_services_path),
    );
    match command {
        Command::Connect => crate::connect(options, daemon_executable),
        Command::Lifecycle(command) => {
            let output = crate::run_lifecycle(command, options, daemon_executable)?;
            println!(
                "{}",
                serde_json::to_string(&output).map_err(|error| error.to_string())?
            );
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Connect,
    Lifecycle(LifecycleCommand),
}

fn parse(arguments: &[String]) -> Result<(Command, Option<PathBuf>), String> {
    let Some((command, remaining)) = arguments.split_first() else {
        return Err(usage().into());
    };
    let command = match command.as_str() {
        "connect" => Command::Connect,
        "start" => Command::Lifecycle(LifecycleCommand::Start),
        "restart" => Command::Lifecycle(LifecycleCommand::Restart),
        "stop" => Command::Lifecycle(LifecycleCommand::Stop),
        "version" => Command::Lifecycle(LifecycleCommand::Version),
        _ => return Err(usage().into()),
    };
    let product_services = match remaining {
        [] => None,
        [flag, path] if flag == "--product-services" => Some(PathBuf::from(path)),
        _ => return Err(usage().into()),
    };
    Ok((command, product_services))
}

fn usage() -> &'static str {
    "usage: zeta-app-server-daemon <connect|start|restart|stop|version> [--product-services PATH]"
}

/// Resolves the daemon executable for a product that embeds its command adapter.
pub fn executable_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os(crate::DAEMON_PATH_ENV) {
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            return Err(format!(
                "{} must be an absolute path",
                crate::DAEMON_PATH_ENV
            ));
        }
        return Ok(path);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or("daemon executable has no parent directory")?;
    Ok(directory.join(if cfg!(windows) {
        "zeta-app-server-daemon.exe"
    } else {
        "zeta-app-server-daemon"
    }))
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
