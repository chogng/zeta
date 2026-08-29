use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::QueuedChatInput;
use crate::ui::background;
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
pub(crate) struct QueueId(u64);

impl QueueId {
    fn new(value: u64) -> Self {
        Self(value)
    }
}

pub(crate) enum QueueSendNowOutcome {
    Empty,
    SkillBound,
    Submission(ChatSubmission),
}

#[derive(Debug, Eq, PartialEq)]
struct QueueEntry {
    id: QueueId,
    input: QueuedChatInput,
    sending: bool,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct Queue {
    next_id: u64,
    entries: Vec<QueueEntry>,
}

impl Queue {
    pub(crate) fn push(&mut self, input: QueuedChatInput) -> QueueId {
        let id = QueueId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.entries.push(QueueEntry {
            id,
            input,
            sending: false,
        });
        id
    }

    pub(crate) fn take_latest_for_edit(&mut self) -> Option<QueuedChatInput> {
        let index = self.entries.iter().rposition(|entry| !entry.sending)?;
        Some(self.entries.remove(index).input)
    }

    pub(crate) fn take_latest_for_send_now(&mut self) -> QueueSendNowOutcome {
        let Some(index) = self.entries.iter().rposition(|entry| !entry.sending) else {
            return QueueSendNowOutcome::Empty;
        };
        if self.entries[index]
            .input
            .submission()
            .input
            .iter()
            .any(|item| matches!(item, ChatInputItem::Skill { .. }))
        {
            return QueueSendNowOutcome::SkillBound;
        }
        QueueSendNowOutcome::Submission(self.entries.remove(index).input.into_submission())
    }

    pub(crate) fn restore_latest(&mut self, input: QueuedChatInput) {
        self.push(input);
    }

    pub(crate) fn begin_next_send(&mut self) -> Option<(QueueId, ChatSubmission)> {
        let entry = self.entries.iter_mut().find(|entry| !entry.sending)?;
        entry.sending = true;
        Some((entry.id, entry.input.submission().clone()))
    }

    pub(crate) fn finish_send(&mut self, id: QueueId) -> bool {
        let previous_len = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != previous_len
    }

    pub(crate) fn fail_send(&mut self, id: QueueId) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        entry.sending = false;
        true
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn has_editable(&self) -> bool {
        self.entries.iter().any(|entry| !entry.sending)
    }

    pub(crate) fn view(&self) -> QueueView<'_> {
        QueueView {
            items: self
                .entries
                .iter()
                .map(|entry| QueueItemView {
                    text: entry.input.display_text(),
                    sending: entry.sending,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueueItemView<'a> {
    pub(crate) text: &'a str,
    pub(crate) sending: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueView<'a> {
    pub(crate) items: Vec<QueueItemView<'a>>,
}

pub(crate) fn desired_height(view: QueueView<'_>) -> u16 {
    u16::try_from(view.items.len().min(MAX_VISIBLE_ITEMS).saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, view: QueueView<'_>) {
    let lines = view
        .items
        .iter()
        .take(MAX_VISIBLE_ITEMS)
        .map(|item| {
            let marker = if item.sending { "↥" } else { "•" };
            Line::from(format!("{marker} {}", item.text))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(format!("Queue  {}  ·  ↑ edit", view.items.len()))
                .title_style(Style::default().add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(muted()))
                .style(Style::default().bg(background())),
        ),
        area,
    );
}

#[cfg(test)]
#[path = "queue/state_tests.rs"]
mod tests;
