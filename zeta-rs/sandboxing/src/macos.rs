use crate::{
    FileSystemAccess, NetworkAccess, PreparedCommand, SandboxBackend, SandboxCommand, SandboxError,
    SandboxKind, SandboxPolicy, WorkspaceRoot,
};
use std::ffi::OsString;
use std::path::PathBuf;

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
        workspace: &WorkspaceRoot,
    ) -> Result<PreparedCommand, SandboxError> {
        if !policy.requires_platform_sandbox() {
            return Ok(PreparedCommand::unrestricted(command));
        }

        let profile = seatbelt_profile(policy, workspace);
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
}

fn seatbelt_profile(policy: SandboxPolicy, workspace: &WorkspaceRoot) -> String {
    let mut profile = String::from("(version 1)\n(allow default)\n");
    match policy.file_system() {
        FileSystemAccess::ReadOnly => profile.push_str("(deny file-write*)\n"),
        FileSystemAccess::WorkspaceWrite => {
            profile.push_str("(deny file-write*)\n");
            profile.push_str(&format!(
                "(allow file-write* (subpath \"{}\"))\n",
                escape_profile_literal(workspace.path().to_string_lossy().as_ref())
            ));
        }
        FileSystemAccess::FullAccess => {}
    }
    if policy.network() == NetworkAccess::Denied {
        profile.push_str("(deny network*)\n");
    }
    profile
}

fn escape_profile_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "macos_tests.rs"]
mod tests;
