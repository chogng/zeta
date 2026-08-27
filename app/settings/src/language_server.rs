use std::collections::BTreeMap;
use std::path::Path;

use zeta_app_server_protocol::protocol::config::{
    ConfigReadResult, LanguageServerConfigDto, LanguageServerModeDto,
};
use zeta_lsp_server_provider::{
    BASH_LANGUAGE_SERVER_ID, JSON_LANGUAGE_SERVER_ID, RUST_ANALYZER_SERVER_ID,
};
use zeta_ui_components::SwitchSelection;
use zui::ui::{TextInput, TextInputCommand, TextInputCompositionEvent};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LanguageServerSettingsTarget {
    RustAnalyzer,
    Json,
    Bash,
}

impl LanguageServerSettingsTarget {
    pub const ALL: [Self; 3] = [Self::RustAnalyzer, Self::Json, Self::Bash];

    pub const fn server_id(self) -> &'static str {
        match self {
            Self::RustAnalyzer => RUST_ANALYZER_SERVER_ID,
            Self::Json => JSON_LANGUAGE_SERVER_ID,
            Self::Bash => BASH_LANGUAGE_SERVER_ID,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::RustAnalyzer => "Rust",
            Self::Json => "JSON",
            Self::Bash => "Shell",
        }
    }

    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::RustAnalyzer => "rust-analyzer",
            Self::Json => "vscode-json-language-server",
            Self::Bash => "bash-language-server",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SaveStatus {
    Idle,
    Saving,
    Saved,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
struct LanguageServerDraft {
    configured: bool,
    mode: LanguageServerModeDto,
    last_enabled_mode: LanguageServerModeDto,
    executable: TextInput,
    dirty: bool,
    status: SaveStatus,
}

impl Default for LanguageServerDraft {
    fn default() -> Self {
        Self {
            configured: false,
            mode: LanguageServerModeDto::Disabled,
            last_enabled_mode: LanguageServerModeDto::Automatic,
            executable: TextInput::default(),
            dirty: false,
            status: SaveStatus::Idle,
        }
    }
}

/// Native drafts for catalog preferences whose durable authority remains App Server Config.
#[derive(Clone, Debug, PartialEq)]
pub struct LanguageServerSettingsState {
    visible: bool,
    revision: Option<u64>,
    generation: Option<u64>,
    selected: LanguageServerSettingsTarget,
    drafts: BTreeMap<LanguageServerSettingsTarget, LanguageServerDraft>,
    search: TextInput,
}

impl Default for LanguageServerSettingsState {
    fn default() -> Self {
        Self {
            visible: false,
            revision: None,
            generation: None,
            selected: LanguageServerSettingsTarget::RustAnalyzer,
            drafts: LanguageServerSettingsTarget::ALL
                .into_iter()
                .map(|target| (target, LanguageServerDraft::default()))
                .collect(),
            search: TextInput::default(),
        }
    }
}

impl LanguageServerSettingsState {
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.current_mut().status = SaveStatus::Idle;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.search.cancel_composition();
        for draft in self.drafts.values_mut() {
            draft.executable.cancel_composition();
        }
    }

    pub fn synchronize(&mut self, configuration: &ConfigReadResult) {
        if self
            .generation
            .is_some_and(|generation| configuration.generation <= generation)
        {
            return;
        }
        self.revision = Some(configuration.revision);
        self.generation = Some(configuration.generation);
        for target in LanguageServerSettingsTarget::ALL {
            let preference = configuration.language_servers.get(target.server_id());
            let draft = self.drafts.get_mut(&target).expect("all drafts exist");
            draft.configured = preference.is_some();
            if !self.visible || !draft.dirty || matches!(draft.status, SaveStatus::Saving) {
                draft.mode = preference
                    .map(|preference| preference.mode)
                    .unwrap_or(LanguageServerModeDto::Disabled);
                draft.last_enabled_mode = match draft.mode {
                    LanguageServerModeDto::Disabled => LanguageServerModeDto::Automatic,
                    mode => mode,
                };
                replace_text(
                    &mut draft.executable,
                    preference.and_then(|preference| preference.executable.as_deref()),
                );
                draft.dirty = false;
                draft.status = SaveStatus::Saved;
            }
        }
    }

    pub const fn selected_server(&self) -> LanguageServerSettingsTarget {
        self.selected
    }

    pub fn search_input(&self) -> &TextInput {
        &self.search
    }

    pub fn selected_search_text(&self) -> Option<&str> {
        self.search.selected_text()
    }

    pub fn apply_search(&mut self, command: TextInputCommand) {
        self.search.apply(command);
    }

    pub fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search.apply_composition(event);
    }

    pub fn cancel_search_composition(&mut self) {
        self.search.cancel_composition();
    }

    pub fn select_server(&mut self, target: LanguageServerSettingsTarget) {
        self.current_mut().executable.cancel_composition();
        self.selected = target;
    }

    pub fn mode(&self) -> LanguageServerModeDto {
        self.current().mode
    }

    pub fn select_mode(&mut self, mode: LanguageServerModeDto) {
        let draft = self.current_mut();
        draft.mode = mode;
        if mode != LanguageServerModeDto::Disabled {
            draft.last_enabled_mode = mode;
        }
        draft.dirty = true;
        draft.status = SaveStatus::Idle;
    }

    pub fn is_enabled(&self) -> bool {
        self.mode() != LanguageServerModeDto::Disabled
    }

    pub fn switch_selection(&self) -> SwitchSelection {
        switch_selection(self.current().mode)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mode = if enabled {
            match self.mode() {
                LanguageServerModeDto::Disabled => self.current().last_enabled_mode,
                mode => mode,
            }
        } else {
            LanguageServerModeDto::Disabled
        };
        self.select_mode(mode);
    }

    pub fn executable_input(&self) -> &TextInput {
        &self.current().executable
    }

    pub fn apply_executable(&mut self, command: TextInputCommand) {
        let draft = self.current_mut();
        draft.executable.apply(command);
        draft.dirty = true;
        draft.status = SaveStatus::Idle;
    }

    pub fn apply_executable_composition(&mut self, event: TextInputCompositionEvent) {
        let draft = self.current_mut();
        draft.executable.apply_composition(event);
        draft.dirty = true;
        draft.status = SaveStatus::Idle;
    }

    pub fn cancel_executable_composition(&mut self) {
        self.current_mut().executable.cancel_composition();
    }

    pub fn selected_executable_text(&self) -> Option<&str> {
        self.current().executable.selected_text()
    }

    pub fn configuration(
        &self,
    ) -> Result<(u64, &'static str, LanguageServerConfigDto), &'static str> {
        let revision = self.revision.ok_or("Configuration is still loading")?;
        let draft = self.current();
        let executable = draft.executable.text().trim();
        if !executable.is_empty() && !Path::new(executable).is_absolute() {
            return Err("Executable override must be an absolute path");
        }
        Ok((
            revision,
            self.selected.server_id(),
            LanguageServerConfigDto {
                mode: draft.mode,
                executable: (!executable.is_empty()).then(|| executable.to_owned()),
            },
        ))
    }

    pub fn reset_target(&self) -> Result<(u64, &'static str), &'static str> {
        self.revision
            .map(|revision| (revision, self.selected.server_id()))
            .ok_or("Configuration is still loading")
    }

    pub fn can_reset(&self) -> bool {
        self.current().configured && self.revision.is_some()
    }

    pub fn can_save(&self) -> bool {
        self.revision.is_some() && !matches!(self.current().status, SaveStatus::Saving)
    }

    pub fn saving(&mut self) {
        self.current_mut().status = SaveStatus::Saving;
    }

    pub fn save_succeeded(&mut self) {
        let draft = self.current_mut();
        draft.dirty = false;
        draft.status = SaveStatus::Saved;
    }

    pub fn save_failed(&mut self, error: impl Into<String>) {
        self.current_mut().status = SaveStatus::Error(error.into());
    }

    pub fn status_message(&self) -> Option<(&str, bool)> {
        match &self.current().status {
            SaveStatus::Idle => None,
            SaveStatus::Saving => Some(("Saving…", false)),
            SaveStatus::Saved => Some(("Saved", false)),
            SaveStatus::Error(error) => Some((error, true)),
        }
    }

    fn current(&self) -> &LanguageServerDraft {
        self.drafts
            .get(&self.selected)
            .expect("selected draft exists")
    }

    fn current_mut(&mut self) -> &mut LanguageServerDraft {
        self.drafts
            .get_mut(&self.selected)
            .expect("selected draft exists")
    }
}

fn replace_text(input: &mut TextInput, value: Option<&str>) {
    input.take_text();
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        input.apply(TextInputCommand::Insert(value.to_owned()));
    }
}

fn switch_selection(mode: LanguageServerModeDto) -> SwitchSelection {
    if mode == LanguageServerModeDto::Disabled {
        SwitchSelection::Off
    } else {
        SwitchSelection::On
    }
}
