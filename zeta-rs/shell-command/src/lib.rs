//! Approved local process execution as one model-visible tool.
//!
//! This crate owns the `shell-command` schema and executor. It delegates process approval,
//! output capture, timeout enforcement, and working-directory containment to `zeta-exec`.

use serde::Deserialize;
use serde_json::json;
use std::fmt;
use std::future;
use std::path::PathBuf;
use zeta_exec::{CommandExecutor, CommandRequest, ExecutionError};
use zeta_sandboxing::WorkspaceRoot;
use zeta_tools::{
    ToolConcurrency, ToolDefinition, ToolExecutionFuture, ToolExecutionOutcome, ToolExecutor,
    ToolInputSchema, ToolInvocation, ToolLoading, ToolName, ToolOutput, ToolOutputSchema,
    ToolPayload, ToolSchemaMode, ToolStartFailure,
};

pub use zeta_exec::{ApprovalPolicy, ExecutionLimits as ShellCommandLimits};

/// Error raised while constructing the shell-command executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShellCommandToolError {
    Definition(String),
}

impl fmt::Display for ShellCommandToolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Definition(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ShellCommandToolError {}

/// Executes one approved local process with an explicit program, arguments, and workspace path.
///
/// The selected environment ID must match the immutable ID configured at construction. The
/// executor never starts a shell implicitly: callers must choose the program they intend to run.
pub struct ShellCommandTool<P> {
    environment_id: zeta_tools::ToolEnvironmentId,
    executor: CommandExecutor<P>,
    definition: ToolDefinition,
}

impl<P: ApprovalPolicy> ShellCommandTool<P> {
    pub fn new(
        environment_id: zeta_tools::ToolEnvironmentId,
        workspace: WorkspaceRoot,
        approval_policy: P,
        limits: ShellCommandLimits,
    ) -> Result<Self, ShellCommandToolError> {
        Ok(Self {
            environment_id,
            executor: CommandExecutor::new(workspace, approval_policy, limits),
            definition: shell_command_definition()?,
        })
    }

    fn run(&self, invocation: ToolInvocation) -> ToolExecutionOutcome {
        if invocation.context().environment_id() != &self.environment_id {
            return not_started("tool invocation selected a different local environment");
        }
        if let Err(outcome) = validate_invocation(&self.definition, &invocation) {
            return outcome;
        }
        let input: ShellCommandInput = match decode_arguments(&invocation) {
            Ok(input) => input,
            Err(outcome) => return outcome,
        };
        if input.program.trim().is_empty() {
            return returned_error("program must not be empty");
        }

        match self.executor.execute(CommandRequest {
            program: input.program,
            arguments: input.arguments,
            working_directory: input.working_directory,
        }) {
            Ok(output) => returned_json(json!({
                "tool": "shell-command",
                "result": {
                    "exit_code": output.exit_code,
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                }
            })),
            Err(error) => returned_error(command_error_message(error)),
        }
    }
}

impl<P: ApprovalPolicy> ToolExecutor for ShellCommandTool<P> {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::Exclusive
    }

    fn execute(&self, invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(future::ready(self.run(invocation)))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShellCommandInput {
    program: String,
    arguments: Vec<String>,
    working_directory: PathBuf,
}

fn shell_command_definition() -> Result<ToolDefinition, ShellCommandToolError> {
    ToolDefinition::function(
        ToolName::new("shell-command").map_err(definition_error)?,
        "Run one approved command in a workspace directory. Pass the program and each argument separately; no shell is started implicitly.",
        ToolInputSchema::parse(json!({
            "type": "object",
            "properties": {
                "program": { "type": "string", "description": "Program to execute." },
                "arguments": { "type": "array", "items": { "type": "string" }, "description": "Arguments passed to the program in order." },
                "working_directory": { "type": "string", "description": "Relative workspace directory used as the process working directory." }
            },
            "required": ["program", "arguments", "working_directory"],
            "additionalProperties": false
        }))
        .map_err(definition_error)?,
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .map_err(definition_error)
}

fn definition_error(error: impl fmt::Display) -> ShellCommandToolError {
    ShellCommandToolError::Definition(error.to_string())
}

fn validate_invocation(
    definition: &ToolDefinition,
    invocation: &ToolInvocation,
) -> Result<(), ToolExecutionOutcome> {
    if invocation.context().cancellation().is_cancelled() {
        return Err(not_started(
            "tool invocation was cancelled before it started",
        ));
    }
    if invocation.binding().exposed_name() != definition.name()
        || invocation.binding().definition_digest() != &definition.digest()
    {
        return Err(not_started(
            "tool binding does not match this executor definition",
        ));
    }
    Ok(())
}

fn decode_arguments<T: serde::de::DeserializeOwned>(
    invocation: &ToolInvocation,
) -> Result<T, ToolExecutionOutcome> {
    let ToolPayload::FunctionArguments(arguments) = invocation.payload() else {
        return Err(not_started("tool requires structured function arguments"));
    };
    serde_json::from_value(arguments.clone())
        .map_err(|error| returned_error(format!("invalid tool arguments: {error}")))
}

fn returned_error(message: impl Into<String>) -> ToolExecutionOutcome {
    ToolExecutionOutcome::Returned(ToolOutput::error(vec![zeta_tools::ToolContent::Text(
        message.into(),
    )]))
}

fn returned_json(value: serde_json::Value) -> ToolExecutionOutcome {
    match serde_json::to_string_pretty(&value) {
        Ok(text) => ToolExecutionOutcome::Returned(ToolOutput::success(vec![
            zeta_tools::ToolContent::Text(text),
        ])),
        Err(error) => returned_error(format!("could not encode tool output: {error}")),
    }
}

fn not_started(message: impl Into<String>) -> ToolExecutionOutcome {
    ToolExecutionOutcome::NotStarted(ToolStartFailure::new(message))
}

fn command_error_message(error: ExecutionError) -> String {
    match error {
        ExecutionError::ApprovalRequired => "command requires approval".to_owned(),
        ExecutionError::Denied => "command execution is denied by policy".to_owned(),
        ExecutionError::Spawn(message) => format!("could not execute command: {message}"),
        ExecutionError::TimedOut => "command exceeded its configured timeout".to_owned(),
        ExecutionError::OutputLimitExceeded => {
            "command output exceeded its configured capture limit".to_owned()
        }
        ExecutionError::Sandbox(message) => {
            format!("command working directory is not allowed: {message}")
        }
    }
}

#[cfg(test)]
#[path = "shell_command_tests.rs"]
mod tests;
