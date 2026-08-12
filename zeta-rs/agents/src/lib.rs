//! Bounded discovery and immutable snapshots for Zeta Agent definition artifacts.
//!
//! This crate owns the native Workspace layout below `.zeta/agents`, definition frontmatter,
//! deterministic diagnostics, and content bounds. It does not create Threads, invoke models,
//! resolve tools, grant permissions, or coordinate multiple Agents.

mod catalog;
mod model;

pub use catalog::AgentDefinitionCatalog;
pub use model::AgentDefinition;
pub use model::AgentDefinitionCatalogSnapshot;
pub use model::AgentDefinitionDiagnostic;
pub use model::AgentDefinitionDiagnosticCode;
