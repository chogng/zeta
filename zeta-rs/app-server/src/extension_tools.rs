use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::PolicyService;
use zeta_extension_api::ExtensionRegistry;
use zeta_policy::ActionDigest;
use zeta_policy::ActionKind;
use zeta_policy::ActionProvenance;
use zeta_policy::ActionReviewPhase;
use zeta_policy::ActionReviewRequest;
use zeta_policy::ActionSource;
use zeta_policy::Capability;
use zeta_policy::CapabilityKind;
use zeta_policy::CapabilitySet;
use zeta_policy::ExecutionDecision;
use zeta_policy::GrantId;
use zeta_policy::PolicyRevision;
use zeta_policy::ResolvedAction;
use zeta_policy::SandboxCompatibility;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolName;
use zeta_tools::ToolDefinition;
use zeta_tools::ToolEnvironmentId;
use zeta_tools::ToolInvocationKind;
use zeta_tools::ToolPayload;

use crate::tool_composition::ToolPort;
use crate::tool_executor_adapter::PreparedToolExecution;
use crate::tool_executor_adapter::ToolExecutorReviewer;

const EXTENSION_TOOL_POLICY_REVISION: &str = "host-read-only-extension-v1";

pub(crate) fn compose_extension_tools(
    registry: &ExtensionRegistry,
) -> Result<Option<ToolPort>, ExtensionToolCompositionError> {
    let executors = registry
        .contribute_read_only_tools()
        .map_err(|error| ExtensionToolCompositionError(error.to_string()))?;
    if executors.is_empty() {
        return Ok(None);
    }
    let definitions = executors
        .iter()
        .map(|executor| {
            let definition = executor.definition();
            (definition.name().clone(), definition)
        })
        .collect::<BTreeMap<_, _>>();
    if definitions.len() != executors.len() {
        return Err(ExtensionToolCompositionError(
            "extension tool names must be unique".into(),
        ));
    }
    let definitions = Arc::new(definitions);
    let environment_id = ToolEnvironmentId::new("host-extensions")
        .map_err(|error| ExtensionToolCompositionError(error.to_string()))?;
    ToolPort::extension(
        executors,
        environment_id,
        Arc::new(ExtensionToolReviewer {
            definitions: Arc::clone(&definitions),
        }),
        Arc::new(ExtensionToolPolicy { definitions }),
    )
    .map(Some)
    .map_err(|error| ExtensionToolCompositionError(error.to_string()))
}

struct ExtensionToolReviewer {
    definitions: Arc<BTreeMap<ToolName, ToolDefinition>>,
}

impl ToolExecutorReviewer for ExtensionToolReviewer {
    fn prepare(&self, call: &ToolCall) -> Result<PreparedToolExecution, CoreError> {
        let definition = self.definitions.get(&call.name).ok_or_else(|| {
            CoreError::Policy(format!("extension tool is not available: {}", call.name))
        })?;
        let payload = match definition.invocation() {
            ToolInvocationKind::Function { .. } => {
                if !call.arguments.is_object() {
                    return Err(CoreError::Policy(
                        "extension function tool arguments must be a JSON object".into(),
                    ));
                }
                ToolPayload::FunctionArguments(call.arguments.clone())
            }
            ToolInvocationKind::Freeform { .. } => ToolPayload::FreeformInput(
                call.arguments
                    .as_str()
                    .ok_or_else(|| {
                        CoreError::Policy(
                            "extension freeform tool input must be a JSON string".into(),
                        )
                    })?
                    .to_owned(),
            ),
        };
        let canonical = serde_json::to_vec(&json!({
            "tool": call.name.as_str(),
            "definition_digest": definition.digest().as_str(),
            "arguments": call.arguments,
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        let review = ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::SystemOperation,
                format!("run host-installed read-only extension tool '{}'", call.name),
                extension_capabilities(&call.name),
            ),
            ActionProvenance::new(ActionSource::Plugin, call.name.as_str()),
            SandboxCompatibility::NotApplicable {
                reason: "the host extension executes in process and is constrained to the read-only extension contract".into(),
            },
            PolicyRevision::new(EXTENSION_TOOL_POLICY_REVISION),
        );
        Ok(PreparedToolExecution::new(review, payload))
    }
}

struct ExtensionToolPolicy {
    definitions: Arc<BTreeMap<ToolName, ToolDefinition>>,
}

impl PolicyService for ExtensionToolPolicy {
    fn revision(&self) -> String {
        EXTENSION_TOOL_POLICY_REVISION.into()
    }

    fn decide(
        &self,
        request: &ActionReviewRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let tool_name = ToolName::new(request.provenance().source_id()).ok();
        let capabilities = tool_name
            .as_ref()
            .filter(|name| self.definitions.contains_key(*name))
            .map(extension_capabilities);
        if request.policy_revision().as_str() != EXTENSION_TOOL_POLICY_REVISION
            || request.provenance().source() != &ActionSource::Plugin
            || capabilities.as_ref() != Some(request.action().required_capabilities())
            || request.action().kind() != &ActionKind::SystemOperation
            || !matches!(request.phase(), ActionReviewPhase::Initial)
            || !matches!(
                request.sandbox(),
                SandboxCompatibility::NotApplicable { .. }
            )
        {
            return Err(CoreError::Policy(
                "extension tool policy rejected an action outside its read-only contract".into(),
            ));
        }
        Ok(ExecutionDecision::RunUnsandboxed {
            grant_id: GrantId::new(format!(
                "host-read-only-extension:{}",
                request.provenance().source_id()
            )),
        })
    }
}

fn extension_capabilities(name: &ToolName) -> CapabilitySet {
    CapabilitySet::new([Capability::new(
        CapabilityKind::FileRead,
        format!("extension-tool:{}", name.as_str()),
    )])
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExtensionToolCompositionError(String);

impl std::fmt::Display for ExtensionToolCompositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExtensionToolCompositionError {}

#[cfg(test)]
#[path = "extension_tools_tests.rs"]
mod tests;
