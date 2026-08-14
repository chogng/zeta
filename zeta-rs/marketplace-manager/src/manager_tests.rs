use std::fs;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
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
use zeta_marketplace_client::OpenResourceRequest;
use zeta_marketplace_client::PackageDetails;
use zeta_marketplace_client::PackageRef;
use zeta_marketplace_client::PackageSummary;
use zeta_marketplace_client::ReleaseCapabilityRequest;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_client::UninstallMode;
use zeta_marketplace_client::UninstallPackageRequest;
use zeta_marketplace_client::UpdatePackageRequest;

use super::MarketplaceManager;

const SKILL_CONTENT: &[u8] = b"# Demo skill\n";
const MCP_DEFINITION: &[u8] = br#"{
  "schemaVersion": 1,
  "transport": "stdio",
  "command": "server/demo",
  "args": ["--stdio"]
}"#;
const MCP_EXECUTABLE: &[u8] = b"#!/bin/sh\n";

#[test]
fn local_manager_persists_the_full_installed_state() {
    let root = tempfile::tempdir().unwrap();
    let registry = Arc::new(FakeRegistry);
    let manager = MarketplaceManager::open(root.path(), registry.clone()).unwrap();
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "example/demo".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    assert_eq!(installed.capabilities.len(), 1);
    assert_eq!(
        manager.list_installed(ListInstalledRequest {}).unwrap(),
        vec![installed.clone()]
    );

    let reopened = MarketplaceManager::open(root.path(), registry).unwrap();
    assert_eq!(
        reopened.list_installed(ListInstalledRequest {}).unwrap(),
        vec![installed]
    );
}

#[test]
fn local_manager_updates_uninstalls_and_owns_resource_leases() {
    let root = tempfile::tempdir().unwrap();
    let manager = MarketplaceManager::open(root.path(), Arc::new(FakeRegistry)).unwrap();
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "example/demo".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: installed.capabilities[0].reference.clone(),
        })
        .unwrap();
    let resource = match &acquired.spec {
        zeta_marketplace_client::ActivationSpec::Skill(skill) => skill.resource.clone(),
        _ => panic!("expected Skill activation"),
    };
    let content = manager
        .open_resource(OpenResourceRequest {
            lease_id: acquired.lease.id.clone(),
            resource,
        })
        .unwrap();
    assert_eq!(STANDARD.decode(content.data_base64).unwrap(), SKILL_CONTENT);
    manager
        .release_capability(ReleaseCapabilityRequest {
            lease_id: acquired.lease.id,
        })
        .unwrap();

    let updated = manager
        .update(UpdatePackageRequest {
            installation_id: installed.installation_id,
            version: Some("2.0.0".into()),
        })
        .unwrap();
    assert_eq!(updated.package.version, "2.0.0");
    assert_eq!(
        manager.list_installed(ListInstalledRequest {}).unwrap(),
        vec![updated.clone()]
    );
    manager
        .uninstall(UninstallPackageRequest {
            installation_id: updated.installation_id,
            mode: UninstallMode::IfUnused,
        })
        .unwrap();
    assert!(
        manager
            .list_installed(ListInstalledRequest {})
            .unwrap()
            .is_empty()
    );
}

#[test]
fn local_capability_sources_revalidate_the_immutable_artifact() {
    let root = tempfile::tempdir().unwrap();
    let manager = MarketplaceManager::open(root.path(), Arc::new(FakeRegistry)).unwrap();
    manager
        .install(InstallPackageRequest {
            package_id: "example/demo".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();

    let sources = manager
        .local_capability_sources(CapabilityKind::Skill)
        .unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].id(), "demo");
    assert_eq!(sources[0].runtime(), None);
    assert!(sources[0].language_ids().is_empty());
    assert_eq!(
        fs::read(sources[0].host_path().join("SKILL.md")).unwrap(),
        SKILL_CONTENT
    );

    fs::write(sources[0].host_path().join("SKILL.md"), b"tampered").unwrap();
    assert!(
        manager
            .local_capability_sources(CapabilityKind::Skill)
            .is_err()
    );
}

#[test]
fn packaged_stdio_mcp_uses_a_path_free_executable_resource() {
    let root = tempfile::tempdir().unwrap();
    let manager = MarketplaceManager::open(root.path(), Arc::new(StdioRegistry)).unwrap();
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "example/demo-mcp".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: installed.capabilities[0].reference.clone(),
        })
        .unwrap();
    let executable = match &acquired.spec {
        zeta_marketplace_client::ActivationSpec::Mcp(spec) => match &spec.transport {
            zeta_marketplace_client::McpTransportSpec::Stdio { executable, args } => {
                assert_eq!(args, &["--stdio"]);
                executable.clone()
            }
            _ => panic!("expected stdio MCP activation"),
        },
        _ => panic!("expected MCP activation"),
    };
    let content = manager
        .open_resource(OpenResourceRequest {
            lease_id: acquired.lease.id.clone(),
            resource: executable,
        })
        .unwrap();
    assert_eq!(
        STANDARD.decode(content.data_base64).unwrap(),
        MCP_EXECUTABLE
    );
    manager
        .release_capability(ReleaseCapabilityRequest {
            lease_id: acquired.lease.id,
        })
        .unwrap();
}

