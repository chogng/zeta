use super::*;
use crate::server::fs_watcher::WorkspaceFileChangeSink;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_workspace::WorkspaceAuthorization;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;

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

    let customizations = customizations(workspace.path());
    let harness = instruction_snapshot(customizations.as_ref(), "session-global");

    assert_eq!(customizations.instruction_snapshot().entries().len(), 2);
    let injected = harness.instructions().workspace_instructions().unwrap();
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
    let customizations = customizations(workspace.path());
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
        instruction_snapshot(customizations.as_ref(), "session-refresh")
            .instructions()
            .workspace_instructions()
            .unwrap()
            .contains("Second version.")
    );
    assert_eq!(
        customizations.agent_snapshot().entries()[0].description(),
        "Reviews code and tests"
    );
}

#[test]
fn workspace_roots_are_rendered_only_for_the_matching_session() {
    let workspace = TempDir::new().unwrap();
    let additional = TempDir::new().unwrap();
    let access = Arc::new(crate::session_workspace_access::SessionWorkspaceAccess::default());
    let first = SessionId::new("session-with-additional-directory").unwrap();
    let second = SessionId::new("session-without-additional-directory").unwrap();
    let authorization = WorkspaceAuthorization::new(
        WorkspaceRoot::open(additional.path()).unwrap(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision),
    );
    access
        .add_directory(
            first.clone(),
            WorkspaceRoot::open(workspace.path()).unwrap(),
            authorization,
            zeta_workspace_access::AdditionalDirectoryPermissions::local_file_tools(),
        )
        .unwrap();
    let customizations =
        WorkspaceCustomizations::discover(workspace.path(), Arc::clone(&access)).unwrap();

    let first_environment = instruction_snapshot(customizations.as_ref(), first.as_str())
        .environment()
        .unwrap()
        .render();
    let second_snapshot = instruction_snapshot(customizations.as_ref(), second.as_str());

    assert!(
        first_environment.contains(
            &additional
                .path()
                .canonicalize()
                .unwrap()
                .display()
                .to_string()
        )
    );
    assert!(first_environment.contains("<filesystem>"));
    assert!(first_environment.contains("<workspace_roots>"));
    assert!(first_environment.contains("Relative paths still resolve from cwd"));
    let primary_position = first_environment
        .find(&workspace.path().display().to_string())
        .expect("primary Workspace root must be rendered");
    let additional_position = first_environment
        .find(
            &additional
                .path()
                .canonicalize()
                .unwrap()
                .display()
                .to_string(),
        )
        .expect("additional Workspace root must be rendered");
    assert!(primary_position < additional_position);
    assert!(
        !second_snapshot.environment().unwrap().render().contains(
            &additional
                .path()
                .canonicalize()
                .unwrap()
                .display()
                .to_string()
        )
    );

    access.clear_session(&first);
    assert!(
        !instruction_snapshot(customizations.as_ref(), first.as_str())
            .environment()
            .unwrap()
            .render()
            .contains(
                &additional
                    .path()
                    .canonicalize()
                    .unwrap()
                    .display()
                    .to_string()
            )
    );
}

fn customizations(workspace: &Path) -> Arc<WorkspaceCustomizations> {
    WorkspaceCustomizations::discover(
        workspace,
        Arc::new(crate::session_workspace_access::SessionWorkspaceAccess::default()),
    )
    .unwrap()
}

fn instruction_snapshot(
    customizations: &WorkspaceCustomizations,
    session_id: &str,
) -> Arc<HarnessContext> {
    let session_id = SessionId::new(session_id).unwrap();
    let thread_id = ThreadId::new("workspace-customizations-thread").unwrap();
    let turn_id = TurnId::new("workspace-customizations-turn").unwrap();
    HarnessContextProvider::snapshot(
        customizations,
        &HarnessContextRequest {
            session_id: &session_id,
            thread_id: &thread_id,
            turn_id: &turn_id,
        },
    )
    .unwrap()
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
