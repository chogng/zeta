use crate::TuiStartupContext;
use crate::config::ConfigChoices;
use crate::config::ConfigEditor;
use crate::config::ConfigEditorOutcome;
use crate::config::ConfigEditorPage;
use crate::connectors::ConnectorChoices;
use crate::connectors::ConnectorSelectionAction;
use crate::dirs::DirChoices;
use crate::dirs::DirPanel;
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
use crate::widgets::panel::PanelLayout;
use crate::widgets::text_prompt;
use crate::widgets::text_prompt::TextPrompt;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
enum CommandPanelBody<'a> {
    Selection(&'a ListSelectionState),
    Prompt(&'a TextPrompt),
    Provider(&'a crate::config::provider::Panel),
    KeyCapture(&'a KeyCapture),
    Status(&'a StatusPanel),
}

#[derive(Debug)]
pub(crate) enum CommandPanel {
    Help(ListSelection<()>),
    Dirs(DirPanel),
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
    pub(crate) fn is_testing(&self) -> bool {
        matches!(self, Self::Config(editor) if editor.is_testing())
    }

    pub(crate) fn help(model: crate::widgets::list_selection::ListSelectionModel) -> Self {
        Self::Help(ListSelection::new(model, BTreeMap::new()))
    }

    pub(crate) fn dirs(spec: DirChoices) -> Self {
        Self::Dirs(DirPanel::new(spec))
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
            super::startup::choices(context),
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

    pub(crate) fn apply_memory_diagnostics(&mut self, status: crate::memory::Status) {
        if let Self::Status(panel) = self {
            panel.apply_memory_diagnostics(status);
        }
    }

    pub(crate) fn process_resources_visible(&self, area: Rect) -> bool {
        match self {
            Self::Status(panel) => {
                let content_width = PanelLayout::content_width(area.width);
                let layout = PanelLayout::new(area, panel.tab_rows(content_width));
                panel.process_resources_visible(layout.body)
            }
            _ => false,
        }
    }

    pub(crate) fn theme(spec: ThemeChoices) -> Self {
        Self::Theme(ThemePicker::new(spec))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent, area: Rect) -> CommandPanelOutcome {
        let body = self.body();
        let layout = PanelLayout::new(area, body.tab_rows(PanelLayout::content_width(area.width)));
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
                if key.kind == crossterm::event::KeyEventKind::Press && key.modifiers.is_empty() && key.code == crossterm::event::KeyCode::Char('p') && content.state().items_focused() {
                    if let Some(ModelSelectionAction::Select { preference, pinned }) = content.state().selected_item().and_then(|item| item.id()).and_then(|id| content.action(id)) {
                        return CommandPanelOutcome::Model(ModelSelectionAction::Pin { preference: preference.clone(), pinned: !pinned });
                    }
                }
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
            Self::Status(content) => match content.handle_key(key, layout.body) {
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

    fn body(&self) -> CommandPanelBody<'_> {
        match self {
            Self::Help(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Dirs(selection) => CommandPanelBody::Selection(selection.state()),
            Self::Config(editor) => match editor.page() {
                ConfigEditorPage::Selection(selection) => CommandPanelBody::Selection(selection),
                ConfigEditorPage::Prompt(prompt) => CommandPanelBody::Prompt(prompt),
                ConfigEditorPage::Provider(panel) => CommandPanelBody::Provider(panel),
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
        let content_width = PanelLayout::content_width(width);
        crate::widgets::panel::HEADER_ROWS
            .saturating_add(body.tab_rows(content_width))
            .saturating_add(body.body_rows(content_width))
    }

    pub(crate) fn draw(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        context: crate::render::RenderContext<'_>,
    ) {
        let body = self.body();
        let content_width = PanelLayout::content_width(area.width);
        let layout = PanelLayout::new(area, body.tab_rows(content_width));
        let presentation_focus = body.presentation_focus().unwrap_or_else(|| context.focus());
        crate::widgets::panel::draw_header(frame, area, body.title(), presentation_focus);
        body.draw_tabs(frame, layout.tabs, None, None, context);
        body.draw_body(frame, layout.body, context);
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

    pub(crate) fn replace_dirs(&mut self, spec: DirChoices) -> bool {
        let Self::Dirs(content) = self else {
            return false;
        };
        content.replace(spec);
        true
    }

    pub(crate) fn finish_dir_add(
        &mut self,
        request_id: u64,
        result: Result<crate::dirs::AddedDir, String>,
    ) {
        if let Self::Dirs(content) = self {
            content.finish_add(request_id, result);
        }
    }

    pub(crate) fn finish_issue_models(
        &mut self,
        request_id: zeta_protocol::CommandId,
        result: Result<ConfigChoices, String>,
    ) {
        if let Self::Config(content) = self {
            content.finish_issue_models(request_id, result);
        }
    }

    pub(crate) fn replace_config(&mut self, spec: ConfigChoices) -> bool {
        let Self::Config(content) = self else {
            return false;
        };
        content.replace(spec);
        true
    }

    pub(crate) fn open_subscription(&mut self, spec: ConfigChoices) {
        if let Self::Config(content) = self {
            content.open_subscription(spec);
        }
    }

    pub(crate) fn update_subscription(&mut self, spec: ConfigChoices) {
        if let Self::Config(content) = self {
            content.update_subscription(spec);
        }
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
        | ListSelectionOutcome::Consumed
        | ListSelectionOutcome::FocusPrevious => CommandPanelOutcome::Consumed,
        ListSelectionOutcome::Dismiss => CommandPanelOutcome::Dismiss,
    }
}

fn map_selection<A>(
    outcome: ListSelectionOutcome<A>,
    activate: impl FnOnce(A) -> CommandPanelOutcome,
) -> CommandPanelOutcome {
    match outcome {
        ListSelectionOutcome::Activate(action) => activate(action),
        ListSelectionOutcome::Adjust(_, _)
        | ListSelectionOutcome::Consumed
        | ListSelectionOutcome::FocusPrevious => CommandPanelOutcome::Consumed,
        ListSelectionOutcome::Dismiss => CommandPanelOutcome::Dismiss,
    }
}

impl<'a> CommandPanelBody<'a> {
    fn title(self) -> &'a str {
        match self {
            Self::Selection(selection) => selection.title(),
            Self::Prompt(prompt) => prompt.title(),
            Self::Provider(_) => "Custom provider",
            Self::KeyCapture(capture) => capture.title(),
            Self::Status(panel) => panel.title(),
        }
    }

    fn tab_rows(self, width: u16) -> u16 {
        match self {
            Self::Selection(selection) => selection.tab_rows(width),
            Self::Status(panel) => panel.tab_rows(width),
            Self::Provider(_) | Self::Prompt(_) | Self::KeyCapture(_) => 0,
        }
    }

    fn body_rows(self, width: u16) -> u16 {
        match self {
            Self::Selection(selection) => selection.body_rows(),
            Self::Prompt(prompt) => prompt.desired_height(),
            Self::KeyCapture(capture) => capture.desired_height(),
            Self::Status(panel) => panel.body_rows(width),
            Self::Provider(panel) => panel.body_rows(),
        }
    }

    fn presentation_focus(self) -> Option<ratatui::style::Color> {
        match self {
            Self::Selection(selection) => selection.presentation_focus(),
            Self::Prompt(_) | Self::KeyCapture(_) | Self::Status(_) | Self::Provider(_) => None,
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
            Self::Provider(_) | Self::Prompt(_) | Self::KeyCapture(_) => {}
        }
    }

    fn draw_body(
        self,
        frame: &mut Frame<'_>,
        area: Rect,
        context: crate::render::RenderContext<'_>,
    ) {
        match self {
            Self::Selection(selection) => list_selection::draw_body_with_pointer(
                frame, area, selection, false, false, None, None, context,
            ),
            Self::Prompt(prompt) => text_prompt::draw(frame, area, prompt, context),
            Self::KeyCapture(capture) => key_capture::draw(frame, area, capture, context),
            Self::Status(panel) => panel.draw_body(frame, area, context),
            Self::Provider(panel) => panel.draw_body(frame, area, context),
        }
    }
}
