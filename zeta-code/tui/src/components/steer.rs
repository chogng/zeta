use crate::ui::background;
use crate::ui::highlight;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

const MAX_VISIBLE_ITEMS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SteerId(u64);

impl SteerId {
    fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingSteer {
    id: SteerId,
    text: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Steer {
    next_id: u64,
    pending: Vec<PendingSteer>,
}

impl Steer {
    pub(crate) fn push(&mut self, text: String) -> SteerId {
        let id = SteerId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.pending.push(PendingSteer { id, text });
        id
    }

    pub(crate) fn remove(&mut self, id: SteerId) -> bool {
        let previous_len = self.pending.len();
        self.pending.retain(|pending| pending.id != id);
        self.pending.len() != previous_len
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub(crate) fn view(&self) -> SteerView<'_> {
        SteerView {
            items: self
                .pending
                .iter()
                .map(|pending| pending.text.as_str())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SteerView<'a> {
    pub(crate) items: Vec<&'a str>,
}

pub(crate) fn desired_height(view: &SteerView<'_>) -> u16 {
    u16::try_from(view.items.len().min(MAX_VISIBLE_ITEMS).saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: &SteerView<'_>) {
    let lines = view
        .items
        .iter()
        .take(MAX_VISIBLE_ITEMS)
        .map(|item| Line::from(format!("↳ {item}")))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(format!("Steer  {} sending", view.items.len()))
                .title_style(
                    Style::default()
                        .fg(highlight())
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(muted()))
                .style(Style::default().bg(background())),
        ),
        area,
    );
}

#[cfg(test)]
#[path = "steer/state_tests.rs"]
mod tests;
