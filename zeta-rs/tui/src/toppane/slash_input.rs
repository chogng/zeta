//! Slash-command input parsing, cursor detection, completion, and submission helpers.

use std::ops::Range;

use super::slash_commands::SlashCommandArgumentMode;
use super::slash_commands::SlashCommandItem;
use super::slash_commands::SlashCommandRegistry;

/// Editable slash-command name under the composer cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SlashPopupQuery<'a> {
    pub(super) text: &'a str,
    command_range: Range<usize>,
}

/// Text replacement required to complete one selected slash command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SlashCompletion {
    pub(super) range: Range<usize>,
    pub(super) replacement: String,
}

/// Complete slash command parsed for submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedSlashCommand {
    pub(super) command: SlashCommandItem,
    pub(super) arguments_range: Range<usize>,
}

/// Interprets composer text against one canonical slash-command registry snapshot.
///
/// This view does not own editor or popup state. Cursor-sensitive discovery, completion ranges,
/// inline-argument parsing, and command-element detection stay here so popup rendering and
/// submission dispatch cannot develop separate command grammars.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SlashInput<'a> {
    text: &'a str,
    cursor: usize,
    registry: &'a SlashCommandRegistry,
}

impl<'a> SlashInput<'a> {
    pub(super) fn at_cursor(
        text: &'a str,
        cursor: usize,
        registry: &'a SlashCommandRegistry,
    ) -> Self {
        Self {
            text,
            cursor,
            registry,
        }
    }

    pub(super) fn for_submission(text: &'a str, registry: &'a SlashCommandRegistry) -> Self {
        Self::at_cursor(text, text.len(), registry)
    }

    /// Returns the command fragment before the cursor while the cursor edits the first token.
    pub(super) fn popup_query(self) -> Option<SlashPopupQuery<'a>> {
        let command_range = command_name_range(self.text)?;
        let first_line_end = self.text.find('\n').unwrap_or(self.text.len());
        if self.cursor > first_line_end || !self.text.is_char_boundary(self.cursor) {
            return None;
        }

        let query_end = if self.cursor <= 1 {
            command_range.end
        } else {
            self.cursor
        };
        if query_end > command_range.end {
            return None;
        }

        Some(SlashPopupQuery {
            text: &self.text[1..query_end],
            command_range,
        })
    }

    /// Returns registered commands matching the command fragment before the cursor.
    pub(super) fn matching_commands(self) -> Option<Vec<SlashCommandItem>> {
        self.popup_query()
            .map(|query| self.registry.matching(query.text))
    }

    /// Builds a range-preserving completion that leaves the cursor after one separator.
    pub(super) fn completion(self, command: &SlashCommandItem) -> Option<SlashCompletion> {
        let query = self.popup_query()?;
        let mut range = query.command_range;
        let mut replacement = format!("/{}", command.command());

        if let Some(next) = self.text[range.end..].chars().next() {
            if next == '\n' {
                replacement.push(' ');
            } else if next.is_whitespace() {
                range.end += next.len_utf8();
                replacement.push(' ');
            } else {
                return None;
            }
        } else {
            replacement.push(' ');
        }

        Some(SlashCompletion { range, replacement })
    }

    /// Recognizes a complete built-in or dynamic command and its trimmed argument range.
    pub(super) fn submission_command(self) -> Option<ParsedSlashCommand> {
        let command_range = command_name_range(self.text)?;
        let name = &self.text[1..command_range.end];
        let command = self.registry.command_named(name)?;
        let arguments_range = trimmed_range(self.text, command_range.end..self.text.len());

        if !arguments_range.is_empty() && command.argument_mode() == SlashCommandArgumentMode::None
        {
            return None;
        }

        Some(ParsedSlashCommand {
            command,
            arguments_range,
        })
    }

    /// Returns the command-name range once a recognized command has a separator after it.
    pub(super) fn command_element_range(self) -> Option<Range<usize>> {
        let command_range = command_name_range(self.text)?;
        if (1..command_range.end).contains(&self.cursor) {
            return None;
        }
        let next = self.text[command_range.end..].chars().next()?;
        if !next.is_whitespace() {
            return None;
        }

        let command = self
            .registry
            .command_named(&self.text[1..command_range.end])?;
        let arguments = self.text[command_range.end..].trim();
        if !arguments.is_empty() && command.argument_mode() == SlashCommandArgumentMode::None {
            return None;
        }
        Some(command_range)
    }
}

fn command_name_range(text: &str) -> Option<Range<usize>> {
    let name = text.strip_prefix('/')?;
    let name_end = name
        .find(char::is_whitespace)
        .map(|index| 1 + index)
        .unwrap_or(text.len());
    Some(0..name_end)
}

fn trimmed_range(text: &str, range: Range<usize>) -> Range<usize> {
    let value = &text[range.clone()];
    let start = value.len() - value.trim_start().len();
    let end = value.trim_end().len();
    range.start + start..range.start + end.max(start)
}

#[cfg(test)]
#[path = "slash_input_tests.rs"]
mod tests;
