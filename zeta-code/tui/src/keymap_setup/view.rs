use std::collections::BTreeMap;
use std::path::Path;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

use crate::components::pane::PaneViewModel;
use crate::components::search_box::SearchBoxModel;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use crate::keymap::KeymapActionSnapshot;
use crate::keymap::compose_config_chord;
use crate::keymap::key_event_to_config_key;

use super::KeymapCaptureMode;
use super::KeymapEdit;
use super::KeymapEditIntent;
use super::KeymapEditKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeymapSetupAction {
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
    ClearCustom {
        command_id: String,
        revision: u64,
    },
}

pub(crate) struct KeymapSetupView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, KeymapSetupAction>,
}

pub(crate) fn keymap_picker(
    actions: Vec<KeymapActionSnapshot>,
    resource_path: &Path,
    diagnostics: &[String],
    revision: u64,
) -> KeymapSetupView {
    let mut item_actions = BTreeMap::new();
    let all_items = actions
        .iter()
        .cloned()
        .map(|action| action_item(action, revision, &mut item_actions))
        .collect();
    let customized = actions
        .iter()
        .filter(|action| !action.custom_bindings.is_empty())
        .cloned()
        .map(|action| action_item(action, revision, &mut item_actions))
        .collect::<Vec<_>>();
    let customized = non_empty(customized, "No customized shortcuts");
    let diagnostic_items = non_empty(
        diagnostics
            .iter()
            .map(|diagnostic| SelectionItem::new(diagnostic))
            .collect(),
        "No keymap diagnostics",
    );
    let footer = format!(
        "Space search  ·  ←/→ tabs  ·  Enter edit  ·  Esc back  ·  {}",
        resource_path.display()
    );
    KeymapSetupView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Keymap",
                vec![
                    SelectionTab::new("All", all_items),
                    SelectionTab::new("Customized", customized),
                    SelectionTab::new("Diagnostics", diagnostic_items),
                ],
            )
            .with_search(SearchBoxModel::new("Search shortcuts"))
            .with_empty_message("No matching shortcuts"),
            footer,
        ),
        actions: item_actions,
    }
}

pub(crate) fn action_menu(action: KeymapActionSnapshot, revision: u64) -> KeymapSetupView {
    let mut actions = BTreeMap::new();
    let mut items = Vec::new();
    push_action(
        &mut items,
        &mut actions,
        "Replace custom shortcut with a key",
        KeymapSetupAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::ReplaceCustom,
            mode: KeymapCaptureMode::SingleKey,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Replace custom shortcut with a chord",
        KeymapSetupAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::ReplaceCustom,
            mode: KeymapCaptureMode::Chord,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Add an alternate key",
        KeymapSetupAction::BeginCapture {
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
        KeymapSetupAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: KeymapEditIntent::AddAlternate,
            mode: KeymapCaptureMode::Chord,
        },
    );
    if !action.custom_bindings.is_empty() {
        push_action(
            &mut items,
            &mut actions,
            "Clear custom shortcuts",
            KeymapSetupAction::ClearCustom {
                command_id: action.command_id.to_owned(),
                revision,
            },
        );
    }
    let summary = binding_summary(&action);
    KeymapSetupView {
        model: PaneViewModel::new(
            SelectionViewModel::new(action.label, vec![SelectionTab::new("Actions", items)])
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
    Pending(PaneViewModel<SelectionViewModel>),
    Cancelled,
    Edit(KeymapEdit),
}

pub(crate) fn capture_view(
    action: KeymapActionSnapshot,
    revision: u64,
    intent: KeymapEditIntent,
    mode: KeymapCaptureMode,
) -> (PaneViewModel<SelectionViewModel>, KeymapCaptureState) {
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

    fn model(&self) -> PaneViewModel<SelectionViewModel> {
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
        let mut items = vec![SelectionItem::new(instruction)];
        if let Some(error) = &self.error {
            items.push(SelectionItem::new(format!("Error: {error}")));
        }
        PaneViewModel::new(
            SelectionViewModel::new("Record shortcut", vec![SelectionTab::new("Capture", items)])
                .without_tab_bar()
                .without_selection(),
            format!(
                "{}  ·  {}",
                self.action.label,
                binding_summary(&self.action)
            ),
        )
    }
}

fn action_item(
    action: KeymapActionSnapshot,
    revision: u64,
    actions: &mut BTreeMap<SelectionItemId, KeymapSetupAction>,
) -> SelectionItem {
    let item_id = SelectionItemId::new(action.command_id);
    let item = SelectionItem::new(action.label)
        .with_id(item_id.clone())
        .with_description(format!(
            "{}  ·  {}  ·  {}",
            binding_summary(&action),
            action.command_id,
            action.description
        ));
    actions.insert(item_id, KeymapSetupAction::OpenAction { action, revision });
    item
}

fn binding_summary(action: &KeymapActionSnapshot) -> String {
    let defaults = if action.default_bindings.is_empty() {
        "unbound".to_owned()
    } else {
        action.default_bindings.join(", ")
    };
    if action.custom_bindings.is_empty() {
        return format!("Default: {defaults}");
    }
    let custom = action
        .custom_bindings
        .iter()
        .map(|binding| match &binding.when {
            Some(condition) => format!("{} when {condition}", binding.key),
            None => binding.key.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("Default: {defaults}  ·  Custom: {custom}")
}

fn push_action(
    items: &mut Vec<SelectionItem>,
    actions: &mut BTreeMap<SelectionItemId, KeymapSetupAction>,
    label: &str,
    action: KeymapSetupAction,
) {
    let item_id = SelectionItemId::new(format!("keymap-action-{}", items.len()));
    items.push(SelectionItem::new(label).with_id(item_id.clone()));
    actions.insert(item_id, action);
}

fn non_empty(items: Vec<SelectionItem>, label: &str) -> Vec<SelectionItem> {
    if items.is_empty() {
        vec![SelectionItem::new(label)]
    } else {
        items
    }
}

fn is_cancel(key: KeyEvent) -> bool {
    (key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE)
        || (matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
            && key.modifiers == KeyModifiers::CONTROL)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
