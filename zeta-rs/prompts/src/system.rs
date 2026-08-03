use crate::artifact::{PromptArtifact, PromptCategory};

const SYSTEM_PROMPT_TEXT: &str = include_str!("../templates/system/base.md");

/// The built-in system-level baseline for a Zeta Agent invocation.
pub const SYSTEM_PROMPT: PromptArtifact = PromptArtifact::new(
    PromptCategory::System,
    "system/base",
    "system-v2",
    SYSTEM_PROMPT_TEXT,
);
