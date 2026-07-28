//! Bounded recursive text search as one model-visible tool.
//!
//! The executor never invokes a shell and does not follow symlinks. It intentionally reports only
//! UTF-8 text matches bounded by configured file, traversal, and result limits.

use serde::Deserialize;
use serde_json::json;
use std::fmt;
use std::fs;
use std::future;
use std::path::{Path, PathBuf};
use zeta_async_utils::CancellationToken;
use zeta_sandboxing::WorkspaceRoot;
use zeta_tools::{
    ToolConcurrency, ToolDefinition, ToolExecutionFuture, ToolExecutionOutcome, ToolExecutor,
    ToolInputSchema, ToolInvocation, ToolLoading, ToolName, ToolOutput, ToolOutputSchema,
    ToolPayload, ToolSchemaMode, ToolStartFailure,
};

const DEFAULT_MAX_FILE_BYTES: usize = 512 * 1024;
const DEFAULT_MAX_RESULTS: usize = 100;
const DEFAULT_MAX_SCANNED_FILES: usize = 10_000;
const MAX_MATCH_LINE_BYTES: usize = 512;

/// Bounds recursive local text search work and model-visible matches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextSearchLimits {
    max_file_bytes: usize,
    max_results: usize,
    max_scanned_files: usize,
}

impl TextSearchLimits {
    pub fn new(
        max_file_bytes: usize,
        max_results: usize,
        max_scanned_files: usize,
    ) -> Result<Self, TextSearchError> {
        for (value, kind) in [
            (max_file_bytes, "maximum searchable file bytes"),
            (max_results, "maximum search results"),
            (max_scanned_files, "maximum scanned files"),
        ] {
            if value == 0 {
                return Err(TextSearchError::InvalidLimit { kind });
            }
        }
        Ok(Self {
            max_file_bytes,
            max_results,
            max_scanned_files,
        })
    }

    pub fn max_file_bytes(self) -> usize {
        self.max_file_bytes
    }

    pub fn max_results(self) -> usize {
        self.max_results
    }

    pub fn max_scanned_files(self) -> usize {
        self.max_scanned_files
    }
}

impl Default for TextSearchLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_results: DEFAULT_MAX_RESULTS,
            max_scanned_files: DEFAULT_MAX_SCANNED_FILES,
        }
    }
}

/// Error raised while configuring the text-search tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextSearchError {
    InvalidLimit { kind: &'static str },
    Definition(String),
}

