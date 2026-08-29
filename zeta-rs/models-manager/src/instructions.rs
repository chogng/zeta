use zeta_prompts::PromptArtifact;

const BASE_INSTRUCTIONS_TEXT: &str = include_str!("../templates/instructions/base.md");

/// The model-facing baseline shared by the currently supported coding models.
///
/// Model-specific instruction selection belongs in this crate. The product host freezes the
/// selected asset before durable Turn acceptance; Core only composes that snapshot with context.
pub const BASE_INSTRUCTIONS: PromptArtifact = PromptArtifact::new(
    "models-manager",
    "model/base-instructions",
    "model-base-v1",
    BASE_INSTRUCTIONS_TEXT,
);

#[cfg(test)]
#[path = "instructions_tests.rs"]
mod tests;
