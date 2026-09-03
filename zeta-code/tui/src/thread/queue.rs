use crate::render::RenderContext;
use crate::thread::composer::ChatInput;
use crate::thread::composer::ChatSubmission;
use crate::thread::composer::QueuedChatInput;
use crate::widgets::list_selection::ListSelectionActivationMode;
use crate::widgets::list_selection::ListSelectionGroup;
use crate::widgets::list_selection::ListSelectionItem;
use crate::widgets::list_selection::ListSelectionItemId;
use crate::widgets::list_selection::ListSelectionModel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;

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

    pub(crate) fn restore(&mut self, id: QueueId, input: &mut ChatInput) -> Result<(), String> {
        if !input.is_empty() {
            return Err("clear the current draft before restoring a queued message".into());
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id && !entry.sending)
            .ok_or_else(|| "the queued message is no longer editable".to_owned())?;
        let entry = self.entries.remove(index);
        match input.restore_queued(entry.input) {
            Ok(()) => Ok(()),
            Err(queued) => {
                self.entries.insert(
                    index,
                    QueueEntry {
                        id,
                        input: *queued,
                        sending: false,
                    },
                );
                Err("clear the current draft before restoring a queued message".into())
            }
        }
    }

    pub(crate) fn begin_next_send(&mut self) -> Option<(QueueId, ChatSubmission)> {
        let entry = self.entries.iter_mut().find(|entry| !entry.sending)?;
        entry.sending = true;
        Some((entry.id, entry.input.submission().clone()))
    }

    pub(crate) fn begin_send(&mut self, id: QueueId) -> Option<ChatSubmission> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id && !entry.sending)?;
        entry.sending = true;
        Some(entry.input.submission().clone())
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
        true
    }

    pub(crate) fn move_up(&mut self, id: QueueId) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        if index == 0 || self.entries[index].sending {
            return false;
        }
        self.entries.swap(index, index - 1);
        true
    }

    pub(crate) fn move_down(&mut self, id: QueueId) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        if index + 1 >= self.entries.len() || self.entries[index].sending {
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
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn view(&self) -> QueueView<'_> {
        QueueView {
            items: self
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| QueueItemView {
                    id: entry.id,
                    position: index.saturating_add(1),
                    text: entry.input.display_text(),
                    sending: entry.sending,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueueItemView<'a> {
    pub(crate) id: QueueId,
    pub(crate) position: usize,
    pub(crate) text: &'a str,
    pub(crate) sending: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueSelectionAction {
    Select(QueueId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueInput {
    Restore,
    Delete,
    MoveUp,
    MoveDown,
    Send,
}

impl QueueInput {
    const ALL: [Self; 5] = [
        Self::Restore,
        Self::Delete,
        Self::MoveUp,
        Self::MoveDown,
        Self::Send,
    ];

    fn key(self) -> (KeyCode, KeyModifiers) {
        match self {
            Self::Restore => (KeyCode::Char('r'), KeyModifiers::NONE),
            Self::Delete => (KeyCode::Char('d'), KeyModifiers::NONE),
            Self::MoveUp => (KeyCode::Up, KeyModifiers::ALT),
            Self::MoveDown => (KeyCode::Down, KeyModifiers::ALT),
            Self::Send => (KeyCode::Enter, KeyModifiers::CONTROL),
        }
    }

    fn hint(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Restore => Some(("r", "restore")),
            Self::Delete => Some(("d", "delete")),
            Self::MoveUp => Some(("Alt+↑/↓", "move")),
            Self::MoveDown => None,
            Self::Send => Some(("Ctrl+Enter", "send")),
        }
    }
}

pub(crate) fn queue_input(key: KeyEvent) -> Option<QueueInput> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    QueueInput::ALL
        .into_iter()
        .find(|input| input.key() == (key.code, key.modifiers))
}

pub(crate) struct QueueChoices {
    pub(crate) model: ListSelectionModel,
    pub(crate) actions: BTreeMap<ListSelectionItemId, QueueSelectionAction>,
}

pub(crate) fn choices(view: &QueueView<'_>) -> QueueChoices {
    let mut actions = BTreeMap::new();
    let items = view
        .items
        .iter()
        .map(|item| {
            let item_id = ListSelectionItemId::new(format!("queue-{}", item.position));
            actions.insert(item_id.clone(), QueueSelectionAction::Select(item.id));
            ListSelectionItem::new(item.text)
                .with_id(item_id)
                .with_description(if item.sending { "sending" } else { "queued" })
        })
        .collect();
    let model = QueueInput::ALL.into_iter().fold(
        ListSelectionModel::new(
            "Queue",
            vec![ListSelectionGroup::new("Current Thread", items)],
        )
        .with_activation_mode(ListSelectionActivationMode::Enter)
        .with_activation_label("view")
        .without_tab_bar()
        .with_empty_message("Queue is empty"),
        |model, input| {
            let Some((keys, label)) = input.hint() else {
                return model;
            };
            model.with_key_hint(keys, label)
        },
    );
    QueueChoices { model, actions }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueueView<'a> {
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
    context: RenderContext<'_>,
) {
    let lines = view
        .items
        .iter()
        .rev()
        .take(max_visible_items)
        .map(|item| {
            let state = if item.sending { " · sending" } else { "" };
            Line::styled(
                format!("Queue {}: {}{state}", item.position, item.text),
                Style::default().fg(context.muted()),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
#[path = "queue/state_tests.rs"]
mod tests;
