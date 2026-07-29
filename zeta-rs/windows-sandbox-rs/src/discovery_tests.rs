use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn search_skips_invalid_probe_and_freezes_first_matching_helper() {
    let directory = TestDirectory::new();
    let wrong = helper(directory.path().join("wrong"), "wrong-protocol");
    let expected = helper(directory.path().join("runner"), RUNNER_PROBE);

    let discovered = discover_helper(
        ExecutableCandidates::SearchPaths(vec![wrong, expected.clone()]),
        "Windows command runner",
        RUNNER_PROBE,
    )
    .unwrap();

    assert_eq!(discovered, expected.canonicalize().unwrap());
}

#[test]
fn search_reports_all_candidate_failures() {
    let directory = TestDirectory::new();
    let missing = directory.path().join("missing");
    let wrong = helper(directory.path().join("wrong"), "wrong-protocol");

    let error = discover_helper(
        ExecutableCandidates::SearchPaths(vec![missing.clone(), wrong.clone()]),
        "Windows sandbox setup",
        SETUP_PROBE,
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains(&missing.display().to_string()));
    assert!(message.contains(&wrong.display().to_string()));
    assert!(message.contains("unexpected probe output"));
}

fn helper(path: PathBuf, probe: &str) -> PathBuf {
    fs::write(&path, format!("#!/bin/sh\nprintf '%s\\n' '{probe}'\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "zeta-windows-sandbox-discovery-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
