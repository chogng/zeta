mod mention;
mod skill;
mod slash;

use super::SuggestView;
use ratatui::Frame;
use ratatui::layout::Rect;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, suggest: Option<SuggestView<'_>>) {
    match suggest {
        Some(SuggestView::Slash(view)) => slash::draw(frame, area, Some(view)),
        Some(SuggestView::Mention(view)) => mention::draw(frame, area, Some(view)),
        Some(SuggestView::Skill(view)) => skill::draw(frame, area, Some(view)),
        None => {}
    }
}

pub(crate) fn index_at(
    area: Rect,
    suggest: Option<SuggestView<'_>>,
    column: u16,
    row: u16,
) -> Option<usize> {
    match suggest {
        Some(SuggestView::Slash(view)) => slash::command_index_at(area, Some(view), column, row),
        Some(SuggestView::Mention(view)) => {
            mention::mention_index_at(area, Some(view), column, row)
        }
        Some(SuggestView::Skill(view)) => skill::skill_index_at(area, Some(view), column, row),
        None => None,
    }
}
