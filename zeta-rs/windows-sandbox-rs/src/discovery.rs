#![forbid(unsafe_code)]

use crate::WindowsSandbox;
use crate::protocol::{PROBE_FLAG, RUNNER_PROBE, SETUP_PROBE};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use zeta_install_context::{ExecutableCandidates, InstallContext, ManagedExecutable};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsSandboxDiscoveryError {
    message: String,
}

impl WindowsSandboxDiscoveryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WindowsSandboxDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for WindowsSandboxDiscoveryError {}

pub(crate) fn discover(
    context: &InstallContext,
) -> Result<WindowsSandbox, WindowsSandboxDiscoveryError> {
    let command_runner = discover_helper(
        context.executable_candidates(ManagedExecutable::WindowsCommandRunner),
        "Windows command runner",
        RUNNER_PROBE,
    )?;
    let sandbox_setup = discover_helper(
        context.executable_candidates(ManagedExecutable::WindowsSandboxSetup),
        "Windows sandbox setup",
        SETUP_PROBE,
    )?;
    Ok(WindowsSandbox::new(command_runner, sandbox_setup))
}

fn discover_helper(
    candidates: ExecutableCandidates,
    description: &str,
    expected_probe: &str,
) -> Result<PathBuf, WindowsSandboxDiscoveryError> {
    match candidates {
        ExecutableCandidates::ExplicitOverride(explicit) => {
            validate_candidate(explicit.path(), expected_probe).map_err(|reason| {
                WindowsSandboxDiscoveryError::new(format!(
                    "{}={} is not a usable {description}: {reason}",
                    explicit.variable(),
                    explicit.path().display()
                ))
            })
        }
        ExecutableCandidates::SearchPaths(paths) => {
            let mut failures = Vec::new();
            for candidate in paths {
                match validate_candidate(&candidate, expected_probe) {
                    Ok(path) => return Ok(path),
                    Err(reason) => failures.push(format!("{}: {reason}", candidate.display())),
                }
            }
            Err(WindowsSandboxDiscoveryError::new(format!(
                "could not resolve {description}: {}",
                if failures.is_empty() {
                    "no candidates were configured".to_owned()
                } else {
                    failures.join("; ")
                }
            )))
        }
    }
}

fn validate_candidate(candidate: &Path, expected_probe: &str) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(candidate)
        .map_err(|error| format!("could not canonicalize candidate: {error}"))?;
    let metadata = canonical
        .metadata()
        .map_err(|error| format!("could not read metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("candidate is not a regular file".to_owned());
    }
    if !is_executable(&metadata) {
        return Err("candidate is not executable".to_owned());
    }
    let output = Command::new(&canonical)
        .arg(PROBE_FLAG)
        .output()
        .map_err(|error| format!("could not run probe: {error}"))?;
    if !output.status.success() {
        return Err(format!("probe exited with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim() != expected_probe {
        return Err(format!("unexpected probe output: {}", stdout.trim()));
    }
    Ok(canonical)
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_: &fs::Metadata) -> bool {
    true
}

#[cfg(all(test, unix))]
#[path = "discovery_tests.rs"]
mod tests;
