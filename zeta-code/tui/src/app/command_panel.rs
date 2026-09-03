use crate::config::ConfigChoices;
use crate::config::ConfigEditor;
use crate::config::ConfigEditorOutcome;
use crate::connectors::ConnectorChoices;
use crate::connectors::ConnectorSelectionAction;
use crate::dirs::DirChoices;
use crate::dirs::DirSelectionAction;
use crate::keymap::KeymapChoices;
use crate::keymap::KeymapEditor;
use crate::keymap::KeymapEditorOutcome;
use crate::mcp::McpChoices;
use crate::mcp::McpSelectionAction;
use crate::models::ModelChoices;
use crate::models::ModelSelectionAction;
use crate::sessions::SessionChoices;
use crate::sessions::SessionSelectionAction;
use crate::skills::SkillChoices;
use crate::skills::SkillSelectionAction;
use crate::status::StatusLineChoices;
use crate::status::StatusLineSelectionAction;
use crate::status::StatusPanel;
use crate::status::StatusPanelOutcome;
use crate::theme::ThemeChoices;
use crate::theme::ThemePicker;
use crate::theme::ThemePickerOutcome;
use crate::thread::queue::QueueChoices;
use crate::thread::queue::QueueInput;
use crate::thread::queue::QueueSelectionAction;
use crate::thread::rewind::RewindChoices;
use crate::thread::rewind::RewindSelectionAction;
use crate::widgets::key_capture;
use crate::widgets::key_capture::KeyCapture;
use crate::widgets::list_selection;
use crate::widgets::list_selection::ListSelection;
use crate::widgets::list_selection::ListSelectionAdjustment;
use crate::widgets::list_selection::ListSelectionOutcome;
use crate::widgets::list_selection::ListSelectionState;
use crate::widgets::text_prompt;
use crate::widgets::text_prompt::TextPrompt;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use std::collections::BTreeMap;

const TITLE_BAR_HEIGHT: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandPanelPointerTarget {
    Tab(usize),
    Search,
    Item(usize),
}

#[derive(Debug)]
pub(crate) enum CommandPanel {
    Help(ListSelection<()>),
    Dirs(ListSelection<DirSelectionAction>),
    Config(ConfigEditor),
    Connectors(ListSelection<ConnectorSelectionAction>),
    Keymap(KeymapEditor),
    Mcp(ListSelection<McpSelectionAction>),
    Model(ListSelection<ModelSelectionAction>),
    Queue(ListSelection<QueueSelectionAction>),
    Rewind(ListSelection<RewindSelectionAction>),
    Sessions(ListSelection<SessionSelectionAction>),
    Skills(ListSelection<SkillSelectionAction>),
    Status(StatusPanel),
    StatusLine(ListSelection<StatusLineSelectionAction>),
    Theme(ThemePicker),
}

#[derive(Debug)]
pub(crate) enum CommandPanelOutcome {
    Dirs(DirSelectionAction),
    Config(ConfigEditorOutcome),
    Connectors(ConnectorSelectionAction),
    Keymap(KeymapEditorOutcome),
    Mcp(McpSelectionAction),
    Model(ModelSelectionAction),
    Queue(QueueSelectionAction),
    QueueInput {
        input: QueueInput,
        action: QueueSelectionAction,
    },
    Rewind(RewindSelectionAction),
    Sessions(SessionSelectionAction),
    Skills(SkillSelectionAction),
    StatusLine(StatusLineSelectionAction),
    Theme(ThemePickerOutcome),
    Consumed,
    Dismiss,
}

impl CommandPanel {
    pub(crate) fn help(model: crate::widgets::list_selection::ListSelectionModel) -> Self {
        Self::Help(ListSelection::new(model, BTreeMap::new()))
    }

