//! Read-only filesystem operations as one model-visible tool.
//!
//! Mutation is deliberately absent from this crate. Hosts register `zeta-apply-patch` separately
//! when a model needs validated workspace writes.

use serde::Deserialize;
use serde_json::json;
use std::fmt;
use std::fs::{self, File};
use std::future;
use std::io::Read;
use std::path::PathBuf;
use zeta_sandboxing::WorkspaceRoot;
use zeta_tools::{
    ToolConcurrency, ToolDefinition, ToolExecutionFuture, ToolExecutionOutcome, ToolExecutor,
    ToolInputSchema, ToolInvocation, ToolLoading, ToolName, ToolOutput, ToolOutputSchema,
    ToolPayload, ToolSchemaMode, ToolStartFailure,
};

const DEFAULT_MAX_READ_BYTES: usize = 64 * 1024;
const DEFAULT_MAX_LIST_ENTRIES: usize = 1_000;

/// Bounded result limits for the read-only file-system tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileSystemLimits {
    max_read_bytes: usize,
    max_list_entries: usize,
}

impl FileSystemLimits {
    pub fn new(max_read_bytes: usize, max_list_entries: usize) -> Result<Self, FileSystemError> {
        if max_read_bytes == 0 {
            return Err(FileSystemError::InvalidLimit {
                kind: "maximum read bytes",
            });
        }
        if max_list_entries == 0 {
            return Err(FileSystemError::InvalidLimit {
                kind: "maximum list entries",
            });
        }
        Ok(Self {
            max_read_bytes,
            max_list_entries,
        })
    }

    pub fn max_read_bytes(self) -> usize {
        self.max_read_bytes
    }

    pub fn max_list_entries(self) -> usize {
        self.max_list_entries
    }
}

impl Default for FileSystemLimits {
    fn default() -> Self {
        Self {
            max_read_bytes: DEFAULT_MAX_READ_BYTES,
            max_list_entries: DEFAULT_MAX_LIST_ENTRIES,
        }
    }
}

/// Error raised while configuring the file-system tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSystemError {
    InvalidLimit { kind: &'static str },
    Definition(String),
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { kind } => write!(formatter, "{kind} must be greater than zero"),
            Self::Definition(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for FileSystemError {}

/// Reads, lists, or describes an existing workspace path without modifying it.
pub struct FileSystemTool {
    environment_id: zeta_tools::ToolEnvironmentId,
    workspace: WorkspaceRoot,
    limits: FileSystemLimits,
    definition: ToolDefinition,
}

impl FileSystemTool {
    pub fn new(
        environment_id: zeta_tools::ToolEnvironmentId,
        workspace: WorkspaceRoot,
        limits: FileSystemLimits,
    ) -> Result<Self, FileSystemError> {
        Ok(Self {
            environment_id,
            workspace,
            limits,
            definition: file_system_definition()?,
        })
    }

    fn run(&self, invocation: ToolInvocation) -> ToolExecutionOutcome {
        if invocation.context().environment_id() != &self.environment_id {
            return not_started("tool invocation selected a different local environment");
        }
        if let Err(outcome) = validate_invocation(&self.definition, &invocation) {
            return outcome;
        }
        let input: FileSystemInput = match decode_arguments(&invocation) {
            Ok(input) => input,
            Err(outcome) => return outcome,
        };
        let path = match self.workspace.resolve_existing(&input.path) {
            Ok(path) => path,
            Err(error) => return returned_error(format!("path is not available: {error}")),
        };

        match input.operation {
            FileSystemOperation::Read => self.read(path, input.path),
            FileSystemOperation::List => self.list(path, input.path),
            FileSystemOperation::Metadata => self.metadata(path, input.path),
        }
    }

    fn read(&self, path: PathBuf, requested_path: PathBuf) -> ToolExecutionOutcome {
        match read_text_bounded(&path, self.limits.max_read_bytes()) {
            Ok(text) => returned_json(json!({
                "tool": "file-system",
                "result": {"operation": "read", "path": requested_path, "content": text}
            })),
            Err(error) => returned_error(format!("could not read file: {error}")),
        }
    }

    fn list(&self, path: PathBuf, requested_path: PathBuf) -> ToolExecutionOutcome {
        if !path.is_dir() {
            return returned_error("list requires a directory path");
        }
        let mut entries = match fs::read_dir(&path) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| {
                    let file_type = entry.file_type().ok();
                    let kind = match file_type {
                        Some(file_type) if file_type.is_dir() => "directory",
                        Some(file_type) if file_type.is_file() => "file",
                        Some(file_type) if file_type.is_symlink() => "symlink",
                        _ => "other",
                    };
                    (entry.file_name().to_string_lossy().into_owned(), kind)
                })
                .collect::<Vec<_>>(),
            Err(error) => return returned_error(format!("could not list directory: {error}")),
        };
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        let truncated = entries.len() > self.limits.max_list_entries();
        entries.truncate(self.limits.max_list_entries());
        let entries = entries
            .into_iter()
            .map(|(name, kind)| json!({"name": name, "kind": kind}))
            .collect::<Vec<_>>();
        returned_json(json!({
            "tool": "file-system",
            "result": {
                "operation": "list",
                "path": requested_path,
                "entries": entries,
                "truncated": truncated,
            }
        }))
    }

    fn metadata(&self, path: PathBuf, requested_path: PathBuf) -> ToolExecutionOutcome {
        match fs::metadata(&path) {
            Ok(metadata) => returned_json(json!({
                "tool": "file-system",
                "result": {
                    "operation": "metadata",
                    "path": requested_path,
                    "kind": if metadata.is_dir() { "directory" } else if metadata.is_file() { "file" } else { "other" },
                    "size_bytes": metadata.len(),
                    "readonly": metadata.permissions().readonly(),
                }
            })),
            Err(error) => returned_error(format!("could not inspect path: {error}")),
        }
    }
}

impl ToolExecutor for FileSystemTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    fn concurrency(&self) -> ToolConcurrency {
        ToolConcurrency::ParallelSafe
    }

    fn execute(&self, invocation: ToolInvocation) -> ToolExecutionFuture<'_> {
        Box::pin(future::ready(self.run(invocation)))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum FileSystemOperation {
    Read,
    List,
    Metadata,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSystemInput {
    operation: FileSystemOperation,
    path: PathBuf,
}

fn file_system_definition() -> Result<ToolDefinition, FileSystemError> {
    ToolDefinition::function(
        ToolName::new("file-system").map_err(definition_error)?,
        "Read an existing workspace file, list one workspace directory, or inspect path metadata. This tool never modifies files.",
        ToolInputSchema::parse(json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "enum": ["read", "list", "metadata"] },
                "path": { "type": "string", "description": "Relative path within the selected workspace." }
            },
            "required": ["operation", "path"],
            "additionalProperties": false
        }))
        .map_err(definition_error)?,
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .map_err(definition_error)
}

fn definition_error(error: impl fmt::Display) -> FileSystemError {
    FileSystemError::Definition(error.to_string())
}

fn read_text_bounded(path: &PathBuf, maximum: usize) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(maximum.min(8 * 1024));
    file.by_ref()
        .take((maximum + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > maximum {
        return Err(format!("file exceeds the {maximum}-byte read limit"));
    }
    String::from_utf8(bytes).map_err(|_| "file is not valid UTF-8 text".to_owned())
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

#[cfg(test)]
#[path = "file_system_tests.rs"]
mod tests;
