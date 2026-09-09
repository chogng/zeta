use super::*;

#[test]
fn issue_role_is_packaged_as_one_validated_definition() {
    let snapshot = built_in_roles();

    assert!(snapshot.diagnostics().is_empty());
    assert_eq!(snapshot.entries().len(), 1);
    let role = &snapshot.entries()[0];
    assert_eq!(role.name(), "issue");
    assert_eq!(role.source(), &AgentRoleSource::BuiltIn);
    assert_eq!(role.version(), Some(1));
    assert_eq!(role.tools(), None);
    assert_eq!(
        role.required_tools(),
        [
            "search_tools",
            "call_mcp_tool",
            "spawn_agent",
            "send_agent_message",
            "wait_agent",
        ]
    );
    assert_eq!(role.skills(), None);
    assert_eq!(role.required_skills(), ["github"]);
    assert!(role.role_instructions().contains("[issue #"));
    assert!(role.content_digest().starts_with("sha256:"));
}
