use std::collections::BTreeSet;
use std::fmt;

use zeta_plugins::PluginId;
use zeta_plugins::PluginManifest;
use zeta_tools::CapabilityDiscoveryId;
use zeta_tools::CapabilityDiscoverySnapshot;
use zeta_tools::DiscoverableCapability;
use zeta_tools::DiscoverableConnectorInfo;
use zeta_tools::DiscoveryAction;
use zeta_tools::DiscoveryValueError;

use crate::ConnectorBinding;
use crate::ConnectorConnectionState;
use crate::ConnectorId;
use crate::ConnectorIdentityError;

/// One connector declaration plus its independent account connection state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorEntry {
    pub id: ConnectorId,
    pub display_name: String,
    pub description: String,
    pub provider_plugin: PluginId,
    pub binding: ConnectorBinding,
    pub state: ConnectorConnectionState,
}

/// Immutable generation-bound connector catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorCatalog {
    generation: u64,
    entries: Vec<ConnectorEntry>,
}

impl ConnectorCatalog {
    /// Projects validated Plugin connector metadata without connecting accounts or starting MCP.
    pub fn from_manifests<'a>(
        generation: u64,
        manifests: impl IntoIterator<Item = &'a PluginManifest>,
    ) -> Result<Self, ConnectorCatalogError> {
        let mut entries = manifests
            .into_iter()
            .flat_map(|manifest| {
                manifest.contributions.connectors.iter().map(|connector| {
                    let id =
                        ConnectorId::new(format!("{}:connector:{}", manifest.id, connector.id));
                    id.map(|id| ConnectorEntry {
                        id,
                        display_name: connector.display_name.clone(),
                        description: connector.description.clone(),
                        provider_plugin: manifest.id.clone(),
                        binding: ConnectorBinding::McpServer {
                            server_id: format!(
                                "plugin:{}:mcp:{}",
                                manifest.id, connector.mcp_server
                            ),
                        },
                        state: ConnectorConnectionState::Disconnected,
                    })
                })
            })
            .collect::<Result<Vec<_>, ConnectorIdentityError>>()?;
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        if entries
            .windows(2)
            .any(|window| window[0].id == window[1].id)
        {
            return Err(ConnectorCatalogError("duplicate connector identity".into()));
        }
        Ok(Self {
            generation,
            entries,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn entries(&self) -> &[ConnectorEntry] {
        &self.entries
    }

    /// Applies a host-authoritative connection state snapshot to one known connector.
    pub fn with_state(
        mut self,
        id: &ConnectorId,
        state: ConnectorConnectionState,
    ) -> Result<Self, ConnectorCatalogError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == *id)
            .ok_or_else(|| ConnectorCatalogError("connector is not declared".into()))?;
        entry.state = state;
        Ok(self)
    }

    /// Returns MCP server identities whose connector account authority is currently ready.
    pub fn ready_mcp_server_ids(&self) -> BTreeSet<&str> {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.state, ConnectorConnectionState::Connected(_)))
            .map(|entry| match &entry.binding {
                ConnectorBinding::McpServer { server_id } => server_id.as_str(),
            })
            .collect()
    }

    /// Projects only disconnected connectors into catalog discovery; ready connectors are tools.
    pub fn discovery_snapshot(&self) -> Result<CapabilityDiscoverySnapshot, DiscoveryValueError> {
        let candidates = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.state, ConnectorConnectionState::Disconnected))
            .map(|entry| {
                DiscoverableCapability::Connector(DiscoverableConnectorInfo {
                    id: CapabilityDiscoveryId::new(entry.id.to_string())
                        .expect("validated connector ID is non-empty"),
                    display_name: entry.display_name.clone(),
                    description: entry.description.clone(),
                    action: DiscoveryAction::Connect,
                })
            })
            .collect();
        CapabilityDiscoverySnapshot::new(self.generation, candidates)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorCatalogError(String);

impl From<ConnectorIdentityError> for ConnectorCatalogError {
    fn from(error: ConnectorIdentityError) -> Self {
        Self(error.to_string())
    }
}

impl fmt::Display for ConnectorCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConnectorCatalogError {}
