use std::num::NonZeroUsize;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::Capability;
use zeta_action_policy::CapabilityKind;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::SandboxCompatibility;
use zeta_async_utils::CancellationToken;
use zeta_code_index::CodeIndex;
use zeta_code_index_cloud::CloudCodeIndexController;
use zeta_code_index_cloud::CodeIndexDeploymentMode;
use zeta_code_index_semantic::CodeIndexSemanticService;
use zeta_code_retrieval::CodeRetrievalQuery;
use zeta_code_retrieval::CodeRetrievalService;
use zeta_core::CoreError;
use zeta_core::ToolAuthorization;
use zeta_core::ToolService;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolName;
use zeta_workspace::TrustedWorkspace;

pub(crate) const CODE_RETRIEVAL_TOOL_NAME: &str = "search_code";

#[derive(Deserialize)]
struct SearchCodeArguments {
    query: String,
    max_results: Option<usize>,
}

/// Agent-facing read-only tool backed by Zeta's canonical code-retrieval coordinator.
pub(crate) struct CodeRetrievalTool {
    workspace: TrustedWorkspace,
    index: Arc<CodeIndex>,
    semantic: Option<Arc<CodeIndexSemanticService>>,
    cloud: Option<Arc<CloudCodeIndexController>>,
    definition: ToolDefinition,
    action_policy_revision: ActionPolicyRevision,
}

impl CodeRetrievalTool {
    pub(crate) fn new(
        workspace: TrustedWorkspace,
        index: Arc<CodeIndex>,
        semantic: Option<Arc<CodeIndexSemanticService>>,
        cloud: Option<Arc<CloudCodeIndexController>>,
    ) -> Self {
        Self {
            workspace,
            index,
            semantic,
            cloud,
            action_policy_revision: super::local_tools::local_policy_revision(),
            definition: ToolDefinition {
                name: ToolName::new(CODE_RETRIEVAL_TOOL_NAME)
                    .expect("static code-retrieval tool name is valid"),
                description: "Search the indexed workspace for code relevant to a natural-language query. Returns bounded, current source excerpts and reports whether semantic retrieval degraded to local lexical search.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural-language description, symbol, or behavior to find."
                        },
                        "max_results": {
                            "type": ["integer", "null"],
                            "minimum": 1,
                            "maximum": 20,
                            "description": "Maximum excerpts. Use null for 10."
                        }
                    },
                    "required": ["query", "max_results"],
                    "additionalProperties": false
                }),
                strict: true,
            },
        }
    }

    pub(crate) fn with_action_policy_revision(mut self, revision: ActionPolicyRevision) -> Self {
        self.action_policy_revision = revision;
        self
    }

    fn query(&self, call: &ToolCall) -> Result<CodeRetrievalQuery, CoreError> {
        self.workspace
            .ensure_active()
            .map_err(|error| CoreError::Policy(error.to_string()))?;
        if call.name != self.definition.name {
            return Err(CoreError::Policy(format!(
                "tool is not available: {}",
                call.name
            )));
        }
        let arguments = serde_json::from_value::<SearchCodeArguments>(call.arguments.clone())
            .map_err(|error| {
                CoreError::Policy(format!("invalid search_code arguments: {error}"))
            })?;
        let limit = NonZeroUsize::new(arguments.max_results.unwrap_or(10))
            .filter(|limit| limit.get() <= 20)
            .ok_or_else(|| CoreError::Policy("max_results must be 1..=20".into()))?;
        CodeRetrievalQuery::new(arguments.query, limit)
            .map_err(|error| CoreError::Policy(error.to_string()))
    }

    fn service(&self) -> Result<CodeRetrievalService, CoreError> {
        let cloud = self.cloud.as_ref().filter(|cloud| {
            !matches!(
                cloud.status(),
                Ok(status) if status.deployment_mode == CodeIndexDeploymentMode::LocalOnly
            )
        });
        match (&self.semantic, cloud) {
            (Some(semantic), Some(cloud)) => CodeRetrievalService::local_semantic_with_cloud(
                Arc::clone(&self.index),
                Arc::clone(semantic),
                Arc::clone(cloud),
            ),
            (Some(semantic), None) => {
                CodeRetrievalService::local_semantic(Arc::clone(&self.index), Arc::clone(semantic))
            }
            (None, Some(cloud)) => {
                CodeRetrievalService::hybrid(Arc::clone(&self.index), Arc::clone(cloud))
            }
            (None, None) => return Ok(CodeRetrievalService::local(Arc::clone(&self.index))),
        }
        .map_err(|error| CoreError::Execution(error.to_string()))
    }
}

impl ToolService for CodeRetrievalTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![self.definition.clone()]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        let query = self.query(call)?;
        let canonical = serde_json::to_vec(&json!({
            "root": self.index.root_id().as_str(),
            "query": query.text(),
            "max_results": query.result_limit().get(),
        }))
        .map_err(|error| CoreError::Policy(error.to_string()))?;
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(canonical),
                ActionKind::SystemOperation,
                "retrieve bounded source excerpts from the active workspace code index",
                CapabilitySet::new([Capability::new(
                    CapabilityKind::FileRead,
                    self.workspace.root().canonical_path().display().to_string(),
                )]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, CODE_RETRIEVAL_TOOL_NAME),
            SandboxCompatibility::NotApplicable {
                reason: "code retrieval reads an authorized in-process workspace index".into(),
            },
            self.action_policy_revision.clone(),
        ))
    }

    fn execute(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        if !matches!(
            authorization,
            ToolAuthorization::UnsandboxedGrant { grant_id }
                if grant_id.as_str() == "workspace-code-index-read-only"
        ) && !matches!(
            authorization,
            ToolAuthorization::ExecPolicyGranted(grant)
                if grant.policy_grant().source().rule_id().as_str()
                    == "workspace-code-index-read-only"
        ) {
            return Err(CoreError::Policy(
                "search_code requires the exact read-only code-index grant".into(),
            ));
        }
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let result = self
            .service()?
            .retrieve(&self.query(call)?)
            .map_err(|error| CoreError::Execution(error.to_string()))?;
        let hits = result
            .hits
            .into_iter()
            .map(|hit| {
                json!({
                    "path": hit.reference.relative_path,
                    "start_line": hit.reference.span.start_line,
                    "end_line_exclusive": hit.reference.span.end_line_exclusive,
                    "language": hit.language.id(),
                    "content": hit.content,
                    "origins": hit.origins.into_iter().map(|origin| format!("{origin:?}")).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string_pretty(&json!({
            "hits": hits,
            "degradations": result.degradations.into_iter().map(|item| format!("{item:?}")).collect::<Vec<_>>(),
        }))
        .map(ToolExecutionOutput::Success)
        .map_err(|error| CoreError::Execution(error.to_string()))
    }
}

#[cfg(test)]
#[path = "code_retrieval_tool_tests.rs"]
mod tests;
