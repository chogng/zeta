use std::sync::Arc;
use std::sync::Mutex;

use zeta_extensions::DynamicExtensionPackageSource;
use zeta_extensions::DynamicExtensionSourceProvider;
use zeta_extensions::DynamicExtensionSourceSnapshot;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_manager::MarketplaceManager;

/// Projects installed Marketplace Language assets into the declarative Extension catalog.
pub(super) struct MarketplaceExtensionSourceProvider {
    manager: Arc<MarketplaceManager>,
    state: Mutex<ProjectionState>,
}

#[derive(Default)]
struct ProjectionState {
    fingerprint: Vec<String>,
    generation: u64,
}

impl MarketplaceExtensionSourceProvider {
    pub(super) fn new(manager: Arc<MarketplaceManager>) -> Self {
        Self {
            manager,
            state: Mutex::new(ProjectionState::default()),
        }
    }
}

impl DynamicExtensionSourceProvider for MarketplaceExtensionSourceProvider {
    fn snapshot(&self) -> Result<DynamicExtensionSourceSnapshot, String> {
        let sources = self
            .manager
            .local_capability_sources(CapabilityKind::Language)
            .map_err(|error| error.to_string())?;
        let fingerprint = sources
            .iter()
            .map(|source| {
                format!(
                    "{}\0{}\0{}\0{}",
                    source.package().id,
                    source.package().version,
                    source.package().digest,
                    source.capability().id
                )
            })
            .collect::<Vec<_>>();
        let generation = next_generation(&self.state, fingerprint)?;
        let packages = sources
            .into_iter()
            .map(|source| {
                DynamicExtensionPackageSource::marketplace(
                    format!("{}:{}", source.package().id, source.id()),
                    source.host_path(),
                )
            })
            .collect();
        Ok(DynamicExtensionSourceSnapshot {
            generation,
            packages,
        })
    }
}

/// Keeps Plugin and Marketplace declarative Extension authorities independent and composable.
pub(super) struct CombinedExtensionSourceProvider {
    providers: Vec<Arc<dyn DynamicExtensionSourceProvider>>,
    state: Mutex<ProjectionState>,
}

impl CombinedExtensionSourceProvider {
    pub(super) fn new(providers: Vec<Arc<dyn DynamicExtensionSourceProvider>>) -> Self {
        Self {
            providers,
            state: Mutex::new(ProjectionState::default()),
        }
    }
}

impl DynamicExtensionSourceProvider for CombinedExtensionSourceProvider {
    fn snapshot(&self) -> Result<DynamicExtensionSourceSnapshot, String> {
        let mut packages = Vec::new();
        let mut fingerprint = Vec::new();
        for provider in &self.providers {
            let snapshot = provider.snapshot()?;
            fingerprint.push(snapshot.generation.to_string());
            for package in snapshot.packages {
                fingerprint.push(format!("{}\0{}", package.subject, package.path.display()));
                packages.push(package);
            }
        }
        let generation = next_generation(&self.state, fingerprint)?;
        Ok(DynamicExtensionSourceSnapshot {
            generation,
            packages,
        })
    }
}

fn next_generation(
    state: &Mutex<ProjectionState>,
    fingerprint: Vec<String>,
) -> Result<u64, String> {
    let mut state = state
        .lock()
        .map_err(|_| "Marketplace Extension projection lock poisoned".to_string())?;
    if state.fingerprint != fingerprint {
        state.fingerprint = fingerprint;
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or_else(|| "Marketplace Extension projection generation exhausted".to_string())?;
    }
    Ok(state.generation.max(1))
}
