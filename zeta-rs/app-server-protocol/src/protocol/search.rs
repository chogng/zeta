use crate::protocol::environment::SessionDirSelector;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

/// Selects how the directory search query is interpreted by the backend.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ContentSearchPatternKind {
    Literal,
    Regex,
}

/// Selects the case-matching behavior used by one directory search.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ContentSearchCaseSensitivity {
    Smart,
    Sensitive,
    Insensitive,
}

/// Starts one bounded, connection-owned directory content search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchStartParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dir_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<SessionDirSelector>,
    #[schemars(length(min = 1, max = 16384))]
    pub query: String,
    pub pattern_kind: ContentSearchPatternKind,
    pub case_sensitivity: ContentSearchCaseSensitivity,
    #[schemars(length(max = 64))]
    pub include_patterns: Vec<String>,
    #[schemars(length(max = 64))]
    pub exclude_patterns: Vec<String>,
    #[schemars(range(min = 1, max = 5000))]
    pub max_results: usize,
}

/// Identity allocated for one running directory search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchStartResult {
    pub search_id: String,
}

/// Reads a bounded result batch after an already observed match cursor.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchReadParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dir_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<SessionDirSelector>,
    #[schemars(length(min = 1))]
    pub search_id: String,
    pub after_match: usize,
    #[schemars(range(min = 1, max = 200))]
    pub max_matches: usize,
}

/// UTF-16 range within one returned preview line.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchMatchRange {
    pub start: usize,
    pub end: usize,
}

/// One line containing one or more matches in a directory-relative file.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub preview: String,
    pub ranges: Vec<ContentSearchMatchRange>,
}

/// Bounded progress snapshot for a running or completed directory search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchReadResult {
    pub search_id: String,
    pub matches: Vec<ContentSearchMatch>,
    pub next_match: usize,
    pub completed: bool,
    pub limit_hit: bool,
    pub error: Option<String>,
}

/// Cancels and releases one connection-owned directory search.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchCancelParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dir_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session_directory: Option<SessionDirSelector>,
    #[schemars(length(min = 1))]
    pub search_id: String,
}
