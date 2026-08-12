use super::*;
use std::fs;

#[test]
fn discovers_three_loading_modes_and_renders_only_global_content() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().join(".zeta/instructions");
    fs::create_dir_all(&root).unwrap();
    write_instruction(&root, "always", "global", &[], "Always follow this.");
    write_instruction(
        &root,
        "rust",
        "contextual",
        &["**/*.rs"],
        "Use Rust conventions.",
    );
    write_instruction(&root, "explain", "on-demand", &[], "Explain carefully.");

    let catalog = InstructionCatalog::discover(workspace.path());
    let snapshot = catalog.snapshot();

    assert_eq!(snapshot.entries().len(), 3);
    assert!(snapshot.diagnostics().is_empty());
    let global = snapshot.global_content().unwrap();
    assert!(global.contains("Always follow this."));
    assert!(!global.contains("Use Rust conventions."));
    assert!(!global.contains("Explain carefully."));
}

#[test]
fn refresh_advances_generation_only_for_visible_changes() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().join(".zeta/instructions");
    fs::create_dir_all(&root).unwrap();
    write_instruction(&root, "always", "global", &[], "First.");
    let mut catalog = InstructionCatalog::discover(workspace.path());

    assert_eq!(catalog.refresh().generation(), 1);
    write_instruction(&root, "always", "global", &[], "Second.");
    let refreshed = catalog.refresh();

    assert_eq!(refreshed.generation(), 2);
    assert_eq!(refreshed.entries()[0].body(), "Second.");
}

#[test]
fn invalid_policy_is_isolated_as_a_diagnostic() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().join(".zeta/instructions");
    fs::create_dir_all(&root).unwrap();
    write_instruction(&root, "bad", "contextual", &[], "Body.");
    write_instruction(&root, "good", "global", &[], "Good.");

    let snapshot = InstructionCatalog::discover(workspace.path()).snapshot();

    assert_eq!(snapshot.entries().len(), 1);
    assert_eq!(snapshot.diagnostics().len(), 1);
    assert_eq!(
        snapshot.diagnostics()[0].code(),
        InstructionDiagnosticCode::InvalidLoadPolicy
    );
}

#[test]
fn native_catalog_does_not_implicitly_scan_agents_md() {
    let workspace = tempfile::tempdir().unwrap();
    fs::write(workspace.path().join("AGENTS.md"), "External instructions.").unwrap();

    let snapshot = InstructionCatalog::discover(workspace.path()).snapshot();

    assert!(snapshot.entries().is_empty());
    assert!(snapshot.diagnostics().is_empty());
}

fn write_instruction(root: &Path, name: &str, load: &str, patterns: &[&str], body: &str) {
    let patterns = if patterns.is_empty() {
        String::new()
    } else {
        format!(
            "patterns:\n{}",
            patterns
                .iter()
                .map(|pattern| format!("  - '{pattern}'"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    fs::write(
        root.join(format!("{name}.md")),
        format!("---\nname: {name}\nload: {load}\n{patterns}\n---\n\n{body}\n"),
    )
    .unwrap();
}
