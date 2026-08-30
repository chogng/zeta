use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::style::Color;

use super::ListSelectionPreview;
use super::matcher::selection_match_score;
use crate::components::search_box::SEARCH_BOX_HEIGHT;
use crate::components::search_box::SearchBoxInputOutcome;
use crate::components::search_box::SearchBoxModel;
use crate::components::search_box::SearchBoxState;
use crate::components::tab_list;
use crate::components::tab_list::TabListInputOutcome;
use crate::components::tab_list::TabListItem;
use crate::components::tab_list::TabListState;

const MAX_VISIBLE_ROWS: usize = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ListSelectionActivationMode {
    Enter,
    EnterOrSpace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelectionItem {
    id: Option<ListSelectionItemId>,
    label: String,
    description: Option<String>,
    columns: Option<ListSelectionItemColumns>,
    selection_foreground: Option<Color>,
    preview: Option<ListSelectionPreview>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListSelectionItemColumns {
    pub(super) leading: String,
    pub(super) middle: String,
    pub(super) trailing: String,
}

impl ListSelectionItem {
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

    pub(crate) fn with_id(mut self, id: ListSelectionItemId) -> Self {
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
        let columns = ListSelectionItemColumns {
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

    pub(crate) fn with_preview(mut self, preview: ListSelectionPreview) -> Self {
        self.preview = Some(preview);
        self
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(super) fn columns(&self) -> Option<&ListSelectionItemColumns> {
        self.columns.as_ref()
    }

    pub(crate) fn id(&self) -> Option<&ListSelectionItemId> {
        self.id.as_ref()
    }

    pub(crate) fn selection_foreground(&self) -> Option<Color> {
        self.selection_foreground
    }

    pub(crate) fn preview(&self) -> Option<&ListSelectionPreview> {
        self.preview.as_ref()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ListSelectionItemId(String);

impl ListSelectionItemId {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelectionGroup {
    label: String,
    items: Vec<ListSelectionItem>,
}

impl ListSelectionGroup {
    pub(crate) fn new(label: impl Into<String>, items: Vec<ListSelectionItem>) -> Self {
        Self {
            label: label.into(),
            items,
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }
}

impl TabListItem for ListSelectionGroup {
    fn tab_label(&self) -> &str {
        self.label()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelectionModel {
    tabs: Vec<ListSelectionGroup>,
    presentation: ListSelectionPresentation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ListSelectionPresentation {
    title: String,
    search: Option<SearchBoxModel>,
    empty_message: String,
    activation_mode: ListSelectionActivationMode,
    show_tabs: bool,
    initial_selected: usize,
    title_top_margin: usize,
    title_bottom_margin: usize,
}

impl ListSelectionModel {
    pub(crate) fn new(title: impl Into<String>, tabs: Vec<ListSelectionGroup>) -> Self {
        assert!(
            !tabs.is_empty(),
            "a selection view requires at least one tab"
        );
        Self {
            tabs,
            presentation: ListSelectionPresentation {
                title: title.into(),
                search: None,
                empty_message: "No matching items".into(),
                activation_mode: ListSelectionActivationMode::Enter,
                show_tabs: true,
                initial_selected: 0,
                title_top_margin: 0,
                title_bottom_margin: 0,
            },
        }
    }

    pub(crate) fn with_activation_mode(mut self, mode: ListSelectionActivationMode) -> Self {
        self.presentation.activation_mode = mode;
        self
    }

    pub(crate) fn without_tab_bar(mut self) -> Self {
        self.presentation.show_tabs = false;
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

    pub(crate) fn with_empty_message(mut self, message: impl Into<String>) -> Self {
        self.presentation.empty_message = message.into();
        self
    }

    fn into_parts(self) -> (ListSelectionPresentation, Vec<ListSelectionGroup>) {
        (self.presentation, self.tabs)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ListSelectionInputOutcome {
    Activate(ListSelectionItemId),
    Adjust(ListSelectionItemId, ListSelectionAdjustment),
    Consumed,
    Dismiss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ListSelectionAdjustment {
    Previous,
    Next,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListSelectionState {
    model: ListSelectionPresentation,
    tabs: TabListState<ListSelectionGroup>,
    selected_visible: Option<usize>,
    search: Option<SearchBoxState>,
}

impl ListSelectionState {
    pub(crate) fn new(model: ListSelectionModel) -> Self {
        let (model, tabs) = model.into_parts();
        let search = model.search.clone().map(SearchBoxState::new);
        let mut state = Self {
            model,
            tabs: TabListState::new(tabs),
            selected_visible: None,
            search,
        };
        state.selected_visible = (state.visible_len() > 0).then_some(
            state
                .model
                .initial_selected
                .min(state.visible_len().saturating_sub(1)),
        );
        state
    }

    pub(crate) fn replace_model(&mut self, model: ListSelectionModel) {
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

    pub(crate) fn tabs(&self) -> &[ListSelectionGroup] {
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

    pub(super) fn tab_list(&self) -> &TabListState<ListSelectionGroup> {
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

    pub(crate) fn visible_items(&self) -> Vec<&ListSelectionItem> {
        self.visible_indices()
            .into_iter()
            .filter_map(|index| self.active_tab().items.get(index))
            .collect()
    }

    pub(crate) fn selected_visible_index(&self) -> Option<usize> {
        self.selected_visible
    }

    #[cfg(test)]
    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        let selectable = self
            .visible_items()
            .get(index)
            .is_some_and(|item| item.id().is_some());
        if !selectable {
            return false;
        }
        self.selected_visible = Some(index);
        true
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<ListSelectionItemId> {
        self.visible_items().get(index)?.id().cloned()
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
            .and_then(ListSelectionItem::preview)
            .map(ListSelectionPreview::desired_height)
            .unwrap_or_default();
        2u16.saturating_add(self.title_top_margin().min(u16::MAX as usize) as u16)
            .saturating_add(self.title_bottom_margin().min(u16::MAX as usize) as u16)
            .saturating_add(tab_rows)
            .saturating_add(search_rows)
            .saturating_add(list_rows.min(u16::MAX as usize) as u16)
            .saturating_add(preview_rows.min(u16::MAX as usize) as u16)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ListSelectionInputOutcome {
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            if key.kind != KeyEventKind::Press {
                return ListSelectionInputOutcome::Consumed;
            }
            return ListSelectionInputOutcome::Dismiss;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return ListSelectionInputOutcome::Dismiss;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return ListSelectionInputOutcome::Consumed;
        }

        if let Some(search) = self.search.as_mut() {
            match search.handle_key(key) {
                SearchBoxInputOutcome::Consumed => return ListSelectionInputOutcome::Consumed,
                SearchBoxInputOutcome::QueryChanged => {
                    self.select_first_visible();
                    return ListSelectionInputOutcome::Consumed;
                }
                SearchBoxInputOutcome::Ignored => {}
            }
        }

        match key.code {
            KeyCode::Up => self.move_selection(ListSelectionDirection::Previous),
            KeyCode::Down => self.move_selection(ListSelectionDirection::Next),
            KeyCode::Home => self.select_first_visible(),
            KeyCode::End => self.select_last_visible(),
            KeyCode::Enter => {
                if let Some(id) = self.selected_item_id() {
                    return ListSelectionInputOutcome::Activate(id);
                }
            }
            KeyCode::Left | KeyCode::Right
                if !self
                    .search
                    .as_ref()
                    .is_some_and(SearchBoxState::input_active) =>
            {
                if let Some(id) = self.selected_item_id() {
                    let adjustment = if key.code == KeyCode::Left {
                        ListSelectionAdjustment::Previous
                    } else {
                        ListSelectionAdjustment::Next
                    };
                    return ListSelectionInputOutcome::Adjust(id, adjustment);
                }
            }
            KeyCode::Char(' ') => {
                if self.model.activation_mode == ListSelectionActivationMode::EnterOrSpace
                    && let Some(id) = self.selected_item_id()
                {
                    return ListSelectionInputOutcome::Activate(id);
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

        ListSelectionInputOutcome::Consumed
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if let Some(search) = self.search.as_mut()
            && search.handle_paste(pasted) == SearchBoxInputOutcome::QueryChanged
        {
            self.select_first_visible();
        }
    }

    pub(crate) fn active_tab(&self) -> &ListSelectionGroup {
        self.tabs.active_tab()
    }

    fn selected_item_id(&self) -> Option<ListSelectionItemId> {
        self.selected_item()?.id().cloned()
    }

    pub(crate) fn selected_item(&self) -> Option<&ListSelectionItem> {
        let selected = self.selected_visible?;
        self.visible_items().get(selected).copied()
    }

    pub(crate) fn presentation_highlight(&self) -> Option<Color> {
        self.selected_item()?.selection_foreground()
    }

    fn visible_indices(&self) -> Vec<usize> {
        let normalized_query = self.query().to_lowercase();
        if normalized_query.is_empty() {
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

    fn move_selection(&mut self, direction: ListSelectionDirection) {
        let visible_len = self.visible_len();
        if visible_len == 0 {
            self.selected_visible = None;
            return;
        }
        let selected = self.selected_visible.unwrap_or(0).min(visible_len - 1);
        self.selected_visible = Some(match direction {
            ListSelectionDirection::Previous => selected.checked_sub(1).unwrap_or(visible_len - 1),
            ListSelectionDirection::Next => (selected + 1) % visible_len,
        });
    }

    fn select_first_visible(&mut self) {
        self.selected_visible = (self.visible_len() > 0).then_some(0);
    }

    fn select_last_visible(&mut self) {
        self.selected_visible = self.visible_len().checked_sub(1);
    }

    fn reconcile_selection(&mut self) {
        let visible_len = self.visible_len();
        self.selected_visible = match (self.selected_visible, visible_len) {
            (_, 0) => None,
            (Some(selected), len) => Some(selected.min(len - 1)),
            (None, _) => Some(0),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ListSelectionDirection {
    Previous,
    Next,
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
