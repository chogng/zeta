use std::collections::BTreeMap;
use std::sync::Arc;

use zeta_app_server_protocol::protocol::language::LanguageMarketplaceCompatibilityDto;
use zeta_app_server_protocol::protocol::language::LanguageMarketplaceEntryDto;
use zeta_app_server_protocol::protocol::language::LanguageMarketplaceInstallParams;
use zeta_app_server_protocol::protocol::language::LanguageMarketplaceListResult;
use zeta_language_marketplace::LanguageMarketplaceCompatibility;
use zeta_language_marketplace::LanguageMarketplaceEntry;
use zeta_language_marketplace::LanguageMarketplaceErrorKind;
use zeta_language_marketplace::LanguageMarketplaceRuntime;
use zeta_language_marketplace::RemoteLanguageMarketplace;
use zeta_language_marketplace::RemoteLanguageMarketplaceSnapshot;
use zeta_language_server_catalog::CSS_LANGUAGE_SERVER_ID;
use zeta_language_server_catalog::LanguageServerProviderRegistry;
use zeta_language_server_catalog::ManagedNodeRuntime;
use zeta_language_server_distribution::LanguageServerActivationAuthority;

pub(crate) struct AppServerLanguageMarketplaceRuntime {
    authority: LanguageServerActivationAuthority,
    node: Option<ManagedNodeRuntime>,
    base_providers: LanguageServerProviderRegistry,
    marketplaces: BTreeMap<String, Arc<RemoteLanguageMarketplace>>,
    entries: Vec<LanguageMarketplaceEntry>,
    catalog_revision: String,
}

impl AppServerLanguageMarketplaceRuntime {
    pub(crate) fn new(
        authority: LanguageServerActivationAuthority,
        node: Option<ManagedNodeRuntime>,
        base_providers: LanguageServerProviderRegistry,
        sources: Vec<(
            Arc<RemoteLanguageMarketplace>,
            RemoteLanguageMarketplaceSnapshot,
        )>,
    ) -> Self {
        let mut marketplaces = BTreeMap::new();
        let mut entries = Vec::new();
        let mut revisions = Vec::new();
        for (marketplace, snapshot) in sources {
            let id = marketplace.id().as_str().to_owned();
            revisions.push(format!("{id}:{}", snapshot.targets_version()));
            entries.extend(snapshot.entries().iter().cloned());
            marketplaces.insert(id, marketplace);
        }
        entries.sort_by(|left, right| {
            (
                left.display_name(),
                left.package_id().as_str(),
                left.version().to_string(),
                left.server_id(),
            )
                .cmp(&(
                    right.display_name(),
                    right.package_id().as_str(),
                    right.version().to_string(),
                    right.server_id(),
                ))
        });
        revisions.sort();
        Self {
            authority,
            node,
            base_providers,
            marketplaces,
            entries,
            catalog_revision: if revisions.is_empty() {
                "none".into()
            } else {
                revisions.join(",")
            },
        }
    }

    pub(crate) fn registry(
        &self,
    ) -> Result<LanguageServerProviderRegistry, LanguageMarketplaceRuntimeError> {
        let snapshot = self
            .authority
            .snapshot()
            .map_err(|_| LanguageMarketplaceRuntimeError::OperationFailed)?;
        let mut registry = self.base_providers.clone();
        if snapshot.servers().is_empty() {
            return Ok(registry);
        }
        let node = self
            .node
            .clone()
            .ok_or(LanguageMarketplaceRuntimeError::Incompatible)?;
        registry
            .merge(
                LanguageServerProviderRegistry::from_activation(&snapshot, node)
                    .map_err(|_| LanguageMarketplaceRuntimeError::OperationFailed)?,
            )
            .map_err(|_| LanguageMarketplaceRuntimeError::OperationFailed)?;
        Ok(registry)
    }

