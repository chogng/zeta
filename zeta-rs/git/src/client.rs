use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::GitError;
use crate::GitResult;

const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_MUTATION_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const DISABLED_HOOKS_PATH: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

/// Resource limits applied to every system Git process started by [`GitClient`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GitExecutionLimits {
    query_timeout: Duration,
    mutation_timeout: Duration,
    max_output_bytes: usize,
}

impl GitExecutionLimits {
    pub fn new(
        query_timeout: Duration,
        mutation_timeout: Duration,
        max_output_bytes: usize,
    ) -> GitResult<Self> {
        if query_timeout.is_zero() {
            return Err(GitError::InvalidConfiguration {
                field: "query_timeout",
                requirement: "must be non-zero",
            });
        }
        if mutation_timeout.is_zero() {
            return Err(GitError::InvalidConfiguration {
                field: "mutation_timeout",
                requirement: "must be non-zero",
            });
        }
        if max_output_bytes == 0 {
            return Err(GitError::InvalidConfiguration {
                field: "max_output_bytes",
                requirement: "must be non-zero",
            });
        }
        Ok(Self {
            query_timeout,
            mutation_timeout,
            max_output_bytes,
        })
    }

    pub fn query_timeout(self) -> Duration {
        self.query_timeout
    }

    pub fn mutation_timeout(self) -> Duration {
        self.mutation_timeout
    }

    pub fn max_output_bytes(self) -> usize {
        self.max_output_bytes
    }
}

impl Default for GitExecutionLimits {
    fn default() -> Self {
        Self {
            query_timeout: DEFAULT_QUERY_TIMEOUT,
            mutation_timeout: DEFAULT_MUTATION_TIMEOUT,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

/// Concrete owner of system Git process identity and execution limits.
#[derive(Clone, Debug)]
pub struct GitClient {
    executable: PathBuf,
    limits: GitExecutionLimits,
}

impl GitClient {
    pub fn system() -> Self {
        Self {
            executable: PathBuf::from("git"),
            limits: GitExecutionLimits::default(),
        }
    }

    pub fn with_executable(executable: PathBuf, limits: GitExecutionLimits) -> GitResult<Self> {
        if executable.as_os_str().is_empty() {
            return Err(GitError::InvalidConfiguration {
                field: "executable",
                requirement: "must not be empty",
            });
        }
        Ok(Self { executable, limits })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn limits(&self) -> GitExecutionLimits {
        self.limits
    }

    pub(crate) async fn run_query<I, S>(&self, cwd: &Path, args: I) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self
            .run(GitInvocation::query(cwd, args, FsmonitorOverride::Disabled))
            .await?;
        output.require_success()
    }

    pub(crate) async fn run_query_with_fsmonitor<I, S>(
        &self,
        cwd: &Path,
        args: I,
        fsmonitor: FsmonitorOverride,
    ) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.run(GitInvocation::query(cwd, args, fsmonitor)).await?;
        output.require_success()
    }

    pub(crate) async fn run_query_unchecked<I, S>(
        &self,
        cwd: &Path,
        args: I,
    ) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(GitInvocation::query(cwd, args, FsmonitorOverride::Disabled))
            .await
    }

    pub(crate) async fn run_configuration_probe<I, S>(
        &self,
        cwd: &Path,
        args: I,
    ) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(GitInvocation::configuration_probe(cwd, args))
            .await
    }

