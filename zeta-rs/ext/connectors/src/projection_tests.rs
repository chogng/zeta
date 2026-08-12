use zeta_plugins::PluginManifest;
use zeta_tools::DiscoverableCapability;
use zeta_tools::DiscoveryAction;

use crate::ConnectedAccount;
use crate::ConnectorCatalog;
use crate::ConnectorConnectionState;

fn manifest() -> PluginManifest {
    PluginManifest::from_json(
        br#"{
          "schemaVersion": 1,
          "id": "acme/github",
          "version": "1.0.0",
          "displayName": "GitHub",
          "compatibility": {"zeta": ">=0.1.0"},
          "contributions": {
            "mcpServers": [{"id": "github", "definition": "mcp/github.json"}],
            "connectors": [{
              "id": "account",
              "displayName": "GitHub account",
              "description": "Connect one GitHub account.",
              "mcpServer": "github"
            }]
          }
        }"#,
    )
    .unwrap()
}

#[test]
fn disconnected_connector_is_discoverable_but_not_runtime_ready() {
    let manifest = manifest();
    let catalog = ConnectorCatalog::from_manifests(7, [&manifest]).unwrap();
    let snapshot = catalog.discovery_snapshot().unwrap();

    assert!(catalog.ready_mcp_server_ids().is_empty());
    assert_eq!(snapshot.generation(), 7);
    assert!(matches!(
        &snapshot.candidates()[0],
        DiscoverableCapability::Connector(info) if info.action == DiscoveryAction::Connect
    ));
}

#[test]
fn connected_account_gates_its_mcp_binding_out_of_discovery() {
    let manifest = manifest();
    let catalog = ConnectorCatalog::from_manifests(8, [&manifest]).unwrap();
    let id = catalog.entries()[0].id.clone();
    let catalog = catalog
        .with_state(
            &id,
            ConnectorConnectionState::Connected(ConnectedAccount {
                account_id: "octocat".into(),
                display_name: "Octocat".into(),
                credential_reference: "secret:github-octocat".into(),
                connection_generation: 4,
            }),
        )
        .unwrap();

    assert_eq!(
        catalog
            .ready_mcp_server_ids()
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["plugin:acme/github:mcp:github"]
    );
    assert!(
        catalog
            .discovery_snapshot()
            .unwrap()
            .candidates()
            .is_empty()
    );
}
