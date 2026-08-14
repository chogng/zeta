use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use zeta_config::McpServerId;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorRuntimeBinding;
use zeta_marketplace_client::AcquireCapabilityRequest;
use zeta_marketplace_client::ActivationSpec;
use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::CapabilityRef;
use zeta_marketplace_client::InstallationState;
use zeta_marketplace_client::ListInstalledRequest;
use zeta_marketplace_client::MarketplaceServiceClient;
use zeta_marketplace_client::ReleaseCapabilityRequest;
use zeta_marketplace_manager::LocalCapabilitySource;
use zeta_marketplace_manager::MarketplaceManager;
use zeta_mcp::McpServerDefinition;
use zeta_mcp::McpServerTransport;
use zeta_mcp_extension::ConnectorMcpRuntimeError;
use zeta_mcp_extension::ConnectorMcpRuntimeProvider;
use zeta_mcp_extension::RuntimeInvocationFence;
use zeta_mcp_extension::RuntimeInvocationLease;
use zeta_mcp_extension::StandaloneMcpServer;
use zeta_secrets::SecretValue;

use self::contract::MarketplaceMcpTransport;
use self::contract::parse_connector;
use self::contract::parse_transport;

mod contract;

pub(crate) struct MarketplaceConnectorProjection {
    definitions: Vec<ConnectorDefinition>,
    provider: Arc<MarketplaceConnectorMcpRuntimeProvider>,
}

pub(crate) fn combined_provider(
    base: Arc<dyn ConnectorMcpRuntimeProvider>,
    marketplace: Arc<dyn ConnectorMcpRuntimeProvider>,
) -> Arc<dyn ConnectorMcpRuntimeProvider> {
    Arc::new(CombinedConnectorMcpRuntimeProvider { base, marketplace })
}

struct CombinedConnectorMcpRuntimeProvider {
    base: Arc<dyn ConnectorMcpRuntimeProvider>,
    marketplace: Arc<dyn ConnectorMcpRuntimeProvider>,
}

impl ConnectorMcpRuntimeProvider for CombinedConnectorMcpRuntimeProvider {
    fn materialize(
        &self,
        connector: &ConnectorDefinition,
        credential: SecretValue,
    ) -> Result<McpServerTransport, ConnectorMcpRuntimeError> {
        if connector.id().as_str().starts_with("marketplace:") {
            self.marketplace.materialize(connector, credential)
        } else {
            self.base.materialize(connector, credential)
        }
    }

    fn standalone_servers(&self) -> Result<Vec<StandaloneMcpServer>, ConnectorMcpRuntimeError> {
        let mut servers = self.base.standalone_servers()?;
        servers.extend(self.marketplace.standalone_servers()?);
        Ok(servers)
    }

    fn invocation_fence(
        &self,
        connector: &ConnectorDefinition,
    ) -> Option<Arc<dyn RuntimeInvocationFence>> {
        if connector.id().as_str().starts_with("marketplace:") {
            self.marketplace.invocation_fence(connector)
        } else {
            self.base.invocation_fence(connector)
        }
    }
}

impl MarketplaceConnectorProjection {
    pub(crate) fn from_manager(manager: Arc<MarketplaceManager>) -> Result<Self, String> {
        let mcp_sources = manager
            .local_capability_sources(CapabilityKind::Mcp)
            .map_err(|error| error.to_string())?;
        let connector_sources = manager
            .local_capability_sources(CapabilityKind::Connector)
            .map_err(|error| error.to_string())?;
        let mut servers = BTreeMap::new();
        let mut local_servers = BTreeMap::new();
        for source in mcp_sources {
            let id = mcp_server_id(&source)?;
            let transport = parse_transport(&source)?;
            let key = (source.package().digest.clone(), source.id().to_string());
            if local_servers.insert(key, id.clone()).is_some()
                || servers
                    .insert(
                        id,
                        MarketplaceMcpServer {
                            display_name: format!("{}: {}", source.package().id, source.id()),
                            transport,
                            capability: source.capability().clone(),
                        },
                    )
                    .is_some()
            {
                return Err("duplicate Marketplace MCP capability identity".into());
            }
        }

        let mut definitions = Vec::new();
        let mut connector_servers = BTreeMap::new();
        let mut referenced_servers = BTreeSet::new();
        for source in connector_sources {
            let descriptor = parse_connector(&source)?;
            if descriptor.id != source.id() {
                return Err("Marketplace Connector ID does not match its signed capability".into());
            }
            if descriptor
                .authentication
                .as_deref()
                .is_some_and(|kind| kind != "oauth")
            {
                return Err("Marketplace Connector declares unsupported authentication".into());
            }
            let local_mcp = descriptor
                .mcp_server
                .ok_or_else(|| "Marketplace Connector has no MCP capability binding".to_string())?;
            let server_id = local_servers
                .get(&(source.package().digest.clone(), local_mcp))
                .cloned()
                .ok_or_else(|| {
                    "Marketplace Connector references an unavailable MCP capability".to_string()
                })?;
            let server = servers
                .get(&server_id)
                .expect("local server identity came from the server map");
            if matches!(server.transport, MarketplaceMcpTransport::Stdio { .. }) {
                return Err(
                    "Marketplace Connector cannot inject credentials into stdio MCP".into(),
                );
            }
            let connector_id = connector_id(&source)?;
            let description = descriptor
                .description
                .unwrap_or_else(|| descriptor.display_name.clone());
            let definition = ConnectorDefinition::new(
                connector_id.clone(),
                descriptor.display_name,
                description,
                ConnectorRuntimeBinding::mcp_server(server_id.as_str())
                    .map_err(|error| error.to_string())?,
            )
            .and_then(|definition| {
                definition.with_authorization_revision(source.package().digest.clone())
            })
            .map_err(|error| error.to_string())?;
            if connector_servers
                .insert(connector_id, server_id.clone())
                .is_some()
            {
                return Err("duplicate Marketplace Connector identity".into());
            }
            referenced_servers.insert(server_id);
            definitions.push(definition);
        }
        definitions.sort_by(|left, right| left.id().cmp(right.id()));
        Ok(Self {
            definitions,
            provider: Arc::new(MarketplaceConnectorMcpRuntimeProvider {
                manager,
                servers,
                connector_servers,
                referenced_servers,
            }),
        })
    }

