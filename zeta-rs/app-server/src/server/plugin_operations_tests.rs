use super::*;
use crate::local::ProviderModelService;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginAuthorityCommandId;
use zeta_plugins::PluginMarketplace;
use zeta_plugins::PluginMarketplaceMode;
use zeta_plugins::PluginMarketplaceService;

#[test]
fn app_server_projects_and_mutates_distinct_plugin_authority_layers() {
    let source = tempdir().unwrap();
    fs::create_dir_all(source.path().join(".zeta-plugin")).unwrap();
    fs::create_dir_all(source.path().join("skills/review")).unwrap();
    fs::write(source.path().join("skills/review/SKILL.md"), "# Review").unwrap();
    fs::write(
        source.path().join(".zeta-plugin/plugin.json"),
        r#"{
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Review",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {"skills": [{"id": "review", "path": "skills/review"}]},
            "permissions": []
        }"#,
    )
    .unwrap();
    let profile = tempdir().unwrap();
    let authority = PluginActivationAuthority::open(profile.path()).unwrap();
    let installed = authority
        .install_local(
            PluginAuthorityCommandId::new("install-review").unwrap(),
            0,
            &LocalPluginPackage::load(source.path()).unwrap(),
        )
        .unwrap()
        .package;
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    let server = AppServer::new(
        sessions,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
    .with_plugin_authority(authority);
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );

    let initial = call(
        &server,
        &mut connection,
        2,
        "plugin/list",
        serde_json::json!({}),
    );
    assert_eq!(initial["result"]["packages"][0]["enabled"], false);
    assert_eq!(initial["result"]["packages"][0]["granted"], false);
    assert_eq!(initial["result"]["packages"][0]["effective"], false);
    let target = |command_id: &str, expected_revision: u64| {
        serde_json::json!({
            "commandId": command_id,
            "expectedRevision": expected_revision,
            "id": installed.id.as_str(),
            "version": installed.version.to_string(),
            "digest": installed.digest.as_str()
        })
    };
    let grant = call(
        &server,
        &mut connection,
        3,
        "plugin/grant",
        target("grant-review", 1),
    );
    assert_eq!(grant["result"]["activationGeneration"], 1);
    let enable = call(
        &server,
        &mut connection,
        4,
        "plugin/enable",
        target("enable-review", 2),
    );
    assert_eq!(enable["result"]["activationGeneration"], 2);

    let effective = call(
        &server,
        &mut connection,
        5,
        "plugin/list",
        serde_json::json!({}),
    );
    assert_eq!(effective["result"]["packages"][0]["enabled"], true);
    assert_eq!(effective["result"]["packages"][0]["granted"], true);
    assert_eq!(effective["result"]["packages"][0]["effective"], true);
}

