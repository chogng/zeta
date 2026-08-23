//! Skill discovery and context orchestration as a shared agent runtime extension.

mod catalog_prompt;
mod extension;
mod runtime;
mod selector;
mod tool;
mod watcher;

pub use extension::install;
pub use runtime::BuiltInSkillSource;
pub use runtime::DynamicSkillSourceProvider;
pub use runtime::DynamicSkillSourceSnapshot;
pub use runtime::SkillCatalogReload;
pub use runtime::SkillConfigSnapshotProvider;
pub use runtime::SkillRuntime;
pub use runtime::SkillRuntimeDiagnostic;
pub use runtime::SkillRuntimeEntry;
pub use runtime::SkillRuntimeEventSink;
pub use runtime::SkillRuntimeSnapshot;
pub use tool::SKILLS_READ_TOOL_NAME;
pub use watcher::SkillWatcher;
pub use zeta_skills::SkillCompatibility;
pub use zeta_skills::SkillDiagnosticCode;
pub use zeta_skills::SkillSourceKind;

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