impl fmt::Display for TextSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { kind } => write!(formatter, "{kind} must be greater than zero"),
            Self::Definition(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for TextSearchError {}

/// Searches text files below an existing workspace path without invoking a shell or following
/// symlinks.
pub struct TextSearchTool {
    environment_id: zeta_tools::ToolEnvironmentId,
    workspace: WorkspaceRoot,
    limits: TextSearchLimits,
    definition: ToolDefinition,
}

impl TextSearchTool {
    pub fn new(
        environment_id: zeta_tools::ToolEnvironmentId,
        workspace: WorkspaceRoot,
        limits: TextSearchLimits,
    ) -> Result<Self, TextSearchError> {
        Ok(Self {
            environment_id,
            workspace,
            limits,
            definition: text_search_definition()?,
        })
    }

    fn run(&self, invocation: ToolInvocation) -> ToolExecutionOutcome {
        if invocation.context().environment_id() != &self.environment_id {
            return not_started("tool invocation selected a different local environment");
        }
        if let Err(outcome) = validate_invocation(&self.definition, &invocation) {
            return outcome;
        }
        let input: TextSearchInput = match decode_arguments(&invocation) {
            Ok(input) => input,
            Err(outcome) => return outcome,
        };
        if input.query.is_empty() {
            return returned_error("query must not be empty");
        }
        let start = match self.workspace.resolve_existing(&input.path) {
            Ok(path) => path,
            Err(error) => return returned_error(format!("search path is not available: {error}")),
        };

        let mut state = SearchState::new(&input, self.limits, invocation.context().cancellation());
        match state.search(&start, self.workspace.path()) {
            Ok(()) => returned_json(json!({
                "tool": "text-search",
                "result": {
                    "query": input.query,
                    "path": input.path,
                    "matches": state.matches,
                    "scanned_files": state.scanned_files,
                    "skipped_large_files": state.skipped_large_files,
                    "skipped_non_text_files": state.skipped_non_text_files,
                    "truncated": state.truncated,
                }
            })),
            Err(SearchFailure::Cancelled) => not_started("text search was cancelled"),
            Err(SearchFailure::Io(error)) => returned_error(format!("text search failed: {error}")),
        }
    }
}

impl ToolExecutor for TextSearchTool {
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
enum SearchCase {
    Sensitive,
    Insensitive,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextSearchInput {
    query: String,
    path: PathBuf,
    case: SearchCase,
}

struct SearchState<'a> {
    query: &'a str,
    case: &'a SearchCase,
    limits: TextSearchLimits,
    cancellation: &'a CancellationToken,
    matches: Vec<serde_json::Value>,
    scanned_files: usize,
    skipped_large_files: usize,
    skipped_non_text_files: usize,
    truncated: bool,
}

impl<'a> SearchState<'a> {
    fn new(
        input: &'a TextSearchInput,
        limits: TextSearchLimits,
        cancellation: &'a CancellationToken,
    ) -> Self {
        Self {
            query: &input.query,
            case: &input.case,
            limits,
            cancellation,
            matches: Vec::new(),
            scanned_files: 0,
            skipped_large_files: 0,
            skipped_non_text_files: 0,
            truncated: false,
        }
    }

    fn search(&mut self, path: &Path, workspace: &Path) -> Result<(), SearchFailure> {
        self.check_cancellation()?;
        let metadata = fs::symlink_metadata(path).map_err(SearchFailure::io)?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        if metadata.is_file() {
            return self.search_file(path, workspace, metadata.len());
        }
        if !metadata.is_dir() {
            return Ok(());
        }
        let mut children = fs::read_dir(path)
            .map_err(SearchFailure::io)?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            if self.truncated {
                break;
            }
            self.search(&child.path(), workspace)?;
        }
        Ok(())
    }

    fn search_file(
        &mut self,
        path: &Path,
        workspace: &Path,
        size: u64,
    ) -> Result<(), SearchFailure> {
        self.check_cancellation()?;
        if self.scanned_files == self.limits.max_scanned_files() {
            self.truncated = true;
            return Ok(());
        }
        self.scanned_files += 1;
        if size > self.limits.max_file_bytes() as u64 {
            self.skipped_large_files += 1;
            return Ok(());
        }
        let bytes = fs::read(path).map_err(SearchFailure::io)?;
        if bytes.contains(&0) {
            self.skipped_non_text_files += 1;
            return Ok(());
        }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                self.skipped_non_text_files += 1;
                return Ok(());
            }
        };
        let relative = path
            .strip_prefix(workspace)
            .map_err(|_| SearchFailure::Io("search result escaped the workspace".to_owned()))?;
        for (line_index, line) in text.lines().enumerate() {
            self.check_cancellation()?;
            for column_byte in self.match_positions(line) {
                if self.matches.len() == self.limits.max_results() {
                    self.truncated = true;
                    return Ok(());
                }
                self.matches.push(json!({
                    "path": relative,
                    "line": line_index + 1,
                    "column_byte": column_byte + 1,
                    "text": truncate_line(line),
                }));
            }
        }
        Ok(())
    }

    fn match_positions(&self, line: &str) -> Vec<usize> {
        match self.case {
            SearchCase::Sensitive => line
                .match_indices(self.query)
                .map(|(index, _)| index)
                .collect(),
            SearchCase::Insensitive => {
                let query = self.query.to_ascii_lowercase();
                line.to_ascii_lowercase()
                    .match_indices(&query)
                    .map(|(index, _)| index)
                    .collect()
            }
        }
    }

    fn check_cancellation(&self) -> Result<(), SearchFailure> {
        if self.cancellation.is_cancelled() {
            Err(SearchFailure::Cancelled)
        } else {
            Ok(())
        }
    }
}

enum SearchFailure {
    Cancelled,
    Io(String),
}

impl SearchFailure {
    fn io(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

fn truncate_line(line: &str) -> String {
    if line.len() <= MAX_MATCH_LINE_BYTES {
        return line.to_owned();
    }
    let mut end = MAX_MATCH_LINE_BYTES;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &line[..end])
}

fn text_search_definition() -> Result<ToolDefinition, TextSearchError> {
    ToolDefinition::function(
        ToolName::new("text-search").map_err(definition_error)?,
        "Search UTF-8 text files below a relative workspace path. Symlinks and oversized or non-text files are skipped.",
        ToolInputSchema::parse(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Non-empty text to find." },
                "path": { "type": "string", "description": "Relative workspace file or directory to search." },
                "case": { "type": "string", "enum": ["sensitive", "insensitive"], "description": "Match case behavior; insensitive matching is ASCII case-insensitive." }
            },
            "required": ["query", "path", "case"],
            "additionalProperties": false
        }))
        .map_err(definition_error)?,
        ToolOutputSchema::Unspecified,
        ToolSchemaMode::ProviderDefault,
        ToolLoading::Eager,
    )
    .map_err(definition_error)
}

fn definition_error(error: impl fmt::Display) -> TextSearchError {
    TextSearchError::Definition(error.to_string())
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
#[path = "text_search_tests.rs"]
mod tests;
