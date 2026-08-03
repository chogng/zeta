use crate::artifact::{PromptArtifact, PromptCategory};

const REVIEW_PROMPT_TEXT: &str = include_str!("../templates/review/code_review.md");

/// The general-purpose prompt used for reviewing code or other requested changes.
pub const REVIEW_PROMPT: PromptArtifact = PromptArtifact::new(
    PromptCategory::Review,
    "review/code",
    "review-v2",
    REVIEW_PROMPT_TEXT,
);
