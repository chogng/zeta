use std::sync::Arc;

use serde_json::Value;
use serde_json::json;
use zeta_marketplace_client::MarketplaceServiceClient;

use super::AppServer;
use crate::local::ProviderModelService;
use crate::server::ConnectionState;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_model_provider::EchoModel;

#[test]
fn app_server_exposes_only_the_manager_business_contract() {
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
    .with_marketplace_manager_client(Arc::new(FakeMarketplaceManager));
    let mut connection = server.connection();
    let initialized = call(
        &server,
        &mut connection,
        1,
        "initialize",
        json!({
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }),
    );
    assert_eq!(initialized["result"]["capabilities"]["marketplace"], true);

    let found = call(
        &server,
        &mut connection,
        2,
        "marketplace/search",
        json!({"query": "github", "packageType": "plugin", "limit": 20}),
    );
    assert_eq!(found["result"]["packages"][0]["id"], "marketplace/github");

    let details = call(
        &server,
        &mut connection,
        20,
        "marketplace/get",
        json!({"packageId": "marketplace/docs-mcp", "version": "1.2.3"}),
    );
    assert_eq!(details["result"]["upstream"]["registry"], "officialMcp");
    assert_eq!(details["result"]["upstream"]["name"], "ac.example/docs-mcp");
    assert!(details.to_string().find("targetUrl").is_none());

    let installed = call(
        &server,
        &mut connection,
        3,
        "marketplace/install",
        json!({"packageId": "marketplace/github", "version": "1.1.0"}),
    );
    assert_eq!(installed["result"]["installationId"], "ins_opaque");
    assert_eq!(installed["result"]["capabilities"][0]["kind"], "skill");
    assert!(installed.to_string().find("path").is_none());

    let acquired = call(
        &server,
        &mut connection,
        4,
        "marketplace/acquireCapability",
        json!({"capability": {"id": "cap_opaque"}}),
    );
    assert_eq!(acquired["result"]["lease"]["id"], "lease_opaque");

    let mut other_connection = server.connection();
    call(
        &server,
        &mut other_connection,
        5,
        "initialize",
        json!({
            "clientInfo": {"name": "other", "version": "1"},
            "capabilities": {}
        }),
    );
    let rejected = call(
        &server,
        &mut other_connection,
        6,
        "marketplace/openResource",
        json!({"leaseId": "lease_opaque", "resource": {"id": "res_opaque"}}),
    );
    assert!(rejected["error"].is_object());

    let released = call(
        &server,
        &mut connection,
        7,
        "marketplace/releaseCapability",
        json!({"leaseId": "lease_opaque"}),
    );
    assert_eq!(released["result"], json!(null));
}

struct FakeMarketplaceManager;

