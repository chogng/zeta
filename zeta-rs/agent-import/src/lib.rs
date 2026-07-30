//! Read-only discovery of importable configuration from supported external coding agents.
//!
//! This crate recognizes documented Codex and Claude configuration locations and produces an
//! immutable [`AgentPathInspection`] for a caller to preview. It does not read candidate contents,
//! mutate Zeta configuration, import credentials, grant permissions, or own Desktop UI.

mod agent_paths;
mod error;
mod import;
mod inspect_path;

pub use error::AgentImportError;
pub use import::{
    AgentImportCandidate, AgentImportDiagnostic, AgentImportDiagnosticCode, AgentImportLocation,
    AgentPathInspection, ExternalAgent, ImportItemKind, ImportReviewCategory, ImportScope,
};
pub use inspect_path::inspect_agent_paths;
