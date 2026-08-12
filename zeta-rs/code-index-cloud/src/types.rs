use std::num::NonZeroU64;
use std::num::NonZeroUsize;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use zeta_code_index::ChunkReference;
use zeta_code_index::MaterializedChunk;

use crate::CloudCodeIndexError;

macro_rules! text_identity {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CloudCodeIndexError> {
                let value = value.into();
                validate_text(&value, $label)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_identity!(CloudCodeIndexGrantId, "grant identity");
text_identity!(CloudCodeIndexProviderId, "provider identity");

/// User-visible deployment choice for code-index retrieval.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CodeIndexDeploymentMode {
    LocalOnly,
    Cloud,
}

/// Deletion guarantee required before any cloud grant can become active.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudCodeIndexDeletionSupport {
    IdempotentGrantDeletion,
    Unsupported,
}

/// Static provider capabilities checked before a grant is persisted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CloudCodeIndexCapabilities {
    pub deletion: CloudCodeIndexDeletionSupport,
}

/// Provider, tenant, and provider-owned collection selected by the user.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloudCodeIndexDestination {
    pub provider: CloudCodeIndexProviderId,
    pub tenant: String,
    pub collection: String,
}

impl CloudCodeIndexDestination {
    pub fn new(
        provider: CloudCodeIndexProviderId,
        tenant: impl Into<String>,
        collection: impl Into<String>,
    ) -> Result<Self, CloudCodeIndexError> {
        let tenant = tenant.into();
        let collection = collection.into();
        validate_text(&tenant, "tenant")?;
        validate_text(&collection, "collection")?;
        Ok(Self {
            provider,
            tenant,
            collection,
        })
    }
}

/// Workspace-relative source selection covered by one durable egress grant.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CloudCodeIndexSelection {
    EntireIndex,
    PathPrefixes(Vec<PathBuf>),
}

impl CloudCodeIndexSelection {
    pub fn path_prefixes(prefixes: Vec<PathBuf>) -> Result<Self, CloudCodeIndexError> {
        if prefixes.is_empty() {
            return Err(CloudCodeIndexError::InvalidInput(
                "path-prefix selection must not be empty",
            ));
        }
        for prefix in &prefixes {
            validate_relative_prefix(prefix)?;
        }
        Ok(Self::PathPrefixes(prefixes))
    }

    pub(crate) fn includes(&self, path: &Path) -> bool {
        match self {
            Self::EntireIndex => true,
            Self::PathPrefixes(prefixes) => prefixes.iter().any(|prefix| path.starts_with(prefix)),
        }
    }
}

/// Durable, root-bound user consent with a source-content byte ceiling.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloudCodeIndexGrant {
    pub id: CloudCodeIndexGrantId,
    pub root_id: String,
    pub destination: CloudCodeIndexDestination,
    pub selection: CloudCodeIndexSelection,
    pub max_egress_bytes: NonZeroU64,
}

/// Whether the current local generation fits within the proposed egress byte ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudCodeIndexLimitDisposition {
    WithinLimit,
    ExceedsLimit,
}

/// Exact local preview of source-content units and bytes for the current generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCodeIndexPreview {
    pub local_generation: u64,
    pub file_count: usize,
    pub chunk_count: usize,
    pub upload_unit_count: usize,
    pub egress_bytes: u64,
    pub limit: CloudCodeIndexLimitDisposition,
}

/// Durable cloud publication/deletion phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CloudCodeIndexState {
    LocalOnly,
    Granted,
    Syncing,
    Ready,
    Stale,
    Revoking,
    Failed,
}

/// Current deployment choice and local/remote generation relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCodeIndexStatus {
    pub deployment_mode: CodeIndexDeploymentMode,
    pub state: CloudCodeIndexState,
    pub grant: Option<CloudCodeIndexGrant>,
    pub local_generation: u64,
    pub synced_local_generation: Option<u64>,
    pub remote_generation: Option<String>,
}

/// Persistence location for egress consent and deletion recovery state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudCodeIndexStorage {
    Memory,
    Persistent(PathBuf),
}

/// Provider publication acknowledgement. The generation must be opaque and non-secret.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCodeIndexPublication {
    pub remote_generation: String,
}

/// Bounded semantic query issued against the exact remote generation recorded by the grant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCodeIndexQuery {
    text: String,
    result_limit: NonZeroUsize,
}

impl CloudCodeIndexQuery {
    pub fn new(
        text: impl Into<String>,
        result_limit: NonZeroUsize,
    ) -> Result<Self, CloudCodeIndexError> {
        let text = text.into();
        if text.trim().is_empty() || text.len() > 8 * 1024 {
            return Err(CloudCodeIndexError::InvalidInput(
                "cloud query must contain at most 8192 bytes of text",
            ));
        }
        if result_limit.get() > 100 {
            return Err(CloudCodeIndexError::InvalidInput(
                "cloud query result limit must not exceed 100",
            ));
        }
        Ok(Self { text, result_limit })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn result_limit(&self) -> NonZeroUsize {
        self.result_limit
    }
}

/// Provider query input bound to the durable grant and one exact remote generation.
#[derive(Clone, Debug)]
pub struct CloudCodeIndexQueryRequest {
    pub grant: CloudCodeIndexGrant,
    pub remote_generation: String,
    pub query: CloudCodeIndexQuery,
}

/// One cloud retrieval candidate at its provider-ranked position in the query result.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudCodeIndexCandidate {
    pub reference: ChunkReference,
}

/// Provider-ranked query result, including the generation that actually served the candidates.
///
/// Candidate order is canonical. The cloud CodeIndex service must finish semantic recall,
/// reranking, filtering, and truncation before returning this result.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudCodeIndexQueryResult {
    pub remote_generation: String,
    pub candidates: Vec<CloudCodeIndexCandidate>,
}

/// Locally chunked, revision-verified source fragments sent for cloud embedding/vector storage.
#[derive(Clone, Debug)]
pub struct CloudCodeIndexPublicationRequest {
    pub grant: CloudCodeIndexGrant,
    pub local_generation: u64,
    pub chunks: Vec<MaterializedChunk>,
}

fn validate_text(value: &str, _label: &'static str) -> Result<(), CloudCodeIndexError> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(CloudCodeIndexError::InvalidInput(
            "cloud identity text must be 1..=256 bytes without control characters",
        ));
    }
    Ok(())
}

fn validate_relative_prefix(prefix: &Path) -> Result<(), CloudCodeIndexError> {
    if prefix.as_os_str().is_empty()
        || prefix.is_absolute()
        || prefix
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CloudCodeIndexError::InvalidInput(
            "egress path prefixes must be non-empty normalized relative paths",
        ));
    }
    Ok(())
}
