use crate::protocol::resources::ResourceMetadataResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

/// Stable filesystem entry kind exposed to clients.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FsFileType {
    Directory,
    File,
    SymbolicLink,
    Other,
}

/// Read metadata for one path relative to the configured workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsGetMetadataParams {
    pub path: PathBuf,
}

/// Metadata returned for one existing workspace path.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsGetMetadataResult {
    pub file_type: FsFileType,
    #[ts(type = "number")]
    pub size_bytes: u64,
    pub readonly: bool,
    #[ts(type = "number | null")]
    pub modified_at_millis: Option<u64>,
}

/// List direct children for one directory relative to the configured workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryParams {
    pub path: PathBuf,
}

/// One direct child returned by `fs/readDirectory`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryEntry {
    pub name: String,
    pub file_type: FsFileType,
}

/// Direct children returned by `fs/readDirectory`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadDirectoryResult {
    pub entries: Vec<FsReadDirectoryEntry>,
}

/// Read one UTF-8 file relative to the configured workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadFileParams {
    pub path: PathBuf,
}

/// UTF-8 text returned by `fs/readFile`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadFileResult {
    pub content: String,
    /// Opaque exact-content revision required to protect a later conditional write.
    pub revision: String,
}

/// Read one binary file relative to the configured workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadBinaryFileParams {
    pub path: PathBuf,
}

/// Connection-owned binary resource opened from one workspace file.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsReadBinaryFileResult {
    pub resource: ResourceMetadataResult,
    /// Opaque exact-content revision of the resource bytes.
    pub revision: String,
}

/// Atomically write one UTF-8 file relative to the configured workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteFileParams {
    pub path: PathBuf,
    pub content: String,
    /// When supplied, rejects the write if the file no longer has this exact revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub expected_revision: Option<String>,
}

/// Metadata returned after one successful `fs/writeFile`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FsWriteFileResult {
    pub metadata: FsGetMetadataResult,
    /// Opaque exact-content revision of the successfully written file.
    pub revision: String,
}

/// Coarse workspace filesystem invalidation published by `fs/changed`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type")]
#[ts(tag = "type")]
pub enum FsChanged {
    /// The backend observed changes near these sorted workspace-relative paths.
    PathsChanged { paths: Vec<PathBuf> },
    /// The watcher may have lost events and consumers must rescan their visible scope.
    RescanRequired,
}
