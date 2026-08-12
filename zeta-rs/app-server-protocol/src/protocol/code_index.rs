use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use ts_rs::TS;

/// Lifecycle state of the workspace-side code-index projection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CodeIndexStateDto {
    Empty,
    Indexing,
    Ready,
    Stale,
    Failed,
}

/// Published counters for the current or most recent usable code-index generation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexStatusResult {
    pub state: CodeIndexStateDto,
    pub root_id: String,
    #[ts(type = "number")]
    pub generation: u64,
    pub indexed_file_count: usize,
    pub indexed_chunk_count: usize,
    pub indexed_source_bytes: usize,
    pub skipped_file_count: usize,
    pub truncated_file_count: usize,
    pub file_limit_hit: bool,
    pub source_bytes_limit_hit: bool,
}

/// Performs one bounded literal lookup against the workspace-side code index.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexSearchParams {
    #[schemars(length(min = 1, max = 8192))]
    pub query: String,
    #[schemars(range(min = 1, max = 100))]
    pub max_results: usize,
}

/// Exact UTF-8 byte and zero-based line coverage of one indexed chunk.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexChunkSpanDto {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line_exclusive: usize,
}

/// One revision-bound lexical match from the local code index.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexSearchHitDto {
    pub path: PathBuf,
    pub language: String,
    pub source_revision: String,
    pub chunk_key: String,
    pub content_hash: String,
    pub span: CodeIndexChunkSpanDto,
    pub content: String,
    pub score: f64,
}

/// Bounded code-index matches paired with the generation that served them.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeIndexSearchResult {
    pub status: CodeIndexStatusResult,
    pub hits: Vec<CodeIndexSearchHitDto>,
}

/// Performs one bounded retrieval across every source enabled for the workspace.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeRetrievalParams {
    #[schemars(length(min = 1, max = 8192))]
    pub query: String,
    #[schemars(range(min = 1, max = 100))]
    pub max_results: usize,
}

/// Candidate source that contributed to one fused retrieval hit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CodeRetrievalOriginDto {
    LocalLexical,
    CloudSemantic,
}

/// Non-fatal loss reported while preserving usable retrieval results.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CodeRetrievalDegradationDto {
    CloudQueryFailed,
    CandidateVerificationFailed { discarded: usize },
    ContentBudgetExceeded { discarded: usize },
}

/// One deduplicated, current-source-verified retrieval hit.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeRetrievalHitDto {
    pub path: PathBuf,
    pub language: String,
    pub source_revision: String,
    pub content_hash: String,
    pub span: CodeIndexChunkSpanDto,
    pub content: String,
    pub rrf_score: f64,
    pub origins: Vec<CodeRetrievalOriginDto>,
}

/// Fused code excerpts and explicit degradation facts paired with local index status.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CodeRetrievalResult {
    pub status: CodeIndexStatusResult,
    pub hits: Vec<CodeRetrievalHitDto>,
    pub degradations: Vec<CodeRetrievalDegradationDto>,
}

/// User-visible code-index deployment choice.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CodeIndexDeploymentModeDto {
    LocalOnly,
    Cloud,
}

/// Durable publication/deletion phase of the selected cloud deployment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CloudCodeIndexStateDto {
    LocalOnly,
    Granted,
    Syncing,
    Ready,
    Stale,
    Revoking,
    Failed,
}

/// Workspace-relative source selection covered by a preview or grant.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CloudCodeIndexSelectionDto {
    EntireIndex,
    PathPrefixes { prefixes: Vec<PathBuf> },
}

/// Non-secret remote provider namespace chosen by the user.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexDestinationDto {
    pub provider: String,
    pub tenant: String,
    pub collection: String,
}

/// Root-bound cloud source-egress grant projected through the App Server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexGrantDto {
    pub grant_id: String,
    pub destination: CloudCodeIndexDestinationDto,
    pub selection: CloudCodeIndexSelectionDto,
    #[schemars(range(min = 1))]
    #[ts(type = "number")]
    pub max_egress_bytes: u64,
}

/// Computes the current source-content egress shape without persisting consent or networking.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexPreviewParams {
    pub selection: CloudCodeIndexSelectionDto,
    #[schemars(range(min = 1))]
    #[ts(type = "number")]
    pub max_egress_bytes: u64,
}

/// Bounded local preview of the Workspace-produced chunks eligible for publication.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexPreviewResult {
    #[ts(type = "number")]
    pub local_generation: u64,
    pub file_count: usize,
    pub chunk_count: usize,
    pub upload_unit_count: usize,
    #[ts(type = "number")]
    pub egress_bytes: u64,
    pub within_limit: bool,
}

/// Persists one explicit cloud destination, selection, and byte ceiling.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexAuthorizeParams {
    pub grant: CloudCodeIndexGrantDto,
}

/// Current cloud deployment and local/remote generation relationship.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CloudCodeIndexStatusResult {
    pub deployment_mode: CodeIndexDeploymentModeDto,
    pub state: CloudCodeIndexStateDto,
    pub root_id: String,
    pub grant: Option<CloudCodeIndexGrantDto>,
    #[ts(type = "number")]
    pub local_generation: u64,
    #[ts(type = "number | null")]
    pub synced_local_generation: Option<u64>,
    pub remote_generation: Option<String>,
}
