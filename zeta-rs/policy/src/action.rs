use crate::ReviewContext;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use zeta_sandboxing::SandboxPolicy;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    FileRead,
    FileWrite,
    ProcessSpawn,
    Network,
    CredentialUse,
    ExternalMutation,
    SystemConfiguration,
    UserInterface,
}

/// One scoped authority needed by a resolved action.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Capability {
    kind: CapabilityKind,
    scope: String,
}

impl Capability {
    pub fn new(kind: CapabilityKind, scope: impl Into<String>) -> Self {
        Self {
            kind,
            scope: scope.into(),
        }
    }

    pub fn kind(&self) -> &CapabilityKind {
        &self.kind
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }
}

/// Canonically ordered capabilities used for exact grant and recommendation comparisons.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CapabilitySet(BTreeSet<Capability>);

impl CapabilitySet {
    pub fn new(capabilities: impl IntoIterator<Item = Capability>) -> Self {
        Self(capabilities.into_iter().collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.is_subset(&other.0)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.0.iter()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessInvocationKind {
    Direct,
    Shell,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ActionKind {
    LocalProcess(ProcessInvocationKind),
    FileSystemMutation,
    NetworkRequest,
    BrowserInteraction,
    ExternalServiceMutation,
    CredentialUse,
    SystemOperation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSource {
    BuiltInTool,
    Plugin,
    McpServer,
    DynamicTool,
    User,
}

/// Trusted provenance assigned by the host after resolving the exact tool binding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActionProvenance {
    source: ActionSource,
    source_id: String,
}

impl ActionProvenance {
    pub fn new(source: ActionSource, source_id: impl Into<String>) -> Self {
        Self {
            source,
            source_id: source_id.into(),
        }
    }

    pub fn source(&self) -> &ActionSource {
        &self.source
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }
}

/// SHA-256 identity of the host-canonical action, including all security-relevant fields.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ActionDigest(String);

impl ActionDigest {
    pub fn from_canonical_bytes(bytes: impl AsRef<[u8]>) -> Self {
        let digest = Sha256::digest(bytes.as_ref());
        Self(format!("{digest:x}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Revision of the deterministic policy snapshot used for one review.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PolicyRevision(String);

impl PolicyRevision {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A fully materialized action safe to summarize for review.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResolvedAction {
    digest: ActionDigest,
    kind: ActionKind,
    summary: String,
    required_capabilities: CapabilitySet,
}

impl ResolvedAction {
    pub fn new(
        digest: ActionDigest,
        kind: ActionKind,
        summary: impl Into<String>,
        required_capabilities: CapabilitySet,
    ) -> Self {
        Self {
            digest,
            kind,
            summary: summary.into(),
            required_capabilities,
        }
    }

    pub fn digest(&self) -> &ActionDigest {
        &self.digest
    }

    pub fn kind(&self) -> &ActionKind {
        &self.kind
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn required_capabilities(&self) -> &CapabilitySet {
        &self.required_capabilities
    }
}

/// Whether the platform sandbox can enforce the action's requested authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SandboxCompatibility {
    Supported(SandboxPolicy),
    Unsupported { reason: String },
    NotApplicable { reason: String },
}

/// Complete, immutable input to deterministic policy and optional classifier review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionReviewRequest {
    action: ResolvedAction,
    provenance: ActionProvenance,
    sandbox: SandboxCompatibility,
    policy_revision: PolicyRevision,
    context: ReviewContext,
}

impl ActionReviewRequest {
    pub fn new(
        action: ResolvedAction,
        provenance: ActionProvenance,
        sandbox: SandboxCompatibility,
        policy_revision: PolicyRevision,
    ) -> Self {
        Self {
            action,
            provenance,
            sandbox,
            policy_revision,
            context: ReviewContext::default(),
        }
    }

    /// Attaches a compact, secret-free context snapshot for the advisory reviewer.
    pub fn with_context(mut self, context: ReviewContext) -> Self {
        self.context = context;
        self
    }

    pub fn action(&self) -> &ResolvedAction {
        &self.action
    }

    pub fn provenance(&self) -> &ActionProvenance {
        &self.provenance
    }

    pub fn sandbox(&self) -> &SandboxCompatibility {
        &self.sandbox
    }

    pub fn policy_revision(&self) -> &PolicyRevision {
        &self.policy_revision
    }

    pub fn context(&self) -> &ReviewContext {
        &self.context
    }
}
