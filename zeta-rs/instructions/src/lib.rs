//! Bounded discovery and immutable snapshots for Zeta Instruction artifacts.
//!
//! This crate owns the directory layout below `.zeta/instructions`, frontmatter
//! validation, loading policy, deterministic diagnostics, and bounded content reads. It does not
//! assemble model requests, watch files, interpret external Agent formats, or own directory access.

mod catalog;
mod model;

pub use catalog::InstructionCatalog;
pub use model::InstructionArtifact;
pub use model::InstructionCatalogSnapshot;
pub use model::InstructionDiagnostic;
pub use model::InstructionDiagnosticCode;
pub use model::InstructionLoadPolicy;
