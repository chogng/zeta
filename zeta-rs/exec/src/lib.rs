//! The single process-execution boundary used by Zeta tools.

use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};
use zeta_protocol::{ProcessExecutionOutput, ProcessExitStatus, SandboxDenialOutput};
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, SandboxBackend, SandboxCommand, SandboxDenialTiming,
    SandboxError, SandboxManager, SandboxPolicy, SandboxProcessExitStatus, WorkspaceRoot,
};

/// Decides whether a fully materialized local process action can start.
///
/// Hosts implement this policy from their approval authority. The executor asks only about the
/// exact program-and-arguments digest and never turns a required approval into permission itself.
pub trait ApprovalPolicy: Send + Sync {
    fn requirement_for(&self, action_digest: &str) -> ApprovalRequirement;
}

/// Distinguishes a command that may start from one awaiting user approval or prohibited by policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalRequirement {
    NotRequired,
    Required,
    Denied,
}

impl ApprovalRequirement {
    pub fn allows_execution(self) -> bool {
        matches!(self, Self::NotRequired)
    }
}

#[derive(Clone, Debug)]
pub struct CommandRequest {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}

/// Exact process authority selected before execution reaches the host spawn boundary.
///
/// `Sandboxed` requires the configured backend to enforce the supplied policy. `Unrestricted`
/// remains explicit so approval or allow-list decisions never silently become sandbox bypasses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandExecutionAuthority {
    Sandboxed(SandboxPolicy),
    Unrestricted,
}

