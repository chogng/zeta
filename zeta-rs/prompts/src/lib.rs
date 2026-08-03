//! Compile-time embedded, model-facing prompt assets owned by Zeta.
//!
//! This crate owns prompt text, stable asset identity, and prompt revisions. It does not decide
//! whether a prompt is needed, when it is injected, how instruction precedence is resolved, or how
//! a provider-specific request is encoded. Those decisions remain with the caller and the Core
//! context pipeline.

mod artifact;
mod compact;
mod goals;
mod review;
mod system;

pub use artifact::PromptArtifact;
pub use artifact::PromptCategory;
pub use artifact::RenderedPrompt;
pub use compact::COMPACTION_PROMPT;
pub use goals::GOALS_PROMPT;
pub use goals::GoalBudget;
pub use goals::GoalPromptContext;
pub use goals::GoalPromptError;
pub use goals::render_goals_prompt;
pub use review::REVIEW_PROMPT;
pub use system::SYSTEM_PROMPT;

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
