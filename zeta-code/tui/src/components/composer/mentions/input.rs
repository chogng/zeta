//! Cursor-aware mention token parsing.

use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ActiveMention<'a> {
    pub(super) range: Range<usize>,
    pub(super) query: &'a str,
}

/// Resolves the editable whitespace-delimited `@token` touching the cursor.
pub(super) fn active_mention(text: &str, cursor: usize) -> Option<ActiveMention<'_>> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }

    let start = text[..cursor]
        .char_indices()
        .rfind(|(_, character)| character.is_whitespace())
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    let end = text[cursor..]
        .char_indices()
        .find(|(_, character)| character.is_whitespace())
        .map(|(offset, _)| cursor + offset)
        .unwrap_or(text.len());
    let token = text.get(start..end)?;
    let query = token.strip_prefix('@')?;
    Some(ActiveMention {
        range: start..end,
        query,
    })
}
