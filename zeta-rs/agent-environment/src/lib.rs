//! Immutable Agent-environment values and deterministic model-context rendering.
//!
//! Hosts collect and authorize environment facts. This crate validates those facts, preserves
//! workspace-root ordering, and renders the model-visible environment without owning Session
//! lifecycle, command execution, or access enforcement.

#[path = "render/environment_context.rs"]
mod environment_context;
mod error;
#[path = "model/snapshot.rs"]
mod snapshot;
#[path = "model/workspace_roots.rs"]
mod workspace_roots;

pub use error::AgentEnvironmentError;
pub use snapshot::AgentEnvironmentSnapshot;
pub use snapshot::HostEnvironment;
pub use snapshot::RepositoryEnvironment;
pub use workspace_roots::WorkspaceRoots;
