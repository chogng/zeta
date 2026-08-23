use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;
use zeta_marketplace_client::AcquireCapabilityRequest;
use zeta_marketplace_client::AcquiredCapability;
use zeta_marketplace_client::ArtifactHandle;
use zeta_marketplace_client::CapabilityDescriptor;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::CapabilityLease;
use zeta_marketplace_client::CapabilityRef;
use zeta_marketplace_client::DownloadPackageRequest;
use zeta_marketplace_client::GetPackageRequest;
use zeta_marketplace_client::InstallPackageRequest;
use zeta_marketplace_client::InstallationState;
use zeta_marketplace_client::InstalledPackage;
use zeta_marketplace_client::ListInstalledRequest;
use zeta_marketplace_client::MarketplaceClientError;
use zeta_marketplace_client::MarketplaceErrorCode;
use zeta_marketplace_client::MarketplacePackagePayload;
use zeta_marketplace_client::MarketplaceRegistryClient;
use zeta_marketplace_client::MarketplaceServiceClient;
use zeta_marketplace_client::OpenResourceRequest;
use zeta_marketplace_client::PackageDetails;
use zeta_marketplace_client::PackageRef;
use zeta_marketplace_client::ReleaseCapabilityRequest;
use zeta_marketplace_client::ResourceContent;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_client::SearchPackagesResult;
use zeta_marketplace_client::UninstallMode;
use zeta_marketplace_client::UninstallPackageRequest;
use zeta_marketplace_client::UpdatePackageRequest;

use crate::activation;
use crate::store::Store;
use crate::store::opaque_id;

