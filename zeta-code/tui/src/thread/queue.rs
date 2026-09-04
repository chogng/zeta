use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crate::render::selection_marker;
use crate::thread::composer::ChatInput;
use crate::thread::composer::ChatSubmission;
use crate::thread::composer::QueuedChatInput;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
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
    focused: bool,
    selected: Option<QueueId>,
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
                self.blur();
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
        self.reconcile_selection();
    }

    pub(crate) fn begin_next_send(&mut self) -> Option<(QueueId, ChatSubmission)> {
        let entry = self.entries.iter_mut().find(|entry| !entry.sending)?;
        let id = entry.id;
        let submission = entry.input.as_ref()?.submission().clone();
        entry.sending = true;
        self.blur();
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
        self.blur();
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
        if self.focused && self.selected == Some(id) {
            self.selected = self.entries[index..]
                .iter()
                .find(|entry| !entry.sending && entry.input.is_some())
                .or_else(|| {
                    self.entries[..index]
                        .iter()
                        .rev()
                        .find(|entry| !entry.sending && entry.input.is_some())
                })
                .map(|entry| entry.id);
            if self.selected.is_none() {
                self.blur();
            }
        } else {
            self.reconcile_selection();
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
        self.reconcile_selection();
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
        self.focused = false;
        self.selected = None;
        self.editing = None;
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn view(&self) -> QueueView<'_> {
        QueueView {
            focused: self.focused,
            selected: self.selected,
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

    pub(crate) const fn focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn focus_latest(&mut self) -> bool {
        let selected = self
            .entries
            .iter()
            .rev()
            .find(|entry| !entry.sending && entry.input.is_some())
            .map(|entry| entry.id);
        let Some(selected) = selected else {
            return false;
        };
        self.focused = true;
        self.selected = Some(selected);
        true
    }

    pub(crate) fn select(&mut self, id: QueueId) -> bool {
        if !self
            .entries
            .iter()
            .any(|entry| entry.id == id && !entry.sending && entry.input.is_some())
        {
            return false;
        }
        self.focused = true;
        self.selected = Some(id);
        true
    }

    pub(crate) fn blur(&mut self) {
        self.focused = false;
        self.selected = None;
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> QueueKeyOutcome {
        if !self.focused {
            return QueueKeyOutcome::Unhandled;
        }
        if key.kind != KeyEventKind::Press {
            return QueueKeyOutcome::Consumed;
        }
        let selected = self.selected;
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Up) => {
                self.select_previous();
                QueueKeyOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                if !self.select_next() {
                    self.blur();
                }
                QueueKeyOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::Up) => {
                if let Some(id) = selected {
                    self.move_up(id);
                }
                QueueKeyOutcome::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::Down) => {
                if let Some(id) = selected {
                    self.move_down(id);
                }
                QueueKeyOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Enter) => selected
                .map(QueueKeyOutcome::Restore)
                .unwrap_or(QueueKeyOutcome::Consumed),
            (KeyModifiers::CONTROL, KeyCode::Enter) => selected
                .map(QueueKeyOutcome::Send)
                .unwrap_or(QueueKeyOutcome::Consumed),
            (KeyModifiers::NONE, KeyCode::Delete) => {
                if let Some(id) = selected {
                    self.delete(id);
                }
                QueueKeyOutcome::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.blur();
                QueueKeyOutcome::Consumed
            }
            _ if key.modifiers.contains(KeyModifiers::CONTROL) => QueueKeyOutcome::Unhandled,
            _ => QueueKeyOutcome::Consumed,
        }
    }

    fn select_previous(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return self.focus_latest();
        };
        let Some(entry) = self.entries[..index]
            .iter()
            .rev()
            .find(|entry| !entry.sending && entry.input.is_some())
        else {
            return false;
        };
        self.selected = Some(entry.id);
        true
    }

    fn select_next(&mut self) -> bool {
        let Some(index) = self.selected_index() else {
            return self.focus_latest();
        };
        let Some(entry) = self.entries[index.saturating_add(1)..]
            .iter()
            .find(|entry| !entry.sending && entry.input.is_some())
        else {
            return false;
        };
        self.selected = Some(entry.id);
        true
    }

    fn selected_index(&self) -> Option<usize> {
        let selected = self.selected?;
        self.entries.iter().position(|entry| entry.id == selected)
    }

    fn reconcile_selection(&mut self) {
        if self.entries.is_empty() {
            self.blur();
            return;
        }
        if self.selected.is_some_and(|selected| {
            self.entries
                .iter()
                .any(|entry| entry.id == selected && !entry.sending && entry.input.is_some())
        }) {
            return;
        }
        if self.focused {
            self.focus_latest();
        } else {
            self.selected = None;
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

pub(crate) fn pointer_target_at(
    area: Rect,
    view: &QueueView<'_>,
    max_visible_items: usize,
    column: u16,
    row: u16,
) -> Option<QueueId> {
    if column < area.x || column >= area.right() || row < area.y || row >= area.bottom() {
        return None;
    }
    let range = visible_range(view, max_visible_items);
    let index = range.start.saturating_add(usize::from(row - area.y));
    view.items
        .get(index)
        .filter(|item| !item.sending && !item.editing)
        .map(|item| item.id)
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
