use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use zeta_connectors::ConnectorConnectionState;
use zeta_connectors::ConnectorConnectionUpdate;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorError;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorRuntimeBinding;
use zeta_connectors::ConnectorSnapshot;
use zeta_connectors::ConnectorSnapshotGeneration;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginId;
use zeta_plugins::PluginManifest;
use zeta_tools::CapabilityDiscoveryId;
use zeta_tools::CapabilityDiscoverySnapshot;
use zeta_tools::DiscoverableCapability;
use zeta_tools::DiscoverableConnectorInfo;
use zeta_tools::DiscoveryAction;
use zeta_tools::DiscoveryValueError;

/// Plugin-projected Connector definitions plus their backend-neutral domain snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorCatalog {
    snapshot: ConnectorSnapshot,
    provider_plugins: BTreeMap<ConnectorId, PluginId>,
}

impl ConnectorCatalog {
    /// Projects validated Plugin connector declarations without connecting accounts or starting MCP.
    pub fn from_manifests<'a>(
        generation: ConnectorSnapshotGeneration,
        manifests: impl IntoIterator<Item = &'a PluginManifest>,
    ) -> Result<Self, ConnectorCatalogError> {
        Self::from_manifest_revisions(
            generation,
            manifests.into_iter().map(|manifest| {
                (
                    manifest,
                    format!("manifest:{}@{}", manifest.id, manifest.version),
                )
            }),
        )
    }

    /// Projects exact validated Plugin packages and binds authorization to package content.
    pub fn from_packages<'a>(
        generation: ConnectorSnapshotGeneration,
        packages: impl IntoIterator<Item = &'a LocalPluginPackage>,
    ) -> Result<Self, ConnectorCatalogError> {
        Self::from_manifest_revisions(
            generation,
            packages.into_iter().map(|package| {
                (
                    package.manifest(),
                    package.package_digest().as_str().to_string(),
                )
            }),
        )
    }

    fn from_manifest_revisions<'a>(
        generation: ConnectorSnapshotGeneration,
        sources: impl IntoIterator<Item = (&'a PluginManifest, String)>,
    ) -> Result<Self, ConnectorCatalogError> {
        let mut definitions = Vec::new();
        let mut provider_plugins = BTreeMap::new();
        for (manifest, authorization_revision) in sources {
            for connector in &manifest.contributions.connectors {
                let id = ConnectorId::new(format!("{}:connector:{}", manifest.id, connector.id))?;
                let definition = ConnectorDefinition::new(
                    id.clone(),
                    connector.display_name.clone(),
                    connector.description.clone(),
                    ConnectorRuntimeBinding::mcp_server(format!(
                        "plugin:{}:mcp:{}",
                        manifest.id, connector.mcp_server
                    ))?,
                )?
                .with_authorization_revision(authorization_revision.clone())?;
                definitions.push(definition);
                if provider_plugins.insert(id, manifest.id.clone()).is_some() {
                    return Err(ConnectorCatalogError(
                        "duplicate Plugin connector identity".into(),
                    ));
                }
            }
        }
        Ok(Self {
            snapshot: ConnectorSnapshot::new(generation, definitions)?,
            provider_plugins,
        })
    }

    pub fn snapshot(&self) -> &ConnectorSnapshot {
        &self.snapshot
    }

    pub fn provider_plugin(&self, id: &ConnectorId) -> Option<&PluginId> {
        self.provider_plugins.get(id)
    }

    /// Applies a host-authoritative connection update to the backend-neutral snapshot.
    pub fn with_connection_update(
        mut self,
        generation: ConnectorSnapshotGeneration,
        id: &ConnectorId,
        update: ConnectorConnectionUpdate,
    ) -> Result<Self, ConnectorCatalogError> {
        self.snapshot = self
            .snapshot
            .with_connection_update(generation, id, update)?;
        Ok(self)
    }

    /// Returns MCP declaration identities whose Connector account is currently ready.
    pub fn ready_mcp_server_ids(&self) -> BTreeSet<&str> {
        self.snapshot
            .ready_entries()
            .map(|entry| entry.definition().runtime_binding().mcp_server_id())
            .collect()
    }

    /// Projects disconnected Connectors into catalog discovery without creating executable tools.
    pub fn discovery_snapshot(&self) -> Result<CapabilityDiscoverySnapshot, DiscoveryValueError> {
        let candidates = self
            .snapshot
            .entries()
            .iter()
            .filter(|entry| {
                matches!(
                    entry.connection().state(),
                    ConnectorConnectionState::Disconnected
                        | ConnectorConnectionState::ReauthorizationRequired { .. }
                )
            })
            .map(|entry| {
                let definition = entry.definition();
                DiscoverableCapability::Connector(DiscoverableConnectorInfo {
                    id: CapabilityDiscoveryId::new(definition.id().to_string())
                        .expect("validated connector ID is non-empty"),
                    display_name: definition.display_name().to_string(),
                    description: definition.description().to_string(),
                    action: DiscoveryAction::Connect,
                })
            })
            .collect();
        CapabilityDiscoverySnapshot::new(self.snapshot.generation().get(), candidates)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorCatalogError(String);

impl From<ConnectorError> for ConnectorCatalogError {
    fn from(error: ConnectorError) -> Self {
        Self(error.to_string())
    }
}

impl fmt::Display for ConnectorCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConnectorCatalogError {}
