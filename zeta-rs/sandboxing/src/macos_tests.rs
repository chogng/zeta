use super::*;
use crate::SandboxDirAccess;
use crate::SandboxDirGrant;
use crate::SandboxScope;
use std::fs;
use zeta_file_access::Dir;

#[test]
fn dir_write_profile_denies_other_writes_and_network() {
    let dir = Dir::open_local(".").unwrap();
    let command = SandboxCommand::new("echo", ["hello"], dir.canonical_path());
    let policy = SandboxPolicy::new(FileSystemAccess::DirectoryWrite, NetworkAccess::Denied);

    let prepared = MacosSeatbeltSandbox::new()
        .prepare(&command, policy, &dir)
        .unwrap();

    assert_eq!(prepared.kind(), SandboxKind::MacosSeatbelt);
    assert_eq!(prepared.program(), SANDBOX_EXEC);
    let profile = prepared.arguments()[1].to_string_lossy();
    assert!(profile.contains("(deny file-write*)"));
    assert!(profile.contains("(allow file-write* (subpath"));
    assert!(profile.contains("(deny network*)"));
    for name in PROTECTED_DIR_METADATA_NAMES {
        let path = dir.canonical_path().join(name);
        assert!(
            profile.contains(&format!(
                "(deny file-write* (literal \"{}\"))",
                path.display()
            )),
            "profile did not protect {name}"
        );
    }
}

#[test]
fn scoped_profile_hides_siblings_and_reopens_every_granted_root() {
    let fixture = tempfile::tempdir().unwrap();
    let first = fixture.path().join("first");
    let second = fixture.path().join("second");
    let sibling = fixture.path().join("sibling");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    fs::create_dir_all(&sibling).unwrap();
    let storage = Dir::open_local(fixture.path()).unwrap();
    let first = Dir::open_local(first).unwrap();
    let second = Dir::open_local(second).unwrap();
    let scope = SandboxScope::new(
        first.clone(),
        vec![
            SandboxDirGrant::new(first.clone(), SandboxDirAccess::ReadWrite),
            SandboxDirGrant::new(second.clone(), SandboxDirAccess::ReadWrite),
        ],
        vec![storage.clone()],
    )
    .unwrap();
    let command = SandboxCommand::new("echo", ["hello"], first.canonical_path());
    let policy = SandboxPolicy::new(FileSystemAccess::DirectoryWrite, NetworkAccess::Denied);

    let prepared = MacosSeatbeltSandbox::new()
        .prepare_scoped(&command, policy, &scope)
        .unwrap();
    let profile = prepared.arguments()[1].to_string_lossy();

    let hidden = storage.canonical_path().display();
    assert!(profile.contains(&format!("(deny file-read* (subpath \"{hidden}\"))")));
    for granted in [first, second] {
        let granted = granted.canonical_path().display();
        assert!(profile.contains(&format!("(allow file-read* (subpath \"{granted}\"))")));
        assert!(profile.contains(&format!("(allow file-write* (subpath \"{granted}\"))")));
    }
    assert!(!profile.contains(&format!(
        "(allow file-read* (subpath \"{}\"))",
        sibling.display()
    )));
}

#[test]
fn real_scoped_process_cannot_read_or_write_a_sibling_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let first_path = fixture.path().join("first");
    let second_path = fixture.path().join("second");
    let sibling_path = fixture.path().join("sibling");
    fs::create_dir_all(&first_path).unwrap();
    fs::create_dir_all(&second_path).unwrap();
    fs::create_dir_all(&sibling_path).unwrap();
    fs::write(second_path.join("visible.txt"), "visible").unwrap();
    fs::write(sibling_path.join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(&sibling_path, first_path.join("sibling-link")).unwrap();

    let storage = Dir::open_local(fixture.path()).unwrap();
    let first = Dir::open_local(&first_path).unwrap();
    let second = Dir::open_local(&second_path).unwrap();
    let scope = SandboxScope::new(
        first.clone(),
        vec![
            SandboxDirGrant::new(first.clone(), SandboxDirAccess::ReadWrite),
            SandboxDirGrant::new(second, SandboxDirAccess::ReadWrite),
        ],
        vec![storage],
    )
    .unwrap();
    let policy = SandboxPolicy::new(FileSystemAccess::DirectoryWrite, NetworkAccess::Denied);
    let backend = MacosSeatbeltSandbox::new();

    let allowed_read = SandboxCommand::new(
        "/bin/cat",
        [second_path.join("visible.txt")],
        first.canonical_path(),
    );
    assert!(
        backend
            .prepare_scoped(&allowed_read, policy, &scope)
            .unwrap()
            .into_command()
            .status()
            .unwrap()
            .success()
    );

    for denied_path in [
        sibling_path.join("secret.txt"),
        first_path.join("sibling-link/secret.txt"),
    ] {
        let denied_read = SandboxCommand::new("/bin/cat", [denied_path], first.canonical_path());
        assert!(
            !backend
                .prepare_scoped(&denied_read, policy, &scope)
                .unwrap()
                .into_command()
                .status()
                .unwrap()
                .success()
        );
    }

    let allowed_write = SandboxCommand::new(
        "/usr/bin/touch",
        [second_path.join("created.txt")],
        first.canonical_path(),
    );
    assert!(
        backend
            .prepare_scoped(&allowed_write, policy, &scope)
            .unwrap()
            .into_command()
            .status()
            .unwrap()
            .success()
    );
    let denied_write = SandboxCommand::new(
        "/usr/bin/touch",
        [sibling_path.join("created.txt")],
        first.canonical_path(),
    );
    assert!(
        !backend
            .prepare_scoped(&denied_write, policy, &scope)
            .unwrap()
            .into_command()
            .status()
            .unwrap()
            .success()
    );
    assert!(second_path.join("created.txt").exists());
    assert!(!sibling_path.join("created.txt").exists());
}

#[test]
fn seatbelt_denial_classification_requires_a_platform_marker() {
    let backend = MacosSeatbeltSandbox::new();

    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "touch: Operation not permitted",
        ),
        Some(SandboxProcessDenial::process_may_have_started(
            "macOS Seatbelt denied the sandboxed process operation"
        ))
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(1),
            "",
            "application returned an error",
        ),
        None
    );
    assert_eq!(
        backend.classify_denial(
            SandboxProcessExitStatus::Code(71),
            "",
            "sandbox-exec: sandbox_apply: Operation not permitted",
        ),
        Some(SandboxProcessDenial::before_process_start(
            "macOS Seatbelt could not apply the sandbox profile"
        ))
    );
}
