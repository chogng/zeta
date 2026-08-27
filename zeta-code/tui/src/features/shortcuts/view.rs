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

use super::ShortcutCaptureMode;
use super::ShortcutEdit;
use super::ShortcutEditIntent;
use super::ShortcutEditKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutAction {
    OpenAction {
        action: KeymapActionSnapshot,
        revision: u64,
    },
    BeginCapture {
        action: KeymapActionSnapshot,
        revision: u64,
        intent: ShortcutEditIntent,
        mode: ShortcutCaptureMode,
    },
    ClearCustom {
        command_id: String,
        revision: u64,
    },
}

pub(crate) struct ShortcutView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) actions: BTreeMap<SelectionItemId, ShortcutAction>,
}

pub(crate) fn shortcut_view(
    actions: Vec<KeymapActionSnapshot>,
    resource_path: &Path,
    diagnostics: &[String],
    revision: u64,
) -> ShortcutView {
    let mut item_actions = BTreeMap::new();
    let all_items = actions
        .iter()
        .cloned()
        .map(|action| action_item(action, revision, &mut item_actions))
        .chain(fixed_shortcut_items())
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
    ShortcutView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Shortcuts",
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

pub(crate) fn action_menu(action: KeymapActionSnapshot, revision: u64) -> ShortcutView {
    let mut actions = BTreeMap::new();
    let mut items = Vec::new();
    push_action(
        &mut items,
        &mut actions,
        "Replace custom shortcut with a key",
        ShortcutAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: ShortcutEditIntent::ReplaceCustom,
            mode: ShortcutCaptureMode::SingleKey,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Replace custom shortcut with a chord",
        ShortcutAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: ShortcutEditIntent::ReplaceCustom,
            mode: ShortcutCaptureMode::Chord,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Add an alternate key",
        ShortcutAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: ShortcutEditIntent::AddAlternate,
            mode: ShortcutCaptureMode::SingleKey,
        },
    );
    push_action(
        &mut items,
        &mut actions,
        "Add an alternate chord",
        ShortcutAction::BeginCapture {
            action: action.clone(),
            revision,
            intent: ShortcutEditIntent::AddAlternate,
            mode: ShortcutCaptureMode::Chord,
        },
    );
    if !action.custom_bindings.is_empty() {
        push_action(
            &mut items,
            &mut actions,
            "Clear custom shortcuts",
            ShortcutAction::ClearCustom {
                command_id: action.command_id.to_owned(),
                revision,
            },
        );
    }
    let summary = binding_summary(&action);
    ShortcutView {
        model: PaneViewModel::new(
            SelectionViewModel::new(action.label, vec![SelectionTab::new("Actions", items)])
                .without_tab_bar(),
            format!("{summary}  ·  Enter choose  ·  Esc back"),
        ),
        actions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShortcutCaptureState {
    action: KeymapActionSnapshot,
    revision: u64,
    intent: ShortcutEditIntent,
    mode: ShortcutCaptureMode,
    first_stroke: Option<String>,
    error: Option<String>,
}

#[derive(Debug)]
pub(crate) enum ShortcutCaptureOutcome {
    Pending(PaneViewModel<SelectionViewModel>),
    Cancelled,
    Edit(ShortcutEdit),
}

pub(crate) fn capture_view(
    action: KeymapActionSnapshot,
    revision: u64,
    intent: ShortcutEditIntent,
    mode: ShortcutCaptureMode,
) -> (PaneViewModel<SelectionViewModel>, ShortcutCaptureState) {
    let state = ShortcutCaptureState {
        action,
        revision,
        intent,
        mode,
        first_stroke: None,
        error: None,
    };
    (state.model(), state)
}

impl ShortcutCaptureState {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ShortcutCaptureOutcome {
        if key.kind != KeyEventKind::Press {
            return ShortcutCaptureOutcome::Pending(self.model());
        }
        if is_cancel(key) {
            return ShortcutCaptureOutcome::Cancelled;
        }
        let stroke = match key_event_to_config_key(&key) {
            Ok(stroke) => stroke,
            Err(error) => {
                self.error = Some(error);
                return ShortcutCaptureOutcome::Pending(self.model());
            }
        };
        self.error = None;
        let key = match self.mode {
            ShortcutCaptureMode::SingleKey => stroke,
            ShortcutCaptureMode::Chord => match self.first_stroke.take() {
                None => {
                    self.first_stroke = Some(stroke);
                    return ShortcutCaptureOutcome::Pending(self.model());
                }
                Some(first) => match compose_config_chord(&first, &stroke) {
                    Ok(chord) => chord,
                    Err(error) => {
                        self.error = Some(error);
                        return ShortcutCaptureOutcome::Pending(self.model());
                    }
                },
            },
        };
        ShortcutCaptureOutcome::Edit(ShortcutEdit {
            expected_revision: self.revision,
            command_id: self.action.command_id.to_owned(),
            kind: ShortcutEditKind::Set {
                key,
                intent: self.intent,
            },
        })
    }

    fn model(&self) -> PaneViewModel<SelectionViewModel> {
        let instruction = match (self.mode, self.first_stroke.as_deref()) {
            (ShortcutCaptureMode::SingleKey, _) => {
                "Press the new key now. Esc or Ctrl-C cancels.".to_owned()
            }
            (ShortcutCaptureMode::Chord, None) => {
                "Press the first key, then the second. Esc or Ctrl-C cancels.".to_owned()
            }
            (ShortcutCaptureMode::Chord, Some(first)) => {
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
    actions: &mut BTreeMap<SelectionItemId, ShortcutAction>,
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
    actions.insert(item_id, ShortcutAction::OpenAction { action, revision });
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
    actions: &mut BTreeMap<SelectionItemId, ShortcutAction>,
    label: &str,
    action: ShortcutAction,
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

fn fixed_shortcut_items() -> impl Iterator<Item = SelectionItem> {
    [
        ("Enter", "send the current prompt or queue a follow-up"),
        (
            "Shift-Enter / Alt-Enter / Ctrl-J",
            "insert a newline in the current prompt",
        ),
        ("Esc Esc", "open rewind checkpoints from the root view"),
        (
            "Tab",
            "complete the selected slash command or workspace path",
        ),
        ("Esc", "close the active popup or view"),
        (
            "↑ / ↓",
            "move the cursor, recall prompts, or select an item",
        ),
        ("← / →", "switch tabs in an interactive view"),
        ("Home / End", "move to the start or end of the input line"),
        ("PageUp / PageDown", "scroll the conversation"),
        (
            "Ctrl-Home / Ctrl-End",
            "load older turns or return to the latest turn",
        ),
    ]
    .into_iter()
    .map(|(key, description)| {
        SelectionItem::new(key).with_description(format!("Built in  ·  {description}"))
    })
}

fn is_cancel(key: KeyEvent) -> bool {
    (key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE)
        || (matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
            && key.modifiers == KeyModifiers::CONTROL)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
