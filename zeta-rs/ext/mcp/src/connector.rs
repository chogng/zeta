use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use zeta_config::McpServerId;
use zeta_connectors::ConnectorConnectionState;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors_extension::ConnectorAuthority;
use zeta_mcp::McpServerDefinition;
use zeta_mcp::McpServerTransport;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretValue;

use crate::composition::ConnectorInvocationFence;
use crate::composition::McpInvocationAuthority;
use crate::composition::McpInvocationTransport;
use crate::composition::McpToolCompositionError;

/// Product/plugin adapter that turns one authorized Connector credential into a ready MCP transport.
///
/// Implementations are expected to resolve the referenced Plugin MCP declaration, enforce its
/// permission ceiling, consume the credential only into connection-time authorization, and return
/// sanitized errors. They must not persist, log, or place the credential in process arguments.
pub trait ConnectorMcpRuntimeProvider: Send + Sync {
    fn materialize(
        &self,
        definition: &ConnectorDefinition,
        credential: SecretValue,
    ) -> Result<McpServerTransport, ConnectorMcpRuntimeError>;

    /// Materializes active Plugin MCP contributions that do not require a Connector account.
    ///
    /// Providers that only bind Connector-backed servers retain the empty default. Returned
    /// declarations must already satisfy package, permission, credential, and transport policy.
    fn standalone_servers(&self) -> Result<Vec<StandaloneMcpServer>, ConnectorMcpRuntimeError> {
        Ok(Vec::new())
    }
}

/// One runtime-ready standalone Plugin MCP contribution.
pub struct StandaloneMcpServer {
    definition: McpServerDefinition,
}

impl StandaloneMcpServer {
    /// Wraps a host-materialized definition for publication by the shared composition layer.
    pub fn new(definition: McpServerDefinition) -> Self {
        Self { definition }
    }

    pub fn definition(&self) -> &McpServerDefinition {
        &self.definition
    }

    pub fn into_definition(self) -> McpServerDefinition {
        self.definition
    }
}

/// Sanitized failure from a product/plugin Connector MCP materializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorMcpRuntimeError(String);

impl ConnectorMcpRuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ConnectorMcpRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConnectorMcpRuntimeError {}

pub(crate) struct MaterializedConnectorServers {
    pub definitions: Vec<McpServerDefinition>,
    pub authorities: BTreeMap<McpServerId, McpInvocationAuthority>,
}

pub(crate) fn materialize_connector_servers(
    authority: ConnectorAuthority,
    secrets: &dyn SecretStore,
    provider: &dyn ConnectorMcpRuntimeProvider,
) -> Result<MaterializedConnectorServers, McpToolCompositionError> {
    let snapshot = authority.snapshot();
    let mut definitions = Vec::new();
    let mut authorities = BTreeMap::new();
    for entry in snapshot.ready_entries() {
        let ConnectorConnectionState::Connected(account) = entry.connection().state() else {
            continue;
        };
        let secret_key = SecretKey::new(account.credential_reference().as_str().to_string())
            .map_err(|_| McpToolCompositionError::new("invalid Connector credential reference"))?;
        let credential = secrets
            .load(&secret_key)
            .map_err(|_| McpToolCompositionError::new("Connector credential store unavailable"))?
            .ok_or_else(|| McpToolCompositionError::new("Connector credential is unavailable"))?;
        let server_id = McpServerId::new(
            entry
                .definition()
                .runtime_binding()
                .mcp_server_id()
                .to_string(),
        )
        .map_err(|error| McpToolCompositionError::new(error.to_string()))?;
        let transport = provider
            .materialize(entry.definition(), credential)
            .map_err(|error| McpToolCompositionError::new(error.to_string()))?;
        let invocation_transport = match &transport {
            McpServerTransport::Stdio(command) => McpInvocationTransport::Stdio {
                executable: command.program().to_string_lossy().into_owned(),
            },
            McpServerTransport::StreamableHttp(_) => McpInvocationTransport::StreamableHttp,
        };
        let definition = McpServerDefinition::new(
            server_id.clone(),
            entry.definition().display_name(),
            transport,
        )
        .map_err(|error| McpToolCompositionError::new(error.to_string()))?;
        let connector_fence = ConnectorInvocationFence {
            authority: authority.clone(),
            connector_id: entry.definition().id().clone(),
            connection_generation: entry.connection().generation(),
            definition_digest: entry.definition().digest(),
        };
        if authorities
            .insert(
                server_id.clone(),
                McpInvocationAuthority {
                    display_name: entry.definition().display_name().to_string(),
                    transport: invocation_transport,
                    connector_fence: Some(Arc::new(connector_fence)),
                },
            )
            .is_some()
        {
            return Err(McpToolCompositionError::new(
                "duplicate ready Connector MCP server identity",
            ));
        }
        definitions.push(definition);
    }
    Ok(MaterializedConnectorServers {
        definitions,
        authorities,
    })
}
