use std::fs;
use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::Digest;
use sha2::Sha256;
use zeta_extensions::DynamicExtensionSourceProvider;
use zeta_extensions::ExtensionCatalog;
use zeta_extensions::ExtensionCatalogReload;
use zeta_extensions::ExtensionRootKind;
use zeta_extensions::ExtensionSourceKind;
use zeta_lsp_server_provider::LspServerLaunch;
use zeta_lsp_server_provider::LspServerProviders;
use zeta_lsp_server_provider::ManagedNodeRuntime;
use zeta_marketplace_client::AcquireCapabilityRequest;
use zeta_marketplace_client::ActivationSpec;
use zeta_marketplace_client::AvailableCapability;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::DownloadPackageRequest;
use zeta_marketplace_client::GetPackageRequest;
use zeta_marketplace_client::InstallPackageRequest;
use zeta_marketplace_client::MarketplaceClientError;
use zeta_marketplace_client::MarketplaceInstallCapability;
use zeta_marketplace_client::MarketplacePackagePayload;
use zeta_marketplace_client::MarketplaceRegistryClient;
use zeta_marketplace_client::MarketplaceServiceClient;
use zeta_marketplace_client::OpenResourceRequest;
use zeta_marketplace_client::PackageDetails;
use zeta_marketplace_client::PackageRef;
use zeta_marketplace_client::PackageSource;
use zeta_marketplace_client::PackageSummary;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_manager::MarketplaceManager;

use super::UpdateBroker;
use super::marketplace_extension_sources::MarketplaceExtensionSourceProvider;
use super::marketplace_language_runtime::MarketplaceLanguageRuntime;
use super::marketplace_runtime::MarketplaceChangeWatcher;
use super::notification_queue::NotificationQueue;

const LANGUAGE_MANIFEST: &[u8] = br#"{
  "name": "demo-language",
  "publisher": "example",
  "version": "1.0.0",
  "contributes": { "languages": [{ "id": "demo" }] }
}"#;
const SERVER_ENTRYPOINT: &[u8] = b"// demo language server\n";
const THEME_MANIFEST: &[u8] = br#"{
  "schemaVersion": 1,
  "themes": [{
    "id": "demo",
    "displayName": "Demo",
    "appearance": "dark",
    "path": "themes/demo.json"
  }]
}"#;
const THEME_DOCUMENT: &[u8] = br#"{"type":"dark","colors":{},"tokenColors":[]}"#;

