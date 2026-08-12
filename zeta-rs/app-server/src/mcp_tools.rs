use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigGeneration, McpCredentialBinding, McpServerEnablement, McpServerId, McpTransportConfig,
    ResolvedConfig,
};
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
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolName};
use zeta_rmcp_client::{StdioServerCommand, StreamableHttpServer};
use zeta_tools::{ToolContent, ToolOutput, ToolOutputStatus};

use crate::mcp_runtime::{McpRuntimeOwner, McpRuntimeOwnerError};

const MCP_POLICY_REVISION: &str = "mcp-user-approval-v1";

#[derive(Clone)]
enum McpInvocationTransport {
    Stdio { executable: String },
    StreamableHttp,
}

#[derive(Clone)]
struct McpInvocationAuthority {
    display_name: String,
    transport: McpInvocationTransport,
}

pub(crate) struct McpToolComposition {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn PolicyService>,
}

pub(crate) fn compose_mcp_tools(
    config: &ResolvedConfig,
    generation: ConfigGeneration,
) -> Result<Option<McpToolComposition>, McpToolCompositionError> {
    let (definitions, authorities) = materialize_servers(config)?;
    if definitions.is_empty() {
        return Ok(None);
    }
    start_mcp_tools(definitions, authorities, generation, None).map(Some)
}

fn start_mcp_tools(
    definitions: Vec<McpServerDefinition>,
    authorities: BTreeMap<McpServerId, McpInvocationAuthority>,
    generation: ConfigGeneration,
    factory: Option<Arc<dyn McpSessionFactory>>,
) -> Result<McpToolComposition, McpToolCompositionError> {
    let catalog_generation = generation
        .get()
        .checked_add(1)
        .ok_or_else(|| McpToolCompositionError("MCP catalog generation overflow".into()))?;
    let options = McpRuntimeOptions::new("zeta-app-server", env!("CARGO_PKG_VERSION"))
        .with_startup_policy(McpStartupPolicy::RequireAll)
        .with_catalog_generation(catalog_generation)
        .with_first_connection_generation(catalog_generation);
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
        match self
            .owner
            .call(binding, call.arguments.clone(), cancellation.clone())
        {
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
            ToolContent::Text(text) => json!({"type": "text", "text": text}),
            ToolContent::Image { url, detail } => json!({
                "type": "image_url",
                "url": url,
                "detail": format!("{detail:?}").to_ascii_lowercase(),
            }),
        })
        .collect::<Vec<_>>();
    let serialized = serde_json::to_string(&json!({"content": content}))
        .map_err(|error| CoreError::Execution(error.to_string()))?;
    Ok(match output.status() {
        ToolOutputStatus::Success => ToolExecutionOutput::Success(serialized),
        ToolOutputStatus::Error => ToolExecutionOutput::Failure(serialized),
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
pub(crate) struct McpToolCompositionError(String);

impl McpToolCompositionError {
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
#[path = "mcp_tools_tests.rs"]
mod tests;