struct FakeRegistry;

impl MarketplaceRegistryClient for FakeRegistry {
    fn search(
        &self,
        _: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        Ok(SearchPackagesResult {
            packages: vec![PackageSummary {
                id: "example/demo".into(),
                version: "2.0.0".into(),
                package_type: "skill".into(),
                display_name: "Demo".into(),
                description: "Demo skill".into(),
            }],
        })
    }

    fn get(&self, request: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        let payload = FakePayload::new(request.version.as_deref().unwrap_or("2.0.0"));
        Ok(PackageDetails {
            package: payload.package.clone(),
            package_type: "skill".into(),
            display_name: "Demo".into(),
            description: "Demo skill".into(),
            license: "MIT".into(),
            source: zeta_marketplace_client::PackageSource::ThirdParty,
            upstream: None,
            capabilities: vec![AvailableCapability {
                kind: CapabilityKind::Skill,
                id: "demo".into(),
                contract_version: "1".into(),
                permissions: Vec::new(),
                authentication_provider: None,
            }],
        })
    }

    fn download(
        &self,
        request: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError> {
        Ok(Box::new(FakePayload::new(
            request.version.as_deref().unwrap_or("2.0.0"),
        )))
    }
}

struct StdioRegistry;

impl MarketplaceRegistryClient for StdioRegistry {
    fn search(
        &self,
        _: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        Ok(SearchPackagesResult {
            packages: Vec::new(),
        })
    }

    fn get(&self, _: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        Err(MarketplaceClientError::storage())
    }

    fn download(
        &self,
        _: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError> {
        Ok(Box::new(StdioPayload::new()))
    }
}

struct StdioPayload {
    package: PackageRef,
    capabilities: Vec<MarketplaceInstallCapability>,
}

impl StdioPayload {
    fn new() -> Self {
        Self {
            package: PackageRef {
                id: "example/demo-mcp".into(),
                version: "1.0.0".into(),
                digest: package_digest_files(&[
                    ("mcp/package.json", MCP_DEFINITION),
                    ("server/demo", MCP_EXECUTABLE),
                ]),
            },
            capabilities: vec![MarketplaceInstallCapability {
                kind: CapabilityKind::Mcp,
                id: "demo".into(),
                path: "mcp/package.json".into(),
                runtime: None,
                language_ids: Vec::new(),
            }],
        }
    }
}

impl MarketplacePackagePayload for StdioPayload {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        "mcp"
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        2
    }

    fn expected_size_bytes(&self) -> u64 {
        (MCP_DEFINITION.len() + MCP_EXECUTABLE.len()) as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        fs::create_dir(destination.join("mcp")).map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("server"))
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("mcp/package.json"), MCP_DEFINITION)
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("server/demo"), MCP_EXECUTABLE)
            .map_err(|_| MarketplaceClientError::storage())
    }
}

struct FakePayload {
    package: PackageRef,
    capabilities: Vec<MarketplaceInstallCapability>,
}

impl FakePayload {
    fn new(version: &str) -> Self {
        Self {
            package: PackageRef {
                id: "example/demo".into(),
                version: version.into(),
                digest: package_digest("skill/SKILL.md", SKILL_CONTENT),
            },
            capabilities: vec![MarketplaceInstallCapability {
                kind: CapabilityKind::Skill,
                id: "demo".into(),
                path: "skill".into(),
                runtime: None,
                language_ids: Vec::new(),
            }],
        }
    }
}

impl MarketplacePackagePayload for FakePayload {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        "skill"
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        1
    }

    fn expected_size_bytes(&self) -> u64 {
        SKILL_CONTENT.len() as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        let skill = destination.join("skill");
        fs::create_dir(&skill).map_err(|_| MarketplaceClientError::storage())?;
        fs::write(skill.join("SKILL.md"), SKILL_CONTENT)
            .map_err(|_| MarketplaceClientError::storage())
    }
}

fn package_digest(path: &str, contents: &[u8]) -> String {
    package_digest_files(&[(path, contents)])
}

fn package_digest_files(files: &[(&str, &[u8])]) -> String {
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
