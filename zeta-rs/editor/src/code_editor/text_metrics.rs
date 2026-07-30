//! Monospace display-column mapping shared by text and decorations.

use unicode_width::UnicodeWidthStr;

use super::TAB_WIDTH;

pub(super) fn display_columns(text: &str) -> usize {
    display_columns_until(text, text.len())
}

pub(super) fn display_columns_until(text: &str, byte_offset: usize) -> usize {
    let prefix = &text[..byte_offset.min(text.len())];
    let mut columns = 0;
    let mut segment_start = 0;
    for (index, character) in prefix.char_indices() {
        if character != '\t' {
            continue;
        }
        columns += UnicodeWidthStr::width(&prefix[segment_start..index]);
        columns += TAB_WIDTH - columns % TAB_WIDTH;
        segment_start = index + character.len_utf8();
    }
    columns + UnicodeWidthStr::width(&prefix[segment_start..])
}

pub(super) fn byte_offset_for_column(text: &str, requested: usize) -> usize {
    let mut best = 0;
    for (index, _) in text.char_indices() {
        if display_columns_until(text, index) > requested {
            break;
        }
        best = index;
    }
    if display_columns(text) <= requested {
        text.len()
    } else {
        best
    }
}

pub(super) fn expand_tabs(text: &str) -> String {
    let mut expanded = String::with_capacity(text.len());
    let mut columns = 0;
    let mut segment_start = 0;
    for (index, character) in text.char_indices() {
        if character != '\t' {
            continue;
        }
        let segment = &text[segment_start..index];
        expanded.push_str(segment);
        columns += UnicodeWidthStr::width(segment);
        let spaces = TAB_WIDTH - columns % TAB_WIDTH;
        expanded.extend(std::iter::repeat_n(' ', spaces));
        columns += spaces;
        segment_start = index + character.len_utf8();
    }
    expanded.push_str(&text[segment_start..]);
    expanded
}
