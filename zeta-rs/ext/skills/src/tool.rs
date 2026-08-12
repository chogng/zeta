use crate::SkillRuntime;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use zeta_extension_api::ExtensionError;
use zeta_extension_api::ReadOnlyToolContributor;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
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

pub const SKILLS_READ_TOOL_NAME: &str = "skills-read";

pub(crate) struct SkillToolContributor {
    runtime: Arc<SkillRuntime>,
}

impl SkillToolContributor {
    pub(crate) fn new(runtime: Arc<SkillRuntime>) -> Self {
        Self { runtime }
    }
}

impl ReadOnlyToolContributor for SkillToolContributor {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        Ok(vec![Arc::new(SkillReadTool::new(Arc::clone(
            &self.runtime,
        )))])
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillReadArguments {
    source: String,
    name: String,
}

struct SkillReadTool {
    runtime: Arc<SkillRuntime>,
    definition: ToolDefinition,
}

impl SkillReadTool {
    fn new(runtime: Arc<SkillRuntime>) -> Self {
        Self {
            runtime,
            definition: skill_read_definition(),
        }
    }

    fn execute_now(&self, invocation: &ToolInvocation) -> ToolExecutionOutcome {
        if invocation.binding().exposed_name() != self.definition.name() {
            return not_started(format!(
                "Skill reader cannot execute tool '{}'",
                invocation.binding().exposed_name()
            ));
        }
        if let Err(signal) = invocation.context().cancellation().check() {
            return not_started(signal.reason().to_string());
        }
        let ToolPayload::FunctionArguments(arguments) = invocation.payload() else {
            return not_started("skills-read requires structured function arguments");
        };
        let arguments = match serde_json::from_value::<SkillReadArguments>(arguments.clone()) {
            Ok(arguments) => arguments,
            Err(error) => return not_started(format!("invalid skills-read arguments: {error}")),
        };
        let source = match SkillSourceId::new(arguments.source) {
            Ok(source) => source,
            Err(error) => return not_started(error.to_string()),
        };
        let name = match SkillName::new(arguments.name) {
            Ok(name) => name,
            Err(error) => return not_started(error.to_string()),
        };
        let selected = SkillRef::follow_latest(SkillId::new(source, name));
        let activated = match self.runtime.activate_model_selected(&selected) {
            Ok(activated) => activated,
            Err(error) => {
                return ToolExecutionOutcome::Returned(ToolOutput::error(vec![ToolContent::Text(
                    error,
                )]));
            }
        };
        let activation = activated.activation();
        let output = json!({
            "source": activation.id.source.as_str(),
            "name": activation.id.name.as_str(),
            "content_digest": activation.content_digest.as_str(),
            "instructions": activated.body(),
        });
        ToolExecutionOutcome::Returned(ToolOutput::success(vec![ToolContent::Text(
            output.to_string(),
        )]))
    }
}

impl ToolExecutor for SkillReadTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::ParallelSafe
    }

    fn execute(&self, invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(std::future::ready(self.execute_now(&invocation)))
    }
}

fn skill_read_definition() -> ToolDefinition {
    ToolDefinition::function(
        ToolName::new(SKILLS_READ_TOOL_NAME).expect("static Skill reader tool name is valid"),
        "Load the complete instructions for one available Skill selected from the current metadata catalog. Use the exact source and name shown in <available-skills>.",
        ToolInputSchema::parse(json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Exact source identity shown in the available Skill catalog."
                },
                "name": {
                    "type": "string",
                    "description": "Exact Skill name shown in the available Skill catalog."
                }
            },
            "required": ["source", "name"],
            "additionalProperties": false
        }))
        .expect("static Skill reader schema is valid"),
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::Strict,
        ToolLoading::Eager,
    )
    .expect("static Skill reader definition is valid")
}

fn not_started(message: impl Into<String>) -> ToolExecutionOutcome {
    ToolExecutionOutcome::NotStarted(ToolStartFailure::new(message))
}
