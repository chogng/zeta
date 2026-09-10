//! Converts Zeta authority into Microsoft MXC SDK requests and adapts its process handles.
mod policy;
mod process;

use std::path::PathBuf;
use zeta_file_access::Dir;
use zeta_install_context::InstallContext;
use zeta_sandboxing::PreparedCommand;
use zeta_sandboxing::SandboxBackend;
use zeta_sandboxing::SandboxCommand;
use zeta_sandboxing::SandboxError;
use zeta_sandboxing::SandboxKind;
use zeta_sandboxing::SandboxPolicy;
use zeta_sandboxing::SandboxProcessDenial;
use zeta_sandboxing::SandboxProcessExitStatus;
use zeta_sandboxing::SandboxScope;

/// MXC-backed execution. The SDK owns backend selection, process creation and OS resource cleanup.
pub struct MxcSandbox {
    runtime: InstallContext,
}

impl MxcSandbox {
    /// Captures runtime locations; the SDK checks the capabilities of each execution request.
    pub fn new(context: InstallContext) -> Self {
        Self { runtime: context }
    }
}

impl SandboxBackend for MxcSandbox {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Restricted
    }
    fn requires_shared_network_proxy(&self) -> bool {
        true
    }

    fn classify_denial(
        &self,
        status: SandboxProcessExitStatus,
        stdout: &str,
        stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        if status == SandboxProcessExitStatus::Code(0) {
            return None;
        }
        let output = format!("{stdout}\n{stderr}").to_ascii_lowercase();
        // Child text is diagnostic only. It never proves that execution did not start.
        [
            "operation not permitted",
            "permission denied",
            "read-only file system",
            "network is unreachable",
        ]
        .iter()
        .any(|marker| output.contains(marker))
        .then(|| {
            SandboxProcessDenial::process_may_have_started(
                "the process reported an access restriction",
            )
        })
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        dir: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        self.prepare_scoped(command, policy, &SandboxScope::single(dir.clone()))
    }

    fn prepare_scoped(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        scope: &SandboxScope,
    ) -> Result<PreparedCommand, SandboxError> {
        if !policy.requires_platform_sandbox() {
            if !scope.is_single_unhidden() {
                return Err(unavailable(
                    "an unrestricted command cannot carry an isolated directory scope",
                ));
            }
            return Ok(PreparedCommand::unrestricted(command));
        }
        let mut request = policy::request(command, policy, scope)?;
        if let Some(path) = bubblewrap(&self.runtime)? {
            request
                .set_bubblewrap_executable(&path)
                .map_err(|error| unavailable(error.to_string()))?;
        }
        Ok(PreparedCommand::sandboxed(
            command,
            process::Launch {
                request,
                scope: scope.clone(),
                cwd: command.working_directory().to_owned(),
            },
        ))
    }
}

fn unavailable(message: impl Into<String>) -> SandboxError {
    SandboxError::BackendUnavailable {
        backend: SandboxKind::Restricted,
        message: message.into(),
    }
}

#[cfg(not(target_os = "linux"))]
fn bubblewrap(_: &InstallContext) -> Result<Option<PathBuf>, SandboxError> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn bubblewrap(context: &InstallContext) -> Result<Option<PathBuf>, SandboxError> {
    use zeta_install_context::ExecutableCandidates;
    use zeta_install_context::ManagedExecutable;
    let paths = match context.executable_candidates(ManagedExecutable::Bubblewrap) {
        ExecutableCandidates::ExplicitOverride(candidate) => vec![candidate.path().to_owned()],
        ExecutableCandidates::SearchPaths(paths) => paths,
    };
    for path in paths {
        if let Ok(path) = std::fs::canonicalize(&path) {
            if path.is_file() {
                return Ok(Some(path));
            }
        }
    }
    Err(unavailable(
        "the required Bubblewrap executable is unavailable",
    ))
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod tests;
