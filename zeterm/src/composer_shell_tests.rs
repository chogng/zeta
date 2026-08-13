use std::sync::atomic::{AtomicU64, Ordering};

use super::ComposerShellDetector;
use zeta_input_classifier::ShellCommandEvidence;

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

    assert_eq!(
        detector.evidence("just zeterm-dev"),
        ShellCommandEvidence::HighConfidence
    );
    assert_eq!(
        detector.evidence("/model gpt-5"),
        ShellCommandEvidence::Absent
    );
    assert_eq!(
        detector.evidence("please fix native dev"),
        ShellCommandEvidence::Absent
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn path_commands_require_short_or_explicit_command_shapes() {
    let detector = ComposerShellDetector::new(std::env::temp_dir());

    assert_eq!(
        detector.evidence("cargo test"),
        ShellCommandEvidence::HighConfidence
    );
    assert_eq!(
        detector.evidence("cargo test 是做什么的"),
        ShellCommandEvidence::Absent
    );
}