    pub(super) fn list(
        &self,
    ) -> Result<LanguageMarketplaceListResult, LanguageMarketplaceRuntimeError> {
        let activation = self
            .authority
            .snapshot()
            .map_err(|_| LanguageMarketplaceRuntimeError::OperationFailed)?;
        let entries = self
            .entries
            .iter()
            .map(|entry| {
                let active = activation.servers().iter().any(|installed| {
                    installed.server_id() == entry.server_id()
                        && installed.version() == entry.version().to_string()
                });
                LanguageMarketplaceEntryDto {
                    marketplace_id: entry.marketplace_id().as_str().to_owned(),
                    package_id: entry.package_id().as_str().to_owned(),
                    version: entry.version().to_string(),
                    digest: entry.digest().as_str().to_owned(),
                    display_name: entry.display_name().to_owned(),
                    description: entry.description().to_owned(),
                    license: entry.license().to_owned(),
                    server_id: entry.server_id().to_owned(),
                    languages: entry.languages().to_vec(),
                    file_extensions: entry.file_extensions().to_vec(),
                    compatibility: compatibility_dto(self.compatibility(entry)),
                    installed: active,
                    active,
                }
            })
            .collect();
        Ok(LanguageMarketplaceListResult {
            catalog_revision: self.catalog_revision.clone(),
            activation_generation: activation.generation(),
            entries,
        })
    }

    pub(super) fn install(
        &self,
        params: &LanguageMarketplaceInstallParams,
    ) -> Result<(u64, LanguageServerProviderRegistry), LanguageMarketplaceRuntimeError> {
        if params.expected_catalog_revision != self.catalog_revision {
            return Err(LanguageMarketplaceRuntimeError::RevisionConflict);
        }
        let entry = self
            .entries
            .iter()
            .find(|entry| {
                entry.marketplace_id().as_str() == params.marketplace_id
                    && entry.package_id().as_str() == params.package_id
                    && entry.version().to_string() == params.version
                    && entry.digest().as_str() == params.digest
                    && entry.server_id() == params.server_id
            })
            .ok_or(LanguageMarketplaceRuntimeError::NotFound)?;
        if !self.compatibility(entry).is_compatible() {
            return Err(LanguageMarketplaceRuntimeError::Incompatible);
        }
        let marketplace = self
            .marketplaces
            .get(&params.marketplace_id)
            .ok_or(LanguageMarketplaceRuntimeError::NotFound)?;
        let activation =
            marketplace
                .install(entry, &self.authority)
                .map_err(|error| match error.kind() {
                    LanguageMarketplaceErrorKind::Incompatible => {
                        LanguageMarketplaceRuntimeError::Incompatible
                    }
                    LanguageMarketplaceErrorKind::InvalidConfiguration
                    | LanguageMarketplaceErrorKind::MetadataUntrusted
                    | LanguageMarketplaceErrorKind::DistributionUnavailable
                    | LanguageMarketplaceErrorKind::PackageUnsafe
                    | LanguageMarketplaceErrorKind::CacheUnavailable
                    | LanguageMarketplaceErrorKind::ActivationUnavailable => {
                        LanguageMarketplaceRuntimeError::OperationFailed
                    }
                })?;
        let registry = self.registry()?;
        Ok((activation.generation(), registry))
    }

    fn compatibility(&self, entry: &LanguageMarketplaceEntry) -> LanguageMarketplaceCompatibility {
        if let LanguageMarketplaceCompatibility::Incompatible(reason) = entry.compatibility() {
            return LanguageMarketplaceCompatibility::Incompatible(reason.clone());
        }
        if entry.server_id() != CSS_LANGUAGE_SERVER_ID {
            return LanguageMarketplaceCompatibility::Incompatible(format!(
                "server '{}' is not supported by this build",
                entry.server_id()
            ));
        }
        match entry.runtime() {
            LanguageMarketplaceRuntime::Node | LanguageMarketplaceRuntime::LegacyUnspecified
                if self.node.is_some() =>
            {
                LanguageMarketplaceCompatibility::Compatible
            }
            LanguageMarketplaceRuntime::Node | LanguageMarketplaceRuntime::LegacyUnspecified => {
                LanguageMarketplaceCompatibility::Incompatible(
                    "the shared Node-compatible runtime is unavailable".into(),
                )
            }
            LanguageMarketplaceRuntime::Direct => LanguageMarketplaceCompatibility::Incompatible(
                "this server requires a native provider adapter not present in this build".into(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LanguageMarketplaceRuntimeError {
    RevisionConflict,
    NotFound,
    Incompatible,
    OperationFailed,
}

fn compatibility_dto(
    compatibility: LanguageMarketplaceCompatibility,
) -> LanguageMarketplaceCompatibilityDto {
    match compatibility {
        LanguageMarketplaceCompatibility::Compatible => {
            LanguageMarketplaceCompatibilityDto::Compatible
        }
        LanguageMarketplaceCompatibility::Incompatible(reason) => {
            LanguageMarketplaceCompatibilityDto::Incompatible { reason }
        }
    }
}
