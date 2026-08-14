use std::fs;
use std::path::Path;
use std::sync::Arc;

use sha2::Digest;
use sha2::Sha256;
use zeta_marketplace_client::AcquireCapabilityRequest;
use zeta_marketplace_client::AvailableCapability;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::DownloadPackageRequest;
use zeta_marketplace_client::GetPackageRequest;
use zeta_marketplace_client::InstallPackageRequest;
use zeta_marketplace_client::ListInstalledRequest;
use zeta_marketplace_client::MarketplaceClientError;
use zeta_marketplace_client::MarketplaceInstallCapability;
use zeta_marketplace_client::MarketplacePackagePayload;
use zeta_marketplace_client::MarketplaceRegistryClient;
use zeta_marketplace_client::MarketplaceServiceClient;
use zeta_marketplace_client::PackageDetails;
use zeta_marketplace_client::PackageRef;
use zeta_marketplace_client::PackageSource;
use zeta_marketplace_client::PackageSummary;
use zeta_marketplace_client::ReleaseCapabilityRequest;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_client::UninstallMode;
use zeta_marketplace_client::UninstallPackageRequest;
use zeta_marketplace_manager::MarketplaceManager;
use zeta_mcp::McpServerTransport;
use zeta_secrets::SecretValue;

use super::MarketplaceConnectorProjection;

const CONNECTOR: &[u8] = br#"{
  "schemaVersion": 1,
  "id": "github",
  "displayName": "GitHub",
  "authentication": "oauth",
  "mcpServer": "github"
}"#;
const MCP: &[u8] = br#"{
  "schemaVersion": 1,
  "transport": "http",
  "url": "https://example.com/mcp"
}"#;

#[test]
fn installed_marketplace_plugin_projects_connector_and_mcp_with_live_lease() {
    let root = tempfile::tempdir().unwrap();
    let manager = Arc::new(
        MarketplaceManager::open(root.path().join("manager"), Arc::new(Registry)).unwrap(),
    );
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "marketplace/github".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    let source = manager
        .local_capability_sources(CapabilityKind::Mcp)
        .unwrap()
        .pop()
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: source.capability().clone(),
        })
        .unwrap();
    manager
        .release_capability(ReleaseCapabilityRequest {
            lease_id: acquired.lease.id,
        })
        .unwrap();
    let projection = MarketplaceConnectorProjection::from_manager(Arc::clone(&manager)).unwrap();
    assert_eq!(projection.definitions().len(), 1);
    let connector = &projection.definitions()[0];
    assert_eq!(
        connector.id().as_str(),
        "marketplace:marketplace/github:connector:github"
    );
    let provider = projection.provider();
    assert!(provider.standalone_servers().unwrap().is_empty());
    let transport = provider
        .materialize(connector, SecretValue::new(b"token".to_vec()))
        .unwrap();
    let McpServerTransport::StreamableHttp(endpoint) = transport else {
        panic!("expected Streamable HTTP MCP transport");
    };
    assert_eq!(endpoint.uri(), "https://example.com/mcp");

    let fence = provider.invocation_fence(connector).unwrap();
    assert!(fence.authorizes());
    let lease = fence.acquire().unwrap();
    manager
        .uninstall(UninstallPackageRequest {
            installation_id: installed.installation_id,
            mode: UninstallMode::WhenUnused,
        })
        .unwrap();
    assert!(!fence.authorizes());
    assert_eq!(
        manager
            .list_installed(ListInstalledRequest {})
            .unwrap()
            .len(),
        1
    );
    drop(lease);
    assert!(
        manager
            .list_installed(ListInstalledRequest {})
            .unwrap()
            .is_empty()
    );
}

struct Registry;

impl MarketplaceRegistryClient for Registry {
    fn search(
        &self,
        _: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        Ok(SearchPackagesResult {
            packages: vec![PackageSummary {
                id: "marketplace/github".into(),
                version: "1.0.0".into(),
                package_type: "plugin".into(),
                display_name: "GitHub".into(),
                description: "GitHub integration".into(),
            }],
        })
    }

    fn get(&self, _: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        let payload = Payload::new();
        Ok(PackageDetails {
            package: payload.package,
            package_type: "plugin".into(),
            display_name: "GitHub".into(),
            description: "GitHub integration".into(),
            license: "MIT".into(),
            source: PackageSource::Official,
            upstream: None,
            capabilities: payload
                .capabilities
                .into_iter()
                .map(|capability| AvailableCapability {
                    kind: capability.kind,
                    id: capability.id,
                    contract_version: "1".into(),
                    permissions: Vec::new(),
                    authentication_provider: None,
                })
                .collect(),
        })
    }

    fn download(
        &self,
        _: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError> {
        Ok(Box::new(Payload::new()))
    }
}

struct Payload {
    package: PackageRef,
    capabilities: Vec<MarketplaceInstallCapability>,
}

impl Payload {
    fn new() -> Self {
        Self {
            package: PackageRef {
                id: "marketplace/github".into(),
                version: "1.0.0".into(),
                digest: package_digest(&[
                    ("connectors/github.json", CONNECTOR),
                    ("mcp/github.json", MCP),
                ]),
            },
            capabilities: vec![
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Connector,
                    id: "github".into(),
                    path: "connectors/github.json".into(),
                    runtime: None,
                    language_ids: Vec::new(),
                },
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Mcp,
                    id: "github".into(),
                    path: "mcp/github.json".into(),
                    runtime: None,
                    language_ids: Vec::new(),
                },
            ],
        }
    }
}

impl MarketplacePackagePayload for Payload {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        "plugin"
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        2
    }

    fn expected_size_bytes(&self) -> u64 {
        (CONNECTOR.len() + MCP.len()) as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        fs::create_dir(destination.join("connectors"))
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("mcp")).map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("connectors/github.json"), CONNECTOR)
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("mcp/github.json"), MCP)
            .map_err(|_| MarketplaceClientError::storage())
    }
}

fn package_digest(files: &[(&str, &[u8])]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"marketplace-package-v1\0");
    for (path, contents) in files {
        hasher.update((path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update((contents.len() as u64).to_be_bytes());
        hasher.update(contents);
    }
    format!("sha256:{}", hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