#[test]
fn app_server_installs_only_exact_host_registered_marketplace_entries() {
    let root = tempdir().unwrap();
    let package_root = root.path().join("packages/review");
    fs::create_dir_all(package_root.join(".zeta-plugin")).unwrap();
    fs::create_dir_all(package_root.join("skills/review")).unwrap();
    fs::create_dir_all(package_root.join("mcp")).unwrap();
    fs::create_dir_all(package_root.join("assets")).unwrap();
    fs::write(package_root.join("skills/review/SKILL.md"), "# Review").unwrap();
    fs::write(package_root.join("mcp/review.json"), "{}").unwrap();
    fs::write(package_root.join("assets/icon.txt"), "review icon").unwrap();
    fs::write(
        package_root.join(".zeta-plugin/plugin.json"),
        r#"{
            "schemaVersion": 1,
            "id": "acme/review",
            "version": "1.0.0",
            "displayName": "Acme Review",
            "description": "Review workspace changes with a repeatable workflow.",
            "license": "Apache-2.0",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {
                "skills": [{"id": "review", "path": "skills/review"}],
                "mcpServers": [{"id": "review", "definition": "mcp/review.json"}],
                "assets": [{"id": "icon", "path": "assets/icon.txt"}]
            },
            "permissions": [
                {"type": "workspace", "access": "read"},
                {"type": "network", "hosts": ["api.example.com"]}
            ],
            "credentialSlots": [
                {"name": "api-token", "kind": "secretText", "requiredFor": ["mcp:review"]}
            ]
        }"#,
    )
    .unwrap();
    let package = LocalPluginPackage::load(&package_root).unwrap();
    fs::create_dir_all(root.path().join(".zeta-marketplace")).unwrap();
    fs::write(
        root.path().join(".zeta-marketplace/marketplace.json"),
        format!(
            r#"{{"schemaVersion":1,"id":"acme","plugins":[{{"id":"acme/review","version":"1.0.0","digest":"{}","path":"packages/review"}}]}}"#,
            package.package_digest()
        ),
    )
    .unwrap();
    let authority = PluginActivationAuthority::open(root.path().join("profile")).unwrap();
    let marketplace = PluginMarketplace::open(root.path(), PluginMarketplaceMode::Managed).unwrap();
    let marketplaces = PluginMarketplaceService::new(authority.clone(), [marketplace]).unwrap();
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    let server = AppServer::new(
        sessions,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
    .with_plugin_authority(authority)
    .with_plugin_marketplaces(marketplaces);
    let mut connection = server.connection();
    call(
        &server,
        &mut connection,
        1,
        "initialize",
        serde_json::json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );

    let available = call(
        &server,
        &mut connection,
        2,
        "plugin/marketplace/list",
        serde_json::json!({}),
    );
    let entry = &available["result"]["packages"][0];
    assert_eq!(entry["marketplaceMode"], "managed");
    assert_eq!(entry["marketplaceTrust"], "productManaged");
    assert_eq!(entry["publisher"], "acme");
    assert_eq!(entry["displayName"], "Acme Review");
    assert_eq!(
        entry["description"],
        "Review workspace changes with a repeatable workflow."
    );
    assert_eq!(entry["license"], "Apache-2.0");
    assert_eq!(entry["compatibilityZeta"], ">=0.1.0");
    assert_eq!(entry["contributions"]["skills"], 1);
    assert_eq!(entry["contributions"]["mcpServers"], 1);
    assert_eq!(entry["contributions"]["assets"], 1);
    assert_eq!(entry["permissions"][0]["type"], "workspace");
    assert_eq!(entry["permissions"][0]["access"], "read");
    assert_eq!(entry["permissions"][1]["hosts"][0], "api.example.com");
    assert_eq!(entry["credentialSlots"][0]["kind"], "secretText");
    assert_eq!(entry["credentialSlots"][0]["requiredFor"][0], "mcp:review");
    assert_eq!(entry["packageFileCount"], 4);
    assert!(entry["packageSizeBytes"].as_u64().unwrap() > 0);
    assert_eq!(entry["installed"], false);
    assert_eq!(entry["enabled"], false);
    assert_eq!(entry["granted"], false);
    assert_eq!(entry["effective"], false);
    assert_eq!(entry["revoked"], false);

    let installed = call(
        &server,
        &mut connection,
        3,
        "plugin/install",
        serde_json::json!({
            "commandId": "install-marketplace-review",
            "expectedRevision": 0,
            "marketplaceId": "acme",
            "id": entry["id"],
            "version": entry["version"],
            "digest": entry["digest"]
        }),
    );
    assert_eq!(installed["result"]["revision"], 1);
    let packages = call(
        &server,
        &mut connection,
        4,
        "plugin/list",
        serde_json::json!({}),
    );
    assert_eq!(packages["result"]["packages"][0]["enabled"], false);
    assert_eq!(packages["result"]["packages"][0]["granted"], false);
}

fn call(
    server: &AppServer,
    connection: &mut crate::server::ConnectionState,
    id: u64,
    method: &str,
    params: Value,
) -> Value {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}
