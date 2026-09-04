use crate::TuiStartupContext;
use crate::config::ConfigChoices;
use crate::config::ConfigEditor;
use crate::config::ConfigEditorOutcome;
use crate::config::ConfigEditorPage;
use crate::connectors::ConnectorChoices;
use crate::connectors::ConnectorSelectionAction;
use crate::dirs::DirChoices;
use crate::dirs::DirSelectionAction;
use crate::keymap::KeymapChoices;
use crate::keymap::KeymapEditor;
use crate::keymap::KeymapEditorOutcome;
use crate::keymap::KeymapEditorPage;
use crate::mcp::McpChoices;
use crate::mcp::McpSelectionAction;
use crate::models::ModelChoices;
use crate::models::ModelSelectionAction;
use crate::sessions::SessionChoices;
use crate::sessions::SessionSelectionAction;
use crate::skills::SkillChoices;
use crate::skills::SkillSelectionAction;
use crate::status::ProcessResourcesView;
use crate::status::StatusLineChoices;
use crate::status::StatusLineSelectionAction;
use crate::status::StatusPanel;
use crate::status::StatusPanelOutcome;
use crate::theme::ThemeChoices;
use crate::theme::ThemePicker;
use crate::theme::ThemePickerOutcome;
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
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use std::collections::BTreeMap;

const TITLE_BAR_ROWS: u16 = 1;
const TITLE_BODY_GAP_ROWS: u16 = 1;
const HEADER_ROWS: u16 = TITLE_BAR_ROWS + TITLE_BODY_GAP_ROWS;
const CONTENT_HORIZONTAL_MARGIN: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandPanelPointerTarget {
    Tab(usize),
    Search,
    Item(usize),
}

