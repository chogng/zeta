//! Linux sandbox backend built on typed Bubblewrap command construction.

mod bwrap;
mod discovery;

use bwrap::BwrapCommandBuilder;
use bwrap::MountAccess;
use std::path::{Path, PathBuf};
use zeta_file_access::Dir;
use zeta_install_context::InstallContext;
use zeta_sandboxing::{
    FileSystemAccess, NetworkAccess, PROTECTED_DIR_METADATA_NAMES, PreparedCommand, SandboxBackend,
    SandboxCommand, SandboxDirAccess, SandboxError, SandboxKind, SandboxPolicy,
    SandboxProcessDenial, SandboxProcessExitStatus, SandboxScope,
};

pub use discovery::LinuxSandboxDiscoveryError;

/// Translates shared sandbox policy into a Bubblewrap launch command.
pub struct LinuxSandbox {
    bwrap_binary: PathBuf,
}

impl LinuxSandbox {
    pub fn new(bwrap_binary: impl Into<PathBuf>) -> Self {
        Self {
            bwrap_binary: bwrap_binary.into(),
        }
    }

    /// Resolves and probes the bundled or host Bubblewrap executable.
    ///
    /// Package resources precede host `PATH`; an explicit `ZETA_BWRAP_PATH` override is
    /// authoritative and therefore cannot silently fall back.
    pub fn discover(install_context: &InstallContext) -> Result<Self, LinuxSandboxDiscoveryError> {
        discovery::discover(install_context)
    }

    pub fn binary(&self) -> &Path {
        &self.bwrap_binary
    }

    /// Builds the Linux Bubblewrap launch command without spawning it.
    ///
    /// This is public so packaging and capability-probe code can inspect the exact executable and
    /// argv that will be used after selecting a Bubblewrap binary.
    pub fn prepare_command(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        dir: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        self.prepare_scoped_command(command, policy, &SandboxScope::single(dir.clone()))
    }

    pub fn prepare_scoped_command(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        scope: &SandboxScope,
    ) -> Result<PreparedCommand, SandboxError> {
        if !policy.requires_platform_sandbox() {
            if scope.grants().len() != 1 || !scope.hidden_dirs().is_empty() {
                return Err(SandboxError::BackendUnavailable {
                    backend: SandboxKind::LinuxBubblewrap,
                    message: "an unrestricted command cannot carry an isolated directory scope"
                        .into(),
                });
            }
            return Ok(PreparedCommand::unrestricted(command));
        }

        let root_access = match policy.file_system() {
            FileSystemAccess::ReadOnly | FileSystemAccess::DirectoryWrite => MountAccess::ReadOnly,
            FileSystemAccess::FullAccess => MountAccess::ReadWrite,
        };
        let mut builder = BwrapCommandBuilder::new(
            &self.bwrap_binary,
            command.program().to_owned(),
        )
        .mount(Path::new("/"), Path::new("/"), root_access);
        let mut hidden = scope.hidden_dirs().iter().collect::<Vec<_>>();
        hidden.sort_by_key(|dir| dir.canonical_path().components().count());
        for dir in &hidden {
            builder = builder.tmpfs(dir.canonical_path());
        }
        for grant in scope.grants() {
            let writable = policy.file_system() != FileSystemAccess::ReadOnly
                && grant.access() == SandboxDirAccess::ReadWrite;
            builder = builder.mount(
                grant.dir().canonical_path(),
                grant.dir().canonical_path(),
                if writable {
                    MountAccess::ReadWrite
                } else {
                    MountAccess::ReadOnly
                },
            );
        }
        for dir in hidden.iter().rev() {
            builder = builder.remount_read_only(dir.canonical_path());
        }
        for grant in scope.grants() {
            if policy.file_system() != FileSystemAccess::ReadOnly
                && grant.access() == SandboxDirAccess::ReadWrite
            {
                for name in PROTECTED_DIR_METADATA_NAMES {
                    let path = grant.dir().canonical_path().join(name);
                    if path.exists() {
                        builder = builder.mount(&path, &path, MountAccess::ReadOnly);
                    }
                }
            }
        }
        if policy.network() == NetworkAccess::Denied {
            builder = builder.isolate_network();
        }
        let bwrap = builder
            .mount_proc()
            .mount_dev()
            .working_directory(command.working_directory())
            .inner_arguments(command.arguments().iter().cloned())
            .build();
        Ok(PreparedCommand::new(
            SandboxKind::LinuxBubblewrap,
            bwrap.program(),
            bwrap.arguments().iter().cloned(),
            command.working_directory(),
        ))
    }
}

impl SandboxBackend for LinuxSandbox {
    fn kind(&self) -> SandboxKind {
        SandboxKind::LinuxBubblewrap
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        dir: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        #[cfg(target_os = "linux")]
        {
            self.prepare_command(command, policy, dir)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (command, policy, dir);
            Err(SandboxError::BackendUnavailable {
                backend: SandboxKind::LinuxBubblewrap,
                message: "the Bubblewrap backend can only run on Linux".to_owned(),
            })
        }
    }

    fn prepare_scoped(
        &self,
        command: &SandboxCommand,
        policy: SandboxPolicy,
        scope: &SandboxScope,
    ) -> Result<PreparedCommand, SandboxError> {
        #[cfg(target_os = "linux")]
        {
            self.prepare_scoped_command(command, policy, scope)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (command, policy, scope);
            Err(SandboxError::BackendUnavailable {
                backend: SandboxKind::LinuxBubblewrap,
                message: "the Bubblewrap backend can only run on Linux".to_owned(),
            })
        }
    }

    fn classify_denial(
        &self,
        exit_status: SandboxProcessExitStatus,
        stdout: &str,
        stderr: &str,
    ) -> Option<SandboxProcessDenial> {
        if exit_status == SandboxProcessExitStatus::Code(0) {
            return None;
        }
        let output = format!("{stdout}\n{stderr}").to_ascii_lowercase();
        if output.contains("bwrap:") {
            return Some(SandboxProcessDenial::before_process_start(
                "Linux Bubblewrap could not establish the sandbox",
            ));
        }
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
                "Linux Bubblewrap denied the sandboxed process operation",
            )
        })
    }
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;
