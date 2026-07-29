use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSystemAccess {
    ReadOnly,
    WorkspaceWrite,
    FullAccess,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkAccess {
    Denied,
    Allowed,
}

/// Immutable filesystem and network authority for one local process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SandboxPolicy {
    file_system: FileSystemAccess,
    network: NetworkAccess,
}

impl SandboxPolicy {
    pub fn new(file_system: FileSystemAccess, network: NetworkAccess) -> Self {
        Self {
            file_system,
            network,
        }
    }

    pub fn file_system(self) -> FileSystemAccess {
        self.file_system
    }

    pub fn network(self) -> NetworkAccess {
        self.network
    }

    pub fn requires_platform_sandbox(self) -> bool {
        self.file_system != FileSystemAccess::FullAccess || self.network != NetworkAccess::Allowed
    }
}

/// A command whose working directory is relative to its selected workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxCommand {
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
}

impl SandboxCommand {
    pub fn new(
        program: impl Into<OsString>,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
        working_directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            program: program.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            working_directory: working_directory.into(),
        }
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub(crate) fn with_working_directory(&self, working_directory: PathBuf) -> Self {
        Self {
            program: self.program.clone(),
            arguments: self.arguments.clone(),
            working_directory,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxKind {
    Unrestricted,
    MacosSeatbelt,
    LinuxBubblewrap,
    WindowsAppContainer,
}

/// Process termination shape used by a backend when classifying enforcement output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxProcessExitStatus {
    Code(i32),
    Terminated,
}

/// Whether sandbox evidence proves the requested child process did not reach its entry point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SandboxDenialTiming {
    BeforeProcessStart,
    ProcessMayHaveStarted,
}

/// Backend-specific classification of a process result caused by sandbox enforcement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxProcessDenial {
    reason: String,
    timing: SandboxDenialTiming,
}

impl SandboxProcessDenial {
    pub fn before_process_start(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            timing: SandboxDenialTiming::BeforeProcessStart,
        }
    }

    pub fn process_may_have_started(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            timing: SandboxDenialTiming::ProcessMayHaveStarted,
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn timing(&self) -> SandboxDenialTiming {
        self.timing
    }
}

/// A host command produced by a sandbox backend and ready for the process executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedCommand {
    kind: SandboxKind,
    program: OsString,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
}

impl PreparedCommand {
    pub fn new(
        kind: SandboxKind,
        program: impl Into<OsString>,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
        working_directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            kind,
            program: program.into(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            working_directory: working_directory.into(),
        }
    }

    pub fn unrestricted(command: &SandboxCommand) -> Self {
        Self::new(
            SandboxKind::Unrestricted,
            command.program.clone(),
            command.arguments.clone(),
            command.working_directory.clone(),
        )
    }

    pub fn kind(&self) -> SandboxKind {
        self.kind
    }

    pub fn program(&self) -> &OsStr {
        &self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn into_command(self) -> Command {
        let mut command = Command::new(self.program);
        command
            .args(self.arguments)
            .current_dir(self.working_directory);
        command
    }
}
