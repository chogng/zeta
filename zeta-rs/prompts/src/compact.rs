use crate::artifact::{PromptArtifact, PromptCategory};

const COMPACTION_PROMPT_TEXT: &str = include_str!("../templates/compact/summary.md");

/// The built-in prompt used to produce a continuation summary.
pub const COMPACTION_PROMPT: PromptArtifact = PromptArtifact::new(
    PromptCategory::Compaction,
    "compaction/summary",
    "compaction-v2",
    COMPACTION_PROMPT_TEXT,
);
