use crate::components::list_selection::ListSelectionAdjustment;
use crate::components::list_selection::ListSelectionState;
use crate::components::region::RegionSpec;
use crate::components::region::RegionView;
use crate::components::region::SelectionRegion;
use crate::components::region::SelectionRegionOutcome;
use crate::features::config::ConfigEditor;
use crate::features::config::ConfigEditorOutcome;
use crate::features::config::ConfigChoices;
use crate::features::connectors::ConnectorChoices;
use crate::features::connectors::ConnectorSelectionAction;
use crate::features::dirs::DirChoices;
use crate::features::dirs::DirSelectionAction;
use crate::features::keymap::KeymapEditor;
use crate::features::keymap::KeymapEditorOutcome;
use crate::features::keymap::KeymapChoices;
use crate::features::mcp::McpChoices;
use crate::features::mcp::McpSelectionAction;
use crate::features::models::ModelChoices;
use crate::features::models::ModelSelectionAction;
use crate::features::queue::QueueInput;
use crate::features::queue::QueueChoices;
use crate::features::queue::QueueSelectionAction;
use crate::features::rewind::RewindChoices;
use crate::features::rewind::RewindSelectionAction;
use crate::features::sessions::SessionChoices;
use crate::features::sessions::SessionSelectionAction;
use crate::features::skills::SkillChoices;
use crate::features::skills::SkillSelectionAction;
use crate::features::status_line::StatusLineChoices;
use crate::features::status_line::StatusLineSelectionAction;
use crate::features::theme::ThemePicker;
use crate::features::theme::ThemePickerOutcome;
use crate::features::theme::ThemeChoices;
use crossterm::event::KeyEvent;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum ComposerMode {
    Help(SelectionRegion<()>),
    Dirs(SelectionRegion<DirSelectionAction>),
    Config(ConfigEditor),
    Connectors(SelectionRegion<ConnectorSelectionAction>),
    Keymap(KeymapEditor),
    Mcp(SelectionRegion<McpSelectionAction>),
    Model(SelectionRegion<ModelSelectionAction>),
    Queue(SelectionRegion<QueueSelectionAction>),
    Rewind(SelectionRegion<RewindSelectionAction>),
    Sessions(SelectionRegion<SessionSelectionAction>),
    Skills(SelectionRegion<SkillSelectionAction>),
    StatusLine(SelectionRegion<StatusLineSelectionAction>),
    Theme(ThemePicker),
}

#[derive(Debug)]
pub(crate) enum ComposerOutcome {
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

impl ComposerMode {
    pub(crate) fn help(
        spec: RegionSpec<crate::components::list_selection::ListSelectionModel>,
    ) -> Self {
        Self::Help(SelectionRegion::new(spec, BTreeMap::new()))
    }