#[test]
fn marketplace_manager_commit_watcher_broadcasts_the_authoritative_change() {
    let root = tempfile::tempdir().unwrap();
    let manager = Arc::new(
        MarketplaceManager::open(root.path().join("manager"), Arc::new(LanguageRegistry)).unwrap(),
    );
    let updates = Arc::new(UpdateBroker::default());
    let queue = NotificationQueue::default();
    updates.register(updates.allocate_connection_id(), &queue);
    let _watcher = MarketplaceChangeWatcher::start(&manager, Arc::clone(&updates)).unwrap();

    manager
        .install(InstallPackageRequest {
            package_id: "example/demo-language".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    for _ in 0..50 {
        if queue.len() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let notifications = queue.drain();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0]["method"], "marketplace/changed");
    assert_eq!(notifications[0]["params"]["generation"], 2);
}

#[test]
fn installed_language_package_projects_assets_and_packaged_server() {
    let root = tempfile::tempdir().unwrap();
    let node = root.path().join("node");
    fs::write(&node, b"#!/bin/sh\n").unwrap();
    make_executable(&node);
    let manager = Arc::new(
        MarketplaceManager::open(root.path().join("manager"), Arc::new(LanguageRegistry)).unwrap(),
    );
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "example/demo-language".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();

    let extension_snapshot = MarketplaceExtensionSourceProvider::new(manager.clone())
        .snapshot()
        .unwrap();
    assert_eq!(extension_snapshot.packages.len(), 1);
    assert_eq!(
        extension_snapshot.packages[0].kind,
        ExtensionRootKind::Marketplace
    );
    assert!(
        extension_snapshot.packages[0]
            .path
            .join("package.json")
            .is_file()
    );

    let runtime = MarketplaceLanguageRuntime::new(
        manager.clone(),
        Some(ManagedNodeRuntime::from_path(&node).unwrap()),
        LspServerProviders::new(),
    );
    let providers = runtime.providers().unwrap();
    assert!(providers.contains("demo-language-server"));
    assert!(providers.activation_enables("demo-language-server"));
    let definition = providers
        .definition(
            "demo-language-server",
            root.path(),
            LspServerLaunch::Packaged,
        )
        .unwrap()
        .unwrap();
    assert_eq!(definition.language_ids().collect::<Vec<_>>(), vec!["demo"]);
    let (_, command, _) = definition.into_launch_parts();
    assert_eq!(command.program(), node.canonicalize().unwrap().as_os_str());
    assert_eq!(command.arguments().last().unwrap(), "--stdio");

    let language = installed
        .capabilities
        .iter()
        .find(|capability| capability.kind == CapabilityKind::Language)
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: language.reference.clone(),
        })
        .unwrap();
    let resource = match acquired.spec {
        ActivationSpec::Language(spec) => spec.manifest,
        _ => panic!("expected Language activation"),
    };
    let content = manager
        .open_resource(OpenResourceRequest {
            lease_id: acquired.lease.id,
            resource,
        })
        .unwrap();
    assert_eq!(
        STANDARD.decode(content.data_base64).unwrap(),
        LANGUAGE_MANIFEST
    );

    let executable = installed
        .capabilities
        .iter()
        .find(|capability| capability.kind == CapabilityKind::Executable)
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: executable.reference.clone(),
        })
        .unwrap();
    let resource = match acquired.spec {
        ActivationSpec::Executable(spec) => spec.entrypoint,
        _ => panic!("expected Executable activation"),
    };
    let content = manager
        .open_resource(OpenResourceRequest {
            lease_id: acquired.lease.id,
            resource,
        })
        .unwrap();
    assert_eq!(
        STANDARD.decode(content.data_base64).unwrap(),
        SERVER_ENTRYPOINT
    );
}

#[test]
fn installed_theme_enters_the_shared_declarative_extension_catalog() {
    let root = tempfile::tempdir().unwrap();
    let manager = Arc::new(
        MarketplaceManager::open(root.path().join("manager"), Arc::new(LanguageRegistry)).unwrap(),
    );
    let installed = manager
        .install(InstallPackageRequest {
            package_id: "example/demo-theme".into(),
            version: Some("1.0.0".into()),
        })
        .unwrap();
    let provider: Arc<dyn DynamicExtensionSourceProvider> = Arc::new(
        MarketplaceExtensionSourceProvider::new(Arc::clone(&manager)),
    );
    let mut catalog = ExtensionCatalog::new(Vec::new()).with_dynamic_sources(provider);
    let snapshot = catalog.list(ExtensionCatalogReload::Refresh);
    assert_eq!(snapshot.extensions.len(), 1);
    assert_eq!(snapshot.extensions[0].id, "example.demo-theme");
    assert_eq!(
        snapshot.extensions[0].source_kind,
        ExtensionSourceKind::Marketplace
    );
    let normalized_manifest: serde_json::Value =
        serde_json::from_str(&snapshot.extensions[0].manifest_json).unwrap();
    assert_eq!(
        normalized_manifest["contributes"]["themes"][0]["uiTheme"],
        "vs-dark"
    );

    let theme = installed
        .capabilities
        .iter()
        .find(|capability| capability.kind == CapabilityKind::Theme)
        .unwrap();
    let acquired = manager
        .acquire_capability(AcquireCapabilityRequest {
            capability: theme.reference.clone(),
        })
        .unwrap();
    let resource = match acquired.spec {
        ActivationSpec::Theme(spec) => spec.manifest,
        _ => panic!("expected Theme activation"),
    };
    let content = manager
        .open_resource(OpenResourceRequest {
            lease_id: acquired.lease.id,
            resource,
        })
        .unwrap();
    assert_eq!(
        STANDARD.decode(content.data_base64).unwrap(),
        THEME_MANIFEST
    );
}

struct LanguageRegistry;

