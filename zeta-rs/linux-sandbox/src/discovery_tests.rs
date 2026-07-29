use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use zeta_install_context::ExecutableCandidates;

#[test]
fn search_skips_invalid_candidates_and_freezes_the_first_capable_binary() {
    let directory = TestDirectory::new();
    let missing = directory.path().join("missing");
    let executable = write_executable(&directory.path().join("bwrap"), b"fake");

    let sandbox = discover_candidates(
        ExecutableCandidates::SearchPaths(vec![missing, executable.clone()]),
        |_| Ok(()),
    )
    .unwrap();

    assert_eq!(sandbox.binary(), fs::canonicalize(executable).unwrap());
}

#[test]
fn real_help_probe_requires_every_flag_used_by_the_builder() {
    let directory = TestDirectory::new();
    let capable = write_executable(
        &directory.path().join("capable-bwrap"),
        b"#!/bin/sh\nprintf '%s\\n' '--bind --ro-bind --unshare-net --unshare-user --unshare-pid --die-with-parent --new-session --proc --dev --chdir'\n",
    );
    let incomplete = write_executable(
        &directory.path().join("incomplete-bwrap"),
        b"#!/bin/sh\nprintf '%s\\n' '--bind --ro-bind'\n",
    );

    assert_eq!(probe_bubblewrap(&capable), Ok(()));
    assert!(
        probe_bubblewrap(&incomplete)
            .unwrap_err()
            .contains("--unshare-net")
    );
}

fn write_executable(path: &Path, contents: &[u8]) -> PathBuf {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
    path.to_owned()
}

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-linux-sandbox-discovery-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