/// Zeta's local owner for Marketplace package installation and capability leases.
///
/// Implementations receive a remote registry client for signed discovery and downloads. All
/// artifact storage, installation state, update/uninstall behavior, resource access, and leases
/// remain local to the Zeta profile.
pub struct MarketplaceManager {
    registry: Arc<dyn MarketplaceRegistryClient>,
    store: Store,
    runtime: Mutex<RuntimeState>,
    lease_sequence: AtomicU64,
    session_nonce: String,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct DurableState {
    installations: BTreeMap<String, InstallationRecord>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InstallationRecord {
    pub installation_id: String,
    pub package: PackageRef,
    pub state: InstallationState,
    pub capabilities: Vec<CapabilityRecord>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityRecord {
    pub descriptor: CapabilityDescriptor,
    pub path: String,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub language_ids: Vec<String>,
}

/// A locally verified capability source for trusted in-process runtime adapters.
///
/// This handle is never serialized or projected through App Server. Product runtimes may use its
/// host path only to construct their own validated, read-only source handles; remote clients must
/// continue to use capability leases and opaque resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalCapabilitySource {
    package: PackageRef,
    capability: CapabilityRef,
    kind: CapabilityKind,
    id: String,
    runtime: Option<String>,
    language_ids: Vec<String>,
    package_root: PathBuf,
    relative_path: String,
    host_path: PathBuf,
}

impl LocalCapabilitySource {
    pub fn package(&self) -> &PackageRef {
        &self.package
    }

    pub fn capability(&self) -> &CapabilityRef {
        &self.capability
    }

    pub fn kind(&self) -> CapabilityKind {
        self.kind
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn runtime(&self) -> Option<&str> {
        self.runtime.as_deref()
    }

    pub fn language_ids(&self) -> &[String] {
        &self.language_ids
    }

    /// Returns the verified immutable artifact root for trusted in-process adapters.
    pub fn package_root(&self) -> &std::path::Path {
        &self.package_root
    }

    /// Returns this capability's signed package-relative path.
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn host_path(&self) -> &std::path::Path {
        &self.host_path
    }
}

#[derive(Default)]
struct RuntimeState {
    durable: DurableState,
    leases: BTreeMap<String, LeaseRecord>,
    generation: u64,
    subscribers: Vec<mpsc::Sender<u64>>,
}

struct LeaseRecord {
    lease: CapabilityLease,
}

impl MarketplaceManager {
    /// Opens a local Manager over one profile-owned state root and remote registry port.
    pub fn open(
        state_root: impl Into<PathBuf>,
        registry: Arc<dyn MarketplaceRegistryClient>,
    ) -> Result<Self, MarketplaceClientError> {
        let store = Store::open(state_root.into())?;
        let mut durable: DurableState = store.read_state()?;
        durable
            .installations
            .retain(|_, installation| installation.state == InstallationState::Installed);
        for installation in durable.installations.values_mut() {
            for capability in &mut installation.capabilities {
                capability.descriptor.reference = capability_reference(
                    &installation.installation_id,
                    capability.descriptor.kind,
                    &capability.descriptor.id,
                );
            }
        }
        store.write_state(&durable)?;
        let session_nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MarketplaceClientError::storage())?
            .as_nanos()
            .to_string();
        Ok(Self {
            registry,
            store,
            runtime: Mutex::new(RuntimeState {
                durable,
                leases: BTreeMap::new(),
                generation: 1,
                subscribers: Vec::new(),
            }),
            lease_sequence: AtomicU64::new(1),
            session_nonce,
        })
    }

    /// Subscribes to committed installation-state changes for local domain adapters.
    pub fn subscribe(&self) -> Result<mpsc::Receiver<u64>, MarketplaceClientError> {
        let (sender, receiver) = mpsc::channel();
        self.lock_runtime()?.subscribers.push(sender);
        Ok(receiver)
    }

    /// Returns the current process-local installation generation.
    pub fn generation(&self) -> Result<u64, MarketplaceClientError> {
        Ok(self.lock_runtime()?.generation)
    }

    /// Identifies this process-local change stream for profile-level notification deduplication.
    pub fn change_source_id(&self) -> &str {
        &self.session_nonce
    }

    /// Returns verified sources of one capability kind for trusted local runtime composition.
    ///
    /// The ordinary Marketplace service contract intentionally exposes no filesystem paths. This
    /// local-only adapter revalidates the complete immutable artifact before returning a host
    /// handle and must not be used by transports or Renderer code.
    pub fn local_capability_sources(
        &self,
        kind: CapabilityKind,
    ) -> Result<Vec<LocalCapabilitySource>, MarketplaceClientError> {
        let durable = self
            .runtime
            .lock()
            .map_err(|_| MarketplaceClientError::storage())?
            .durable
            .clone();
        let mut sources = Vec::new();
        for installation in durable
            .installations
            .values()
            .filter(|installation| installation.state == InstallationState::Installed)
        {
            for capability in installation
                .capabilities
                .iter()
                .filter(|capability| capability.descriptor.kind == kind)
            {
                sources.push(LocalCapabilitySource {
                    package: installation.package.clone(),
                    capability: capability.descriptor.reference.clone(),
                    kind,
                    id: capability.descriptor.id.clone(),
                    runtime: capability.runtime.clone(),
                    language_ids: capability.language_ids.clone(),
                    package_root: self.store.verified_package_root(&installation.package)?,
                    relative_path: capability.path.clone(),
                    host_path: self
                        .store
                        .verified_package_path(&installation.package, &capability.path)?,
                });
            }
        }
        sources.sort_by(|left, right| {
            (
                &left.package.id,
                &left.package.version,
                &left.id,
                &left.capability.id,
            )
                .cmp(&(
                    &right.package.id,
                    &right.package.version,
                    &right.id,
                    &right.capability.id,
                ))
        });
        Ok(sources)
    }

    fn install_downloaded(
        &self,
        downloaded: &dyn MarketplacePackagePayload,
    ) -> Result<InstalledPackage, MarketplaceClientError> {
        let artifact = self.store.materialize(downloaded)?;
        let installation_id = opaque_id(
            "ins",
            &[
                &artifact.package.id,
                &artifact.package.version,
                &artifact.package.digest,
            ],
        );
        let capabilities = downloaded
            .capabilities()
            .iter()
            .map(|capability| CapabilityRecord {
                descriptor: CapabilityDescriptor {
                    reference: capability_reference(
                        &installation_id,
                        capability.kind,
                        &capability.id,
                    ),
                    kind: capability.kind,
                    id: capability.id.clone(),
                    contract_version: "1".into(),
                    permissions: Vec::new(),
                    authentication_provider: None,
                },
                path: capability.path.clone(),
                runtime: capability.runtime.clone(),
                language_ids: capability.language_ids.clone(),
            })
            .collect::<Vec<_>>();
        let mut installation = InstallationRecord {
            installation_id: installation_id.clone(),
            package: artifact.package,
            state: InstallationState::Installed,
            capabilities,
        };
        for capability in &mut installation.capabilities {
            capability.descriptor =
                activation::descriptor(&self.store, &installation.package, capability)?;
        }
        let mut runtime = self.lock_runtime()?;
        let installation = runtime
            .durable
            .installations
            .entry(installation_id)
            .or_insert(installation)
            .clone();
        self.store.write_state(&runtime.durable)?;
        publish_change(&mut runtime)?;
        Ok(installation.public())
    }

    fn lock_runtime(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, RuntimeState>, MarketplaceClientError> {
        self.runtime.lock().map_err(|_| {
            MarketplaceClientError::business(
                MarketplaceErrorCode::ServiceUnavailable,
                "Marketplace Manager is unavailable",
                true,
            )
        })
    }
}

impl MarketplaceServiceClient for MarketplaceManager {
    fn search(
        &self,
        request: SearchPackagesRequest,
    ) -> Result<SearchPackagesResult, MarketplaceClientError> {
        self.registry.search(request)
    }

    fn get(&self, request: GetPackageRequest) -> Result<PackageDetails, MarketplaceClientError> {
        self.registry.get(request)
    }

    fn download(
        &self,
        request: DownloadPackageRequest,
    ) -> Result<ArtifactHandle, MarketplaceClientError> {
        let downloaded = self.registry.download(request)?;
        self.store.materialize(downloaded.as_ref())
    }

    fn install(
        &self,
        request: InstallPackageRequest,
    ) -> Result<InstalledPackage, MarketplaceClientError> {
        let downloaded = self.registry.download(DownloadPackageRequest {
            package_id: request.package_id,
            version: request.version,
        })?;
        self.install_downloaded(downloaded.as_ref())
    }

    fn update(
        &self,
        request: UpdatePackageRequest,
    ) -> Result<InstalledPackage, MarketplaceClientError> {
        let package_id = {
            let runtime = self.lock_runtime()?;
            runtime
                .durable
                .installations
                .get(&request.installation_id)
                .ok_or_else(installation_not_found)?
                .package
                .id
                .clone()
        };
        let downloaded = self.registry.download(DownloadPackageRequest {
            package_id,
            version: request.version,
        })?;
        let installed = self.install_downloaded(downloaded.as_ref())?;
        if installed.installation_id != request.installation_id {
            self.uninstall(UninstallPackageRequest {
                installation_id: request.installation_id,
                mode: UninstallMode::WhenUnused,
            })?;
        }
        Ok(installed)
    }

    fn uninstall(&self, request: UninstallPackageRequest) -> Result<(), MarketplaceClientError> {
        let mut runtime = self.lock_runtime()?;
        if !runtime
            .durable
            .installations
            .contains_key(&request.installation_id)
        {
            return Err(installation_not_found());
        }
        let in_use = runtime
            .leases
            .values()
            .any(|lease| lease.lease.installation_id == request.installation_id);
        match (request.mode, in_use) {
            (UninstallMode::IfUnused, true) => return Err(installation_in_use()),
            (UninstallMode::WhenUnused, true) => {
                runtime
                    .durable
                    .installations
                    .get_mut(&request.installation_id)
                    .expect("installation existence was checked")
                    .state = InstallationState::PendingRemoval;
            }
            (UninstallMode::IfUnused | UninstallMode::WhenUnused, false) => {
                runtime
                    .durable
                    .installations
                    .remove(&request.installation_id);
            }
        }
        self.store.write_state(&runtime.durable)?;
        publish_change(&mut runtime)
    }

    fn list_installed(
        &self,
        _: ListInstalledRequest,
    ) -> Result<Vec<InstalledPackage>, MarketplaceClientError> {
        let runtime = self.lock_runtime()?;
        Ok(runtime
            .durable
            .installations
            .values()
            .map(InstallationRecord::public)
            .collect())
    }

    fn acquire_capability(
        &self,
        request: AcquireCapabilityRequest,
    ) -> Result<AcquiredCapability, MarketplaceClientError> {
        let mut runtime = self.lock_runtime()?;
        let (installation, capability) = find_capability(&runtime.durable, &request.capability)?;
        let installation = installation.clone();
        let capability = capability.clone();
        if installation.state == InstallationState::PendingRemoval {
            return Err(installation_in_use());
        }
        let spec = activation::acquire_spec(&self.store, &installation, &capability)?;
        let sequence = self
            .lease_sequence
            .fetch_add(1, Ordering::Relaxed)
            .to_string();
        let lease = CapabilityLease {
            id: opaque_id(
                "lease",
                &[&self.session_nonce, &sequence, &request.capability.id],
            ),
            capability: request.capability,
            installation_id: installation.installation_id,
        };
        runtime.leases.insert(
            lease.id.clone(),
            LeaseRecord {
                lease: lease.clone(),
            },
        );
        Ok(AcquiredCapability { lease, spec })
    }

    fn release_capability(
        &self,
        request: ReleaseCapabilityRequest,
    ) -> Result<zeta_marketplace_client::ReleaseCapabilityOutcome, MarketplaceClientError> {
        let mut runtime = self.lock_runtime()?;
        let lease = runtime
            .leases
            .remove(&request.lease_id)
            .ok_or_else(lease_not_found)?;
        let installation_id = lease.lease.installation_id;
        let still_in_use = runtime
            .leases
            .values()
            .any(|candidate| candidate.lease.installation_id == installation_id);
        let installation_changed = !still_in_use
            && runtime
                .durable
                .installations
                .get(&installation_id)
                .is_some_and(|installation| {
                    installation.state == InstallationState::PendingRemoval
                });
        if installation_changed {
            runtime.durable.installations.remove(&installation_id);
            self.store.write_state(&runtime.durable)?;
            publish_change(&mut runtime)?;
        }
        Ok(zeta_marketplace_client::ReleaseCapabilityOutcome {
            installation_changed,
        })
    }

    fn open_resource(
        &self,
        request: OpenResourceRequest,
    ) -> Result<ResourceContent, MarketplaceClientError> {
        let runtime = self.lock_runtime()?;
        let lease = runtime
            .leases
            .get(&request.lease_id)
            .ok_or_else(lease_not_found)?;
        let installation = runtime
            .durable
            .installations
            .get(&lease.lease.installation_id)
            .ok_or_else(installation_not_found)?;
        let capability = installation
            .capabilities
            .iter()
            .find(|capability| capability.descriptor.reference == lease.lease.capability)
            .ok_or_else(capability_not_found)?;
        activation::open_resource(&self.store, installation, capability, &request.resource)
    }
}

impl InstallationRecord {
    fn public(&self) -> InstalledPackage {
        InstalledPackage {
            installation_id: self.installation_id.clone(),
            package: self.package.clone(),
            state: self.state,
            capabilities: self
                .capabilities
                .iter()
                .map(|capability| capability.descriptor.clone())
                .collect(),
        }
    }
}

fn find_capability<'a>(
    state: &'a DurableState,
    reference: &CapabilityRef,
) -> Result<(&'a InstallationRecord, &'a CapabilityRecord), MarketplaceClientError> {
    state
        .installations
        .values()
        .find_map(|installation| {
            installation
                .capabilities
                .iter()
                .find(|capability| capability.descriptor.reference == *reference)
                .map(|capability| (installation, capability))
        })
        .ok_or_else(capability_not_found)
}

fn capability_reference(
    installation_id: &str,
    kind: CapabilityKind,
    capability_id: &str,
) -> CapabilityRef {
    CapabilityRef {
        id: opaque_id(
            "cap",
            &[installation_id, capability_kind_tag(kind), capability_id],
        ),
    }
}

fn capability_kind_tag(kind: CapabilityKind) -> &'static str {
    match kind {
        CapabilityKind::Skill => "skill",
        CapabilityKind::Mcp => "mcp",
        CapabilityKind::Connector => "connector",
        CapabilityKind::Theme => "theme",
        CapabilityKind::Language => "language",
        CapabilityKind::Localization => "localization",
        CapabilityKind::Executable => "executable",
        CapabilityKind::Asset => "asset",
    }
}

fn installation_not_found() -> MarketplaceClientError {
    MarketplaceClientError::business(
        MarketplaceErrorCode::InstallationNotFound,
        "Marketplace installation was not found",
        false,
    )
}

fn publish_change(runtime: &mut RuntimeState) -> Result<(), MarketplaceClientError> {
    runtime.generation = runtime
        .generation
        .checked_add(1)
        .ok_or_else(MarketplaceClientError::storage)?;
    let generation = runtime.generation;
    runtime
        .subscribers
        .retain(|subscriber| subscriber.send(generation).is_ok());
    Ok(())
}

fn installation_in_use() -> MarketplaceClientError {
    MarketplaceClientError::business(
        MarketplaceErrorCode::InstallationInUse,
        "Marketplace installation is in use",
        false,
    )
}

fn capability_not_found() -> MarketplaceClientError {
    MarketplaceClientError::business(
        MarketplaceErrorCode::CapabilityNotFound,
        "Marketplace capability was not found",
        false,
    )
}

fn lease_not_found() -> MarketplaceClientError {
    MarketplaceClientError::business(
        MarketplaceErrorCode::LeaseNotFound,
        "Marketplace capability lease was not found",
        false,
    )
}
