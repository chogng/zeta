use std::sync::atomic::{AtomicU64, Ordering};

use super::ComposerShellDetector;

static NEXT_FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

#[test]
fn just_task_is_detected_from_the_workspace_manifest() {
    let root = std::env::temp_dir().join(format!(
        "zeta-composer-shell-{}-{}",
        std::process::id(),
        NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("justfile"), "zeterm-dev:\n    cargo run\n").unwrap();
    let detector = ComposerShellDetector::new(&root);

    assert!(detector.detects_command("just zeterm-dev"));
    assert!(!detector.detects_command("/model gpt-5"));
    assert!(!detector.detects_command("please fix native dev"));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn shell_operators_and_builtins_are_detected_without_a_manifest() {
    let detector = ComposerShellDetector::new(std::env::temp_dir());

    assert!(detector.detects_command("echo hello"));
    assert!(detector.detects_command("missing-command | another-command"));
    assert!(!detector.detects_command("explain 'a | b'"));
}
