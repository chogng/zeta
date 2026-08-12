use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

use zeta_async_utils::CancellationToken;
use zeta_core::{CoreError, PolicyService, ToolAuthorization, ToolOutputSink, ToolService};
use zeta_policy::{
    ActionDigest, ActionReviewRequest, ActionSource, ExecutionDecision, ReviewEvidence,
};
use zeta_protocol::{ToolCall, ToolCallId, ToolDefinition, ToolExecutionOutput, ToolName};

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

struct ToolGeneration {
    tools: Arc<dyn ToolService>,
    policy: Arc<dyn PolicyService>,
}

struct BoundToolCall {
    tools: Arc<dyn ToolService>,
    action_digest: ActionDigest,
}

/// Atomically switches future Tool safe points while retaining bindings for prepared calls.
pub(crate) struct ReloadableToolPorts {
    current: RwLock<Arc<ToolGeneration>>,
    calls: Mutex<BTreeMap<ToolCallId, BoundToolCall>>,
    policies: Mutex<BTreeMap<ActionDigest, Arc<dyn PolicyService>>>,
    diagnostic: Mutex<Option<String>>,
}

impl ReloadableToolPorts {
    pub(crate) fn new(initial: Option<CombinedToolPorts>) -> Arc<Self> {
        Arc::new(Self {
            current: RwLock::new(Arc::new(tool_generation(initial))),
            calls: Mutex::new(BTreeMap::new()),
            policies: Mutex::new(BTreeMap::new()),
            diagnostic: Mutex::new(None),
        })
    }

    pub(crate) fn replace(&self, next: Option<CombinedToolPorts>) {
        *self
            .current
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::new(tool_generation(next));
        *self
            .diagnostic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    pub(crate) fn record_reconcile_failure(&self, error: impl Into<String>) {
        *self
            .diagnostic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error.into());
    }

    #[cfg(test)]
    pub(crate) fn diagnostic(&self) -> Option<String> {
        self.diagnostic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn tools(self: &Arc<Self>) -> Arc<dyn ToolService> {
        Arc::new(ReloadableToolService {
            ports: Arc::clone(self),
        })
    }

    pub(crate) fn policy(self: &Arc<Self>) -> Arc<dyn PolicyService> {
        Arc::new(ReloadablePolicyService {
            ports: Arc::clone(self),
        })
    }

    fn generation(&self) -> Arc<ToolGeneration> {
        Arc::clone(
            &self
                .current
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

struct ReloadableToolService {
    ports: Arc<ReloadableToolPorts>,
}

impl ToolService for ReloadableToolService {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.ports.generation().tools.definitions()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let generation = self.ports.generation();
        let request = generation.tools.prepare(call)?;
        let digest = request.action().digest().clone();
        self.ports
            .policies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(digest.clone(), Arc::clone(&generation.policy));
        self.ports
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                call.id.clone(),
                BoundToolCall {
                    tools: Arc::clone(&generation.tools),
                    action_digest: digest,
                },
            );
        Ok(request)
    }

    fn review_evidence(&self, call: &ToolCall) -> Result<Vec<ReviewEvidence>, CoreError> {
        self.bound_tools(call).review_evidence(call)
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let tools = self.bound_tools(call);
        let result = tools.execute(call, authorization, cancellation);
        self.release(call);
        result
    }

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let tools = self.bound_tools(call);
        let result = tools.execute_streaming(call, authorization, cancellation, sink);
        self.release(call);
        result
    }
}

impl ReloadableToolService {
    fn bound_tools(&self, call: &ToolCall) -> Arc<dyn ToolService> {
        self.ports
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&call.id)
            .map(|binding| Arc::clone(&binding.tools))
            .unwrap_or_else(|| Arc::clone(&self.ports.generation().tools))
    }

    fn release(&self, call: &ToolCall) {
        let binding = self
            .ports
            .calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&call.id);
        if let Some(binding) = binding {
            self.ports
                .policies
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&binding.action_digest);
        }
    }
}

struct ReloadablePolicyService {
    ports: Arc<ReloadableToolPorts>,
}

impl PolicyService for ReloadablePolicyService {
    fn revision(&self) -> String {
        self.ports.generation().policy.revision()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        let policy = self
            .ports
            .policies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(request.action().digest())
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.ports.generation().policy));
        policy.decide(request, cancellation)
    }
}

fn tool_generation(combined: Option<CombinedToolPorts>) -> ToolGeneration {
    match combined {
        Some(combined) => ToolGeneration {
            tools: combined.tools,
            policy: combined.policy,
        },
        None => ToolGeneration {
            tools: Arc::new(EmptyToolService),
            policy: Arc::new(EmptyPolicyService),
        },
    }
}

struct EmptyToolService;

impl ToolService for EmptyToolService {
    fn definitions(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Err(CoreError::Policy(format!(
            "tool is not available: {}",
            call.name
        )))
    }

    fn execute(
        &self,
        call: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Ok(ToolExecutionOutput::Failure(format!(
            "tool is not available: {}",
            call.name
        )))
    }
}

struct EmptyPolicyService;

impl PolicyService for EmptyPolicyService {
    fn revision(&self) -> String {
        "empty-policy-v1".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Err(CoreError::Policy(
            "no policy owns the requested tool action".into(),
        ))
    }
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

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        self.service(call)?
            .execute_streaming(call, authorization, cancellation, sink)
    }
}

struct CompositePolicyService {
    local: Option<Arc<dyn PolicyService>>,
    mcp: Option<Arc<dyn PolicyService>>,
}

impl PolicyService for CompositePolicyService {
    fn revision(&self) -> String {
        format!(
            "composite-policy-v1:local={}:mcp={}",
            self.local
                .as_ref()
                .map_or_else(|| "none".into(), |policy| policy.revision()),
            self.mcp
                .as_ref()
                .map_or_else(|| "none".into(), |policy| policy.revision())
        )
    }

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
