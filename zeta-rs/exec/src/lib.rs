//! The single process-execution boundary used by Zeta tools.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use zeta_core::{ApprovalPolicy, ApprovalRequirement};
use zeta_sandboxing::WorkspaceRoot;

#[derive(Clone, Debug)]
pub struct CommandRequest {
    pub program: String,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}
#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
#[derive(Debug)]
pub enum ExecutionError {
    ApprovalRequired,
    Denied,
    Spawn(String),
    TimedOut,
    OutputLimitExceeded,
    Sandbox(String),
}

pub struct ToolExecutor<P> {
    workspace: WorkspaceRoot,
    approval_policy: P,
    limits: ExecutionLimits,
}

impl<P: ApprovalPolicy> ToolExecutor<P> {
    pub fn new(workspace: WorkspaceRoot, approval_policy: P, limits: ExecutionLimits) -> Self {
        Self {
            workspace,
            approval_policy,
            limits,
        }
    }

    pub fn execute(&self, request: CommandRequest) -> Result<CommandOutput, ExecutionError> {
        let action_digest = format!("{}:{}", request.program, request.arguments.join("\u{1f}"));
        match self.approval_policy.requirement_for(&action_digest) {
            ApprovalRequirement::NotRequired => {}
            ApprovalRequirement::Required => return Err(ExecutionError::ApprovalRequired),
            ApprovalRequirement::Denied => return Err(ExecutionError::Denied),
        }
        let working_directory = self
            .workspace
            .resolve(&request.working_directory)
            .map_err(|error| ExecutionError::Sandbox(error.to_string()))?;
        let mut child = Command::new(&request.program)
            .args(&request.arguments)
            .current_dir(working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| ExecutionError::Spawn(error.to_string()))?;
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
        Ok(CommandOutput {
            exit_code: status.code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }
}

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