    pub(crate) fn dirs(spec: DirChoices) -> Self {
        Self::Dirs(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn config(spec: ConfigChoices) -> Self {
        Self::Config(ConfigEditor::new(spec))
    }

    pub(crate) fn connectors(spec: ConnectorChoices) -> Self {
        Self::Connectors(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn keymap(spec: KeymapChoices) -> Self {
        Self::Keymap(KeymapEditor::new(spec))
    }

    pub(crate) fn mcp(spec: McpChoices) -> Self {
        Self::Mcp(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn model(spec: ModelChoices) -> Self {
        Self::Model(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn queue(spec: QueueChoices) -> Self {
        Self::Queue(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn rewind(spec: RewindChoices) -> Self {
        Self::Rewind(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn sessions(spec: SessionChoices) -> Self {
        Self::Sessions(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn skills(spec: SkillChoices) -> Self {
        Self::Skills(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn status_line(spec: StatusLineChoices) -> Self {
        Self::StatusLine(SelectionRegion::new(spec.model, spec.actions))
    }

    pub(crate) fn theme(spec: ThemeChoices) -> Self {
        Self::Theme(ThemePicker::new(spec))
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ComposerOutcome {
        if let Self::Queue(region) = self
            && let Some(input) = crate::features::queue::region_input(key)
            && let Some(action) = region.selected_action().copied()
        {
            return ComposerOutcome::QueueInput { input, action };
        }
        match self {
            Self::Help(region) => map_read_only(region.handle_key(key)),
            Self::Dirs(region) => map_selection(region.handle_key(key), ComposerOutcome::Dirs),
            Self::Config(region) => ComposerOutcome::Config(region.handle_key(key)),
            Self::Connectors(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Connectors)
            }
            Self::Keymap(region) => ComposerOutcome::Keymap(region.handle_key(key)),
            Self::Mcp(region) => map_selection(region.handle_key(key), ComposerOutcome::Mcp),
            Self::Model(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Model)
            }
            Self::Queue(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Queue)
            }
            Self::Rewind(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Rewind)
            }
            Self::Sessions(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Sessions)
            }
            Self::Skills(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::Skills)
            }
            Self::StatusLine(region) => {
                map_selection(region.handle_key(key), ComposerOutcome::StatusLine)
            }
            Self::Theme(region) => ComposerOutcome::Theme(region.handle_key(key)),
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        match self {
            Self::Help(region) => region.handle_paste(pasted),
            Self::Dirs(region) => region.handle_paste(pasted),
            Self::Config(region) => region.handle_paste(pasted),
            Self::Connectors(region) => region.handle_paste(pasted),
            Self::Keymap(region) => region.handle_paste(pasted),
            Self::Mcp(region) => region.handle_paste(pasted),
            Self::Model(region) => region.handle_paste(pasted),
            Self::Queue(region) => region.handle_paste(pasted),
            Self::Rewind(region) => region.handle_paste(pasted),
            Self::Sessions(region) => region.handle_paste(pasted),
            Self::Skills(region) => region.handle_paste(pasted),
            Self::StatusLine(region) => region.handle_paste(pasted),
            Self::Theme(region) => region.handle_paste(pasted),
        }
    }

    pub(crate) fn view(&self) -> RegionView<'_> {
        match self {
            Self::Help(region) => region.view(),
            Self::Dirs(region) => region.view(),
            Self::Config(region) => region.view(),
            Self::Connectors(region) => region.view(),
            Self::Keymap(region) => region.view(),
            Self::Mcp(region) => region.view(),
            Self::Model(region) => region.view(),
            Self::Queue(region) => region.view(),
            Self::Rewind(region) => region.view(),
            Self::Sessions(region) => region.view(),
            Self::Skills(region) => region.view(),
            Self::StatusLine(region) => region.view(),
            Self::Theme(region) => region.view(),
        }
    }

    pub(crate) fn key_hints(&self) -> &str {
        match self {
            Self::Help(region) => region.key_hints(),
            Self::Dirs(region) => region.key_hints(),
            Self::Config(region) => region.key_hints(),
            Self::Connectors(region) => region.key_hints(),
            Self::Keymap(region) => region.key_hints(),
            Self::Mcp(region) => region.key_hints(),
            Self::Model(region) => region.key_hints(),
            Self::Queue(region) => region.key_hints(),
            Self::Rewind(region) => region.key_hints(),
            Self::Sessions(region) => region.key_hints(),
            Self::Skills(region) => region.key_hints(),
            Self::StatusLine(region) => region.key_hints(),
            Self::Theme(region) => region.key_hints(),
        }
    }

    pub(crate) fn selection(&self) -> Option<&ListSelectionState> {
        match self {
            Self::Help(region) => Some(region.state()),
            Self::Dirs(region) => Some(region.state()),
            Self::Config(region) => region.selection(),
            Self::Connectors(region) => Some(region.state()),
            Self::Keymap(region) => region.selection(),
            Self::Mcp(region) => Some(region.state()),
            Self::Model(region) => Some(region.state()),
            Self::Queue(region) => Some(region.state()),
            Self::Rewind(region) => Some(region.state()),
            Self::Sessions(region) => Some(region.state()),
            Self::Skills(region) => Some(region.state()),
            Self::StatusLine(region) => Some(region.state()),
            Self::Theme(region) => Some(region.selection()),
        }
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        match self {
            Self::Help(region) => region.select_tab(index),
            Self::Dirs(region) => region.select_tab(index),
            Self::Config(region) => region.select_tab(index),
            Self::Connectors(region) => region.select_tab(index),
            Self::Keymap(region) => region.select_tab(index),
            Self::Mcp(region) => region.select_tab(index),
            Self::Model(region) => region.select_tab(index),
            Self::Queue(region) => region.select_tab(index),
            Self::Rewind(region) => region.select_tab(index),
            Self::Sessions(region) => region.select_tab(index),
            Self::Skills(region) => region.select_tab(index),
            Self::StatusLine(region) => region.select_tab(index),
            Self::Theme(region) => region.select_tab(index),
        }
    }

    pub(crate) fn focus_search(&mut self) -> bool {
        match self {
            Self::Help(region) => region.focus_search(),
            Self::Dirs(region) => region.focus_search(),
            Self::Config(region) => region.focus_search(),
            Self::Connectors(region) => region.focus_search(),
            Self::Keymap(region) => region.focus_search(),
            Self::Mcp(region) => region.focus_search(),
            Self::Model(region) => region.focus_search(),
            Self::Queue(region) => region.focus_search(),
            Self::Rewind(region) => region.focus_search(),
            Self::Sessions(region) => region.focus_search(),
            Self::Skills(region) => region.focus_search(),
            Self::StatusLine(region) => region.focus_search(),
            Self::Theme(region) => region.focus_search(),
        }
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<ComposerOutcome> {
        match self {
            Self::Help(region) => region.activate_visible_item(index).map(map_read_only),
            Self::Dirs(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Dirs)),
            Self::Config(region) => region
                .activate_visible_item(index)
                .map(ComposerOutcome::Config),
            Self::Connectors(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Connectors)),
            Self::Keymap(region) => region
                .activate_visible_item(index)
                .map(ComposerOutcome::Keymap),
            Self::Mcp(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Mcp)),
            Self::Model(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Model)),
            Self::Queue(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Queue)),
            Self::Rewind(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Rewind)),
            Self::Sessions(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Sessions)),
            Self::Skills(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::Skills)),
            Self::StatusLine(region) => region
                .activate_visible_item(index)
                .map(|outcome| map_selection(outcome, ComposerOutcome::StatusLine)),
            Self::Theme(region) => region
                .activate_visible_item(index)
                .map(ComposerOutcome::Theme),
        }
    }

    pub(crate) fn replace_dirs(&mut self, spec: DirChoices) -> bool {
        let Self::Dirs(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_config(&mut self, spec: ConfigChoices) -> bool {
        let Self::Config(region) = self else {
            return false;
        };
        region.replace(spec);
        true
    }

    pub(crate) fn finish_config_prompt(&mut self, spec: ConfigChoices) -> bool {
        let Self::Config(region) = self else {
            return false;
        };
        region.close_prompt_and_replace(spec);
        true
    }

    pub(crate) fn replace_connectors(&mut self, spec: ConnectorChoices) -> bool {
        let Self::Connectors(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_keymap_catalog(&mut self, spec: KeymapChoices) -> bool {
        let Self::Keymap(region) = self else {
            return false;
        };
        region.replace_catalog(spec);
        true
    }

    pub(crate) fn replace_mcp(&mut self, spec: McpChoices) -> bool {
        let Self::Mcp(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_queue(&mut self, spec: QueueChoices) -> bool {
        let Self::Queue(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_skills(&mut self, spec: SkillChoices) -> bool {
        let Self::Skills(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn replace_status_line(&mut self, spec: StatusLineChoices) -> bool {
        let Self::StatusLine(region) = self else {
            return false;
        };
        region.replace(spec.model, spec.actions);
        true
    }

    pub(crate) fn push_custom_theme(&mut self, spec: ThemeChoices) -> bool {
        let Self::Theme(region) = self else {
            return false;
        };
        region.push_custom(spec);
        true
    }

    pub(crate) fn is_connectors(&self) -> bool {
        matches!(self, Self::Connectors(_))
    }

    pub(crate) fn is_skills(&self) -> bool {
        matches!(self, Self::Skills(_))
    }
}

fn map_read_only(outcome: SelectionRegionOutcome<()>) -> ComposerOutcome {
    match outcome {
        SelectionRegionOutcome::Activate(())
        | SelectionRegionOutcome::Adjust((), ListSelectionAdjustment::Previous)
        | SelectionRegionOutcome::Adjust((), ListSelectionAdjustment::Next)
        | SelectionRegionOutcome::Consumed => ComposerOutcome::Consumed,
        SelectionRegionOutcome::Dismiss => ComposerOutcome::Dismiss,
    }
}

fn map_selection<A>(
    outcome: SelectionRegionOutcome<A>,
    activate: impl FnOnce(A) -> ComposerOutcome,
) -> ComposerOutcome {
    match outcome {
        SelectionRegionOutcome::Activate(action) => activate(action),
        SelectionRegionOutcome::Adjust(_, _) | SelectionRegionOutcome::Consumed => {
            ComposerOutcome::Consumed
        }
        SelectionRegionOutcome::Dismiss => ComposerOutcome::Dismiss,
    }
}
