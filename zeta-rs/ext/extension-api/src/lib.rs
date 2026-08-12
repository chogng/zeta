//! Shared lifecycle contracts for agent runtime extensions.

mod contributors;
mod fragment;
mod registry;

pub use contributors::ReadOnlyToolContributor;
pub use contributors::SkillActivationContext;
pub use contributors::SkillActivationContributor;
pub use contributors::TurnInputContext;
pub use contributors::TurnInputContributor;
pub use fragment::PromptFragment;
pub use fragment::PromptFragmentLayer;
pub use fragment::PromptFragmentRetention;
pub use fragment::PromptFragmentSource;
pub use registry::ExtensionError;
pub use registry::ExtensionRegistry;
pub use registry::ExtensionRegistryBuilder;

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
