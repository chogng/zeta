//! Product-bound Windows service for Zeta sandbox provisioning.

use anyhow::Result;

#[cfg(target_os = "windows")]
mod authentication;
#[cfg(target_os = "windows")]
mod broker;
#[cfg(target_os = "windows")]
mod service;
#[cfg(target_os = "windows")]
mod worker;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunMode {
    Service,
    #[cfg(debug_assertions)]
    Foreground,
}

/// Parses the service process arguments and enters the selected lifecycle.
pub fn run(arguments: impl Iterator<Item = String>) -> Result<()> {
    let mode = parse_mode(arguments)?;
    #[cfg(target_os = "windows")]
    {
        match mode {
            RunMode::Service => service::run(),
            #[cfg(debug_assertions)]
            RunMode::Foreground => service::run_foreground(),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = mode;
        anyhow::bail!("the Zeta sandbox service is available only on Windows")
    }
}

fn parse_mode(arguments: impl Iterator<Item = String>) -> Result<RunMode> {
    let arguments = arguments.collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => Ok(RunMode::Service),
        [argument] if argument == "--service" => Ok(RunMode::Service),
        #[cfg(debug_assertions)]
        [argument] if argument == "--foreground" => Ok(RunMode::Foreground),
        [argument] => anyhow::bail!("unrecognized service argument {argument:?}"),
        _ => anyhow::bail!("expected at most one service argument"),
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