    pub(crate) fn definitions(&self) -> &[ConnectorDefinition] {
        &self.definitions
    }

    pub(crate) fn provider(&self) -> Arc<dyn ConnectorMcpRuntimeProvider> {
        self.provider.clone()
    }
}

struct MarketplaceConnectorMcpRuntimeProvider {
    manager: Arc<MarketplaceManager>,
    servers: BTreeMap<McpServerId, MarketplaceMcpServer>,
    connector_servers: BTreeMap<ConnectorId, McpServerId>,
    referenced_servers: BTreeSet<McpServerId>,
}

impl ConnectorMcpRuntimeProvider for MarketplaceConnectorMcpRuntimeProvider {
    fn materialize(
        &self,
        connector: &ConnectorDefinition,
        credential: SecretValue,
    ) -> Result<McpServerTransport, ConnectorMcpRuntimeError> {
        let server_id = self
            .connector_servers
            .get(connector.id())
            .ok_or_else(|| runtime_error("Marketplace Connector is not active"))?;
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| runtime_error("Marketplace MCP capability is not active"))?;
        let credential = std::str::from_utf8(credential.expose())
            .map_err(|_| runtime_error("Connector credential is not UTF-8 secret text"))?;
        server.transport.materialize(Some(credential))
    }

    fn standalone_servers(&self) -> Result<Vec<StandaloneMcpServer>, ConnectorMcpRuntimeError> {
        self.servers
            .iter()
            .filter(|(id, _)| !self.referenced_servers.contains(*id))
            .map(|(id, server)| {
                let definition = McpServerDefinition::new(
                    id.clone(),
                    &server.display_name,
                    server.transport.materialize(None)?,
                )
                .map_err(|error| runtime_error(error.to_string()))?;
                Ok(
                    StandaloneMcpServer::new(definition).with_invocation_fence(Arc::new(
                        MarketplaceInvocationFence {
                            manager: Arc::clone(&self.manager),
                            capability: server.capability.clone(),
                        },
                    )),
                )
            })
            .collect()
    }

    fn invocation_fence(
        &self,
        connector: &ConnectorDefinition,
    ) -> Option<Arc<dyn RuntimeInvocationFence>> {
        let server = self
            .servers
            .get(self.connector_servers.get(connector.id())?)?;
        Some(Arc::new(MarketplaceInvocationFence {
            manager: Arc::clone(&self.manager),
            capability: server.capability.clone(),
        }))
    }
}

struct MarketplaceMcpServer {
    display_name: String,
    transport: MarketplaceMcpTransport,
    capability: CapabilityRef,
}

struct MarketplaceInvocationFence {
    manager: Arc<MarketplaceManager>,
    capability: CapabilityRef,
}

impl RuntimeInvocationFence for MarketplaceInvocationFence {
    fn authorizes(&self) -> bool {
        self.manager
            .list_installed(ListInstalledRequest {})
            .is_ok_and(|installed| {
                installed.iter().any(|package| {
                    package.state == InstallationState::Installed
                        && package.capabilities.iter().any(|capability| {
                            capability.reference == self.capability
                                && capability.kind == CapabilityKind::Mcp
                        })
                })
            })
    }

    fn acquire(&self) -> Option<Box<dyn RuntimeInvocationLease>> {
        let acquired = self
            .manager
            .acquire_capability(AcquireCapabilityRequest {
                capability: self.capability.clone(),
            })
            .ok()?;
        if !matches!(acquired.spec, ActivationSpec::Mcp(_)) {
            let _ = self.manager.release_capability(ReleaseCapabilityRequest {
                lease_id: acquired.lease.id,
            });
            return None;
        }
        Some(Box::new(MarketplaceInvocationLease {
            manager: Arc::clone(&self.manager),
            lease_id: acquired.lease.id,
        }))
    }
}

struct MarketplaceInvocationLease {
    manager: Arc<MarketplaceManager>,
    lease_id: String,
}

impl RuntimeInvocationLease for MarketplaceInvocationLease {}

impl Drop for MarketplaceInvocationLease {
    fn drop(&mut self) {
        let _ = self.manager.release_capability(ReleaseCapabilityRequest {
            lease_id: self.lease_id.clone(),
        });
    }
}

fn mcp_server_id(source: &LocalCapabilitySource) -> Result<McpServerId, String> {
    McpServerId::new(format!(
        "marketplace:{}:mcp:{}",
        source.package().id,
        source.id()
    ))
    .map_err(|error| error.to_string())
}

fn connector_id(source: &LocalCapabilitySource) -> Result<ConnectorId, String> {
    ConnectorId::new(format!(
        "marketplace:{}:connector:{}",
        source.package().id,
        source.id()
    ))
    .map_err(|error| error.to_string())
}

fn runtime_error(message: impl Into<String>) -> ConnectorMcpRuntimeError {
    ConnectorMcpRuntimeError::new(message)
}

#[cfg(test)]
#[path = "marketplace_connector_runtime_tests.rs"]
mod tests;
