use std::fs;

use zeta_agents::AgentDefinitionCatalog;
use zeta_instructions::InstructionCatalog;
use zeta_protocol::AgentDefinitionSelectionReason;
use zeta_protocol::ToolName;

use super::resolve_agent_selection;

#[test]
fn automatic_selection_freezes_definition_and_resolves_capability_references() {
    let workspace = tempfile::tempdir().unwrap();
    let agent_root = workspace.path().join(".zeta/agents");
    let instruction_root = workspace.path().join(".zeta/instructions");
    fs::create_dir_all(&agent_root).unwrap();
    fs::create_dir_all(&instruction_root).unwrap();
    fs::write(
        agent_root.join("reviewer.md"),
        "---\nname: reviewer\ndescription: Reviews code changes for correctness and regressions.\ntools:\n  - read_file\ninstructions:\n  - review-policy\n---\n\nReport only actionable findings.\n",
    )
    .unwrap();
    fs::write(
        instruction_root.join("review-policy.md"),
        "---\nload: on-demand\n---\n\nPrioritize correctness over style.\n",
    )
    .unwrap();
    let agents = AgentDefinitionCatalog::discover(workspace.path()).snapshot();
    let instructions = InstructionCatalog::discover(workspace.path()).snapshot();

    let selected = resolve_agent_selection(
        None,
        "Review these code changes for correctness and regressions",
        None,
        vec![
            ToolName::new("read_file").unwrap(),
            ToolName::new("shell").unwrap(),
        ],
        &[],
        &[agents],
        &[instructions],
    )
    .unwrap();

    let frozen = selected.role.definition.unwrap();
    assert_eq!(frozen.name, "reviewer");
    assert_eq!(frozen.catalog_generation, 1);
    assert_eq!(
        frozen.selection_reason,
        AgentDefinitionSelectionReason::Automatic
    );
    assert_eq!(selected.capability_scope.tools.len(), 1);
    assert_eq!(selected.capability_scope.tools[0].as_str(), "read_file");
    assert!(
        selected
            .role
            .instructions
            .contains("Prioritize correctness over style")
    );
}

#[test]
fn selected_definition_cannot_expand_the_parent_tool_ceiling() {
    let workspace = tempfile::tempdir().unwrap();
    let agent_root = workspace.path().join(".zeta/agents");
    fs::create_dir_all(&agent_root).unwrap();
    fs::write(
        agent_root.join("publisher.md"),
        "---\nname: publisher\ndescription: Publishes releases.\ntools:\n  - external_publish\n---\n\nPublish the release.\n",
    )
    .unwrap();
    let agents = AgentDefinitionCatalog::discover(workspace.path()).snapshot();

    let error = resolve_agent_selection(
        Some("publisher"),
        "Publish the release",
        None,
        vec![ToolName::new("read_file").unwrap()],
        &[],
        &[agents],
        &[],
    )
    .err()
    .expect("definition must not add a parent tool");

    assert!(error.to_string().contains("unavailable tool"));
}