    pub(crate) async fn run_mutation_with_stdin<I, S>(
        &self,
        cwd: &Path,
        args: I,
        input: Vec<u8>,
    ) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(GitInvocation::mutation(cwd, args).with_stdin(input))
            .await
    }

    pub(crate) async fn run_mutation<I, S>(
        &self,
        cwd: &Path,
        args: I,
    ) -> GitResult<GitCommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.run(GitInvocation::mutation(cwd, args)).await
    }

    async fn run(&self, invocation: GitInvocation) -> GitResult<GitCommandOutput> {
        let timeout_duration = invocation.profile.timeout(self.limits);
        let command_for_log = render_command(&self.executable, &invocation.args);
        let mut command = Command::new(&self.executable);
        command
            .current_dir(&invocation.cwd)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never")
            .env("LC_ALL", "C")
            .arg("-c")
            .arg(format!("core.hooksPath={DISABLED_HOOKS_PATH}"))
            .arg("-c")
            .arg("color.ui=false")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        invocation.profile.configure(&mut command);
        command.args(&invocation.args);
        if invocation.stdin.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        let mut child = command
            .spawn()
            .map_err(|source| GitError::io("spawn Git process", source))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| GitError::runtime("capture Git stdout", "stdout pipe was missing"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| GitError::runtime("capture Git stderr", "stderr pipe was missing"))?;
        let stdout_task = tokio::spawn(read_bounded(stdout, self.limits.max_output_bytes));
        let stderr_task = tokio::spawn(read_bounded(stderr, self.limits.max_output_bytes));
        let stdin_task = invocation.stdin.map(|input| {
            let stdin = child.stdin.take();
            tokio::spawn(write_stdin(stdin, input))
        });

        let status = match timeout(timeout_duration, child.wait()).await {
            Ok(result) => result.map_err(|source| GitError::io("wait for Git process", source))?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                drain_task(stdout_task).await?;
                drain_task(stderr_task).await?;
                if let Some(stdin_task) = stdin_task {
                    let _ = drain_stdin_task(stdin_task).await;
                }
                return Err(GitError::TimedOut {
                    command: command_for_log,
                    timeout: timeout_duration,
                });
            }
        };

        if let Some(stdin_task) = stdin_task {
            drain_stdin_task(stdin_task).await?;
        }
        let stdout = drain_task(stdout_task).await?;
        let stderr = drain_task(stderr_task).await?;
        if stdout.truncated {
            return Err(GitError::OutputLimitExceeded {
                command: command_for_log,
                stream: "stdout",
                limit_bytes: self.limits.max_output_bytes,
            });
        }
        if stderr.truncated {
            return Err(GitError::OutputLimitExceeded {
                command: command_for_log,
                stream: "stderr",
                limit_bytes: self.limits.max_output_bytes,
            });
        }
        Ok(GitCommandOutput {
            command: command_for_log,
            status,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FsmonitorOverride {
    Disabled,
    BuiltIn,
}

impl FsmonitorOverride {
    fn git_config(self) -> &'static str {
        match self {
            Self::Disabled => "core.fsmonitor=false",
            Self::BuiltIn => "core.fsmonitor=true",
        }
    }
}

enum GitCommandProfile {
    Query { fsmonitor: FsmonitorOverride },
    ConfigurationProbe,
    Mutation,
}

impl GitCommandProfile {
    fn timeout(&self, limits: GitExecutionLimits) -> Duration {
        match self {
            Self::Query { .. } | Self::ConfigurationProbe => limits.query_timeout,
            Self::Mutation => limits.mutation_timeout,
        }
    }

    fn configure(&self, command: &mut Command) {
        match self {
            Self::Query { fsmonitor } => {
                command
                    .env("GIT_OPTIONAL_LOCKS", "0")
                    .arg("-c")
                    .arg(fsmonitor.git_config());
            }
            Self::ConfigurationProbe => {
                command.env("GIT_OPTIONAL_LOCKS", "0");
            }
            Self::Mutation => {
                command
                    .arg("-c")
                    .arg(FsmonitorOverride::Disabled.git_config());
            }
        }
    }
}

struct GitInvocation {
    cwd: PathBuf,
    args: Vec<OsString>,
    profile: GitCommandProfile,
    stdin: Option<Vec<u8>>,
}

impl GitInvocation {
    fn query<I, S>(cwd: &Path, args: I, fsmonitor: FsmonitorOverride) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self {
            cwd: cwd.to_path_buf(),
            args: collect_args(args),
            profile: GitCommandProfile::Query { fsmonitor },
            stdin: None,
        }
    }

    fn configuration_probe<I, S>(cwd: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self {
            cwd: cwd.to_path_buf(),
            args: collect_args(args),
            profile: GitCommandProfile::ConfigurationProbe,
            stdin: None,
        }
    }

    fn mutation<I, S>(cwd: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self {
            cwd: cwd.to_path_buf(),
            args: collect_args(args),
            profile: GitCommandProfile::Mutation,
            stdin: None,
        }
    }

    fn with_stdin(mut self, stdin: Vec<u8>) -> Self {
        self.stdin = Some(stdin);
        self
    }
}

pub(crate) struct GitCommandOutput {
    pub(crate) command: String,
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl GitCommandOutput {
    pub(crate) fn require_success(self) -> GitResult<Self> {
        if self.status.success() {
            return Ok(self);
        }
        Err(GitError::CommandFailed {
            command: self.command,
            exit_code: self.status.code(),
            stderr: String::from_utf8_lossy(&self.stderr).trim().to_string(),
        })
    }
}

struct BoundedRead {
    bytes: Vec<u8>,
    truncated: bool,
}

async fn read_bounded(
    mut reader: impl AsyncRead + Unpin,
    max_bytes: usize,
) -> std::io::Result<BoundedRead> {
    let mut bytes = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0_u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        let remaining = max_bytes.saturating_sub(bytes.len());
        let retained = remaining.min(read);
        bytes.extend_from_slice(&chunk[..retained]);
        truncated |= retained < read;
    }
    Ok(BoundedRead { bytes, truncated })
}

async fn write_stdin(stdin: Option<ChildStdin>, input: Vec<u8>) -> std::io::Result<()> {
    let Some(mut stdin) = stdin else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "Git stdin pipe was missing",
        ));
    };
    match stdin.write_all(&input).await {
        Ok(()) => stdin.shutdown().await,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error),
    }
}

async fn drain_task(task: JoinHandle<std::io::Result<BoundedRead>>) -> GitResult<BoundedRead> {
    task.await
        .map_err(|error| GitError::runtime("join Git output reader", error.to_string()))?
        .map_err(|source| GitError::io("read Git output", source))
}

async fn drain_stdin_task(task: JoinHandle<std::io::Result<()>>) -> GitResult<()> {
    task.await
        .map_err(|error| GitError::runtime("join Git stdin writer", error.to_string()))?
        .map_err(|source| GitError::io("write Git stdin", source))
}

fn collect_args<I, S>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    args.into_iter()
        .map(|argument| argument.as_ref().to_os_string())
        .collect()
}

fn render_command(executable: &Path, args: &[OsString]) -> String {
    std::iter::once(executable.as_os_str())
        .chain(args.iter().map(OsString::as_os_str))
        .map(|argument| quote_for_log(&argument.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_for_log(argument: &str) -> String {
    if argument
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_./:=@%+".contains(character))
    {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
