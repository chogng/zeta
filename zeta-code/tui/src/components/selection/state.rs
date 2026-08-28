use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::style::Color;

use super::SelectionPreview;
use super::matcher::selection_match_score;
use crate::components::search_box::SEARCH_BOX_HEIGHT;
use crate::components::search_box::SearchBoxInputOutcome;
use crate::components::search_box::SearchBoxModel;
use crate::components::search_box::SearchBoxState;
use crate::components::tab_list;
use crate::components::tab_list::TabListInputOutcome;
use crate::components::tab_list::TabListItem;
use crate::components::tab_list::TabListState;
use crate::mouse::MouseMode;

const MAX_VISIBLE_ROWS: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionActivationMode {
    Enter,
    EnterOrSpace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionDismissal {
    Allowed,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionItem {
    id: Option<SelectionItemId>,
    label: String,
    description: Option<String>,
    columns: Option<SelectionItemColumns>,
    selection_foreground: Option<Color>,
    preview: Option<SelectionPreview>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SelectionItemColumns {
    pub(super) leading: String,
    pub(super) middle: String,
    pub(super) trailing: String,
}

impl SelectionItem {
    pub(crate) fn new(label: impl Into<String>) -> Self {
        Self {
            id: None,
            label: label.into(),
            description: None,
            columns: None,
            selection_foreground: None,
            preview: None,
        }
    }

    pub(crate) fn with_id(mut self, id: SelectionItemId) -> Self {
        self.id = Some(id);
        self
    }

    pub(crate) fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub(crate) fn with_columns(
        mut self,
        leading: impl Into<String>,
        middle: impl Into<String>,
        trailing: impl Into<String>,
    ) -> Self {
        let columns = SelectionItemColumns {
            leading: leading.into(),
            middle: middle.into(),
            trailing: trailing.into(),
        };
        self.description = Some(format!("{} {}", columns.middle, columns.trailing));
        self.columns = Some(columns);
        self
    }

    pub(crate) fn with_selection_foreground(mut self, color: Color) -> Self {
        self.selection_foreground = Some(color);
        self
    }

    pub(crate) fn with_preview(mut self, preview: SelectionPreview) -> Self {
        self.preview = Some(preview);
        self
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(super) fn columns(&self) -> Option<&SelectionItemColumns> {
        self.columns.as_ref()
    }

    pub(crate) fn id(&self) -> Option<&SelectionItemId> {
        self.id.as_ref()
    }

    pub(crate) fn selection_foreground(&self) -> Option<Color> {
        self.selection_foreground
    }

    pub(crate) fn preview(&self) -> Option<&SelectionPreview> {
        self.preview.as_ref()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SelectionItemId(String);

impl SelectionItemId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionTab {
    label: String,
    items: Vec<SelectionItem>,
}

impl SelectionTab {
    pub(crate) fn new(label: impl Into<String>, items: Vec<SelectionItem>) -> Self {
        Self {
            label: label.into(),
            items,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }
}

impl TabListItem for SelectionTab {
    fn tab_label(&self) -> &str {
        self.label()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionViewModel {
    tabs: Vec<SelectionTab>,
    presentation: SelectionViewPresentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectionViewPresentation {
    title: String,
    search: Option<SearchBoxModel>,
    query_mode: SelectionQueryMode,
    free_form_action: Option<SelectionItemId>,
    empty_message: String,
    activation_mode: SelectionActivationMode,
    dismissal: SelectionDismissal,
    show_tabs: bool,
    selection_enabled: bool,
    initial_selected: usize,
    title_top_margin: usize,
    title_bottom_margin: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionQueryMode {
    FilterItems,
    InputOnly,
}

impl SelectionViewModel {
    pub(crate) fn new(title: impl Into<String>, tabs: Vec<SelectionTab>) -> Self {
        assert!(
            !tabs.is_empty(),
            "a selection view requires at least one tab"
        );
        Self {
            tabs,
            presentation: SelectionViewPresentation {
                title: title.into(),
                search: None,
                query_mode: SelectionQueryMode::FilterItems,
                free_form_action: None,
                empty_message: "No matching items".into(),
                activation_mode: SelectionActivationMode::Enter,
                dismissal: SelectionDismissal::Allowed,
                show_tabs: true,
                selection_enabled: true,
                initial_selected: 0,
                title_top_margin: 0,
                title_bottom_margin: 0,
            },
        }
    }

    pub(crate) fn with_activation_mode(mut self, mode: SelectionActivationMode) -> Self {
        self.presentation.activation_mode = mode;
        self
    }

    pub(crate) fn with_dismissal(mut self, dismissal: SelectionDismissal) -> Self {
        self.presentation.dismissal = dismissal;
        self
    }

    pub(crate) fn without_tab_bar(mut self) -> Self {
        self.presentation.show_tabs = false;
        self
    }

    pub(crate) fn without_selection(mut self) -> Self {
        self.presentation.selection_enabled = false;
        self
    }

    pub(crate) fn with_initial_selected(mut self, index: usize) -> Self {
        self.presentation.initial_selected = index;
        self
    }

    pub(crate) fn with_title_top_margin(mut self, rows: usize) -> Self {
        self.presentation.title_top_margin = rows;
        self
    }

    pub(crate) fn with_title_bottom_margin(mut self, rows: usize) -> Self {
        self.presentation.title_bottom_margin = rows;
        self
    }

    pub(crate) fn with_search(mut self, search: SearchBoxModel) -> Self {
        self.presentation.search = Some(search);
        self
    }

    pub(crate) fn with_free_form(
        mut self,
        placeholder: impl Into<String>,
        action: SelectionItemId,
    ) -> Self {
        self.presentation.search = Some(SearchBoxModel::new(placeholder).initially_active());
        self.presentation.free_form_action = Some(action);
        self
    }

    pub(crate) fn with_secret_input(
        mut self,
        placeholder: impl Into<String>,
        action: SelectionItemId,
    ) -> Self {
        self.presentation.search =
            Some(SearchBoxModel::new(placeholder).initially_active().masked());
        self.presentation.query_mode = SelectionQueryMode::InputOnly;
        self.presentation.free_form_action = Some(action);
        self
    }

    pub(crate) fn with_empty_message(mut self, message: impl Into<String>) -> Self {
        self.presentation.empty_message = message.into();
        self
    }

    fn into_parts(self) -> (SelectionViewPresentation, Vec<SelectionTab>) {
        (self.presentation, self.tabs)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SelectionInputOutcome {
    Activate(SelectionItemId),
    ActivateFreeForm {
        item_id: SelectionItemId,
        value: String,
    },
    Consumed,
    Dismiss,
    Unhandled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionViewState {
    model: SelectionViewPresentation,
    tabs: TabListState<SelectionTab>,
    selected_visible: Option<usize>,
    search: Option<SearchBoxState>,
}

impl SelectionViewState {
    pub(crate) fn new(model: SelectionViewModel) -> Self {
        let (model, tabs) = model.into_parts();
        let search = model.search.clone().map(SearchBoxState::new);
        let mut state = Self {
            model,
            tabs: TabListState::new(tabs),
            selected_visible: None,
            search,
        };
        state.selected_visible = (state.model.selection_enabled && state.visible_len() > 0)
            .then_some(
                state
                    .model
                    .initial_selected
                    .min(state.visible_len().saturating_sub(1)),
            );
        state
    }

    pub(crate) fn replace_model(&mut self, model: SelectionViewModel) {
        let (model, tabs) = model.into_parts();
        self.search = match (self.search.take(), model.search.clone()) {
            (Some(mut state), Some(search_model)) => {
                state.replace_model(search_model);
                Some(state)
            }
            (None, Some(search_model)) => Some(SearchBoxState::new(search_model)),
            (_, None) => None,
        };
        self.model = model;
        self.tabs.replace_tabs(tabs);
        self.reconcile_selection();
    }

    pub(crate) fn title(&self) -> &str {
        &self.model.title
    }

    pub(crate) fn title_top_margin(&self) -> usize {
        self.model.title_top_margin
    }

    pub(crate) fn title_bottom_margin(&self) -> usize {
        self.model.title_bottom_margin
    }

    pub(crate) fn tabs(&self) -> &[SelectionTab] {
        self.tabs.tabs()
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        match self.tabs.select(index) {
            TabListInputOutcome::ActiveChanged => {
                self.select_first_visible();
                true
            }
            TabListInputOutcome::Consumed => true,
            TabListInputOutcome::Unhandled => false,
        }
    }

    pub(super) fn tab_list(&self) -> &TabListState<SelectionTab> {
        &self.tabs
    }

    pub(crate) fn query(&self) -> &str {
        self.search
            .as_ref()
            .map(SearchBoxState::query)
            .unwrap_or_default()
    }

    pub(crate) fn show_tabs(&self) -> bool {
        self.model.show_tabs
    }

    #[cfg(test)]
    pub(crate) fn search_active(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(SearchBoxState::input_active)
    }

    pub(crate) fn search(&self) -> Option<&SearchBoxState> {
        self.search.as_ref()
    }

    pub(crate) fn empty_message(&self) -> &str {
        &self.model.empty_message
    }

    pub(crate) fn visible_items(&self) -> Vec<&SelectionItem> {
        self.visible_indices()
            .into_iter()
            .filter_map(|index| self.active_tab().items.get(index))
            .collect()
    }

    pub(crate) fn selected_visible_index(&self) -> Option<usize> {
        self.selected_visible
    }

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        let tabs_are_clickable = self.model.show_tabs && self.tabs.tabs().len() > 1;
        let items_are_clickable = self.model.selection_enabled
            && self
                .visible_items()
                .into_iter()
                .any(|item| item.id().is_some());
        if tabs_are_clickable || items_are_clickable {
            MouseMode::UiClick
        } else {
            MouseMode::TerminalSelection
        }
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        let selectable = self.model.selection_enabled
            && self
                .visible_items()
                .get(index)
                .is_some_and(|item| item.id().is_some());
        if !selectable {
            return false;
        }
        self.selected_visible = Some(index);
        true
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<SelectionItemId> {
        self.select_visible_item(index)
            .then(|| self.selected_item_id())
            .flatten()
    }

    pub(crate) fn first_rendered_row(&self, visible_rows: usize) -> usize {
        let Some(selected) = self.selected_visible else {
            return 0;
        };
        selected
            .saturating_add(1)
            .saturating_sub(visible_rows)
            .min(self.visible_len().saturating_sub(visible_rows))
    }

    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        let tab_rows = if self.show_tabs() {
            tab_list::desired_height(self.tabs(), width.saturating_sub(4))
        } else {
            0
        };
        let search_rows = self.search.as_ref().map(|_| SEARCH_BOX_HEIGHT).unwrap_or(0);
        let list_rows = self.visible_len().clamp(1, MAX_VISIBLE_ROWS);
        let preview_rows = self
            .selected_item()
            .and_then(SelectionItem::preview)
            .map(SelectionPreview::desired_height)
            .unwrap_or_default();
        2u16.saturating_add(self.title_top_margin().min(u16::MAX as usize) as u16)
            .saturating_add(self.title_bottom_margin().min(u16::MAX as usize) as u16)
            .saturating_add(tab_rows)
            .saturating_add(search_rows)
            .saturating_add(list_rows.min(u16::MAX as usize) as u16)
            .saturating_add(preview_rows.min(u16::MAX as usize) as u16)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SelectionInputOutcome {
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            if key.kind != KeyEventKind::Press {
                return SelectionInputOutcome::Consumed;
            }
            return match self.model.dismissal {
                SelectionDismissal::Allowed => SelectionInputOutcome::Dismiss,
                SelectionDismissal::Blocked => SelectionInputOutcome::Consumed,
            };
        }
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Enter {
            if let Some(outcome) = self.free_form_outcome() {
                return outcome;
            }
            return SelectionInputOutcome::Consumed;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return match self.model.dismissal {
                SelectionDismissal::Allowed => SelectionInputOutcome::Dismiss,
                SelectionDismissal::Blocked => SelectionInputOutcome::Unhandled,
            };
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return SelectionInputOutcome::Consumed;
        }

        if let Some(search) = self.search.as_mut() {
            match search.handle_key(key) {
                SearchBoxInputOutcome::Consumed => return SelectionInputOutcome::Consumed,
                SearchBoxInputOutcome::QueryChanged => {
                    self.select_first_visible();
                    return SelectionInputOutcome::Consumed;
                }
                SearchBoxInputOutcome::Ignored => {}
            }
        }

        match key.code {
            KeyCode::Up => self.move_selection(SelectionDirection::Previous),
            KeyCode::Down => self.move_selection(SelectionDirection::Next),
            KeyCode::Home => self.select_first_visible(),
            KeyCode::End => self.select_last_visible(),
            KeyCode::Enter => {
                if let Some(id) = self.selected_item_id() {
                    return SelectionInputOutcome::Activate(id);
                }
                if let Some(outcome) = self.free_form_outcome() {
                    return outcome;
                }
            }
            KeyCode::Char(' ') => {
                if self.model.activation_mode == SelectionActivationMode::EnterOrSpace
                    && let Some(id) = self.selected_item_id()
                {
                    return SelectionInputOutcome::Activate(id);
                }
            }
            _ => {}
        }

        match self.tabs.handle_key(key) {
            TabListInputOutcome::ActiveChanged | TabListInputOutcome::Consumed => {
                self.select_first_visible();
            }
            TabListInputOutcome::Unhandled => {}
        }

        SelectionInputOutcome::Consumed
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if let Some(search) = self.search.as_mut()
            && search.handle_paste(pasted) == SearchBoxInputOutcome::QueryChanged
        {
            self.select_first_visible();
        }
    }

    pub(crate) fn active_tab(&self) -> &SelectionTab {
        self.tabs.active_tab()
    }

    fn free_form_outcome(&self) -> Option<SelectionInputOutcome> {
        let value = self.query().trim();
        if value.is_empty() {
            return None;
        }
        Some(SelectionInputOutcome::ActivateFreeForm {
            item_id: self.model.free_form_action.clone()?,
            value: value.to_owned(),
        })
    }

    fn selected_item_id(&self) -> Option<SelectionItemId> {
        self.selected_item()?.id().cloned()
    }

    pub(crate) fn selected_item(&self) -> Option<&SelectionItem> {
        let selected = self.selected_visible?;
        self.visible_items().get(selected).copied()
    }

    pub(crate) fn presentation_highlight(&self) -> Option<Color> {
        self.selected_item()?.selection_foreground()
    }

    fn visible_indices(&self) -> Vec<usize> {
        let normalized_query = self.query().to_lowercase();
        if normalized_query.is_empty() || self.model.query_mode == SelectionQueryMode::InputOnly {
            return (0..self.active_tab().items.len()).collect();
        }
        let mut matches = self
            .active_tab()
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                selection_match_score(item.label(), item.description(), &normalized_query)
                    .map(|score| (index, score))
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|(_, score)| *score);
        matches.into_iter().map(|(index, _)| index).collect()
    }

    fn visible_len(&self) -> usize {
        self.visible_indices().len()
    }

    fn move_selection(&mut self, direction: SelectionDirection) {
        if !self.model.selection_enabled {
            return;
        }
        let visible_len = self.visible_len();
        if visible_len == 0 {
            self.selected_visible = None;
            return;
        }
        let selected = self.selected_visible.unwrap_or(0).min(visible_len - 1);
        self.selected_visible = Some(match direction {
            SelectionDirection::Previous => selected.checked_sub(1).unwrap_or(visible_len - 1),
            SelectionDirection::Next => (selected + 1) % visible_len,
        });
    }

    fn select_first_visible(&mut self) {
        self.selected_visible =
            (self.model.selection_enabled && self.visible_len() > 0).then_some(0);
    }

    fn select_last_visible(&mut self) {
        self.selected_visible = self
            .model
            .selection_enabled
            .then(|| self.visible_len().checked_sub(1))
            .flatten();
    }

    fn reconcile_selection(&mut self) {
        if !self.model.selection_enabled {
            self.selected_visible = None;
            return;
        }
        let visible_len = self.visible_len();
        self.selected_visible = match (self.selected_visible, visible_len) {
            (_, 0) => None,
            (Some(selected), len) => Some(selected.min(len - 1)),
            (None, _) => Some(0),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionDirection {
    Previous,
    Next,
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
