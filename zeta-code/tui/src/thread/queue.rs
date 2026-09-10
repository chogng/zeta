use crate::keymap::bindings;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crate::render::selection_marker;
use crate::thread::composer::ChatInput;
use crate::thread::composer::ChatSubmission;
use crate::thread::composer::QueuedChatInput;
use crate::widgets::navigation::Navigation;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use std::ops::Range;

pub(crate) const DEFAULT_MAX_VISIBLE_ITEMS: usize = 3;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct QueueId(u64);

impl QueueId {
    fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct QueueEntry {
    id: QueueId,
    input: Option<QueuedChatInput>,
    display_text: String,
    sending: bool,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct Queue {
    next_id: u64,
    entries: Vec<QueueEntry>,
    editing: Option<QueueId>,
}

impl Queue {
    pub(crate) fn push(&mut self, input: QueuedChatInput) -> QueueId {
        if let Some(id) = self.editing.take()
            && let Some(entry) = self
                .entries
                .iter_mut()
                .find(|entry| entry.id == id && entry.input.is_none())
        {
            entry.display_text = input.display_text().to_owned();
            entry.input = Some(input);
            return id;
        }
        let id = QueueId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.entries.push(QueueEntry {
            id,
            display_text: input.display_text().to_owned(),
            input: Some(input),
            sending: false,
        });
        id
    }

    pub(crate) fn restore(&mut self, id: QueueId, input: &mut ChatInput) -> Result<(), String> {
        if !input.is_empty() {
            return Err("clear the current draft before restoring a queued message".into());
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id && !entry.sending && entry.input.is_some())
            .ok_or_else(|| "the queued message is no longer editable".to_owned())?;
        let queued = self.entries[index]
            .input
            .take()
            .expect("an editable Queue entry contains its input");
        match input.restore_queued(queued) {
            Ok(()) => {
                self.editing = Some(id);
                Ok(())
            }
            Err(queued) => {
                self.entries[index].input = Some(*queued);
                Err("clear the current draft before restoring a queued message".into())
            }
        }
    }

    pub(crate) fn finish_edit(&mut self) {
        let Some(id) = self.editing.take() else {
            return;
        };
        self.entries
            .retain(|entry| entry.id != id || entry.input.is_some());
    }

    pub(crate) fn begin_next_send(&mut self) -> Option<(QueueId, ChatSubmission)> {
        let entry = self.entries.iter_mut().find(|entry| !entry.sending)?;
        let id = entry.id;
        let submission = entry.input.as_ref()?.submission().clone();
        entry.sending = true;
        Some((id, submission))
    }

    pub(crate) fn begin_send(&mut self, id: QueueId) -> Option<ChatSubmission> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id && !entry.sending && entry.input.is_some())?;
        let submission = entry
            .input
            .as_ref()
            .expect("an editable Queue entry contains its input")
            .submission()
            .clone();
        entry.sending = true;
        Some(submission)
    }

    pub(crate) fn delete(&mut self, id: QueueId) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.id == id && !entry.sending)
        else {
            return false;
        };
        self.entries.remove(index);
        if self.editing == Some(id) {
            self.editing = None;
        }
        true
    }

    pub(crate) fn move_up(&mut self, id: QueueId) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        if index == 0
            || self.entries[index].sending
            || self.entries[index].input.is_none()
            || self.entries[index - 1].sending
            || self.entries[index - 1].input.is_none()
        {
            return false;
        }
        self.entries.swap(index, index - 1);
        true
    }

    pub(crate) fn move_down(&mut self, id: QueueId) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        if index + 1 >= self.entries.len()
            || self.entries[index].sending
            || self.entries[index].input.is_none()
            || self.entries[index + 1].sending
            || self.entries[index + 1].input.is_none()
        {
            return false;
        }
        self.entries.swap(index, index + 1);
        true
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
        self.editing = None;
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn view(&self, navigation: &QueueNavigation) -> QueueView<'_> {
        QueueView {
            focused: navigation.focused(self),
            selected: navigation.selected,
            items: self
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| QueueItemView {
                    id: entry.id,
                    position: index.saturating_add(1),
                    text: &entry.display_text,
                    sending: entry.sending,
                    editing: entry.input.is_none(),
                })
                .collect(),
        }
    }
}

/// Selection and focus for one mode's view of a shared message queue.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct QueueNavigation {
    selected: Option<QueueId>,
}

impl QueueNavigation {
    pub(crate) fn focused(&self, queue: &Queue) -> bool {
        self.selected.is_some_and(|selected| {
            queue
                .entries
                .iter()
                .any(|entry| entry.id == selected && !entry.sending && entry.input.is_some())
        })
    }

    pub(crate) fn blur(&mut self) {
        self.selected = None;
    }