#[derive(Clone, Copy, Debug)]
enum CommandPanelBody<'a> {
    Selection(&'a ListSelectionState),
    Prompt(&'a TextPrompt),
    KeyCapture(&'a KeyCapture),
    Status(&'a StatusPanel),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommandPanelLayout {
    tabs: Rect,
    body: Rect,
}

impl CommandPanelLayout {
    fn new(area: Rect, tab_rows: u16) -> Self {
        let header_rows = HEADER_ROWS.min(area.height);
        let available_rows = area.height.saturating_sub(header_rows);
        let tab_rows = tab_rows.min(available_rows);
        let tabs = crate::render::horizontal_margin(
            Rect::new(
                area.x,
                area.y.saturating_add(header_rows),
                area.width,
                tab_rows,
            ),
            CONTENT_HORIZONTAL_MARGIN,
        );
        let body = crate::render::horizontal_margin(
            Rect::new(
                area.x,
                area.y.saturating_add(header_rows).saturating_add(tab_rows),
                area.width,
                available_rows.saturating_sub(tab_rows),
            ),
            CONTENT_HORIZONTAL_MARGIN,
        );
        Self { tabs, body }
    }

    fn content_width(width: u16) -> u16 {
        width.saturating_sub(CONTENT_HORIZONTAL_MARGIN.saturating_mul(2))
    }
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
    Rewind(ListSelection<RewindSelectionAction>),
    Sessions(ListSelection<SessionSelectionAction>),
    Skills(ListSelection<SkillSelectionAction>),
    Startup(ListSelection<()>),
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

    pub(crate) fn rewind(spec: RewindChoices) -> Self {
        Self::Rewind(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn sessions(spec: SessionChoices) -> Self {
        Self::Sessions(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn skills(spec: SkillChoices) -> Self {
        Self::Skills(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn startup(context: &TuiStartupContext) -> Self {
        Self::Startup(ListSelection::new(
            crate::startup::choices(context),
            BTreeMap::new(),
        ))
    }

    pub(crate) fn status_line(spec: StatusLineChoices) -> Self {
        Self::StatusLine(ListSelection::new(spec.model, spec.actions))
    }

    pub(crate) fn status(panel: StatusPanel) -> Self {
        Self::Status(panel)
    }

    pub(crate) fn apply_process_resources(&mut self, resources: ProcessResourcesView) {
        if let Self::Status(panel) = self {
            panel.apply_process_resources(resources);
        }
    }

    pub(crate) fn process_resources_visible(&self, area: Rect) -> bool {
        match self {
            Self::Status(panel) => {
                let content_width = CommandPanelLayout::content_width(area.width);
                let layout = CommandPanelLayout::new(area, panel.tab_rows(content_width));
                panel.process_resources_visible(layout.body)
            }
            _ => false,
        }
    }

    pub(crate) fn theme(spec: ThemeChoices) -> Self {
        Self::Theme(ThemePicker::new(spec))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> CommandPanelOutcome {
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
            Self::Rewind(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Rewind)
            }
            Self::Sessions(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Sessions)
            }
            Self::Skills(content) => {
                map_selection(content.handle_key(key), CommandPanelOutcome::Skills)
            }
            Self::Startup(content) => map_read_only(content.handle_key(key)),
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
            Self::Rewind(content) => content.handle_paste(pasted),
            Self::Sessions(content) => content.handle_paste(pasted),
            Self::Skills(content) => content.handle_paste(pasted),
            Self::Startup(content) => content.handle_paste(pasted),
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
            Self::Rewind(selection) => Some(selection.state()),
            Self::Sessions(selection) => Some(selection.state()),
            Self::Skills(selection) => Some(selection.state()),
            Self::Startup(selection) => Some(selection.state()),
            Self::Status(_) => None,
            Self::StatusLine(selection) => Some(selection.state()),
            Self::Theme(picker) => Some(picker.selection()),
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
            | Self::Rewind(_)
            | Self::Sessions(_)
            | Self::Skills(_)
            | Self::Startup(_)
            | Self::Status(_)
            | Self::StatusLine(_)
            | Self::Theme(_) => None,
        }
    }

    fn body(&self) -> CommandPanelBody<'_> {
        match self {
            Self::Help(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Dirs(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Config(editor) => match editor.page() {
                ConfigEditorPage::Selection(selection) => CommandPanelBody::Selection(selection),
                ConfigEditorPage::Prompt(prompt) => CommandPanelBody::Prompt(prompt),
            },
            Self::Connectors(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Keymap(editor) => match editor.page() {
                KeymapEditorPage::Selection(selection) => CommandPanelBody::Selection(selection),
                KeymapEditorPage::Capture(capture) => CommandPanelBody::KeyCapture(capture),
            },
            Self::Mcp(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Model(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Rewind(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Sessions(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Skills(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Startup(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Status(panel) => CommandPanelBody::Status(panel),
            Self::StatusLine(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Theme(picker) => CommandPanelBody::Selection(picker.selection()),
        }
    }

    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        let body = self.body();
        let content_width = CommandPanelLayout::content_width(width);
        HEADER_ROWS
            .saturating_add(body.tab_rows(content_width))
            .saturating_add(body.body_rows(content_width))
    }

    pub(crate) fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        hovered: Option<CommandPanelPointerTarget>,
        pressed: Option<CommandPanelPointerTarget>,
        context: crate::render::RenderContext<'_>,
    ) {
        let body = self.body();
        let content_width = CommandPanelLayout::content_width(area.width);
        let layout = CommandPanelLayout::new(area, body.tab_rows(content_width));
        let presentation_focus = body.presentation_focus().unwrap_or_else(|| context.focus());
        let title_style = Style::default()
            .fg(presentation_focus)
            .add_modifier(Modifier::BOLD);
        frame.render_widget(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(presentation_focus))
                .title(Line::from(vec![
                    Span::styled("─", Style::default().fg(presentation_focus)),
                    Span::styled(format!(" {} ", body.title()), title_style),
                ])),
            area,
        );
        body.draw_tabs(
            frame,
            layout.tabs,
            tab_index(hovered),
            tab_index(pressed),
            context,
        );
        body.draw_body(frame, layout.body, hovered, pressed, context);
    }

    pub(crate) fn pointer_target_at(
        &self,
        area: Rect,
        column: u16,
        row: u16,
    ) -> Option<CommandPanelPointerTarget> {
        let body = self.body();
        let content_width = CommandPanelLayout::content_width(area.width);
        let layout = CommandPanelLayout::new(area, body.tab_rows(content_width));
        body.pointer_target_at(layout, column, row)
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
            Self::Rewind(content) => content.key_hints(),
            Self::Sessions(content) => content.key_hints(),
            Self::Skills(content) => content.key_hints(),
            Self::Startup(content) => content.key_hints(),
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
            Self::Rewind(content) => content.select_tab(index),
            Self::Sessions(content) => content.select_tab(index),
            Self::Skills(content) => content.select_tab(index),
            Self::Startup(content) => content.select_tab(index),
            Self::Status(content) => content.select_tab(index),
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
            Self::Rewind(content) => content.focus_search(),
            Self::Sessions(content) => content.focus_search(),
            Self::Skills(content) => content.focus_search(),
            Self::Startup(content) => content.focus_search(),
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
            Self::Rewind(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Rewind)),
            Self::Sessions(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Sessions)),
            Self::Skills(content) => content
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, CommandPanelOutcome::Skills)),
            Self::Startup(content) => content.activate_visible_item(index).map(map_read_only),
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

impl<'a> CommandPanelBody<'a> {
    fn title(self) -> &'a str {
        match self {
            Self::Selection(selection) => selection.title(),
            Self::Prompt(prompt) => prompt.title(),
            Self::KeyCapture(capture) => capture.title(),
            Self::Status(panel) => panel.title(),
        }
    }

    fn tab_rows(self, width: u16) -> u16 {
        match self {
            Self::Selection(selection) => selection.tab_rows(width),
            Self::Status(panel) => panel.tab_rows(width),
            Self::Prompt(_) | Self::KeyCapture(_) => 0,
        }
    }

    fn body_rows(self, width: u16) -> u16 {
        match self {
            Self::Selection(selection) => selection.body_rows(),
            Self::Prompt(prompt) => prompt.desired_height(),
            Self::KeyCapture(capture) => capture.desired_height(),
            Self::Status(panel) => panel.body_rows(width),
        }
    }

    fn presentation_focus(self) -> Option<ratatui::style::Color> {
        match self {
            Self::Selection(selection) => selection.presentation_focus(),
            Self::Prompt(_) | Self::KeyCapture(_) | Self::Status(_) => None,
        }
    }

    fn draw_tabs(
        self,
        frame: &mut Frame<'_>,
        area: Rect,
        hovered_tab: Option<usize>,
        pressed_tab: Option<usize>,
        context: crate::render::RenderContext<'_>,
    ) {
        match self {
            Self::Selection(selection) => {
                list_selection::draw_tabs(frame, area, selection, hovered_tab, pressed_tab, context)
            }
            Self::Status(panel) => panel.draw_tabs(frame, area, hovered_tab, pressed_tab, context),
            Self::Prompt(_) | Self::KeyCapture(_) => {}
        }
    }

    fn draw_body(
        self,
        frame: &mut Frame<'_>,
        area: Rect,
        hovered: Option<CommandPanelPointerTarget>,
        pressed: Option<CommandPanelPointerTarget>,
        context: crate::render::RenderContext<'_>,
    ) {
        match self {
            Self::Selection(selection) => list_selection::draw_body_with_pointer(
                frame,
                area,
                selection,
                hovered == Some(CommandPanelPointerTarget::Search),
                pressed == Some(CommandPanelPointerTarget::Search),
                item_index(hovered),
                item_index(pressed),
                context,
            ),
            Self::Prompt(prompt) => text_prompt::draw(frame, area, prompt, context),
            Self::KeyCapture(capture) => key_capture::draw(frame, area, capture, context),
            Self::Status(panel) => panel.draw_body(frame, area, context),
        }
    }

    fn pointer_target_at(
        self,
        layout: CommandPanelLayout,
        column: u16,
        row: u16,
    ) -> Option<CommandPanelPointerTarget> {
        match self {
            Self::Selection(selection) => selection
                .tab_index_in(layout.tabs, column, row)
                .map(CommandPanelPointerTarget::Tab)
                .or_else(|| {
                    selection
                        .search_contains_in(layout.body, column, row)
                        .then_some(CommandPanelPointerTarget::Search)
                })
                .or_else(|| {
                    selection
                        .item_index_in(layout.body, column, row)
                        .map(CommandPanelPointerTarget::Item)
                }),
            Self::Status(panel) => panel
                .tab_index_in(layout.tabs, column, row)
                .map(CommandPanelPointerTarget::Tab),
            Self::Prompt(_) | Self::KeyCapture(_) => None,
        }
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
