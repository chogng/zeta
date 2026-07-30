//! Read-only discovery of importable configuration from supported external coding agents.
//!
//! This crate recognizes documented Codex and Claude configuration locations and produces an
//! immutable [`AgentImportPlan`] for a caller to preview. It does not read candidate contents,
//! mutate Zeta configuration, import credentials, grant permissions, or own Desktop UI.

mod discovery;
mod error;
mod layout;
mod model;

pub use error::AgentImportError;
pub use model::{
    AgentImportCandidate, AgentImportDiagnostic, AgentImportDiagnosticCode, AgentImportLocation,
    AgentImportPlan, ExternalAgent, ImportItemKind, ImportReviewCategory, ImportScope,
};
