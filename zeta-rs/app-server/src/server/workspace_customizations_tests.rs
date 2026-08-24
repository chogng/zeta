use super::*;
use crate::server::fs_watcher::WorkspaceFileChangeSink;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn global_instructions_are_injected_but_other_load_policies_are_not() {
    let workspace = TempDir::new().unwrap();
    write_instruction(
        workspace.path(),
        "global",
        "global",
        "Always use the Workspace formatter.",
    );
    write_instruction(
        workspace.path(),
        "rust-files",
        "contextual\npatterns:\n  - '**/*.rs'",
        "Use Rust-specific review guidance.",
    );
    fs::write(
        workspace.path().join("AGENTS.md"),
        "Legacy root instructions must not be injected.",
    )
    .unwrap();

    let customizations = WorkspaceCustomizations::discover(workspace.path());
    let harness = HarnessInstructionsProvider::snapshot(customizations.as_ref());

    assert_eq!(customizations.instruction_snapshot().entries().len(), 2);
    let injected = harness.workspace_instructions().unwrap();
    assert!(injected.contains("Always use the Workspace formatter."));
    assert!(!injected.contains("Rust-specific review guidance."));
    assert!(!injected.contains("Legacy root instructions"));
}

#[test]
fn projected_changes_refresh_instruction_and_agent_snapshots() {
    let workspace = TempDir::new().unwrap();
    write_instruction(workspace.path(), "global", "global", "First version.");
    write_agent(
        workspace.path(),
        "reviewer",
        "Reviews code",
        "Review carefully.",
    );
    let customizations = WorkspaceCustomizations::discover(workspace.path());
    let instruction_generation = customizations.instruction_snapshot().generation();
    let agent_generation = customizations.agent_snapshot().generation();

    write_instruction(workspace.path(), "global", "global", "Second version.");
    write_agent(
        workspace.path(),
        "reviewer",
        "Reviews code and tests",
        "Review carefully.",
    );
    customizations.files_changed(&FsChanged::PathsChanged {
        workspace_folder_id: None,
        paths: vec![
            PathBuf::from(".zeta/instructions/global.md"),
            PathBuf::from(".zeta/agents/reviewer.md"),
        ],
    });

    assert!(customizations.instruction_snapshot().generation() > instruction_generation);
    assert!(customizations.agent_snapshot().generation() > agent_generation);
    assert!(
        HarnessInstructionsProvider::snapshot(customizations.as_ref())
            .workspace_instructions()
            .unwrap()
            .contains("Second version.")
    );
    assert_eq!(
        customizations.agent_snapshot().entries()[0].description(),
        "Reviews code and tests"
    );
}

fn write_instruction(workspace: &Path, name: &str, load: &str, body: &str) {
    let root = workspace.join(".zeta/instructions");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join(format!("{name}.md")),
        format!("---\nname: {name}\nload: {load}\n---\n\n{body}\n"),
    )
    .unwrap();
}

fn write_agent(workspace: &Path, name: &str, description: &str, body: &str) {
    let root = workspace.join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join(format!("{name}.md")),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"),
    )
    .unwrap();
}
