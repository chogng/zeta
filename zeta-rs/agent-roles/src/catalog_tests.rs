use super::*;
use std::fs;

#[test]
fn discovers_a_typed_agent_role() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("reviewer.md"),
        "---\nname: reviewer\ndescription: Reviews code changes.\nmodel: openai/gpt-5\ntools:\n  - read_file\nskills:\n  - code-review\ninstructions:\n  - rust-style\n---\n\nReview correctness and regressions.\n",
    )
    .unwrap();

    let snapshot = AgentRoleCatalog::discover("test", dir.path()).snapshot();

    assert!(snapshot.diagnostics().is_empty());
    assert_eq!(snapshot.entries().len(), 1);
    let role = &snapshot.entries()[0];
    assert_eq!(role.name(), "reviewer");
    assert_eq!(
        role.source(),
        &AgentRoleSource::Directory { id: "test".into() }
    );
    assert!(role.content_digest().starts_with("sha256:"));
    assert_eq!(role.model(), Some("openai/gpt-5"));
    assert_eq!(role.tools().unwrap(), ["read_file"]);
    assert_eq!(role.skills().unwrap(), ["code-review"]);
    assert_eq!(role.instructions(), ["rust-style"]);
}

#[test]
fn filename_mismatch_is_isolated() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("reviewer.md"),
        "---\nname: other\ndescription: Reviews code.\n---\n\nReview.\n",
    )
    .unwrap();

    let snapshot = AgentRoleCatalog::discover("test", dir.path()).snapshot();

    assert!(snapshot.entries().is_empty());
    assert_eq!(snapshot.diagnostics().len(), 1);
    assert_eq!(
        snapshot.diagnostics()[0].code(),
        AgentRoleDiagnosticCode::InvalidName
    );
}

#[test]
fn refresh_advances_generation_after_a_definition_changes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("reviewer.md");
    fs::write(
        &path,
        "---\nname: reviewer\ndescription: Reviews code.\n---\n\nReview.\n",
    )
    .unwrap();
    let mut catalog = AgentRoleCatalog::discover("test", dir.path());

    fs::write(
        path,
        "---\nname: reviewer\ndescription: Reviews code deeply.\n---\n\nReview.\n",
    )
    .unwrap();
    let snapshot = catalog.refresh();

    assert_eq!(snapshot.generation(), 2);
    assert_eq!(snapshot.entries()[0].description(), "Reviews code deeply.");
}
