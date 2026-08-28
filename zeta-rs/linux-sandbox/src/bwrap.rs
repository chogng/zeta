use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MountAccess {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BwrapCommand {
    program: PathBuf,
    arguments: Vec<OsString>,
}

impl BwrapCommand {
    pub(crate) fn program(&self) -> &Path {
        &self.program
    }

    pub(crate) fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

pub(crate) struct BwrapCommandBuilder {
    binary: PathBuf,
    arguments: Vec<OsString>,
    inner_program: OsString,
    inner_arguments: Vec<OsString>,
}

impl BwrapCommandBuilder {
    pub(crate) fn new(binary: impl Into<PathBuf>, inner_program: impl Into<OsString>) -> Self {
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

    pub(crate) fn inner_arguments(
        mut self,
        arguments: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        self.inner_arguments
            .extend(arguments.into_iter().map(Into::into));
        self
    }

    pub(crate) fn mount(
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

    pub(crate) fn isolate_network(mut self) -> Self {
        self.arguments.push("--unshare-net".into());
        self
    }

    pub(crate) fn mount_proc(mut self) -> Self {
        self.arguments.push("--proc".into());
        self.arguments.push("/proc".into());
        self
    }

    pub(crate) fn mount_dev(mut self) -> Self {
        self.arguments.push("--dev".into());
        self.arguments.push("/dev".into());
        self
    }

    pub(crate) fn working_directory(mut self, path: impl AsRef<OsStr>) -> Self {
        self.arguments.push("--chdir".into());
        self.arguments.push(path.as_ref().to_owned());
        self
    }

    pub(crate) fn build(mut self) -> BwrapCommand {
        self.arguments.push("--".into());
        self.arguments.push(self.inner_program);
        self.arguments.append(&mut self.inner_arguments);
        BwrapCommand {
            program: self.binary,
            arguments: self.arguments,
        }
    }
}

#[cfg(test)]
#[path = "bwrap_tests.rs"]
mod tests;
