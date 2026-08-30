use crate::SandboxDirAccess;
use crate::{
    FileSystemAccess, NetworkAccess, PROTECTED_DIR_METADATA_NAMES, PreparedCommand, SandboxBackend,
    SandboxCommand, SandboxError, SandboxKind, SandboxPolicy, SandboxProcessDenial,
    SandboxProcessExitStatus, SandboxScope,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use zeta_file_access::Dir;

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

/// Builds macOS Seatbelt launch commands from the shared sandbox policy.
#[derive(Default)]
pub struct MacosSeatbeltSandbox;

impl MacosSeatbeltSandbox {
    pub fn new() -> Self {
        Self
    }
}

impl SandboxBackend for MacosSeatbeltSandbox {
    fn kind(&self) -> SandboxKind {
        SandboxKind::MacosSeatbelt
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
                return Err(SandboxError::BackendUnavailable {
                    backend: SandboxKind::MacosSeatbelt,
                    message: "an unrestricted command cannot carry an isolated directory scope"
                        .into(),
                });
            }
            return Ok(PreparedCommand::unrestricted(command));
        }

        let profile = seatbelt_profile(policy, scope);
        let mut arguments = vec![OsString::from("-p"), OsString::from(profile), "--".into()];
        arguments.push(command.program().to_owned());
        arguments.extend(command.arguments().iter().cloned());
        Ok(PreparedCommand::new(
            SandboxKind::MacosSeatbelt,
            PathBuf::from(SANDBOX_EXEC),
            arguments,
            command.working_directory(),
        ))
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
        if output.contains("sandbox-exec: sandbox_apply") {
            return Some(SandboxProcessDenial::before_process_start(
                "macOS Seatbelt could not apply the sandbox profile",
            ));
        }
        ["operation not permitted", "sandbox: deny", "sandbox-exec:"]
            .iter()
            .any(|marker| output.contains(marker))
            .then(|| {
                SandboxProcessDenial::process_may_have_started(
                    "macOS Seatbelt denied the sandboxed process operation",
                )
            })
    }
}

fn seatbelt_profile(policy: SandboxPolicy, scope: &SandboxScope) -> String {
    let mut profile = String::from("(version 1)\n(allow default)\n");
    match policy.file_system() {
        FileSystemAccess::ReadOnly => profile.push_str("(deny file-write*)\n"),
        FileSystemAccess::DirectoryWrite => profile.push_str("(deny file-write*)\n"),
        FileSystemAccess::FullAccess => {}
    }
    for hidden in scope.hidden_dirs() {
        let path = escape_profile_literal(hidden.canonical_path().to_string_lossy().as_ref());
        profile.push_str(&format!(
            "(deny file-read* (literal \"{path}\"))\n\
             (deny file-read* (subpath \"{path}\"))\n\
             (deny file-write* (literal \"{path}\"))\n\
             (deny file-write* (subpath \"{path}\"))\n"
        ));
    }
    for grant in scope.grants() {
        let path = escape_profile_literal(grant.dir().canonical_path().to_string_lossy().as_ref());
        profile.push_str(&format!(
            "(allow file-read* (literal \"{path}\"))\n\
             (allow file-read* (subpath \"{path}\"))\n"
        ));
        if policy.file_system() != FileSystemAccess::ReadOnly
            && grant.access() == SandboxDirAccess::ReadWrite
        {
            profile.push_str(&format!(
                "(allow file-write* (literal \"{path}\"))\n\
                 (allow file-write* (subpath \"{path}\"))\n"
            ));
        } else if policy.file_system() == FileSystemAccess::FullAccess {
            profile.push_str(&format!(
                "(deny file-write* (literal \"{path}\"))\n\
                 (deny file-write* (subpath \"{path}\"))\n"
            ));
        }
        if policy.file_system() != FileSystemAccess::ReadOnly
            && grant.access() == SandboxDirAccess::ReadWrite
        {
            for name in PROTECTED_DIR_METADATA_NAMES {
                push_protected_metadata_policy(&mut profile, grant.dir().canonical_path(), name);
            }
        }
    }
    if policy.network() == NetworkAccess::Denied {
        profile.push_str("(deny network*)\n");
    }
    profile
}

fn push_protected_metadata_policy(profile: &mut String, dir: &Path, name: &str) {
    let path = dir.join(name);
    let path = escape_profile_literal(path.to_string_lossy().as_ref());
    profile.push_str(&format!(
        "(deny file-write* (literal \"{path}\"))\n\
         (deny file-write* (subpath \"{path}\"))\n"
    ));
}

fn escape_profile_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "macos_tests.rs"]
mod tests;
