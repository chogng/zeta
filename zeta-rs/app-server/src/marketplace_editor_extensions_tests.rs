use std::fs;
use std::path::Path;
use std::sync::Arc;

use sha2::Digest;
use sha2::Sha256;
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
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_client::UninstallMode;
use zeta_marketplace_client::UninstallPackageRequest;
use zeta_marketplace_manager::MarketplaceManager;

use super::MarketplaceEditorExtensionAdmission;
use super::MarketplaceEditorExtensionAdmissionLease;
use super::MarketplaceEditorExtensionBinding;
use super::deployments;
use super::valid_event_selector;
use super::valid_local_id;

const EXECUTABLE: &[u8] = b"#!/bin/sh\nexit 0\n";
const MCP: &[u8] = br#"{
  "schemaVersion": 1,
  "transport": "http",
  "url": "https://example.com/mcp"
}"#;
const PRODUCT_MANIFEST: &[u8] = br#"{
  "schemaVersion": 1,
  "editorExtensions": [{
    "id": "demo",
    "executable": "editor-runtime",
    "runtimeApiVersion": 1,
    "activationEvents": [{"type": "onCommand", "id": "demo.run"}],
    "capabilities": ["command"]
  }]
}"#;

#[test]
fn product_adapter_uses_manifest_local_identifiers() {
    assert!(valid_local_id("demo-extension"));
    assert!(!valid_local_id("DemoExtension"));
    assert!(!valid_local_id("demo--extension"));
    assert!(valid_event_selector("demo.run"));
    assert!(!valid_event_selector(""));
    assert!(!valid_event_selector("demo run"));
}

#[test]
fn signed_product_sidecar_requires_independent_admission_and_manager_lease() {
    let root = tempfile::tempdir().unwrap();
    let manager = Arc::new(
        MarketplaceManager::open(root.path().join("manager"), Arc::new(Registry)).unwrap(),
    );
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "marketplace/demo-plugin".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    let admission: Arc<dyn MarketplaceEditorExtensionAdmission> = Arc::new(AllowAdmission);
    let projected = deployments(&manager, &admission).unwrap();
    assert_eq!(projected.len(), 1);
    let deployment = &projected[0];
    assert_eq!(
        deployment.id,
        "marketplace:marketplace/demo-plugin:editor-extension:demo"
    );
    assert_eq!(deployment.params.activation_events, ["onCommand:demo.run"]);
    assert!(deployment.authority.authorizes());
    let lease = deployment.authority.acquire().unwrap();
    manager
        .uninstall(UninstallPackageRequest {
            installation_id: installed.installation_id,
            mode: UninstallMode::WhenUnused,
        })
        .unwrap();
    assert!(!deployment.authority.authorizes());
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

struct AllowAdmission;

impl MarketplaceEditorExtensionAdmission for AllowAdmission {
    fn generation(&self) -> u64 {
        1
    }

    fn authorizes(&self, _: &MarketplaceEditorExtensionBinding) -> bool {
        true
    }

    fn acquire(
        &self,
        _: &MarketplaceEditorExtensionBinding,
    ) -> Option<Box<dyn MarketplaceEditorExtensionAdmissionLease>> {
        Some(Box::new(AdmissionLease))
    }
}

struct AdmissionLease;

impl MarketplaceEditorExtensionAdmissionLease for AdmissionLease {}

struct Registry;

impl MarketplaceRegistryClient for Registry {
    fn search(
        &self,
        _: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        Ok(SearchPackagesResult {
            packages: vec![PackageSummary {
                id: "marketplace/demo-plugin".into(),
                version: "1.0.0".into(),
                package_type: "plugin".into(),
                display_name: "Demo".into(),
                description: "Demo integration bundle".into(),
            }],
        })
    }

    fn get(&self, _: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        let payload = Payload::new();
        Ok(PackageDetails {
            package: payload.package,
            package_type: "plugin".into(),
            display_name: "Demo".into(),
            description: "Demo integration bundle".into(),
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
                id: "marketplace/demo-plugin".into(),
                version: "1.0.0".into(),
                digest: package_digest(&[
                    ("bin/demo", EXECUTABLE),
                    ("mcp/package.json", MCP),
                    ("zeta/editor-extensions.json", PRODUCT_MANIFEST),
                ]),
            },
            capabilities: vec![
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Mcp,
                    id: "demo".into(),
                    path: "mcp/package.json".into(),
                    runtime: None,
                    language_ids: Vec::new(),
                },
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Executable,
                    id: "editor-runtime".into(),
                    path: "bin/demo".into(),
                    runtime: Some("direct".into()),
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
        3
    }

    fn expected_size_bytes(&self) -> u64 {
        (EXECUTABLE.len() + MCP.len() + PRODUCT_MANIFEST.len()) as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        fs::create_dir(destination.join("bin")).map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("mcp")).map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("zeta")).map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("bin/demo"), EXECUTABLE)
            .map_err(|_| MarketplaceClientError::storage())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                destination.join("bin/demo"),
                fs::Permissions::from_mode(0o755),
            )
            .map_err(|_| MarketplaceClientError::storage())?;
        }
        fs::write(destination.join("mcp/package.json"), MCP)
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(
            destination.join("zeta/editor-extensions.json"),
            PRODUCT_MANIFEST,
        )
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
