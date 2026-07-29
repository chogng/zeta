use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use zeta_install_context::{ExecutableCandidates, InstallContext, ManagedExecutable};

use crate::LinuxSandbox;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const REQUIRED_HELP_MARKERS: &[&str] = &[
    "--bind",
    "--ro-bind",
    "--unshare-net",
    "--unshare-user",
    "--unshare-pid",
    "--die-with-parent",
    "--new-session",
    "--proc",
    "--dev",
    "--chdir",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxSandboxDiscoveryError {
    message: String,
}

impl LinuxSandboxDiscoveryError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LinuxSandboxDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LinuxSandboxDiscoveryError {}

pub(crate) fn discover(
    install_context: &InstallContext,
) -> Result<LinuxSandbox, LinuxSandboxDiscoveryError> {
    discover_candidates(
        install_context.executable_candidates(ManagedExecutable::Bubblewrap),
        probe_bubblewrap,
    )
}

fn discover_candidates(
    candidates: ExecutableCandidates,
    probe: impl Fn(&Path) -> Result<(), String>,
) -> Result<LinuxSandbox, LinuxSandboxDiscoveryError> {
    match candidates {
        ExecutableCandidates::ExplicitOverride(explicit) => {
            let path = validate_candidate(explicit.path(), &probe).map_err(|message| {
                LinuxSandboxDiscoveryError::new(format!(
                    "{}={} is not a usable Bubblewrap executable: {message}",
                    explicit.variable(),
                    explicit.path().display()
                ))
            })?;
            Ok(LinuxSandbox::new(path))
        }
        ExecutableCandidates::SearchPaths(paths) => {
            let mut failures = Vec::new();
            for candidate in paths {
                match validate_candidate(&candidate, &probe) {
                    Ok(path) => return Ok(LinuxSandbox::new(path)),
                    Err(message) => failures.push(format!("{}: {message}", candidate.display())),
                }
            }
            let details = if failures.is_empty() {
                "no candidates were configured".to_owned()
            } else {
                failures.join("; ")
            };
            Err(LinuxSandboxDiscoveryError::new(format!(
                "could not resolve a Bubblewrap executable with required capabilities: {details}"
            )))
        }
    }
}

fn validate_candidate(
    candidate: &Path,
    probe: impl Fn(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
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
    probe(&canonical)?;
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

fn probe_bubblewrap(path: &Path) -> Result<(), String> {
    let output = Command::new(path)
        .arg("--help")
        .output()
        .map_err(|error| format!("could not run --help probe: {error}"))?;
    if !output.status.success() {
        return Err(format!("--help probe exited with {}", output.status));
    }
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let missing = REQUIRED_HELP_MARKERS
        .iter()
        .filter(|marker| !help.contains(**marker))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "--help output is missing required capabilities: {}",
            missing.join(", ")
        ))
    }
}

#[cfg(all(test, unix))]
#[path = "discovery_tests.rs"]
mod tests;