impl CommandExecutionAuthority {
    fn sandbox_policy(self) -> SandboxPolicy {
        match self {
            Self::Sandboxed(policy) => policy,
            Self::Unrestricted => {
                SandboxPolicy::new(FileSystemAccess::FullAccess, NetworkAccess::Allowed)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Structured result of a process that reached sandbox preparation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandExecutionOutcome {
    Completed(CommandOutput),
    SandboxDenied(SandboxDenialOutput),
}

#[derive(Debug)]
pub enum ExecutionError {
    ApprovalRequired,
    Denied,
    Spawn(String),
    TimedOut,
    OutputLimitExceeded,
    Sandbox(SandboxError),
}

/// Starts approved commands only after the selected sandbox backend prepares their host process.
pub struct CommandExecutor<P, B> {
    sandbox: SandboxManager<B>,
    approval_policy: P,
    limits: ExecutionLimits,
}

impl<P: ApprovalPolicy, B: SandboxBackend> CommandExecutor<P, B> {
    pub fn new(
        workspace: WorkspaceRoot,
        backend: B,
        approval_policy: P,
        limits: ExecutionLimits,
    ) -> Self {
        Self {
            sandbox: SandboxManager::new(workspace, backend),
            approval_policy,
            limits,
        }
    }

    pub fn execute(
        &self,
        request: CommandRequest,
        authority: CommandExecutionAuthority,
    ) -> Result<CommandExecutionOutcome, ExecutionError> {
        let action_digest = format!("{}:{}", request.program, request.arguments.join("\u{1f}"));
        match self.approval_policy.requirement_for(&action_digest) {
            ApprovalRequirement::NotRequired => {}
            ApprovalRequirement::Required => return Err(ExecutionError::ApprovalRequired),
            ApprovalRequirement::Denied => return Err(ExecutionError::Denied),
        }
        let command = SandboxCommand::new(
            request.program,
            request.arguments,
            request.working_directory,
        );
        let prepared = match self.sandbox.prepare(&command, authority.sandbox_policy()) {
            Ok(prepared) => prepared,
            Err(error @ SandboxError::BackendUnavailable { .. })
                if matches!(authority, CommandExecutionAuthority::Sandboxed(_)) =>
            {
                return Ok(CommandExecutionOutcome::SandboxDenied(
                    SandboxDenialOutput::safe_to_retry(
                        error.to_string(),
                        ProcessExecutionOutput::from_captured_streams(
                            ProcessExitStatus::Terminated,
                            "",
                            "",
                        ),
                    ),
                ));
            }
            Err(error) => return Err(ExecutionError::Sandbox(error)),
        };
        let prepared_kind = prepared.kind();
        let mut command = prepared.into_command();
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error)
                if matches!(authority, CommandExecutionAuthority::Sandboxed(_))
                    && prepared_kind != zeta_sandboxing::SandboxKind::Unrestricted =>
            {
                return Ok(CommandExecutionOutcome::SandboxDenied(
                    SandboxDenialOutput::safe_to_retry(
                        format!("sandbox launcher could not start: {error}"),
                        ProcessExecutionOutput::from_captured_streams(
                            ProcessExitStatus::Terminated,
                            "",
                            "",
                        ),
                    ),
                ));
            }
            Err(error) => return Err(ExecutionError::Spawn(error.to_string())),
        };
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let max_output_bytes = self.limits.max_output_bytes;
        let stdout_reader = thread::spawn(move || drain_stream(stdout, max_output_bytes));
        let stderr_reader = thread::spawn(move || drain_stream(stderr, max_output_bytes));
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| ExecutionError::Spawn(error.to_string()))?
            {
                break status;
            }
            if started.elapsed() >= self.limits.timeout {
                child
                    .kill()
                    .map_err(|error| ExecutionError::Spawn(error.to_string()))?;
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(ExecutionError::TimedOut);
            }
            thread::sleep(Duration::from_millis(10));
        };
        let (stdout, stdout_exceeded) = stdout_reader
            .join()
            .map_err(|_| ExecutionError::Spawn("stdout reader panicked".into()))?
            .map_err(ExecutionError::Spawn)?;
        let (stderr, stderr_exceeded) = stderr_reader
            .join()
            .map_err(|_| ExecutionError::Spawn("stderr reader panicked".into()))?
            .map_err(ExecutionError::Spawn)?;
        if stdout_exceeded
            || stderr_exceeded
            || stdout.len() + stderr.len() > self.limits.max_output_bytes
        {
            return Err(ExecutionError::OutputLimitExceeded);
        }
        let output = CommandOutput {
            exit_code: status.code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        };
        if matches!(authority, CommandExecutionAuthority::Sandboxed(_))
            && let Some(denial) = self.sandbox.classify_denial(
                output.exit_code.map_or(
                    SandboxProcessExitStatus::Terminated,
                    SandboxProcessExitStatus::Code,
                ),
                &output.stdout,
                &output.stderr,
            )
        {
            let output = ProcessExecutionOutput::from_captured_streams(
                output
                    .exit_code
                    .map_or(ProcessExitStatus::Terminated, ProcessExitStatus::Code),
                output.stdout,
                output.stderr,
            );
            let denial = match denial.timing() {
                SandboxDenialTiming::BeforeProcessStart => {
                    SandboxDenialOutput::safe_to_retry(denial.reason(), output)
                }
                SandboxDenialTiming::ProcessMayHaveStarted => {
                    SandboxDenialOutput::may_have_side_effects(denial.reason(), output)
                }
            };
            return Ok(CommandExecutionOutcome::SandboxDenied(denial));
        }
        Ok(CommandExecutionOutcome::Completed(output))
    }
}

/// Backwards-compatible name for the local process execution boundary.
pub type ToolExecutor<P, B> = CommandExecutor<P, B>;

fn drain_stream(mut stream: impl Read, max_output_bytes: usize) -> Result<(Vec<u8>, bool), String> {
    let mut captured = Vec::new();
    let mut exceeded = false;
    let mut chunk = [0_u8; 8192];
    loop {
        let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        let remaining = max_output_bytes.saturating_sub(captured.len());
        let retained = remaining.min(count);
        captured.extend_from_slice(&chunk[..retained]);
        exceeded |= retained != count;
    }
    Ok((captured, exceeded))
}

#[cfg(test)]
#[path = "command_executor_tests.rs"]
mod tests;
