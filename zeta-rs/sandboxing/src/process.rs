use crate::SandboxProcessExitStatus;
use std::io::Read;
use std::io::Write;

/// Backend-owned running process. Implementations close the complete process tree and its
/// isolation resources; callers drain taken streams and invoke close before releasing a proxy.
pub trait SandboxProcess: Send {
    fn take_stdin(&mut self) -> Option<Box<dyn Write + Send>>;
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>>;
    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>>;
    fn try_wait(&mut self) -> io::Result<Option<SandboxProcessExitStatus>>;
    fn close(&mut self) -> io::Result<()>;
}

/// Ensures a backend process is closed on success, cancellation and early error returns.
pub struct ProcessHandle {
    inner: Box<dyn SandboxProcess>,
}

impl ProcessHandle {
    pub fn new(process: impl SandboxProcess + 'static) -> Self {
        Self {
            inner: Box::new(process),
        }
    }
    pub(crate) fn spawn_command(command: Command) -> io::Result<Self> {
        CommandProcess::spawn(command).map(Self::new)
    }
    pub fn take_stdin(&mut self) -> Option<Box<dyn Write + Send>> {
        self.inner.take_stdin()
    }
    pub fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.inner.take_stdout()
    }
    pub fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.inner.take_stderr()
    }
    pub fn try_wait(&mut self) -> io::Result<Option<SandboxProcessExitStatus>> {
        self.inner.try_wait()
    }
    pub fn close(&mut self) -> io::Result<()> {
        self.inner.close()
    }
}
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let _ = self.inner.close();
    }
}

use std::io;
use std::process::Child;
use std::process::Command;
use std::process::ExitStatus;

/// Owns one prepared launch and its process tree through exit, cancellation and drop.
/// Platform helpers keep their sandbox resources alive until their descendants terminate.
struct CommandProcess {
    child: Child,
    closed: bool,
}

impl CommandProcess {
    fn spawn(mut command: Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        Ok(Self {
            child: command.spawn()?,
            closed: false,
        })
    }

    /// A completed result includes process-tree cleanup, so output readers can reach EOF.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.closed {
            return self.child.try_wait();
        }
        #[cfg(unix)]
        {
            // Observe exit without reaping: the leader's PID stays reserved while its group
            // is killed, so cleanup cannot signal a recycled process-group identity.
            if !zeta_utils_pty::process_group::child_has_exited(self.child.id())? {
                return Ok(None);
            }
            self.close()?;
            self.child.try_wait()
        }
        #[cfg(not(unix))]
        {
            let status = self.child.try_wait()?;
            if status.is_some() {
                self.close()?;
            }
            Ok(status)
        }
    }

    pub fn close(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        let tree = zeta_utils_pty::process_group::kill_process_group(self.child.id());
        let _ = self.child.kill();
        let reaped = self.child.wait();
        self.closed = reaped.is_ok();
        tree?;
        reaped?;
        Ok(())
    }
}

impl Drop for CommandProcess {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl SandboxProcess for CommandProcess {
    fn take_stdin(&mut self) -> Option<Box<dyn Write + Send>> {
        self.child.stdin.take().map(|stream| Box::new(stream) as _)
    }
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child.stdout.take().map(|stream| Box::new(stream) as _)
    }
    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child.stderr.take().map(|stream| Box::new(stream) as _)
    }
    fn try_wait(&mut self) -> io::Result<Option<SandboxProcessExitStatus>> {
        CommandProcess::try_wait(self).map(|status| {
            status.map(|status| {
                status.code().map_or(
                    SandboxProcessExitStatus::Terminated,
                    SandboxProcessExitStatus::Code,
                )
            })
        })
    }
    fn close(&mut self) -> io::Result<()> {
        CommandProcess::close(self)
    }
}
#[cfg(all(test, unix))]
#[path = "process_tests.rs"]
mod tests;
