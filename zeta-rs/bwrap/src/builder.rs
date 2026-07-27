use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Selects how a host path is exposed inside a Bubblewrap mount namespace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MountAccess {
    ReadOnly,
    ReadWrite,
}

/// A fully materialized Bubblewrap invocation that can be inspected before it is spawned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BwrapCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
}

impl BwrapCommand {
    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn into_command(self) -> Command {
        let mut command = Command::new(self.program);
        command.args(self.arguments);
        command
    }
}

/// Builds Bubblewrap argv from typed operations.
///
/// Callers are expected to choose mounts and namespaces from their own security policy. The
/// builder always adds parent-death, session, user-namespace, and PID-namespace containment.
pub struct BwrapCommandBuilder {
    binary: PathBuf,
    arguments: Vec<OsString>,
    inner_program: OsString,
    inner_arguments: Vec<OsString>,
}

impl BwrapCommandBuilder {
    pub fn new(binary: impl Into<PathBuf>, inner_program: impl Into<OsString>) -> Self {
        Self {
            binary: binary.into(),
            arguments: vec![
                "--die-with-parent".into(),
                "--new-session".into(),
                "--unshare-user".into(),
                "--unshare-pid".into(),
            ],
            inner_program: inner_program.into(),
            inner_arguments: Vec::new(),
        }
    }

    pub fn inner_arguments(
        mut self,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        self.inner_arguments
            .extend(arguments.into_iter().map(Into::into));
        self
    }

    pub fn mount(
        mut self,
        source: impl AsRef<OsStr>,
        destination: impl AsRef<OsStr>,
        access: MountAccess,
    ) -> Self {
        let operation = match access {
            MountAccess::ReadOnly => "--ro-bind",
            MountAccess::ReadWrite => "--bind",
        };
        self.arguments.push(operation.into());
        self.arguments.push(source.as_ref().to_owned());
        self.arguments.push(destination.as_ref().to_owned());
        self
    }

    pub fn isolate_network(mut self) -> Self {
        self.arguments.push("--unshare-net".into());
        self
    }

    pub fn mount_proc(mut self) -> Self {
        self.arguments.push("--proc".into());
        self.arguments.push("/proc".into());
        self
    }

    pub fn mount_dev(mut self) -> Self {
        self.arguments.push("--dev".into());
        self.arguments.push("/dev".into());
        self
    }

    pub fn working_directory(mut self, path: impl AsRef<OsStr>) -> Self {
        self.arguments.push("--chdir".into());
        self.arguments.push(path.as_ref().to_owned());
        self
    }

    pub fn build(mut self) -> BwrapCommand {
        self.arguments.push("--".into());
        self.arguments.push(self.inner_program);
        self.arguments.append(&mut self.inner_arguments);
        BwrapCommand {
            program: self.binary,
            arguments: self.arguments,
        }
    }
}
