use std::fs;

use agent_roles::AgentRoleCatalog;
use zeta_instructions::InstructionCatalog;
use zeta_protocol::AgentDefinitionSelectionReason;
use zeta_protocol::AgentRoleSource;
use zeta_protocol::ContentDigest;
use zeta_protocol::FrozenSkillActivation;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillSourceId;
use zeta_protocol::ToolName;

use super::resolve_agent_selection;

#[test]
fn automatic_selection_freezes_definition_and_resolves_capability_references() {
    let dir = tempfile::tempdir().unwrap();
    let agent_root = dir.path().join(".zeta/agents");
    let instruction_root = dir.path().join(".zeta/instructions");
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
    let agents = AgentRoleCatalog::discover("test", dir.path()).snapshot();
    let instructions = InstructionCatalog::discover(dir.path()).snapshot();

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
        frozen.source,
        AgentRoleSource::Directory { id: "test".into() }
    );
    assert_eq!(frozen.version, None);
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
    let dir = tempfile::tempdir().unwrap();
    let agent_root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&agent_root).unwrap();
    fs::write(
        agent_root.join("publisher.md"),
        "---\nname: publisher\ndescription: Publishes releases.\ntools:\n  - external_publish\n---\n\nPublish the release.\n",
    )
    .unwrap();
    let agents = AgentRoleCatalog::discover("test", dir.path()).snapshot();

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

#[test]
fn omitted_tools_inherit_parent_then_apply_denials() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("reader.md"),
        "---\nname: reader\ndescription: Reads code without editing.\nrequiredTools:\n  - read_file\ndisallowedTools:\n  - write_file\n---\n\nRead the requested code.\n",
    )
    .unwrap();
    let roles = AgentRoleCatalog::discover("test", dir.path()).snapshot();

    let selected = resolve_agent_selection(
        Some("reader"),
        "Read the parser",
        None,
        vec![
            ToolName::new("read_file").unwrap(),
            ToolName::new("write_file").unwrap(),
        ],
        &[],
        &[roles],
        &[],
    )
    .unwrap();

    assert_eq!(
        selected.capability_scope.tools,
        [ToolName::new("read_file").unwrap()]
    );
}

#[test]
fn explicit_empty_tool_list_creates_a_no_tool_role() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(".zeta/agents");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("advisor.md"),
        "---\nname: advisor\ndescription: Answers from supplied context only.\ntools: []\n---\n\nAnswer without calling tools.\n",
    )
    .unwrap();
    let roles = AgentRoleCatalog::discover("test", dir.path()).snapshot();

    let selected = resolve_agent_selection(
        Some("advisor"),
        "Answer this question",
        None,
        vec![ToolName::new("read_file").unwrap()],
        &[],
        &[roles],
        &[],
    )
    .unwrap();

    assert!(selected.capability_scope.tools.is_empty());
}

#[test]
fn missing_required_tool_rejects_the_role_before_spawn() {
    let error = resolve_agent_selection(
        Some("issue"),
        "Coordinate [issue #123]",
        None,
        vec![ToolName::new("spawn_agent").unwrap()],
        &[],
        &[agent_roles::built_in_roles()],
        &[],
    )
    .err()
    .expect("Issue role must require its coordination and GitHub tools");

    assert!(error.to_string().contains("requires unavailable tool"));
}

#[test]
fn built_in_issue_role_freezes_github_and_coordination_capabilities() {
    let tools = [
        "read_file",
        "search_tools",
        "call_mcp_tool",
        "spawn_agent",
        "send_agent_message",
        "wait_agent",
    ]
    .into_iter()
    .map(|name| ToolName::new(name).unwrap())
    .collect();
    let github = FrozenSkillActivation {
        id: SkillId::new(
            SkillSourceId::new("plugin:skill-source:github").unwrap(),
            SkillName::new("github").unwrap(),
        ),
        content_digest: ContentDigest::sha256(b"github skill"),
        catalog_generation: 1,
        reason: SkillActivationReason::Explicit,
    };
    let rust = FrozenSkillActivation {
        id: SkillId::new(
            SkillSourceId::new("directory:skill-source:rust").unwrap(),
            SkillName::new("rust").unwrap(),
        ),
        content_digest: ContentDigest::sha256(b"rust skill"),
        catalog_generation: 2,
        reason: SkillActivationReason::Explicit,
    };

    let selected = resolve_agent_selection(
        None,
        "Coordinate [issue #123]",
        None,
        tools,
        &[github, rust],
        &[agent_roles::built_in_roles()],
        &[],
    )
    .unwrap();

    assert_eq!(selected.role.name, "issue");
    let frozen = selected.role.definition.unwrap();
    assert_eq!(frozen.name, "issue");
    assert_eq!(frozen.source, AgentRoleSource::BuiltIn);
    assert_eq!(frozen.version, Some(1));
    assert_eq!(selected.capability_scope.tools.len(), 6);
    assert!(
        selected
            .capability_scope
            .tools
            .iter()
            .any(|tool| tool.as_str() == "read_file")
    );
    assert_eq!(selected.capability_scope.skills.len(), 2);
    assert!(selected.role.instructions.contains("Issue coordinator"));
}

#[test]
fn built_in_issue_role_does_not_capture_unrelated_delegation() {
    let selected = resolve_agent_selection(
        None,
        "Inspect the parser call graph",
        None,
        vec![ToolName::new("read_file").unwrap()],
        &[],
        &[agent_roles::built_in_roles()],
        &[],
    )
    .unwrap();

    assert_eq!(selected.role.name, "general");
    assert!(selected.role.definition.is_none());
}

#[test]
fn built_in_issue_role_requires_the_github_skill() {
    let tools = [
        "search_tools",
        "call_mcp_tool",
        "spawn_agent",
        "send_agent_message",
        "wait_agent",
    ]
    .into_iter()
    .map(|name| ToolName::new(name).unwrap())
    .collect();

    let error = resolve_agent_selection(
        Some("issue"),
        "Coordinate [issue #123]",
        None,
        tools,
        &[],
        &[agent_roles::built_in_roles()],
        &[],
    )
    .err()
    .expect("Issue role must require GitHub");

    assert!(
        error
            .to_string()
            .contains("requires inactive Skill 'github'")
    );
}
