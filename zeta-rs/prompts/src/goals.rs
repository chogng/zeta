use crate::artifact::{PromptArtifact, PromptCategory, RenderedPrompt};
use std::fmt;

const GOALS_PROMPT_TEXT: &str = include_str!("../templates/goals/active.md");

/// The built-in prompt used while an active task goal is present.
pub const GOALS_PROMPT: PromptArtifact = PromptArtifact::new(
    PromptCategory::Goals,
    "goals/active",
    "goals-v2",
    GOALS_PROMPT_TEXT,
);

/// The budget state made visible to an active-goal prompt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalBudget {
    /// The goal has no token budget.
    Unbounded,
    /// The goal has a fixed budget and has consumed the supplied number of tokens.
    Limited {
        /// The total token budget for the goal.
        token_budget: u64,
        /// The number of tokens already consumed by the goal.
        tokens_used: u64,
    },
}

/// The prompt-specific projection of an active goal.
///
/// Callers own the goal lifecycle and must project their domain state into this value before
/// rendering. The goal text is task data; it cannot raise its own instruction precedence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoalPromptContext<'a> {
    objective: &'a str,
    budget: GoalBudget,
}

impl<'a> GoalPromptContext<'a> {
    /// Creates a goal prompt context and rejects an empty objective.
    pub fn new(objective: &'a str, budget: GoalBudget) -> Result<Self, GoalPromptError> {
        if objective.trim().is_empty() {
            return Err(GoalPromptError::EmptyObjective);
        }
        Ok(Self { objective, budget })
    }

    /// Returns the user-provided objective.
    pub const fn objective(&self) -> &'a str {
        self.objective
    }

    /// Returns the budget snapshot.
    pub const fn budget(&self) -> GoalBudget {
        self.budget
    }
}

/// Errors raised while constructing an active-goal prompt context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalPromptError {
    /// The active goal has no usable objective text.
    EmptyObjective,
}

impl fmt::Display for GoalPromptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObjective => formatter.write_str("active goal objective must not be empty"),
        }
    }
}

impl std::error::Error for GoalPromptError {}

/// Renders the active-goal prompt with its objective and budget snapshot.
pub fn render_goals_prompt(context: GoalPromptContext<'_>) -> RenderedPrompt {
    let objective = escape_xml_text(context.objective());
    let budget = budget_text(context.budget());
    let body = GOALS_PROMPT
        .body()
        .replace("{{ budget }}", &budget)
        .replace("{{ objective }}", &objective);
    GOALS_PROMPT.render(body)
}

fn budget_text(budget: GoalBudget) -> String {
    match budget {
        GoalBudget::Unbounded => "mode: unbounded".to_string(),
        GoalBudget::Limited {
            token_budget,
            tokens_used,
        } => format!(
            "mode: limited\ntoken budget: {token_budget}\ntokens used: {tokens_used}\ntokens remaining: {}",
            token_budget.saturating_sub(tokens_used)
        ),
    }
}

fn escape_xml_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "goals_tests.rs"]
mod tests;
