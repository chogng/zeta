use super::has_unknown_external_effect;
use zeta_protocol::ToolCallBinding;
use zeta_protocol::ToolCallCaller;
use zeta_protocol::ToolSourceProvenance;

#[test]
fn only_started_product_tools_with_a_confined_contract_have_known_effects() {
    let product = binding(ToolSourceProvenance::Product {
        component: "zeta-app-server".into(),
    });
    let mcp = binding(ToolSourceProvenance::Mcp {
        server_id: "server".into(),
        remote_name: "write_file".into(),
        catalog_generation: 1,
        connection_generation: 1,
    });

    assert!(!has_unknown_external_effect(
        "write_file",
        Some(&product),
        true,
    ));
    assert!(has_unknown_external_effect(
        "shell-command",
        Some(&product),
        true,
    ));
    assert!(has_unknown_external_effect("write_file", Some(&mcp), true,));
    assert!(has_unknown_external_effect("write_file", None, true));
    assert!(!has_unknown_external_effect("write_file", None, false));
}

fn binding(source: ToolSourceProvenance) -> ToolCallBinding {
    ToolCallBinding {
        registry_incarnation: Some("registry".into()),
        registry_generation: 1,
        definition_digest: "sha256:definition".into(),
        source_chain: vec![source],
        caller: ToolCallCaller::Direct,
    }
}

#[test]
fn scoped_process_and_read_only_helper_receipts_require_trusted_host_provenance() {
    let product = binding(ToolSourceProvenance::Product {
        component: "zeta-app-server".into(),
    });
    let remote = binding(ToolSourceProvenance::Mcp {
        server_id: "remote".into(),
        remote_name: "shell-command".into(),
        catalog_generation: 1,
        connection_generation: 1,
    });
    let result = |text: &str| zeta_protocol::ThreadItem::ToolResult {
        item_id: zeta_protocol::ItemId::new("item").unwrap(),
        turn_id: zeta_protocol::TurnId::new("turn").unwrap(),
        tool_call_id: zeta_protocol::ToolCallId::new("call").unwrap(),
        text: text.into(),
        content: None,
        is_error: false,
    };
    let confined = result(r#"{"result":{"managed_scope":true,"stdout":"ok"}}"#);
    assert!(super::is_confined_process(
        "shell-command",
        Some(&product),
        &confined
    ));
    assert!(!super::is_confined_process(
        "shell-command",
        Some(&remote),
        &confined
    ));
    assert!(!super::is_confined_process(
        "shell-command",
        None,
        &confined
    ));
    assert!(!super::is_confined_process(
        "shell-command",
        Some(&product),
        &result(r#"{"result":{"stdout":"{\"managed_scope\":true}"}}"#)
    ));
    let helper = result(r#"{"issue_read_only_helper":true}"#);
    assert!(super::is_issue_investigation(
        "spawn_agent",
        Some(&product),
        &helper
    ));
    assert!(!super::is_issue_investigation(
        "spawn_agent",
        Some(&remote),
        &helper
    ));
    assert!(!super::is_issue_investigation(
        "shell-command",
        Some(&product),
        &helper
    ));
}
