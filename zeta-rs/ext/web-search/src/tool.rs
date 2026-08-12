use std::sync::Arc;

use serde_json::json;
use zeta_tools::ToolConcurrency;
use zeta_tools::ToolContent;
use zeta_tools::ToolDefinition;
use zeta_tools::ToolExecutionFuture;
use zeta_tools::ToolExecutionOutcome;
use zeta_tools::ToolExecutor;
use zeta_tools::ToolInputSchema;
use zeta_tools::ToolInvocation;
use zeta_tools::ToolLoading;
use zeta_tools::ToolName;
use zeta_tools::ToolOutput;
use zeta_tools::ToolOutputSchema;
use zeta_tools::ToolPayload;
use zeta_tools::ToolSchemaMode;
use zeta_tools::ToolStartFailure;

use crate::WebSearchBackend;
use crate::WebSearchRequest;

pub const WEB_SEARCH_TOOL_NAME: &str = "web_search";

pub(crate) struct WebSearchTool {
    backend: Arc<dyn WebSearchBackend>,
    definition: ToolDefinition,
}

impl WebSearchTool {
    pub(crate) fn new(backend: Arc<dyn WebSearchBackend>) -> Self {
        Self {
            backend,
            definition: definition(),
        }
    }
}

impl ToolExecutor for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::ParallelSafe
    }

    fn execute(&self, invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(async move {
            let ToolPayload::FunctionArguments(arguments) = invocation.payload() else {
                return ToolExecutionOutcome::NotStarted(ToolStartFailure::new(
                    "Web Search requires function arguments",
                ));
            };
            let request = match serde_json::from_value::<WebSearchRequest>(arguments.clone()) {
                Ok(request) => request,
                Err(error) => {
                    return ToolExecutionOutcome::NotStarted(ToolStartFailure::new(format!(
                        "invalid Web Search arguments: {error}"
                    )));
                }
            };
            if let Err(error) = request.validate() {
                return ToolExecutionOutcome::NotStarted(ToolStartFailure::new(error.to_string()));
            }
            match self
                .backend
                .search(&request, invocation.context().cancellation())
            {
                Ok(response) => match serde_json::to_string(&response) {
                    Ok(response) => ToolExecutionOutcome::Returned(ToolOutput::success(vec![
                        ToolContent::Text(response),
                    ])),
                    Err(error) => {
                        ToolExecutionOutcome::NotStarted(ToolStartFailure::new(error.to_string()))
                    }
                },
                Err(error) => {
                    ToolExecutionOutcome::Returned(ToolOutput::error(vec![ToolContent::Text(
                        error.to_string(),
                    )]))
                }
            }
        })
    }
}

fn definition() -> ToolDefinition {
    ToolDefinition::function(
        ToolName::new(WEB_SEARCH_TOOL_NAME).expect("Web Search tool name is valid"),
        "Search the public web through the host-configured provider. Supports up to four exact queries with optional domain and recency filters.",
        ToolInputSchema::parse(json!({
            "type": "object",
            "properties": {
                "search_query": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 4,
                    "items": {
                        "type": "object",
                        "properties": {
                            "q": {"type": "string"},
                            "domains": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "recency_days": {"type": "integer", "minimum": 1}
                        },
                        "required": ["q"],
                        "additionalProperties": false
                    }
                },
                "response_length": {
                    "type": "string",
                    "enum": ["short", "medium", "long"]
                }
            },
            "required": ["search_query"],
            "additionalProperties": false
        }))
        .expect("Web Search schema is valid"),
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::Strict,
        ToolLoading::Eager,
    )
    .expect("Web Search definition is valid")
}
