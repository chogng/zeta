use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use unicode_width::UnicodeWidthStr;

const TAB_GAP: usize = 2;
const MAX_VISIBLE_ROWS: usize = 12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionItem {
    id: Option<SelectionItemId>,
    label: String,
    description: Option<String>,
}

impl SelectionItem {
    pub(crate) fn new(label: impl Into<String>) -> Self {
        Self {
            id: None,
            label: label.into(),
            description: None,
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

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub(crate) fn id(&self) -> Option<&SelectionItemId> {
        self.id.as_ref()
    }

    fn matches(&self, normalized_query: &str) -> bool {
        normalized_query.is_empty()
            || self.label.to_lowercase().contains(normalized_query)
            || self
                .description
                .as_ref()
                .is_some_and(|description| description.to_lowercase().contains(normalized_query))
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionViewModel {
    title: String,
    tabs: Vec<SelectionTab>,
    search_placeholder: String,
    empty_message: String,
    footer_hint: String,
}

impl SelectionViewModel {
    pub(crate) fn new(title: impl Into<String>, tabs: Vec<SelectionTab>) -> Self {
        assert!(
            !tabs.is_empty(),
            "a selection view requires at least one tab"
        );
        Self {
            title: title.into(),
            tabs,
            search_placeholder: "Search…".into(),
            empty_message: "No matching items".into(),
            footer_hint: "Type to search  ·  ←/→ tabs  ·  ↑/↓ select  ·  Esc back".into(),
        }
    }

    pub(crate) fn with_search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    pub(crate) fn with_empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = message.into();
        self
    }

    pub(crate) fn with_footer_hint(mut self, hint: impl Into<String>) -> Self {
        self.footer_hint = hint.into();
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SelectionInputOutcome {
    Activate(SelectionItemId),
    Consumed,
    Dismiss,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectionViewState {
    model: SelectionViewModel,
    active_tab: usize,
    selected_visible: Option<usize>,
    query: String,
}

impl SelectionViewState {
    pub(crate) fn new(model: SelectionViewModel) -> Self {
        let mut state = Self {
            model,
            active_tab: 0,
            selected_visible: None,
            query: String::new(),
        };
        state.select_first_visible();
        state
    }

    pub(crate) fn replace_model(&mut self, model: SelectionViewModel) {
        self.model = model;
        self.active_tab = self.active_tab.min(self.model.tabs.len().saturating_sub(1));
        self.reconcile_selection();
    }

    pub(crate) fn title(&self) -> &str {
        &self.model.title
    }

    pub(crate) fn tabs(&self) -> &[SelectionTab] {
        &self.model.tabs
    }

    pub(crate) fn active_tab_index(&self) -> usize {
        self.active_tab
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn search_placeholder(&self) -> &str {
        &self.model.search_placeholder
    }

    pub(crate) fn empty_message(&self) -> &str {
        &self.model.empty_message
    }

    pub(crate) fn footer_hint(&self) -> &str {
        &self.model.footer_hint
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
        let tab_rows = tab_row_count(self.tabs(), width.saturating_sub(4));
        let list_rows = self.visible_len().clamp(1, MAX_VISIBLE_ROWS);
        2u16.saturating_add(tab_rows)
            .saturating_add(3)
            .saturating_add(list_rows.min(u16::MAX as usize) as u16)
            .saturating_add(1)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SelectionInputOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return SelectionInputOutcome::Dismiss;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return SelectionInputOutcome::Consumed;
        }

        match key.code {
            KeyCode::Esc => return SelectionInputOutcome::Dismiss,
            KeyCode::Left | KeyCode::BackTab => self.switch_tab(TabDirection::Previous),
            KeyCode::Right | KeyCode::Tab => self.switch_tab(TabDirection::Next),
            KeyCode::Up => self.move_selection(SelectionDirection::Previous),
            KeyCode::Down => self.move_selection(SelectionDirection::Next),
            KeyCode::Home => self.select_first_visible(),
            KeyCode::End => self.select_last_visible(),
            KeyCode::Backspace => {
                self.query.pop();
                self.reconcile_selection();
            }
            KeyCode::Char(' ') => {
                if let Some(id) = self.selected_item_id() {
                    return SelectionInputOutcome::Activate(id);
                }
                self.query.push(' ');
                self.reconcile_selection();
            }
            KeyCode::Char(character) if !character.is_ascii_control() => {
                self.query.push(character);
                self.reconcile_selection();
            }
            _ => {}
        }

        SelectionInputOutcome::Consumed
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        let normalized = pasted.split_whitespace().collect::<Vec<_>>().join(" ");
        self.query.push_str(&normalized);
        self.reconcile_selection();
    }

    fn active_tab(&self) -> &SelectionTab {
        &self.model.tabs[self.active_tab]
    }

    fn selected_item_id(&self) -> Option<SelectionItemId> {
        let selected = self.selected_visible?;
        self.visible_items().get(selected)?.id().cloned()
    }

    fn visible_indices(&self) -> Vec<usize> {
        let normalized_query = self.query.to_lowercase();
        self.active_tab()
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.matches(&normalized_query).then_some(index))
            .collect()
    }

    fn visible_len(&self) -> usize {
        self.visible_indices().len()
    }

    fn switch_tab(&mut self, direction: TabDirection) {
        let tab_count = self.model.tabs.len();
        self.active_tab = match direction {
            TabDirection::Previous => self.active_tab.checked_sub(1).unwrap_or(tab_count - 1),
            TabDirection::Next => (self.active_tab + 1) % tab_count,
        };
        self.select_first_visible();
    }

    fn move_selection(&mut self, direction: SelectionDirection) {
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
enum TabDirection {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionDirection {
    Previous,
    Next,
}

pub(crate) fn tab_row_count(tabs: &[SelectionTab], width: u16) -> u16 {
    if tabs.is_empty() {
        return 0;
    }
    let available_width = usize::from(width.max(1));
    let mut rows = 1u16;
    let mut row_width = 0usize;
    for tab in tabs {
        let tab_width = tab.label().width().saturating_add(2);
        let gap = usize::from(row_width > 0) * TAB_GAP;
        if row_width > 0
            && row_width.saturating_add(gap).saturating_add(tab_width) > available_width
        {
            rows = rows.saturating_add(1);
            row_width = tab_width;
        } else {
            row_width = row_width.saturating_add(gap).saturating_add(tab_width);
        }
    }
    rows
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
