//! Shared lifecycle contracts for agent runtime extensions.

mod context;
mod contributors;
mod fragment;
mod lifecycle;
mod registry;
pub use lifecycle::IdleContributor;
pub use lifecycle::ItemContributor;
pub use lifecycle::LifecycleObserver;
pub use lifecycle::ThreadContext;
pub use lifecycle::ThreadLifecycle;

pub use context::ContextContributor;
pub use context::ContextEvidence;
pub use context::ContextSourceRequest;
pub use contributors::CapabilityToolContribution;
pub use contributors::CapabilityToolContributor;
pub use contributors::ExtensionToolAuthority;
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
