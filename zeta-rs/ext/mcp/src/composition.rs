use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigGeneration, McpCredentialBinding, McpServerEnablement, McpServerId, McpTransportConfig,
    ResolvedConfig,
};
use zeta_connectors::ConnectorConnectionGeneration;
use zeta_connectors::ConnectorDefinitionDigest;
use zeta_connectors::ConnectorId;
use zeta_connectors_extension::ConnectorAuthority;
use zeta_core::{CoreError, PolicyService, ToolAuthorization, ToolService};
use zeta_mcp::{
    McpCallError, McpRuntimeOptions, McpServerDefinition, McpServerTransport, McpSessionFactory,
    McpStartupPolicy, McpToolBinding,
};
use zeta_policy::{
    ActionDigest, ActionKind, ActionProvenance, ActionReviewPhase, ActionReviewRequest,
    ActionSource, ApprovalRequest, Capability, CapabilityKind, CapabilitySet, ExecutionDecision,
    PolicyRevision, ResolvedAction, SandboxCompatibility,
};
use zeta_protocol::ContentPart;
use zeta_protocol::ToolSourceProvenance;
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolName};
use zeta_rmcp_client::{StdioServerCommand, StreamableHttpServer};
use zeta_secrets::SecretStore;
use zeta_tools::{ToolContent, ToolOutput, ToolOutputStatus};

use crate::connector::ConnectorMcpRuntimeProvider;
use crate::connector::materialize_connector_servers;
use crate::runtime::McpRuntimeOwner;
use crate::runtime::McpRuntimeOwnerError;
use crate::updates::McpCatalogUpdates;

const MCP_POLICY_REVISION: &str = "mcp-user-approval-v1";

#[derive(Clone)]
pub(crate) enum McpInvocationTransport {
    Stdio { executable: String },
    StreamableHttp,
}

#[derive(Clone)]
pub(crate) struct ConnectorInvocationFence {
    pub authority: ConnectorAuthority,
    pub connector_id: ConnectorId,
    pub connection_generation: ConnectorConnectionGeneration,
    pub definition_digest: ConnectorDefinitionDigest,
}

#[derive(Clone)]
pub(crate) struct McpInvocationAuthority {
    pub display_name: String,
    pub transport: McpInvocationTransport,
    pub connector_fence: Option<Arc<ConnectorInvocationFence>>,
}

pub struct McpToolComposition {
    pub tools: Arc<dyn ToolService>,
    pub policy: Arc<dyn PolicyService>,
}

/// Resolves enabled MCP declarations into one live tool service and its exact approval policy.
pub fn compose_mcp_tools(
    config: &ResolvedConfig,
    generation: ConfigGeneration,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    let (definitions, authorities) = materialize_servers(config)?;
    if definitions.is_empty() {
        return Ok(None);
    }
    let catalog_generation = generation
        .get()
        .checked_add(1)
        .ok_or_else(|| McpToolCompositionError("MCP catalog generation overflow".into()))?;
    start_mcp_tools(definitions, authorities, catalog_generation, None).map(Some)
}

/// Resolves enabled MCP declarations and publishes tool-list invalidations to the host.
pub fn compose_mcp_tools_with_updates(
    config: &ResolvedConfig,
    generation: ConfigGeneration,
    updates: McpCatalogUpdates,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    let (definitions, authorities) = materialize_servers(config)?;
    if definitions.is_empty() {
        return Ok(None);
    }
    let catalog_generation = generation
        .get()
        .checked_add(1)
        .ok_or_else(|| McpToolCompositionError("MCP catalog generation overflow".into()))?;
    start_mcp_tools_with_updates(
        definitions,
        authorities,
        catalog_generation,
        None,
        Some(updates),
    )
    .map(Some)
}

