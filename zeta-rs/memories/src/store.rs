use crate::Memory;
use crate::MemoryDeleteResult;
use crate::MemoryId;
use crate::MemoryMutationResult;
use crate::MemoryScope;
use std::fmt;
use zeta_protocol::CommandId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryAddCommit {
    pub command_id: CommandId,
    pub fingerprint: String,
    pub memory: Memory,
    pub normalized_search_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryDeleteCommit {
    pub command_id: CommandId,
    pub fingerprint: String,
    pub memory_id: MemoryId,
    pub scope: MemoryScope,
    pub expected_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryStoreListRequest {
    pub scope: MemoryScope,
    pub expected_catalog_revision: Option<u64>,
    pub after: Option<MemoryId>,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryStoreSearchRequest {
    pub scope: MemoryScope,
    pub expected_catalog_revision: Option<u64>,
    pub after: Option<MemoryId>,
    pub normalized_query: String,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryStorePage {
    pub catalog_revision: u64,
    pub memories: Vec<Memory>,
    pub has_more: bool,
}

/// Persists complete Memory values and retry-safe mutation receipts.
///
/// Implementations atomically update the catalog revision with each mutation, reject reuse of a
/// deleted Memory ID, and retain no title/body in live rows, receipts, or tombstones after delete.
pub trait MemoryStore: Send + Sync {
    fn policy(&self, scope: &MemoryScope) -> Result<crate::MemoryPolicy, MemoryStoreError>;
    fn update_policy(
        &self,
        commit: &MemoryPolicyCommit,
    ) -> Result<crate::MemoryPolicyMutationResult, MemoryStoreError>;
    /// Reads policy and matching content in one storage snapshot. Disabled scopes must not load
    /// bodies; deletion and consent changes committed before this snapshot are immediately visible.
    fn context(&self, request: &MemoryStoreContextRequest)
    -> Result<Vec<Memory>, MemoryStoreError>;
    fn add(&self, commit: &MemoryAddCommit) -> Result<MemoryMutationResult, MemoryStoreError>;
    fn delete(&self, commit: &MemoryDeleteCommit) -> Result<MemoryDeleteResult, MemoryStoreError>;
    fn read(&self, scope: &MemoryScope, memory_id: &MemoryId) -> Result<Memory, MemoryStoreError>;
    /// Reads an opted-in Memory and its policy in one snapshot, before loading any body.
    fn read_for_context(
        &self,
        scope: &MemoryScope,
        memory_id: &MemoryId,
    ) -> Result<Memory, MemoryStoreError>;
    fn list(&self, request: &MemoryStoreListRequest) -> Result<MemoryStorePage, MemoryStoreError>;
    fn search(
        &self,
        request: &MemoryStoreSearchRequest,
    ) -> Result<MemoryStorePage, MemoryStoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryStoreError {
    ReadDenied,
    NotFound,
    AlreadyExists,
    CommandConflict,
    RevisionConflict { expected: u64, actual: u64 },
    StaleCursor { expected: u64, actual: u64 },
    Storage(String),
}

impl fmt::Display for MemoryStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadDenied => {
                formatter.write_str("Memory scope is not enabled for model reading")
            }
            Self::NotFound => formatter.write_str("Memory was not found"),
            Self::AlreadyExists => formatter.write_str("Memory ID already exists or was deleted"),
            Self::CommandConflict => {
                formatter.write_str("Memory command ID was reused with different input")
            }
            Self::RevisionConflict { expected, actual } => {
                write!(
                    formatter,
                    "Memory revision conflict: expected {expected}, actual {actual}"
                )
            }
            Self::StaleCursor { expected, actual } => {
                write!(
                    formatter,
                    "Memory cursor is stale: expected catalog {expected}, actual {actual}"
                )
            }
            Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MemoryStoreError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPolicyCommit {
    pub request: crate::UpdateMemoryPolicyRequest,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryStoreContextRequest {
    pub scopes: Vec<MemoryScope>,
    pub normalized_terms: Vec<String>,
    pub limit: usize,
}
