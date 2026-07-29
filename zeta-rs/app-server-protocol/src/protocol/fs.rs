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
}
