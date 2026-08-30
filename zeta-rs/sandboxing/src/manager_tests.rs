use super::*;
use crate::{FileSystemAccess, NetworkAccess};
use std::path::Path;
use zeta_file_access::Dir;

struct RecordingBackend;

impl SandboxBackend for RecordingBackend {
    fn kind(&self) -> SandboxKind {
        SandboxKind::Unrestricted
    }

    fn prepare(
        &self,
        command: &SandboxCommand,
        _policy: SandboxPolicy,
        _dir: &Dir,
    ) -> Result<PreparedCommand, SandboxError> {
        assert!(command.working_directory().is_absolute());
        Ok(PreparedCommand::unrestricted(command))
    }
}

#[test]
fn manager_resolves_the_working_directory_before_backend_dispatch() {
    let dir = Dir::open_local(".").unwrap();
    let manager = SandboxManager::new(dir.clone(), RecordingBackend);
    let command = SandboxCommand::new("echo", ["hello"], ".");

    let prepared = manager
        .prepare(
            &command,
            SandboxPolicy::new(FileSystemAccess::ReadOnly, NetworkAccess::Denied),
        )
        .unwrap();

    assert_eq!(prepared.working_directory(), dir.canonical_path());
    assert_eq!(prepared.program(), "echo");
    assert_eq!(prepared.arguments(), ["hello"]);
    assert_eq!(prepared.kind(), SandboxKind::Unrestricted);
    assert!(Path::new(prepared.working_directory()).is_absolute());
}
