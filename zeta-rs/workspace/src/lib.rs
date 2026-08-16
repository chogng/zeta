//! Workspace identity, containment, observed-path projection, and execution trust.
//!
//! This crate deliberately does not own editor, Git, terminal, MCP, LSP, configuration
//! persistence, or trust UI. Hosts establish a [`WorkspaceRoot`], resolve their own trust policy,
//! and require a [`TrustedWorkspace`] before enabling workspace-controlled execution.

mod binding;
mod identity;
mod root;
mod trust;

pub use binding::WorkspaceBinding;
pub use identity::{WorkspaceTrustId, WorkspaceTrustIdError};
pub use root::{WorkspacePathError, WorkspaceRoot};
pub use trust::{
    TrustedWorkspace, WorkspaceAuthorization, WorkspaceCapability, WorkspaceTrustDecision,
    WorkspaceTrustError, WorkspaceTrustSource,
};
