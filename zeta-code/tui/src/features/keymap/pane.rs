use std::collections::BTreeMap;
use std::path::Path;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

use crate::components::key_capture::KeyCapture;
use crate::components::list_selection::ListSelectionGroup;
use crate::components::list_selection::ListSelectionItem;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::search_box::SearchBoxModel;
use crate::keymap::KeymapActionSnapshot;
use crate::keymap::compose_config_chord;
use crate::keymap::key_event_to_config_key;

use super::KeymapCaptureMode;
use super::KeymapEdit;
use super::KeymapEditIntent;
use super::KeymapEditKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeymapAction {
    OpenAction {
        action: KeymapActionSnapshot,
        revision: u64,
    },
    BeginCapture {
        action: KeymapActionSnapshot,
        revision: u64,
        intent: KeymapEditIntent,
        mode: KeymapCaptureMode,
    },
    ClearUser {
        command_id: String,
        revision: u64,
    },
}

pub(crate) struct KeymapPaneSpec {
    pub(crate) model: PaneSpec<ListSelectionModel>,
    pub(crate) actions: BTreeMap<ListSelectionItemId, KeymapAction>,
}

pub(crate) fn keymap_pane_spec(
    actions: Vec<KeymapActionSnapshot>,
    resource_path: &Path,
    diagnostics: &[String],
    revision: u64,
) -> KeymapPaneSpec {
    let mut item_actions = BTreeMap::new();
    let mut all_items = Vec::new();
    let mut user_items = Vec::new();
    for action in actions {
        append_action_items(
            &mut all_items,
            action.clone(),
            revision,
            true,
            true,
            &mut item_actions,
        );
        append_action_items(
            &mut user_items,
            action,
            revision,
            false,
            true,
            &mut item_actions,
        );
    }
    all_items.extend(fixed_shortcut_items());
    let user_items = non_empty(user_items, "No user shortcuts");
    let diagnostic_items = non_empty(
        diagnostics
            .iter()
            .map(|diagnostic| ListSelectionItem::new(diagnostic))
            .collect(),
        "No keymap diagnostics",
    );
    let key_hints = format!(
        "↑/↓ focus  ·  ←/→ or Tab/Shift-Tab tabs  ·  Enter edit  ·  Esc back  ·  {}",
        resource_path.display()
    );
    KeymapPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                "Keymap",
                vec![
                    ListSelectionGroup::new("All", all_items),
                    ListSelectionGroup::new("User", user_items),
                    ListSelectionGroup::new("Diagnostics", diagnostic_items),
                ],
            )
            .with_search(SearchBoxModel::new("Search shortcuts"))
            .with_empty_message("No matching shortcuts"),
            key_hints,
        ),
        actions: item_actions,
    }
}