    pub(crate) fn dirs(spec: DirChoices) -> Self {
        Self::Dirs(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn config(spec: ConfigChoices) -> Self {
        Self::Config(ConfigEditor::new(spec))
    }

    pub(crate) fn connectors(spec: ConnectorChoices) -> Self {
        Self::Connectors(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn keymap(spec: KeymapChoices) -> Self {
        Self::Keymap(KeymapEditor::new(spec))
    }

    pub(crate) fn mcp(spec: McpChoices) -> Self {
        Self::Mcp(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn model(spec: ModelChoices) -> Self {
        Self::Model(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn queue(spec: QueueChoices) -> Self {
        Self::Queue(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn rewind(spec: RewindChoices) -> Self {
        Self::Rewind(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn sessions(spec: SessionChoices) -> Self {
        Self::Sessions(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn skills(spec: SkillChoices) -> Self {
        Self::Skills(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn status_line(spec: StatusLineChoices) -> Self {
        Self::StatusLine(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn status(panel: StatusPanel) -> Self {
        Self::Status(panel)
    }

    pub(crate) fn theme(spec: ThemeChoices) -> Self {
        Self::Theme(ThemePicker::new(spec))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> CommandPanelOutcome {
        if let Self::Queue(selection) = self
            && let Some(input) = crate::thread::queue::queue_input(key)
            && let Some(action) = selection.selected_action().copied()
        {
            return CommandPanelOutcome::QueueInput { input, action };
        }
        match self {
            Self::Help(content) => map_read_only(content.handle_key(key)),
            Self::Dirs(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Dirs)
            }
            Self::Config(content) => CommandPanelOutcome::Config(content.handle_key(key)),
            Self::Connectors(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Connectors)
            }
            Self::Keymap(content) => CommandPanelOutcome::Keymap(content.handle_key(key)),
            Self::Mcp(content) => map_selection(content.handle_key(key), CommandPanelOutcome::Mcp),
            Self::Model(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Model)
            }
            Self::Queue(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Queue)
            }
            Self::Rewind(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Rewind)
            }
            Self::Sessions(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Sessions)
            }
            Self::Skills(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Skills)
            }
            Self::Status(content) => match content.handle_key(key) {
                StatusPanelOutcome::Consumed => CommandPanelOutcome::Consumed,
                StatusPanelOutcome::Dismiss => CommandPanelOutcome::Dismiss,
            },
            Self::StatusLine(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::StatusLine)
            }
            Self::Theme(content) => CommandPanelOutcome::Theme(content.handle_key(key)),
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        match self {
            Self::Help(content) => content.handle_paste(pasted),
            Self::Dirs(content) => content.handle_paste(pasted),
            Self::Config(content) => content.handle_paste(pasted),
            Self::Connectors(content) => content.handle_paste(pasted),
            Self::Keymap(content) => content.handle_paste(pasted),
            Self::Mcp(content) => content.handle_paste(pasted),
            Self::Model(content) => content.handle_paste(pasted),
            Self::Queue(content) => content.handle_paste(pasted),
            Self::Rewind(content) => content.handle_paste(pasted),
            Self::Sessions(content) => content.handle_paste(pasted),
            Self::Skills(content) => content.handle_paste(pasted),
            Self::Status(_) => {}
            Self::StatusLine(content) => content.handle_paste(pasted),
            Self::Theme(content) => content.handle_paste(pasted),
        }
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        match self {
            Self::Help(selection) => Some(selection.state()),
            Self::Dirs(selection) => Some(selection.state()),
            Self::Config(editor) => editor.selection(),
            Self::Connectors(selection) => Some(selection.state()),
            Self::Keymap(editor) => editor.selection(),
            Self::Mcp(selection) => Some(selection.state()),
            Self::Model(selection) => Some(selection.state()),
            Self::Queue(selection) => Some(selection.state()),
            Self::Rewind(selection) => Some(selection.state()),
            Self::Sessions(selection) => Some(selection.state()),
            Self::Skills(selection) => Some(selection.state()),
            Self::Status(_) => None,
            Self::StatusLine(selection) => Some(selection.state()),
            Self::Theme(picker) => Some(picker.selection()),
        }
    }

    pub(crate) fn text_prompt(&self) -> Option<&TextPrompt> {
        match self {
            Self::Config(editor) => editor.prompt(),
            Self::Help(_)
            | Self::Dirs(_)
            | Self::Connectors(_)
            | Self::Keymap(_)
            | Self::Mcp(_)
            | Self::Model(_)
            | Self::Queue(_)
            | Self::Rewind(_)
            | Self::Sessions(_)
            | Self::Skills(_)
            | Self::Status(_)
            | Self::StatusLine(_)
            | Self::Theme(_) => None,
        }
    }

    pub(crate) fn key_capture(&self) -> Option<&KeyCapture> {
        match self {
            Self::Keymap(editor) => editor.capture(),
            Self::Help(_)
            | Self::Dirs(_)
            | Self::Config(_)
            | Self::Connectors(_)
            | Self::Mcp(_)
            | Self::Model(_)
            | Self::Queue(_)
            | Self::Rewind(_)
            | Self::Sessions(_)
            | Self::Skills(_)
            | Self::Status(_)
            | Self::StatusLine(_)
            | Self::Theme(_) => None,
        }
    }

    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        if let Self::Status(panel) = self {
            return panel.desired_height(width);
        }
        let body_height = if let Some(selection) = self.list_selection() {
            selection.desired_height(width)
        } else if let Some(prompt) = self.text_prompt() {
            prompt.desired_height()
        } else if let Some(capture) = self.key_capture() {
            capture.desired_height()
        } else {
            0
        };
        body_height.saturating_add(TITLE_BAR_HEIGHT)
    }

    pub(crate) fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        hovered: Option<CommandPanelPointerTarget>,
        pressed: Option<CommandPanelPointerTarget>,
        context: crate::render::RenderContext<'_>,
    ) {
        if let Self::Status(panel) = self {
            panel.draw(frame, area, context);
            return;
        }
        let body = composer_body_area(area);
        let presentation_focus = self
            .list_selection()
            .and_then(ListSelectionState::presentation_focus)
            .unwrap_or_else(|| context.focus());
        let title_style = crate::render::interaction_style(
            context,
            crate::render::InteractionState {
                target: crate::render::InteractionTarget::Active,
                selected: false,
                hovered: false,
                pressed: false,
            },
        );
        frame.render_widget(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(presentation_focus))
                .title(Line::from(vec![
                    Span::styled("─", Style::default().fg(presentation_focus)),
                    Span::styled(format!(" {} ", self.title()), title_style),
                ])),
            area,
        );
        if let Some(selection) = self.list_selection() {
            list_selection::draw_with_pointer(
                frame,
                body,
                selection,
                tab_index(hovered),
                tab_index(pressed),
                hovered == Some(CommandPanelPointerTarget::Search),
                pressed == Some(CommandPanelPointerTarget::Search),
                item_index(hovered),
                item_index(pressed),
                context,
            );
        } else if let Some(prompt) = self.text_prompt() {
            text_prompt::draw(frame, body, prompt, context);
        } else if let Some(capture) = self.key_capture() {
            key_capture::draw(frame, body, capture, context);
        }
    }

    pub(crate) fn pointer_target_at(
        &self,
        area: Rect,
        column: u16,
        row: u16,
    ) -> Option<CommandPanelPointerTarget> {
        let selection = self.list_selection()?;
        let body = composer_body_area(area);
        selection
            .tab_index_at(body, column, row)
            .map(CommandPanelPointerTarget::Tab)
            .or_else(|| {
                selection
                    .search_contains(body, column, row)
                    .then_some(CommandPanelPointerTarget::Search)
            })
            .or_else(|| {
                selection
                    .item_index_at(body, column, row)
                    .map(CommandPanelPointerTarget::Item)
            })
    }

    fn title(&self) -> &str {
        if let Some(selection) = self.list_selection() {
            selection.title()
        } else if let Some(prompt) = self.text_prompt() {
            prompt.title()
        } else if let Some(capture) = self.key_capture() {
            capture.title()
        } else {
            ""
        }
    }

    pub(crate) fn key_hints(&self) -> &str {
        match self {
            Self::Help(content) => content.key_hints(),
            Self::Dirs(content) => content.key_hints(),
            Self::Config(content) => content.key_hints(),
            Self::Connectors(content) => content.key_hints(),
            Self::Keymap(content) => content.key_hints(),
            Self::Mcp(content) => content.key_hints(),
            Self::Model(content) => content.key_hints(),
            Self::Queue(content) => content.key_hints(),
            Self::Rewind(content) => content.key_hints(),
            Self::Sessions(content) => content.key_hints(),
            Self::Skills(content) => content.key_hints(),
            Self::Status(content) => content.key_hints(),
            Self::StatusLine(content) => content.key_hints(),
            Self::Theme(content) => content.key_hints(),
        }
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        match self {
            Self::Help(content) => content.select_tab(index),
            Self::Dirs(content) => content.select_tab(index),
            Self::Config(content) => content.select_tab(index),
            Self::Connectors(content) => content.select_tab(index),
            Self::Keymap(content) => content.select_tab(index),
            Self::Mcp(content) => content.select_tab(index),
            Self::Model(content) => content.select_tab(index),
            Self::Queue(content) => content.select_tab(index),
            Self::Rewind(content) => content.select_tab(index),
            Self::Sessions(content) => content.select_tab(index),
            Self::Skills(content) => content.select_tab(index),
            Self::Status(_) => false,
            Self::StatusLine(content) => content.select_tab(index),
            Self::Theme(content) => content.select_tab(index),
        }
    }

    pub(crate) fn focus_search(&mut self) -> bool {
        match self {
            Self::Help(content) => content.focus_search(),
            Self::Dirs(content) => content.focus_search(),
            Self::Config(content) => content.focus_search(),
            Self::Connectors(content) => content.focus_search(),
            Self::Keymap(content) => content.focus_search(),
            Self::Mcp(content) => content.focus_search(),
            Self::Model(content) => content.focus_search(),
            Self::Queue(content) => content.focus_search(),
            Self::Rewind(content) => content.focus_search(),
            Self::Sessions(content) => content.focus_search(),
            Self::Skills(content) => content.focus_search(),
            Self::Status(_) => false,
            Self::StatusLine(content) => content.focus_search(),
            Self::Theme(content) => content.focus_search(),
        }
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<CommandPanelOutcome> {
        match self {
            Self::Help(content) => content.activate_visible_item(index).map(map_read_only),
            Self::Dirs(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Dirs)),
            Self::Config(content) => content
                .activate_visible_item(index)
                .map(CommandPanelOutcome::Config),
            Self::Connectors(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Connectors)),
            Self::Keymap(content) => content
                .activate_visible_item(index)
                .map(CommandPanelOutcome::Keymap),
            Self::Mcp(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Mcp)),
            Self::Model(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Model)),
            Self::Queue(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Queue)),
            Self::Rewind(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Rewind)),
            Self::Sessions(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Sessions)),
            Self::Skills(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Skills)),
            Self::Status(_) => None,
            Self::StatusLine(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::StatusLine)),
            Self::Theme(content) => content
                .activate_visible_item(index)
                .map(CommandPanelOutcome::Theme),
        }
    }

    pub(crate) fn replace_dirs(&mut self, spec: DirChoices) -> bool {
        let Self::Dirs(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_config(&mut self, spec: ConfigChoices) -> bool {
        let Self::Config(content) = self else {
            return false;
        };
        content.replace(spec);
        true
    }

    pub(crate) fn finish_config_prompt(&mut self, spec: ConfigChoices) -> bool {
        let Self::Config(content) = self else {
            return false;
        };
        content.close_prompt_and_replace(spec);
        true
    }

    pub(crate) fn replace_connectors(&mut self, spec: ConnectorChoices) -> bool {
        let Self::Connectors(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_keymap_catalog(&mut self, spec: KeymapChoices) -> bool {
        let Self::Keymap(content) = self else {
            return false;
        };
        content.replace_catalog(spec);
        true
    }

    pub(crate) fn replace_mcp(&mut self, spec: McpChoices) -> bool {
        let Self::Mcp(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_queue(&mut self, spec: QueueChoices) -> bool {
        let Self::Queue(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_skills(&mut self, spec: SkillChoices) -> bool {
        let Self::Skills(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_status_line(&mut self, spec: StatusLineChoices) -> bool {
        let Self::StatusLine(content) = self else {
            return false;
        };
        content.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn push_custom_theme(&mut self, spec: ThemeChoices) -> bool {
        let Self::Theme(content) = self else {
            return false;
        };
        content.push_custom(spec);
        true
    }

    pub(crate) fn is_connectors(&self) -> bool {
        matches!(self, Self::Connectors(_))
    }

    pub(crate) fn is_skills(&self) -> bool {
        matches!(self, Self::Skills(_))
    }
}

fn map_read_only(outcome: ListSelectionOutcome<()>) -> CommandPanelOutcome {
    match outcome {
        ListSelectionOutcome::Activate(())
        | ListSelectionOutcome::Adjust((), ListSelectionAdjustment::Previous)
        | ListSelectionOutcome::Adjust((), ListSelectionAdjustment::Next)
        | ListSelectionOutcome::Consumed => CommandPanelOutcome::Consumed,
        ListSelectionOutcome::Dismiss => CommandPanelOutcome::Dismiss,
    }
}

fn map_selection<A>(
    outcome: ListSelectionOutcome<A>,
    activate: impl FnOnce(A) -> CommandPanelOutcome,
) -> CommandPanelOutcome {
    match outcome {
        ListSelectionOutcome::Activate(action) => activate(action),
        ListSelectionOutcome::Adjust(_, _) | ListSelectionOutcome::Consumed => {
            CommandPanelOutcome::Consumed
        }
        ListSelectionOutcome::Dismiss => CommandPanelOutcome::Dismiss,
    }
}

fn composer_body_area(area: Rect) -> Rect {
    let title_height = TITLE_BAR_HEIGHT.min(area.height);
    Rect {
        y: area.y.saturating_add(title_height),
        height: area.height.saturating_sub(title_height),
        ..area
    }
}

fn tab_index(target: Option<CommandPanelPointerTarget>) -> Option<usize> {
    match target {
        Some(CommandPanelPointerTarget::Tab(index)) => Some(index),
        Some(CommandPanelPointerTarget::Search | CommandPanelPointerTarget::Item(_)) | None => None,
    }
}

fn item_index(target: Option<CommandPanelPointerTarget>) -> Option<usize> {
    match target {
        Some(CommandPanelPointerTarget::Item(index)) => Some(index),
        Some(CommandPanelPointerTarget::Tab(_) | CommandPanelPointerTarget::Search) | None => None,
    }
}
