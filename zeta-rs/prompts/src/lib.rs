//! Shared prompt infrastructure and stable prompts used by more than one product path.
//!
//! Model-specific and feature-specific prompts stay with their owning crate. This crate owns the
//! common asset contract plus shared compaction and code-review instructions.

mod artifact;
mod compact;
mod review;

pub use artifact::PromptArtifact;
pub use artifact::RenderedPrompt;
pub use compact::COMPACTION_PROMPT;
pub use review::REVIEW_PROMPT;
pub use review::ReviewPromptError;
pub use review::review_target_prompt;

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