/// Resolves enabled MCP declarations at one host-owned reconcile generation.
pub fn compose_mcp_tools_at_generation_with_updates(
    config: &ResolvedConfig,
    catalog_generation: u64,
    updates: McpCatalogUpdates,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    if catalog_generation == 0 {
        return Err(McpToolCompositionError(
            "MCP catalog generation must be non-zero".into(),
        ));
    }
    let (definitions, authorities) = materialize_servers(config)?;
    if definitions.is_empty() {
        return Ok(None);
    }
    start_mcp_tools_with_updates(
        definitions,
        authorities,
        catalog_generation,
        None,
        Some(updates),
    )
    .map(Some)
}

/// Composes user-configured MCP servers with the exact ready Connector authority projection.
///
/// `catalog_generation` is owned by the host reconcile loop and must advance whenever either its
/// Config or Connector inputs change. The runtime provider is the Plugin activation boundary that
/// materializes each referenced MCP declaration with its connection-time credential.
pub fn compose_mcp_tools_with_connectors(
    config: &ResolvedConfig,
    catalog_generation: u64,
    connector_authority: ConnectorAuthority,
    secrets: Arc<dyn SecretStore>,
    provider: Arc<dyn ConnectorMcpRuntimeProvider>,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    if catalog_generation == 0 {
        return Err(McpToolCompositionError(
            "MCP catalog generation must be non-zero".into(),
        ));
    }
    let (mut definitions, mut authorities) = materialize_servers(config)?;
    materialize_standalone_plugin_servers(provider.as_ref(), &mut definitions, &mut authorities)?;
    let connectors =
        materialize_connector_servers(connector_authority, secrets.as_ref(), provider.as_ref())?;
    for server in connectors.authorities.keys() {
        if authorities.contains_key(server) {
            return Err(McpToolCompositionError(format!(
                "MCP server identity is declared by Config and a Connector: {server}"
            )));
        }
    }
    definitions.extend(connectors.definitions);
    authorities.extend(connectors.authorities);
    if definitions.is_empty() {
        return Ok(None);
    }
    start_mcp_tools(definitions, authorities, catalog_generation, None).map(Some)
}

/// Composes Config and ready Connector MCP servers with host-visible catalog invalidations.
pub fn compose_mcp_tools_with_connectors_and_updates(
    config: &ResolvedConfig,
    catalog_generation: u64,
    connector_authority: ConnectorAuthority,
    secrets: Arc<dyn SecretStore>,
    provider: Arc<dyn ConnectorMcpRuntimeProvider>,
    updates: McpCatalogUpdates,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    if catalog_generation == 0 {
        return Err(McpToolCompositionError(
            "MCP catalog generation must be non-zero".into(),
        ));
    }
    let (mut definitions, mut authorities) = materialize_servers(config)?;
    materialize_standalone_plugin_servers(provider.as_ref(), &mut definitions, &mut authorities)?;
    let connectors =
        materialize_connector_servers(connector_authority, secrets.as_ref(), provider.as_ref())?;
    for server in connectors.authorities.keys() {
        if authorities.contains_key(server) {
            return Err(McpToolCompositionError(format!(
                "MCP server identity is declared by Config and a Connector: {server}"
            )));
        }
    }
    definitions.extend(connectors.definitions);
    authorities.extend(connectors.authorities);
    if definitions.is_empty() {
        return Ok(None);
    }
    start_mcp_tools_with_updates(
        definitions,
        authorities,
        catalog_generation,
        None,
        Some(updates),
    )
    .map(Some)
}

fn materialize_standalone_plugin_servers(
    provider: &dyn ConnectorMcpRuntimeProvider,
    definitions: &mut Vec<McpServerDefinition>,
    authorities: &mut BTreeMap<McpServerId, McpInvocationAuthority>,
) -> Result<(), McpToolCompositionError> {
    for standalone in provider
        .standalone_servers()
        .map_err(|error| McpToolCompositionError::new(error.to_string()))?
    {
        let definition = standalone.into_definition();
        let server_id = definition.id().clone();
        let display_name = definition.display_name().to_string();
        let transport = match definition.transport() {
            McpServerTransport::Stdio(command) => McpInvocationTransport::Stdio {
                executable: command.program().to_string_lossy().into_owned(),
            },
            McpServerTransport::StreamableHttp(_) => McpInvocationTransport::StreamableHttp,
        };
        if authorities
            .insert(
                server_id,
                McpInvocationAuthority {
                    display_name,
                    transport,
                    connector_fence: None,
                },
            )
            .is_some()
        {
            return Err(McpToolCompositionError::new(
                "MCP server identity is declared by Config and a Plugin",
            ));
        }
        definitions.push(definition);
    }
    Ok(())
}

