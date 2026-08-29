use super::ChatInputAreaHeightEntryKind;
use ratatui::layout::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChatInputAreaAreas {
    pub(crate) height_entries: Vec<ChatInputAreaHeightEntryArea>,
    pub(crate) input: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChatInputAreaHeightEntryArea {
    pub(crate) kind: ChatInputAreaHeightEntryKind,
    pub(crate) area: Rect,
}

pub(crate) fn desired_height(
    input_height: u16,
    height_entries: &[(ChatInputAreaHeightEntryKind, u16)],
) -> u16 {
    height_entries
        .iter()
        .fold(input_height, |height, (_, entry_height)| {
            height.saturating_add(*entry_height)
        })
}

pub(crate) fn areas(
    area: Rect,
    input_desired_height: u16,
    height_entries: &[(ChatInputAreaHeightEntryKind, u16)],
) -> ChatInputAreaAreas {
    let input_height = input_desired_height.min(area.height);
    let input_y = area
        .y
        .saturating_add(area.height)
        .saturating_sub(input_height);
    let input = Rect {
        y: input_y,
        height: input_height,
        ..area
    };
    let mut next_y = input_y;
    let mut remaining_height = area.height.saturating_sub(input_height);
    let mut allocated = Vec::with_capacity(height_entries.len());
    for (kind, desired_height) in height_entries {
        let height = (*desired_height).min(remaining_height);
        next_y = next_y.saturating_sub(height);
        remaining_height = remaining_height.saturating_sub(height);
        allocated.push(ChatInputAreaHeightEntryArea {
            kind: *kind,
            area: Rect {
                y: next_y,
                height,
                ..area
            },
        });
    }

    ChatInputAreaAreas {
        height_entries: allocated,
        input,
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
