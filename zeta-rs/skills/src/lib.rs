//! Bounded discovery and metadata-only catalogs for Agent Skills.
//!
//! This crate owns controlled Skill source roots, Agent Skills frontmatter validation, exact
//! `SKILL.md` digests, isolated diagnostics, and immutable catalog generations. It does not
//! activate Skill instructions, read referenced resources, execute scripts, or own configuration.

mod catalog;
mod compatibility;
mod diagnostic;
mod error;
mod format;
mod identity;
mod source;

pub use catalog::{
    SkillAvailability, SkillCatalog, SkillCatalogEntry, SkillCatalogSnapshot, SkillMetadata,
};
pub use compatibility::SkillCompatibility;
pub use diagnostic::{SkillDiagnostic, SkillDiagnosticCode};
pub use error::{SkillError, SkillErrorKind};
pub use identity::{ContentDigest, InvalidContentDigest, SkillCatalogGeneration};
pub use source::{SkillSourceKind, SkillSourceRoot, SkillSourceView, SkillTrust};
pub use zeta_protocol::{
    InvalidSkillName, InvalidSkillSourceId, SkillId, SkillName, SkillSourceId,
};

#[cfg(test)]
#[path = "built_in_assets_tests.rs"]
mod built_in_assets_tests;