impl MarketplaceRegistryClient for LanguageRegistry {
    fn search(
        &self,
        _: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        Ok(SearchPackagesResult {
            packages: vec![PackageSummary {
                id: "example/demo-language".into(),
                version: "1.0.0".into(),
                package_type: "language".into(),
                display_name: "Demo Language".into(),
                description: "Demo language support".into(),
            }],
        })
    }

    fn get(&self, _: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        let payload = LanguagePayload::new();
        Ok(PackageDetails {
            package: payload.package,
            package_type: "language".into(),
            display_name: "Demo Language".into(),
            description: "Demo language support".into(),
            license: "MIT".into(),
            source: PackageSource::ThirdParty,
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
        request: DownloadPackageRequest,
    ) -> Result<Box<dyn MarketplacePackagePayload>, MarketplaceClientError> {
        match request.package_id.as_str() {
            "example/demo-language" => Ok(Box::new(LanguagePayload::new())),
            "example/demo-theme" => Ok(Box::new(ThemePayload::new())),
            _ => Err(MarketplaceClientError::storage()),
        }
    }
}

struct ThemePayload {
    package: PackageRef,
    capabilities: Vec<MarketplaceInstallCapability>,
}

impl ThemePayload {
    fn new() -> Self {
        Self {
            package: PackageRef {
                id: "example/demo-theme".into(),
                version: "1.0.0".into(),
                digest: package_digest(&[
                    ("theme/package.json", THEME_MANIFEST),
                    ("theme/themes/demo.json", THEME_DOCUMENT),
                ]),
            },
            capabilities: vec![MarketplaceInstallCapability {
                kind: CapabilityKind::Theme,
                id: "theme-assets".into(),
                path: "theme".into(),
                runtime: None,
                language_ids: Vec::new(),
            }],
        }
    }
}

impl MarketplacePackagePayload for ThemePayload {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        "theme"
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        2
    }

    fn expected_size_bytes(&self) -> u64 {
        (THEME_DOCUMENT.len() + THEME_MANIFEST.len()) as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        fs::create_dir(destination.join("theme")).map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("theme/themes"))
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("theme/themes/demo.json"), THEME_DOCUMENT)
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("theme/package.json"), THEME_MANIFEST)
            .map_err(|_| MarketplaceClientError::storage())
    }
}

struct LanguagePayload {
    package: PackageRef,
    capabilities: Vec<MarketplaceInstallCapability>,
}

impl LanguagePayload {
    fn new() -> Self {
        Self {
            package: PackageRef {
                id: "example/demo-language".into(),
                version: "1.0.0".into(),
                digest: package_digest(&[
                    ("language/package.json", LANGUAGE_MANIFEST),
                    ("server/demo.js", SERVER_ENTRYPOINT),
                ]),
            },
            capabilities: vec![
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Language,
                    id: "language-assets".into(),
                    path: "language".into(),
                    runtime: None,
                    language_ids: Vec::new(),
                },
                MarketplaceInstallCapability {
                    kind: CapabilityKind::Executable,
                    id: "demo-language-server".into(),
                    path: "server/demo.js".into(),
                    runtime: Some("node".into()),
                    language_ids: vec!["demo".into()],
                },
            ],
        }
    }
}

impl MarketplacePackagePayload for LanguagePayload {
    fn package(&self) -> &PackageRef {
        &self.package
    }

    fn package_type(&self) -> &str {
        "language"
    }

    fn capabilities(&self) -> &[MarketplaceInstallCapability] {
        &self.capabilities
    }

    fn expected_file_count(&self) -> u64 {
        2
    }

    fn expected_size_bytes(&self) -> u64 {
        (LANGUAGE_MANIFEST.len() + SERVER_ENTRYPOINT.len()) as u64
    }

    fn copy_to(&self, destination: &Path) -> Result<(), MarketplaceClientError> {
        fs::create_dir(destination.join("language"))
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::create_dir(destination.join("server"))
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("language/package.json"), LANGUAGE_MANIFEST)
            .map_err(|_| MarketplaceClientError::storage())?;
        fs::write(destination.join("server/demo.js"), SERVER_ENTRYPOINT)
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

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_: &Path) {}
