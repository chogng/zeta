use crate::WorkspaceRoot;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Host-owned source that established one workspace authorization decision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkspaceTrustSource {
    /// The user explicitly approved this exact workspace identity.
    ExplicitUserDecision,
    /// An administrator or organization policy approved this exact workspace identity.
    OrganizationPolicy,
    /// A trusted host composition root fixed this workspace before accepting client requests.
    HostConfiguration,
    /// The host enabled bounded read-only inspection without approving workspace execution.
    RestrictedMode,
}

/// Trust result resolved by the host for one exact [`WorkspaceRoot`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkspaceTrustDecision {
    /// Workspace content may be viewed and edited but cannot activate executable behavior.
    Restricted,
    /// The exact root was approved by the accompanying host-owned source.
    Trusted(WorkspaceTrustSource),
}

/// Workspace behavior capabilities used to enforce the trust boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WorkspaceCapability {
    /// Read repository metadata and bounded read-only content without approving workspace code.
    InspectRepository,
    ExecuteProcess,
    ObserveFileChanges,
    LoadExecutableConfiguration,
    ActivateWorkspaceExtension,
    UseWorkspaceDeclaredTool,
    MutateRepository,
}

impl fmt::Display for WorkspaceCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InspectRepository => {
                formatter.write_str("inspect repository metadata and read-only content")
            }
            Self::ExecuteProcess => formatter.write_str("execute a process"),
            Self::ObserveFileChanges => formatter.write_str("observe file changes"),
            Self::LoadExecutableConfiguration => {
                formatter.write_str("load executable configuration")
            }
            Self::ActivateWorkspaceExtension => {
                formatter.write_str("activate a workspace extension")
            }
            Self::UseWorkspaceDeclaredTool => formatter.write_str("use a workspace-declared tool"),
            Self::MutateRepository => formatter.write_str("mutate a repository"),
        }
    }
}

/// Failure to acquire a trusted-workspace capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceTrustError {
    root: WorkspaceRoot,
    capability: WorkspaceCapability,
}

impl WorkspaceTrustError {
    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn capability(&self) -> WorkspaceCapability {
        self.capability
    }
}

impl fmt::Display for WorkspaceTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "workspace trust is required to {}: {}",
            self.capability,
            self.root.canonical_path().display()
        )
    }
}

impl std::error::Error for WorkspaceTrustError {}

/// One host-resolved trust decision bound to one exact workspace identity.
///
/// Only the host trust-store or policy boundary should construct this value from a trusted
/// decision. Workspace files and ordinary client payloads are inputs to policy, never authority.
#[derive(Debug)]
struct WorkspaceTrustLease {
    active: AtomicBool,
}

impl WorkspaceTrustLease {
    fn new() -> Self {
        Self {
            active: AtomicBool::new(true),
        }
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    fn revoke(&self) {
        self.active.store(false, Ordering::Release);
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceAuthorization {
    root: WorkspaceRoot,
    decision: WorkspaceTrustDecision,
    lease: Arc<WorkspaceTrustLease>,
}

impl WorkspaceAuthorization {
    /// Binds a host-owned decision to one exact canonical root.
    pub fn new(root: WorkspaceRoot, decision: WorkspaceTrustDecision) -> Self {
        Self {
            root,
            decision,
            lease: Arc::new(WorkspaceTrustLease::new()),
        }
    }

    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn decision(&self) -> WorkspaceTrustDecision {
        self.decision
    }

    /// Permanently invalidates every capability token issued from this authorization.
    pub fn revoke(&self) {
        self.lease.revoke();
    }

    /// Reports whether this authorization can still issue or exercise capabilities.
    pub fn is_active(&self) -> bool {
        self.lease.is_active()
    }

    /// Requires one capability and returns a root- and capability-bound token.
    ///
    /// Repository inspection is intentionally available in Restricted mode. Every other
    /// capability still requires a trusted workspace decision.
    pub fn require(
        &self,
        capability: WorkspaceCapability,
    ) -> Result<TrustedWorkspace, WorkspaceTrustError> {
        let permitted = matches!(capability, WorkspaceCapability::InspectRepository)
            || matches!(self.decision, WorkspaceTrustDecision::Trusted(_));
        if !permitted {
            return Err(WorkspaceTrustError {
                root: self.root.clone(),
                capability,
            });
        }
        let source = match self.decision {
            WorkspaceTrustDecision::Trusted(source) => source,
            WorkspaceTrustDecision::Restricted => WorkspaceTrustSource::RestrictedMode,
        };
        if !self.lease.is_active() {
            return Err(WorkspaceTrustError {
                root: self.root.clone(),
                capability,
            });
        };
        Ok(TrustedWorkspace {
            root: self.root.clone(),
            source,
            capability,
            lease: Arc::clone(&self.lease),
        })
    }
}

/// Capability token bound to one exact canonical workspace identity.
///
/// Services should retain this type rather than copying its path into an untyped `PathBuf`. The
/// token carries the host decision source and capability for diagnostics and audit.
#[derive(Clone, Debug)]
pub struct TrustedWorkspace {
    root: WorkspaceRoot,
    source: WorkspaceTrustSource,
    capability: WorkspaceCapability,
    lease: Arc<WorkspaceTrustLease>,
}

impl TrustedWorkspace {
    /// Resolves a host decision into a root- and capability-bound token.
    pub fn require(
        root: WorkspaceRoot,
        decision: WorkspaceTrustDecision,
        capability: WorkspaceCapability,
    ) -> Result<Self, WorkspaceTrustError> {
        WorkspaceAuthorization::new(root, decision).require(capability)
    }

    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn source(&self) -> WorkspaceTrustSource {
        self.source
    }

    pub fn capability(&self) -> WorkspaceCapability {
        self.capability
    }

    /// Fails closed after the host revokes the authorization that issued this token.
    pub fn ensure_active(&self) -> Result<(), WorkspaceTrustError> {
        if self.lease.is_active() {
            Ok(())
        } else {
            Err(WorkspaceTrustError {
                root: self.root.clone(),
                capability: self.capability,
            })
        }
    }

    /// Reports whether the issuing host authorization remains active.
    pub fn is_active(&self) -> bool {
        self.lease.is_active()
    }
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