fn start_mcp_tools(
    definitions: Vec<McpServerDefinition>,
    authorities: BTreeMap<McpServerId, McpInvocationAuthority>,
    catalog_generation: u64,
    factory: Option<Arc<dyn McpSessionFactory>>,
) -> Result<McpToolComposition, McpToolCompositionError> {
    start_mcp_tools_with_updates(definitions, authorities, catalog_generation, factory, None)
}

fn start_mcp_tools_with_updates(
    definitions: Vec<McpServerDefinition>,
    authorities: BTreeMap<McpServerId, McpInvocationAuthority>,
    catalog_generation: u64,
    factory: Option<Arc<dyn McpSessionFactory>>,
    updates: Option<McpCatalogUpdates>,
) -> Result<McpToolComposition, McpToolCompositionError> {
    let mut options = McpRuntimeOptions::new("zeta-app-server", env!("CARGO_PKG_VERSION"))
        .with_startup_policy(McpStartupPolicy::RequireAll)
        .with_catalog_generation(catalog_generation)
        .with_first_connection_generation(catalog_generation);
    if let Some(updates) = updates {
        options = options.with_client_host(updates.client_host());
    }
    let owner = match factory {
        Some(factory) => McpRuntimeOwner::start_with_factory(definitions, options, factory),
        None => McpRuntimeOwner::start(definitions, options),
    }
    .map_err(McpToolCompositionError::runtime)?;
    let owner = Arc::new(owner);
    let capabilities = authorities
        .iter()
        .map(|(server, authority)| {
            (
                server.to_string(),
                invocation_capabilities(server, authority),
            )
        })
        .collect();
    Ok(McpToolComposition {
        tools: Arc::new(McpToolService { owner, authorities }),
        policy: Arc::new(McpApprovalPolicy { capabilities }),
    })
}

fn materialize_servers(
    config: &ResolvedConfig,
) -> Result<
    (
        Vec<McpServerDefinition>,
        BTreeMap<McpServerId, McpInvocationAuthority>,
    ),
    McpToolCompositionError,
> {
    let mut definitions = Vec::new();
    let mut authorities = BTreeMap::new();
    for server in config.mcp.servers.values() {
        if server.enablement != McpServerEnablement::Enabled {
            continue;
        }
        if let McpCredentialBinding::Reference { credential_ref } = &server.credential {
            return Err(McpToolCompositionError(format!(
                "enabled MCP server '{}' requires unsupported credential reference '{}'",
                server.id, credential_ref
            )));
        }
        let (transport, authority) = match &server.transport {
            McpTransportConfig::Stdio { command, args } => {
                let executable = canonical_executable(command, &server.id)?;
                let transport = StdioServerCommand::new(&executable).with_args(args);
                (
                    McpServerTransport::Stdio(transport),
                    McpInvocationTransport::Stdio {
                        executable: executable.to_string_lossy().into_owned(),
                    },
                )
            }
            McpTransportConfig::StreamableHttp { url } => (
                McpServerTransport::StreamableHttp(StreamableHttpServer::new(url).map_err(
                    |error| {
                        McpToolCompositionError(format!(
                            "invalid MCP endpoint for '{}': {error}",
                            server.id
                        ))
                    },
                )?),
                McpInvocationTransport::StreamableHttp,
            ),
        };
        definitions.push(
            McpServerDefinition::new(server.id.clone(), &server.display_name, transport)
                .map_err(|error| McpToolCompositionError(error.to_string()))?,
        );
        authorities.insert(
            server.id.clone(),
            McpInvocationAuthority {
                display_name: server.display_name.clone(),
                transport: authority,
                connector_fence: None,
            },
        );
    }
    Ok((definitions, authorities))
}

