use crate::InputClassification;
use crate::InputClassificationSource;
use crate::InputRoute;

const NATURAL_LANGUAGE_INPUTS: &[&str] = &[
    "continue",
    "do it",
    "explain",
    "hello",
    "hey",
    "hi",
    "hola",
    "nice",
    "no",
    "thanks",
    "thank you",
    "what",
    "yes",
    "你好",
    "您好",
    "多谢",
    "谢谢",
];

const SHELL_COMMAND_KEYWORDS: &[&str] = &[
    "#", "claude", "codex", "echo", "gemini", "man", "sudo", "zeta",
];

pub(super) fn classify_deterministic_input(input: &str) -> Option<InputClassification> {
    let input = input.trim();
    if input.is_empty() {
        return Some(InputClassification::deterministic(
            InputRoute::Agent,
            InputClassificationSource::EmptyInput,
        ));
    }
    let normalized = input
        .trim_end_matches(['!', '?', '.', '。', '！', '？'])
        .trim_end()
        .to_lowercase();
    let word_count = normalized.split_whitespace().count();
    let is_natural_language = NATURAL_LANGUAGE_INPUTS.contains(&normalized.as_str())
        || (word_count == 1
            && !normalized.is_empty()
            && NATURAL_LANGUAGE_INPUTS
                .iter()
                .any(|candidate| candidate.starts_with(normalized.as_str())));
    if is_natural_language {
        return Some(InputClassification::deterministic(
            InputRoute::Agent,
            InputClassificationSource::NaturalLanguageHeuristic,
        ));
    }
    if normalized
        .split_whitespace()
        .next()
        .is_some_and(|word| SHELL_COMMAND_KEYWORDS.contains(&word))
        || contains_unquoted_shell_syntax(input)
    {
        return Some(InputClassification::deterministic(
            InputRoute::Shell,
            InputClassificationSource::ShellHeuristic,
        ));
    }
    None
}

fn contains_unquoted_shell_syntax(input: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            continue;
        }
        if quote.is_some() {
            continue;
        }
        if matches!(character, '|' | ';' | '<' | '>' | '`') {
            return true;
        }
        if character == '&' && characters.peek() == Some(&'&') {
            return true;
        }
        if character == '$' && matches!(characters.peek(), Some('(' | '{')) {
            return true;
        }
    }
    false
}