pub(crate) fn keymap_action_menu(action: KeymapActionSnapshot, revision: u64) -> KeymapPaneSpec {
    let mut actions = BTreeMap::new();
    let mut items = Vec::new();
    push_action(
        &mut items,
        &mut actions,
        "Replace user shortcut with a key",
        KeymapAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::ReplaceUser,
            mode: KeymapCaptureMode::SingleKey,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Replace user shortcut with a chord",
        KeymapAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::ReplaceUser,
            mode: KeymapCaptureMode::Chord,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Add an alternate key",
        KeymapAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::AddAlternate,
            mode: KeymapCaptureMode::SingleKey,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Add an alternate chord",
        KeymapAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::AddAlternate,
            mode: KeymapCaptureMode::Chord,
        },
    );
    if !action.user_bindings.is_empty() {
        push_action(
            &mut items,
            &mut actions,
            "Clear user shortcuts",
            KeymapAction::ClearUser {
                command_id: action.command_id.to_owned(),
                revision,
            },
        );
    }
    let summary = binding_summary(&action);
    KeymapPaneSpec {
        model: PaneSpec::new(
            ListSelectionModel::new(
                action.label,
                vec![ListSelectionGroup::new("Actions", items)],
            )
            .without_tab_bar(),
            format!("{summary}  ·  Enter choose  ·  Esc back"),
        ),
        actions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeymapCaptureState {
    action: KeymapActionSnapshot,
    revision: u64,
    intent: KeymapEditIntent,
    mode: KeymapCaptureMode,
    first_stroke: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
pub(crate) enum KeymapCaptureOutcome {
    Pending(PaneSpec<KeyCapture>),
    Cancelled,
    Edit(KeymapEdit),
}

pub(crate) fn keymap_capture_pane_spec(
    action: KeymapActionSnapshot,
    revision: u64,
    intent: KeymapEditIntent,
    mode: KeymapCaptureMode,
) -> (PaneSpec<KeyCapture>, KeymapCaptureState) {
    let state = KeymapCaptureState {
        action,
        revision,
        intent,
        mode,
        first_stroke: None,
        error: None,
    };
    (state.model(), state)
}

impl KeymapCaptureState {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> KeymapCaptureOutcome {
        if key.kind != KeyEventKind::Press {
            return KeymapCaptureOutcome::Pending(self.model());
        }
        if is_cancel(key) {
            return KeymapCaptureOutcome::Cancelled;
        }
        let stroke = match key_event_to_config_key(&key) {
            Ok(stroke) => stroke,
            Err(error) => {
                self.error = Some(error);
                return KeymapCaptureOutcome::Pending(self.model());
            }
        };
        self.error = None;
        let key = match self.mode {
            KeymapCaptureMode::SingleKey => stroke,
            KeymapCaptureMode::Chord => match self.first_stroke.take() {
                None => {
                    self.first_stroke = Some(stroke);
                    return KeymapCaptureOutcome::Pending(self.model());
                }
                Some(first) => match compose_config_chord(&first, &stroke) {
                    Ok(chord) => chord,
                    Err(error) => {
                        self.error = Some(error);
                        return KeymapCaptureOutcome::Pending(self.model());
                    }
                },
            },
        };
        KeymapCaptureOutcome::Edit(KeymapEdit {
            expected_revision: self.revision,
            command_id: self.action.command_id.to_owned(),
            kind: KeymapEditKind::Set {
                key,
                intent: self.intent,
            },
        })
    }

    fn model(&self) -> PaneSpec<KeyCapture> {
        let instruction = match (self.mode, self.first_stroke.as_deref()) {
            (KeymapCaptureMode::SingleKey, _) => {
                "Press the new key now. Esc or Ctrl-C cancels.".to_owned()
            }
            (KeymapCaptureMode::Chord, None) => {
                "Press the first key, then the second. Esc or Ctrl-C cancels.".to_owned()
            }
            (KeymapCaptureMode::Chord, Some(first)) => {
                format!("First key: {first}. Press the second key. Esc or Ctrl-C cancels.")
            }
        };
        let mut lines = vec![instruction];
        if let Some(error) = &self.error {
            lines.push(format!("Error: {error}"));
        }
        PaneSpec::new(
            KeyCapture::new("Record shortcut", lines),
            format!(
                "{}  ·  {}",
                self.action.label,
                binding_summary(&self.action)
            ),
        )
    }
}

fn append_action_items(
    items: &mut Vec<ListSelectionItem>,
    action: KeymapActionSnapshot,
    revision: u64,
    include_default: bool,
    include_user: bool,
    actions: &mut BTreeMap<ListSelectionItemId, KeymapAction>,
) {
    let default_bindings = if action.default_bindings.is_empty() {
        vec!["unbound".to_owned()]
    } else {
        action.default_bindings.clone()
    };
    if include_default {
        for (index, binding) in default_bindings.into_iter().enumerate() {
            push_action_item(
                items,
                actions,
                &action,
                revision,
                binding,
                action.label.to_owned(),
                "default",
                format!("default-{index}"),
            );
        }
    }
    if include_user {
        for (index, binding) in action.user_bindings.iter().enumerate() {
            let responsibility = match &binding.when {
                Some(condition) => format!("{}  when {condition}", action.label),
                None => action.label.to_owned(),
            };
            push_action_item(
                items,
                actions,
                &action,
                revision,
                binding.key.clone(),
                responsibility,
                "user",
                format!("user-{index}"),
            );
        }
    }
}

fn push_action_item(
    items: &mut Vec<ListSelectionItem>,
    actions: &mut BTreeMap<ListSelectionItemId, KeymapAction>,
    action: &KeymapActionSnapshot,
    revision: u64,
    key: String,
    responsibility: String,
    source: &'static str,
    suffix: String,
) {
    let item_id = ListSelectionItemId::new(format!("{}-{suffix}", action.command_id));
    items.push(
        ListSelectionItem::new(&key)
            .with_id(item_id.clone())
            .with_columns(key, responsibility, source),
    );
    actions.insert(
        item_id,
        KeymapAction::OpenAction {
            action: action.clone(),
            revision,
        },
    );
}

fn binding_summary(action: &KeymapActionSnapshot) -> String {
    let defaults = if action.default_bindings.is_empty() {
        "unbound".to_owned()
    } else {
        action.default_bindings.join(", ")
    };
    if action.user_bindings.is_empty() {
        return format!("default: {defaults}");
    }
    let user = action
        .user_bindings
        .iter()
        .map(|binding| match &binding.when {
            Some(condition) => format!("{} when {condition}", binding.key),
            None => binding.key.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("default: {defaults}  ·  user: {user}")
}

fn push_action(
    items: &mut Vec<ListSelectionItem>,
    actions: &mut BTreeMap<ListSelectionItemId, KeymapAction>,
    label: &str,
    action: KeymapAction,
) {
    let item_id = ListSelectionItemId::new(format!("keymap-action-{}", items.len()));
    items.push(ListSelectionItem::new(label).with_id(item_id.clone()));
    actions.insert(item_id, action);
}

fn non_empty(items: Vec<ListSelectionItem>, label: &str) -> Vec<ListSelectionItem> {
    if items.is_empty() {
        vec![ListSelectionItem::new(label)]
    } else {
        items
    }
}

fn fixed_shortcut_items() -> impl Iterator<Item = ListSelectionItem> {
    [
        ("Esc Esc", "open rewind checkpoints from the root view"),
        (
            "Tab",
            "complete the selected slash command or directory path",
        ),
        ("Home / End", "move to the start or end of the input line"),
        ("PageUp / PageDown", "scroll the conversation"),
        (
            "Ctrl-Home / Ctrl-End",
            "load older turns or return to the latest turn",
        ),
    ]
    .into_iter()
    .map(|(key, description)| ListSelectionItem::new(key).with_columns(key, description, "default"))
}

fn is_cancel(key: KeyEvent) -> bool {
    (key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE)
        || (matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
            && key.modifiers == KeyModifiers::CONTROL)
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