    pub(crate) fn reconcile(&mut self, queue: &Queue) {
        if self.selected.is_some() && !self.focused(queue) {
            self.selected = None;
            self.focus_latest(queue);
        }
    }
    pub(crate) fn focus_latest(&mut self, queue: &Queue) -> bool {
        let selected = queue
            .entries
            .iter()
            .rev()
            .find(|entry| !entry.sending && entry.input.is_some())
            .map(|entry| entry.id);
        let Some(selected) = selected else {
            return false;
        };
        self.selected = Some(selected);
        true
    }
    pub(crate) fn handle_key(&mut self, queue: &mut Queue, key: KeyEvent) -> QueueKeyOutcome {
        if !self.focused(queue) {
            return QueueKeyOutcome::Unhandled;
        }
        if let Some(navigation) = Navigation::from_key(key) {
            let ids = queue
                .entries
                .iter()
                .filter(|entry| !entry.sending && entry.input.is_some())
                .map(|entry| entry.id)
                .collect::<Vec<_>>();
            if let Some(last) = ids.len().checked_sub(1) {
                let current = ids
                    .iter()
                    .position(|id| Some(*id) == self.selected)
                    .unwrap_or(0);
                self.selected = Some(ids[navigation.offset(current, last, 3)]);
            }
            return QueueKeyOutcome::Consumed;
        }
        if key.kind != KeyEventKind::Press {
            return QueueKeyOutcome::Consumed;
        }
        let selected = self.selected;
        match (key.modifiers, key.code) {
            _ if bindings::QUEUE_UP.matches(key) => {
                if let Some(id) = selected {
                    queue.move_up(id);
                }
                QueueKeyOutcome::Consumed
            }
            _ if bindings::QUEUE_DOWN.matches(key) => {
                if let Some(id) = selected {
                    queue.move_down(id);
                }
                QueueKeyOutcome::Consumed
            }
            _ if bindings::QUEUE_EDIT.matches(key) => selected
                .map(QueueKeyOutcome::Restore)
                .unwrap_or(QueueKeyOutcome::Consumed),
            _ if bindings::QUEUE_SEND.matches(key) => selected
                .map(QueueKeyOutcome::Send)
                .unwrap_or(QueueKeyOutcome::Consumed),
            _ if bindings::QUEUE_REMOVE.matches(key) => {
                if let Some(id) = selected {
                    let index = queue
                        .entries
                        .iter()
                        .position(|entry| entry.id == id)
                        .unwrap_or(0);
                    queue.delete(id);
                    self.selected = queue
                        .entries
                        .iter()
                        .skip(index)
                        .chain(queue.entries.iter().take(index).rev())
                        .find(|entry| !entry.sending && entry.input.is_some())
                        .map(|entry| entry.id);
                }
                QueueKeyOutcome::Consumed
            }
            _ if bindings::RETURN_INPUT.matches(key) => {
                self.blur();
                QueueKeyOutcome::Consumed
            }
            _ if bindings::INTERRUPT.matches(key) => QueueKeyOutcome::Unhandled,
            _ => QueueKeyOutcome::Consumed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueueItemView<'a> {
    pub(crate) id: QueueId,
    pub(crate) position: usize,
    pub(crate) text: &'a str,
    pub(crate) sending: bool,
    pub(crate) editing: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueKeyOutcome {
    Restore(QueueId),
    Send(QueueId),
    Consumed,
    Unhandled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueView<'a> {
    pub(crate) focused: bool,
    pub(crate) selected: Option<QueueId>,
    pub(crate) items: Vec<QueueItemView<'a>>,
}

pub(crate) fn desired_height(view: &QueueView<'_>, max_visible_items: usize) -> u16 {
    u16::try_from(view.items.len().min(max_visible_items)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: &QueueView<'_>,
    max_visible_items: usize,
    hovered: Option<QueueId>,
    pressed: Option<QueueId>,
    context: RenderContext<'_>,
) {
    let range = visible_range(view, max_visible_items);
    let lines = view
        .items
        .get(range)
        .unwrap_or_default()
        .iter()
        .map(|item| {
            let selected = view.focused && view.selected == Some(item.id);
            let state = if item.sending {
                " · sending"
            } else if item.editing {
                " · editing"
            } else {
                ""
            };
            let next = if item.position == 1 { " · next" } else { "" };
            Line::styled(
                format!(
                    "{}Queue {}: {}{next}{state}",
                    selection_marker(selected),
                    item.position,
                    item.text
                ),
                if selected || hovered == Some(item.id) || pressed == Some(item.id) {
                    interaction_style(
                        context,
                        InteractionState {
                            target: InteractionTarget::Rest,
                            selected,
                            hovered: hovered == Some(item.id),
                            pressed: pressed == Some(item.id),
                        },
                    )
                } else {
                    Style::default().fg(context.muted())
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn visible_range(view: &QueueView<'_>, max_visible_items: usize) -> Range<usize> {
    let visible = view.items.len().min(max_visible_items);
    let selected = view
        .selected
        .and_then(|selected| view.items.iter().position(|item| item.id == selected));
    let start = selected
        .map(|selected| selected.saturating_add(1).saturating_sub(visible))
        .unwrap_or_else(|| view.items.len().saturating_sub(visible));
    start..start.saturating_add(visible)
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;
