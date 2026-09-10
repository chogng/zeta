use crate::PromptArtifact;

/// Shared working rules applied to every Agent, independently of its model or role.
pub const AGENT_INSTRUCTIONS: PromptArtifact = PromptArtifact::new(
    "prompts",
    "agent/common",
    "agent-common-v1",
    include_str!("../templates/agent/common.md"),
);

/// Continuation notice for an interrupted ordinary Turn, without guessing its cause.
pub const TURN_INTERRUPTED_PROMPT: PromptArtifact = PromptArtifact::new(
    "prompts",
    "agent/interrupted",
    "agent-interrupted-v1",
    include_str!("../templates/agent/interrupted.md"),
);
