use std::collections::BTreeMap;
use std::sync::Arc;

use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, PolicyService, ToolAuthorization, ToolService};
use zeta_policy::{ActionReviewRequest, ActionSource, ExecutionDecision, ReviewEvidence};
use zeta_protocol::{ToolCall, ToolDefinition, ToolExecutionOutput, ToolName};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ToolPortKind {
    Local,
    Mcp,
}

#[derive(Clone)]
pub(crate) struct ToolPort {
    kind: ToolPortKind,
    tools: Arc<dyn ToolService>,
    policy: Arc<dyn PolicyService>,
}

impl ToolPort {
    pub(crate) fn local(tools: Arc<dyn ToolService>, policy: Arc<dyn PolicyService>) -> Self {
        Self {
            kind: ToolPortKind::Local,
            tools,
            policy,
        }
    }

    pub(crate) fn mcp(tools: Arc<dyn ToolService>, policy: Arc<dyn PolicyService>) -> Self {
        Self {
            kind: ToolPortKind::Mcp,
            tools,
            policy,
        }
    }
}

pub(crate) struct CombinedToolPorts {
    pub(crate) tools: Arc<dyn ToolService>,
    pub(crate) policy: Arc<dyn PolicyService>,
}

pub(crate) fn combine_tool_ports(
    ports: Vec<ToolPort>,
) -> Result<Option<CombinedToolPorts>, ToolCompositionError> {
    if ports.is_empty() {
        return Ok(None);
    }
    let mut definitions = Vec::new();
    let mut routes = BTreeMap::new();
    let mut local_policy = None;
    let mut mcp_policy = None;
    let mut services = Vec::new();
    for port in ports {
        let service_index = services.len();
        for definition in port.tools.definitions() {
            if routes
                .insert(definition.name.clone(), service_index)
                .is_some()
            {
                return Err(ToolCompositionError(format!(
                    "duplicate model tool name during App Server composition: {}",
                    definition.name
                )));
            }
            definitions.push(definition);
        }
        match port.kind {
            ToolPortKind::Local if local_policy.is_none() => {
                local_policy = Some(Arc::clone(&port.policy));
            }
            ToolPortKind::Mcp if mcp_policy.is_none() => {
                mcp_policy = Some(Arc::clone(&port.policy));
            }
            ToolPortKind::Local => {
                return Err(ToolCompositionError(
                    "multiple local policy ports are not supported".into(),
                ));
            }
            ToolPortKind::Mcp => {
                return Err(ToolCompositionError(
                    "multiple MCP policy ports are not supported".into(),
                ));
            }
        }
        services.push(port.tools);
    }
    Ok(Some(CombinedToolPorts {
        tools: Arc::new(CompositeToolService {
            definitions,
            routes,
            services,
        }),
        policy: Arc::new(CompositePolicyService {
            local: local_policy,
            mcp: mcp_policy,
        }),
    }))
}

struct CompositeToolService {
    definitions: Vec<ToolDefinition>,
    routes: BTreeMap<ToolName, usize>,
    services: Vec<Arc<dyn ToolService>>,
}

impl CompositeToolService {
    fn service(&self, call: &ToolCall) -> Result<&Arc<dyn ToolService>, CoreError> {
        self.routes
            .get(&call.name)
            .and_then(|index| self.services.get(*index))
            .ok_or_else(|| CoreError::Policy(format!("tool is not available: {}", call.name)))
    }
}

impl ToolService for CompositeToolService {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.definitions.clone()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        self.service(call)?.prepare(call)
    }

    fn review_evidence(&self, call: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        self.service(call)?.review_evidence(call)
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.service(call)?
            .execute(call, authorization, cancellation)
    }
}

struct CompositePolicyService {
    local: Option<Arc<dyn PolicyService>>,
    mcp: Option<Arc<dyn PolicyService>>,
}

impl PolicyService for CompositePolicyService {
    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let policy = match request.provenance().source() {
            ActionSource::BuiltInTool => self.local.as_ref(),
            ActionSource::McpServer => self.mcp.as_ref(),
            _ => None,
        }
        .ok_or_else(|| {
            CoreError::Policy("no policy owns the prepared tool action provenance".into())
        })?;
        policy.decide(request, cancellation)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolCompositionError(String);

impl std::fmt::Display for ToolCompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ToolCompositionError {}

#[cfg(test)]
#[path = "tool_composition_tests.rs"]
mod tests;
