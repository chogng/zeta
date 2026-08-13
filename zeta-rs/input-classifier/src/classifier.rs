use std::path::Path;
use std::path::PathBuf;

use crate::ShellAlias;
use crate::ShellCompletion;
use crate::ShellCompletionSnapshot;
use crate::history::InputHistory;
use crate::history::InputHistoryEntry;
use crate::model::ModelAttempt;
use crate::model::classify_with_embedded_model;
use crate::natural_language::classify_with_fallback_heuristic;
use crate::parser::parse_query_into_tokens;
use crate::rules::classify_allowlisted_input;
use crate::rules::classify_contextual_input;
use crate::shell::ShellContext;

/// Product-neutral destination selected for a piece of terminal input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputRoute {
    #[default]
    Agent,
    Shell,
}

/// Conversation position used to interpret short replies.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputConversation {
    #[default]
    Standalone,
    AgentFollowUp,
}

/// Runtime context for one automatic classification decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputClassificationContext {
    pub current_route: InputRoute,
    pub conversation: InputConversation,
}

impl InputClassificationContext {
    pub const fn new(current_route: InputRoute, conversation: InputConversation) -> Self {
        Self {
            current_route,
            conversation,
        }
    }
}

/// Component that produced an input classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputClassificationSource {
    EmptyInput,
    AgentFollowUp,
    NaturalLanguageOneOff,
    ShellAllowlist,
    ShellTokenHeuristic,
    HistoryMatch,
    Model,
    HeuristicFallback,
    CurrentRouteFallback,
}

/// Route, confidence, and provenance for one classified input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputClassification {
    pub route: InputRoute,
    pub confidence: f32,
    pub source: InputClassificationSource,
}

impl InputClassification {
    pub(crate) const fn deterministic(
        route: InputRoute,
        source: InputClassificationSource,
    ) -> Self {
        Self {
            route,
            confidence: 1.0,
            source,
        }
    }

    const fn fallback(route: InputRoute) -> Self {
        Self {
            route,
            confidence: 0.0,
            source: InputClassificationSource::CurrentRouteFallback,
        }
    }
}

/// Stateful local classifier that owns workspace and executable-resolution context.
#[derive(Clone, Debug)]
pub struct InputClassifier {
    shell_context: ShellContext,
    history: InputHistory,
}

impl Default for InputClassifier {
    fn default() -> Self {
        Self::for_working_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl InputClassifier {
    /// Creates a classifier whose shell evidence is resolved relative to `working_directory`.
    pub fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            shell_context: ShellContext::new(working_directory),
            history: InputHistory::default(),
        }
    }

    /// Rebinds workspace-relative command and path evidence.
    pub fn set_working_directory(&mut self, working_directory: &Path) {
        self.shell_context.set_working_directory(working_directory);
    }

    /// Replaces the executable search path supplied by the active Shell environment.
    pub fn set_shell_path_entries(&mut self, entries: impl IntoIterator<Item = PathBuf>) {
        self.shell_context.set_path_entries(entries);
    }

    /// Replaces aliases supplied by the active Shell environment.
    pub fn replace_shell_aliases(&mut self, aliases: impl IntoIterator<Item = ShellAlias>) {
        self.shell_context.replace_aliases(aliases);
    }

    /// Re-reads workspace-owned package scripts, Just recipes, and Make targets.
    pub fn refresh_shell_workspace(&mut self) {
        self.shell_context.refresh_workspace();
    }

    /// Returns structural Shell completion candidates using the same evidence as classification.
    pub fn shell_completions(&self, input: &str, cursor: usize) -> Vec<ShellCompletion> {
        self.shell_context.complete(input, cursor)
    }

    /// Returns Shell completion candidates and exact-token metadata from the shared context.
    pub fn shell_completion_snapshot(&self, input: &str, cursor: usize) -> ShellCompletionSnapshot {
        self.shell_context.complete_snapshot(input, cursor)
    }

    /// Replaces chronological Shell and Agent submissions used for close-match routing.
    pub fn replace_history(&mut self, entries: impl IntoIterator<Item = InputHistoryEntry>) {
        self.history.replace(entries);
    }

    /// Records one successful submission as the newest history candidate.
    pub fn record_submission(&mut self, entry: InputHistoryEntry) {
        self.history.record(entry);
    }

    /// Runs allowlists, token semantics, and finally the embedded model.
    pub fn classify(
        &self,
        input: &str,
        context: InputClassificationContext,
    ) -> InputClassification {
        classify_with_model(
            input,
            context,
            &self.shell_context,
            &self.history,
            classify_with_embedded_model,
        )
    }
}

fn classify_with_model(
    input: &str,
    context: InputClassificationContext,
    shell_context: &ShellContext,
    history: &InputHistory,
    model: impl FnOnce(&str) -> ModelAttempt,
) -> InputClassification {
    let word_tokens = parse_query_into_tokens(input);
    if let Some(classification) = classify_contextual_input(input, &word_tokens, context) {
        return classification;
    }
    if let Some(classification) = history.classify(input) {
        return classification;
    }
    if let Some(classification) = classify_allowlisted_input(&word_tokens) {
        return classification;
    }

    let shell_snapshot = shell_context.analyze(input);
    if shell_snapshot.is_likely_shell_command(word_tokens.len()) {
        return InputClassification::deterministic(
            InputRoute::Shell,
            InputClassificationSource::ShellTokenHeuristic,
        );
    }

    match model(input.trim()) {
        ModelAttempt::Classified(classification) => classification,
        ModelAttempt::Failed => InputClassification::fallback(context.current_route),
        ModelAttempt::Unavailable | ModelAttempt::Panicked => classify_with_fallback_heuristic(
            input,
            word_tokens,
            &shell_snapshot,
            context.current_route,
        ),
    }
}

#[cfg(test)]
pub(crate) fn classify_with_model_attempt(
    classifier: &InputClassifier,
    input: &str,
    context: InputClassificationContext,
    attempt: ModelAttempt,
) -> InputClassification {
    classify_with_model(
        input,
        context,
        &classifier.shell_context,
        &classifier.history,
        |_| attempt,
    )
}
