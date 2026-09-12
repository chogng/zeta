use super::filesystem;
#[cfg(target_os = "windows")]
use super::windows_process_container;
#[cfg(target_os = "windows")]
use super::windows_ui;
use zeta_file_access::Dir;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::SandboxDirAccess;
use zeta_sandboxing::SandboxDirGrant;
use zeta_sandboxing::SandboxScope;

#[cfg(target_os = "windows")]
#[test]
fn windows_policy_allows_required_desktop_resources_but_keeps_sensitive_ui_blocked() {
    let ui = windows_ui().unwrap();
    assert!(ui.allow_windows);
    assert_eq!(ui.clipboard, mxc_sdk::policy::ClipboardPolicy::None);
    assert!(!ui.allow_input_injection);

    let process_container = windows_process_container();
    let ui = process_container.ui.unwrap();
    assert_eq!(
        ui.isolation,
        mxc_sdk::configs::ProcessContainerUiIsolation::Desktop
    );
    assert!(!ui.desktop_system_control);
    assert_eq!(
        ui.system_settings,
        mxc_sdk::configs::ProcessContainerSystemSettings::None
    );
    assert!(!ui.ime);
}

#[test]
fn writable_grants_protect_existing_metadata_without_creating_absent_paths() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["work", "reference", "work/.agents"] {
        std::fs::create_dir(temp.path().join(name)).unwrap();
    }
    // Git worktrees use a metadata file rather than a directory.
    std::fs::write(temp.path().join("work/.git"), "gitdir: elsewhere").unwrap();
    let storage = Dir::open_local(temp.path()).unwrap();
    let work = Dir::open_local(temp.path().join("work")).unwrap();
    let reference = Dir::open_local(temp.path().join("reference")).unwrap();
    let scope = SandboxScope::new(
        work.clone(),
        vec![
            SandboxDirGrant::new(work.clone(), SandboxDirAccess::ReadWrite),
            SandboxDirGrant::new(reference.clone(), SandboxDirAccess::ReadOnly),
        ],
        vec![storage.clone()],
    )
    .unwrap();
    for access in [
        FileSystemAccess::DirectoryWrite,
        FileSystemAccess::FullAccess,
    ] {
        let policy = filesystem(access, &scope).unwrap();
        assert_eq!(
            policy.readwrite_paths,
            [work.canonical_path().to_str().unwrap()]
        );
        assert_eq!(
            policy.readonly_paths,
            [
                work.canonical_path().join(".git"),
                work.canonical_path().join(".agents"),
                reference.canonical_path().to_owned(),
            ]
            .map(|path| path.to_str().unwrap().to_owned())
        );
        assert_eq!(
            policy.denied_paths,
            [storage.canonical_path().to_str().unwrap()]
        );
        assert_eq!(policy.clear_policy_on_exit, Some(true));
        for name in [".codex", ".zeta"] {
            assert!(!work.canonical_path().join(name).exists());
        }
    }
}

#[test]
fn an_empty_work_directory_keeps_its_write_grant() {
    let temp = tempfile::tempdir().unwrap();
    let dir = Dir::open_local(temp.path()).unwrap();
    let policy = filesystem(
        FileSystemAccess::DirectoryWrite,
        &SandboxScope::single(dir.clone()),
    )
    .unwrap();
    assert_eq!(
        policy.readwrite_paths,
        [dir.canonical_path().to_str().unwrap()]
    );
    assert!(policy.readonly_paths.is_empty());
    assert!(policy.denied_paths.is_empty());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