impl MarketplaceServiceClient for FakeMarketplaceManager {
    fn search(
        &self,
        _: zeta_marketplace_client::SearchPackagesRequest,
    ) -> Result<
        zeta_marketplace_client::SearchPackagesResult,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        Ok(zeta_marketplace_client::SearchPackagesResult {
            packages: vec![zeta_marketplace_client::PackageSummary {
                id: "marketplace/github".to_owned(),
                version: "1.1.0".to_owned(),
                package_type: "plugin".to_owned(),
                display_name: "GitHub".to_owned(),
                description: "GitHub integration".to_owned(),
            }],
        })
    }

    fn get(
        &self,
        _: zeta_marketplace_client::GetPackageRequest,
    ) -> Result<
        zeta_marketplace_client::PackageDetails,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        Ok(zeta_marketplace_client::PackageDetails {
            package: zeta_marketplace_client::PackageRef {
                id: "marketplace/docs-mcp".to_owned(),
                version: "1.2.3".to_owned(),
                digest: format!("sha256:{}", "b".repeat(64)),
            },
            package_type: "mcp".to_owned(),
            display_name: "Docs MCP".to_owned(),
            description: "Search documentation".to_owned(),
            license: "MIT".to_owned(),
            source: zeta_marketplace_client::PackageSource::ThirdParty,
            upstream: Some(zeta_marketplace_client::UpstreamReference {
                registry: zeta_marketplace_client::UpstreamRegistry::OfficialMcp,
                name: "ac.example/docs-mcp".to_owned(),
                version: "1.2.3".to_owned(),
                record_url: "https://registry.modelcontextprotocol.io/v0.1/servers/ac.example%2Fdocs-mcp/versions/1.2.3".to_owned(),
                repository_url: Some("https://github.com/example/docs-mcp".to_owned()),
            }),
            capabilities: Vec::new(),
        })
    }

    fn download(
        &self,
        _: zeta_marketplace_client::DownloadPackageRequest,
    ) -> Result<
        zeta_marketplace_client::ArtifactHandle,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        unimplemented!()
    }

    fn install(
        &self,
        _: zeta_marketplace_client::InstallPackageRequest,
    ) -> Result<
        zeta_marketplace_client::InstalledPackage,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        Ok(zeta_marketplace_client::InstalledPackage {
            installation_id: "ins_opaque".to_owned(),
            package: zeta_marketplace_client::PackageRef {
                id: "marketplace/github".to_owned(),
                version: "1.1.0".to_owned(),
                digest: format!("sha256:{}", "a".repeat(64)),
            },
            state: zeta_marketplace_client::InstallationState::Installed,
            capabilities: vec![zeta_marketplace_client::CapabilityDescriptor {
                reference: zeta_marketplace_client::CapabilityRef {
                    id: "cap_opaque".to_owned(),
                },
                kind: zeta_marketplace_client::CapabilityKind::Skill,
                id: "github".to_owned(),
                contract_version: "1".to_owned(),
                permissions: Vec::new(),
                authentication_provider: None,
            }],
        })
    }

    fn update(
        &self,
        _: zeta_marketplace_client::UpdatePackageRequest,
    ) -> Result<
        zeta_marketplace_client::InstalledPackage,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        unimplemented!()
    }

    fn uninstall(
        &self,
        _: zeta_marketplace_client::UninstallPackageRequest,
    ) -> Result<(), zeta_marketplace_client::MarketplaceClientError> {
        unimplemented!()
    }

    fn list_installed(
        &self,
        _: zeta_marketplace_client::ListInstalledRequest,
    ) -> Result<
        Vec<zeta_marketplace_client::InstalledPackage>,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        unimplemented!()
    }

    fn acquire_capability(
        &self,
        _: zeta_marketplace_client::AcquireCapabilityRequest,
    ) -> Result<
        zeta_marketplace_client::AcquiredCapability,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        Ok(zeta_marketplace_client::AcquiredCapability {
            lease: zeta_marketplace_client::CapabilityLease {
                id: "lease_opaque".to_owned(),
                capability: zeta_marketplace_client::CapabilityRef {
                    id: "cap_opaque".to_owned(),
                },
                installation_id: "ins_opaque".to_owned(),
            },
            spec: zeta_marketplace_client::ActivationSpec::Skill(
                zeta_marketplace_client::SkillActivationSpec {
                    contract_version: "1".to_owned(),
                    resource: zeta_marketplace_client::ResourceRef {
                        id: "res_opaque".to_owned(),
                    },
                },
            ),
        })
    }

    fn release_capability(
        &self,
        _: zeta_marketplace_client::ReleaseCapabilityRequest,
    ) -> Result<(), zeta_marketplace_client::MarketplaceClientError> {
        Ok(())
    }

    fn open_resource(
        &self,
        _: zeta_marketplace_client::OpenResourceRequest,
    ) -> Result<
        zeta_marketplace_client::ResourceContent,
        zeta_marketplace_client::MarketplaceClientError,
    > {
        unimplemented!()
    }
}

fn call(
    server: &AppServer,
    connection: &mut ConnectionState,
    id: u64,
    method: &str,
    params: Value,
) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}
