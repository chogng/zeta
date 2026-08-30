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
