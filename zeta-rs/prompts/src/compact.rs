use crate::PromptArtifact;

const COMPACTION_PROMPT_TEXT: &str = include_str!("../templates/compact/summary.md");

/// Shared instructions used to produce a durable continuation checkpoint.
pub const COMPACTION_PROMPT: PromptArtifact = PromptArtifact::new(
    "prompts",
    "context/compaction",
    "context-compaction-v2",
    COMPACTION_PROMPT_TEXT,
);