fn canonical_executable(
    command: &str,
    server: &McpServerId,
) -> Result<std::path::PathBuf, McpToolCompositionError> {
    let path = Path::new(command);
    if !path.is_absolute() {
        return Err(McpToolCompositionError(format!(
            "enabled stdio MCP server '{server}' must use an absolute executable path"
        )));
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        McpToolCompositionError(format!(
            "could not resolve stdio MCP executable for '{server}': {error}"
        ))
    })?;
    if !canonical.is_file() {
        return Err(McpToolCompositionError(format!(
            "stdio MCP executable for '{server}' is not a file: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

struct McpToolService {
    owner: Arc<McpRuntimeOwner>,
    authorities: BTreeMap<McpServerId, McpInvocationAuthority>,
}

impl McpToolService {
    fn binding(&self, name: &ToolName) -> Result<&McpToolBinding, CoreError> {
        self.owner
            .resolve(name)
            .ok_or_else(|| CoreError::Policy(format!("MCP tool is not available: {name}")))
    }

    fn review_request(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let binding = self.binding(&call.name)?;
        if !call.arguments.is_object() {
            return Err(CoreError::Policy(
                "MCP tool arguments must be a JSON object".into(),
            ));
        }
        let server = binding.remote().server();
        let authority = self.authorities.get(server).ok_or_else(|| {
            CoreError::Policy(format!("MCP server authority is unavailable: {server}"))
        })?;
        if let Some(fence) = &authority.connector_fence
            && !fence.authority.authorizes(
                &fence.connector_id,
                fence.connection_generation,
                &fence.definition_digest,
            )
        {
            return Err(CoreError::Policy(format!(
                "Connector is no longer authorized: {}",
                fence.connector_id
            )));
        }
        let authority_scope = match &authority.transport {
            McpInvocationTransport::Stdio { executable } => {
                json!({"type": "stdio", "executable": executable})
            }
            McpInvocationTransport::StreamableHttp => {
                json!({"type": "streamable_http", "server": server.as_str()})
            }
        };
        let canonical = serde_json::to_vec(&json!({
            "server": server.as_str(),
            "remote_tool": binding.remote().remote_name(),
            "definition_digest": binding.definition_digest().as_str(),
            "connection_generation": binding.connection_generation(),
            "catalog_generation": binding.catalog_generation(),
            "authority": authority_scope,
            "arguments": &call.arguments,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::ExternalServiceMutation,
                format!(
                    "call MCP tool '{}' on {}",
                    binding.exposed_name(),
                    authority.display_name
                ),
                invocation_capabilities(server, authority),
            ),
            ActionProvenance::new(ActionSource::McpServer, server.as_str()),
            SandboxCompatibility::NotApplicable {
                reason: "remote MCP side effects cannot be enforced by the local sandbox".into(),
            },
            PolicyRevision::new(MCP_POLICY_REVISION),
        ))
    }
}

fn invocation_capabilities(
    server: &McpServerId,
    authority: &McpInvocationAuthority,
) -> CapabilitySet {
    let mut capabilities = vec![Capability::new(
        CapabilityKind::ExternalMutation,
        server.as_str(),
    )];
    if matches!(&authority.transport, McpInvocationTransport::StreamableHttp) {
        capabilities.push(Capability::new(CapabilityKind::Network, server.as_str()));
    }
    CapabilitySet::new(capabilities)
}

impl ToolService for McpToolService {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.owner.definitions().to_vec()
    }

    fn source_provenance(&self, name: &ToolName) -> Vec<ToolSourceProvenance> {
        self.owner
            .resolve(name)
            .map(|binding| {
                vec![ToolSourceProvenance::Mcp {
                    server_id: binding.remote().server().to_string(),
                    remote_name: binding.remote().remote_name().to_owned(),
                    catalog_generation: binding.catalog_generation(),
                    connection_generation: binding.connection_generation(),
                }]
            })
            .unwrap_or_default()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        self.review_request(call)
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        if !matches!(
            authorization,
            ToolAuthorization::ApprovedOnce(_)
                | ToolAuthorization::AutoReviewed(_)
                | ToolAuthorization::PermissionBypassed(_)
        ) {
            return Err(CoreError::Policy(
                "MCP tools require exact user, automatic-review, or permission-bypass authority"
                    .into(),
            ));
        }
        let binding = self.binding(&call.name)?.clone();
        let authority = self
            .authorities
            .get(binding.remote().server())
            .cloned()
            .ok_or_else(|| CoreError::Policy("MCP invocation authority is unavailable".into()))?;
        let invoke = || {
            self.owner
                .call(binding, call.arguments.clone(), cancellation.clone())
        };
        let invocation = match authority.connector_fence {
            Some(fence) => fence
                .authority
                .with_authorized_invocation(
                    &fence.connector_id,
                    fence.connection_generation,
                    &fence.definition_digest,
                    invoke,
                )
                .ok_or_else(|| {
                    CoreError::Policy(format!(
                        "Connector was disconnected before MCP dispatch: {}",
                        fence.connector_id
                    ))
                })?,
            None => invoke(),
        };
        match invocation {
            Ok(output) => protocol_execution_output(output),
            Err(McpCallError::OutcomeUncertain(message)) => {
                Ok(ToolExecutionOutput::OutcomeUnknown(message))
            }
            Err(McpCallError::NotStarted(message) | McpCallError::InvalidResult(message)) => {
                Ok(ToolExecutionOutput::Failure(message))
            }
            Err(error) => Ok(ToolExecutionOutput::OutcomeUnknown(error.to_string())),
        }
    }
}

fn protocol_execution_output(output: ToolOutput) -> Result<ToolExecutionOutput, CoreError> {
    let content = output
        .content()
        .iter()
        .map(|content| match content {
            ToolContent::Text(text) => ContentPart::Text(text.clone()),
            ToolContent::Image { url, detail } => ContentPart::ImageUrl {
                url: url.clone(),
                detail: *detail,
            },
        })
        .collect::<Vec<_>>();
    Ok(match output.status() {
        ToolOutputStatus::Success => ToolExecutionOutput::SuccessContent(content),
        ToolOutputStatus::Error => ToolExecutionOutput::FailureContent(content),
    })
}

struct McpApprovalPolicy {
    capabilities: BTreeMap<String, CapabilitySet>,
}

impl PolicyService for McpApprovalPolicy {
    fn revision(&self) -> String {
        MCP_POLICY_REVISION.into()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        if request.policy_revision().as_str() != MCP_POLICY_REVISION
            || request.provenance().source() != &ActionSource::McpServer
            || self.capabilities.get(request.provenance().source_id())
                != Some(request.action().required_capabilities())
            || request.action().kind() != &ActionKind::ExternalServiceMutation
            || !matches!(request.phase(), ActionReviewPhase::Initial)
            || !matches!(
                request.sandbox(),
                SandboxCompatibility::NotApplicable { .. }
            )
        {
            return Err(CoreError::Policy(
                "MCP policy rejected an action outside its exact review contract".into(),
            ));
        }
        Ok(ExecutionDecision::AskUser(ApprovalRequest::new(
            request.action().digest().clone(),
            request.action().required_capabilities().clone(),
            "MCP tools execute outside the local sandbox and require one-time approval",
        )))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpToolCompositionError(String);

impl McpToolCompositionError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn runtime(error: McpRuntimeOwnerError) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for McpToolCompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for McpToolCompositionError {}

#[cfg(test)]
#[path = "composition_tests.rs"]
mod tests;
