use crate::protocol::workspace::WorkspaceSessionDirectorySelector;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

/// Selects how the workspace search query is interpreted by the backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceSearchPatternKind {
    Literal,
    Regex,
}

/// Selects the case-matching behavior used by one workspace search.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceSearchCaseSensitivity {
    Smart,
    Sensitive,
    Insensitive,
}

/// Starts one bounded, connection-owned workspace content search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchStartParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub workspace_folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<WorkspaceSessionDirectorySelector>,
    #[schemars(length(min = 1, max = 16384))]
    pub query: String,
    pub pattern_kind: WorkspaceSearchPatternKind,
    pub case_sensitivity: WorkspaceSearchCaseSensitivity,
    #[schemars(length(max = 64))]
    pub include_patterns: Vec<String>,
    #[schemars(length(max = 64))]
    pub exclude_patterns: Vec<String>,
    #[schemars(range(min = 1, max = 5000))]
    pub max_results: usize,
}

/// Identity allocated for one running workspace search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchStartResult {
    pub search_id: String,
}

/// Reads a bounded result batch after an already observed match cursor.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchReadParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub workspace_folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<WorkspaceSessionDirectorySelector>,
    #[schemars(length(min = 1))]
    pub search_id: String,
    pub after_match: usize,
    #[schemars(range(min = 1, max = 200))]
    pub max_matches: usize,
}

/// UTF-16 range within one returned preview line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchMatchRange {
    pub start: usize,
    pub end: usize,
}

/// One line containing one or more matches in a workspace-relative file.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub preview: String,
    pub ranges: Vec<WorkspaceSearchMatchRange>,
}

/// Bounded progress snapshot for a running or completed workspace search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchReadResult {
    pub search_id: String,
    pub matches: Vec<WorkspaceSearchMatch>,
    pub next_match: usize,
    pub completed: bool,
    pub limit_hit: bool,
    pub error: Option<String>,
}

/// Cancels and releases one connection-owned workspace search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchCancelParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub workspace_folder_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<WorkspaceSessionDirectorySelector>,
    #[schemars(length(min = 1))]
    pub search_id: String,
}
