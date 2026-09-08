use super::ConfigChoices;
use super::ConfigEditorOutcome;
use super::ConfigSelectionAction;
use super::ProviderApiKeyEdit;
use crate::client::new_command_id;
use crate::keymap::bindings;
use crate::keymap::bindings::Keybinding;
use crate::render::RenderContext;
use crate::widgets::key_hint::KeyHints;
use crate::widgets::list_selection;
use crate::widgets::list_selection::ListSelection;
use crate::widgets::list_selection::ListSelectionOutcome;
use crate::widgets::search_box;
use crate::widgets::search_box::SearchBoxModel;
use crate::widgets::search_box::SearchBoxState;
use crate::widgets::tab_list;
use crate::widgets::tab_list::FocusedTabListInputOutcome;
use crate::widgets::tab_list::TabListItem;
use crate::widgets::tab_list::TabListState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::config::CustomProviderConfigDto;
use zeta_app_server_protocol::protocol::config::CustomProviderProtocolDto;
use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
use zeta_app_server_protocol::protocol::provider::ProviderListResult;

const OFFICIAL_URL: &str = "https://api.openai.com/v1";
const FIELD_ROWS: u16 = 4;

fn form_hints(action: Keybinding, dismiss: Keybinding) -> KeyHints {
    KeyHints::new().with_binding(action)
        .with_binding(bindings::PROVIDER_NEXT_FIELD)
        .with_binding(bindings::PROVIDER_SWITCH_TABS)
        .with_binding(dismiss)
}
static EDIT_HINTS: LazyLock<KeyHints> = LazyLock::new(|| form_hints(bindings::PROVIDER_CONFIRM, bindings::CANCEL));
static SELECT_HINTS: LazyLock<KeyHints> = LazyLock::new(|| form_hints(bindings::PROVIDER_EDIT, bindings::PROVIDER_RETURN_TABS));
static CREATE_HINTS: LazyLock<KeyHints> = LazyLock::new(|| form_hints(bindings::PROVIDER_CREATE, bindings::PROVIDER_RETURN_TABS));
static FETCH_HINTS: LazyLock<KeyHints> = LazyLock::new(|| form_hints(bindings::PROVIDER_FETCH_MODELS, bindings::PROVIDER_RETURN_TABS));
static TAB_HINTS: LazyLock<KeyHints> = LazyLock::new(|| KeyHints::new()
    .with_binding(bindings::TABS)
    .with_binding(bindings::PROVIDER_ENTER_TAB)
    .with_binding(bindings::PROVIDER_RETURN));

enum InputVisibility {
    Visible,
    Hidden,
}
enum FormKind {
    Draft,
    Saved { key_saved: bool },
}
enum Direction {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldFocusOutcome {
    Moved,
    BeforeFirst,
    AfterLast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanelFocus {
    Tabs,
    Content,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Settings {
    pub(crate) revision: u64,
    configs: BTreeMap<String, ProviderConfigDto>,
    keys: Vec<String>,
}

impl Settings {
    pub(crate) fn new(config: &ConfigReadResult, providers: &ProviderListResult) -> Self {
        Self {
            revision: config.revision,
            configs: config.providers.clone(),
            keys: providers
                .providers
                .iter()
                .filter(|provider| provider.api_key_configured)
                .map(|provider| provider.provider.clone())
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Save,
    FetchModels,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Request {
    pub(crate) id: zeta_protocol::CommandId,
    pub(crate) revision: u64,
    pub(crate) config: ProviderConfigDto,
    pub(crate) key: Option<ProviderApiKeyEdit>,
    pub(crate) operation: Operation,
}

pub(crate) struct Reply {
    pub(crate) id: zeta_protocol::CommandId,
    pub(crate) result: Result<(ConfigChoices, Option<Result<Vec<String>, String>>), String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Tab {
    id: String,
    label: String,
}
impl TabListItem for Tab {
    fn tab_label(&self) -> &str {
        &self.label
    }
}

#[derive(Debug)]
struct Form {
    saved: ProviderConfigDto,
    revision: u64,
    name: SearchBoxState,
    url: SearchBoxState,
    key: SearchBoxState,
    protocol: CustomProviderProtocolDto,
    draft: bool,
    key_saved: bool,
    focus: usize,
    editing: bool,
    message: String,
    models: Vec<String>,
    confirmed_key: zeroize::Zeroizing<String>,
}

fn input(value: &str, placeholder: &str, visibility: InputVisibility) -> SearchBoxState {
    let model = SearchBoxModel::new(placeholder).initially_active();
    let mut state = SearchBoxState::new(match visibility {
        InputVisibility::Hidden => model.masked(),
        InputVisibility::Visible => model,
    });
    state.handle_paste(value.into());
    state.set_input_active(false);
    state
}

fn empty_config(id: String) -> ProviderConfigDto {
    ProviderConfigDto {
        provider: id,
        custom: None,
        base_url: None,
        max_output_tokens: None,
        model_context: BTreeMap::new(),
    }
}

fn new_provider_id() -> String {
    // Timestamp first keeps persisted tab order stable across process restarts.
    let command = new_command_id("custom").to_string();
    let parts = command.split('-').collect::<Vec<_>>();
    format!("custom-{}-{}", parts[2], parts[1])
}

impl Form {
    fn new(saved: ProviderConfigDto, kind: FormKind) -> Self {
        let (draft, key_saved) = match kind {
            FormKind::Draft => (true, false),
            FormKind::Saved { key_saved } => (false, key_saved),
        };
        let custom = saved.custom.as_ref();
        let official = saved.provider == "openai";
        let mut form = Self {
            name: input(
                custom.map_or("", |custom| custom.name.as_str()),
                "Provider name",
                InputVisibility::Visible,
            ),
            url: input(
                if official {
                    OFFICIAL_URL
                } else {
                    saved.base_url.as_deref().unwrap_or_default()
                },
                "https://your-service.example/v1",
                InputVisibility::Visible,
            ),
            key: input(
                "",
                if key_saved {
                    "Key saved · Enter a new key to replace"
                } else {
                    "API key"
                },
                InputVisibility::Hidden,
            ),
            protocol: custom.map_or(CustomProviderProtocolDto::Responses, |custom| {
                custom.protocol
            }),
            saved,
            revision: 0,
            draft,
            key_saved,
            focus: if official { 2 } else { 0 },
            editing: true,
            message: String::new(),
            models: Vec::new(),
            confirmed_key: zeroize::Zeroizing::new(String::new()),
        };
        form.focus(form.focus);
        form
    }

    fn official(&self) -> bool {
        self.saved.provider == "openai"
    }
    fn fields(&self) -> Vec<usize> {
        if self.official() {
            vec![1, 2, 4]
        } else {
            vec![0, 1, 2, 3, 4]
        }
    }
    fn editable(&self) -> Vec<usize> {
        if self.official() {
            vec![2, 4]
        } else {
            vec![0, 1, 2, 3, 4]
        }
    }
    fn focus(&mut self, index: usize) {
        self.focus = index;
        self.editing = index < 4;
        self.name.set_input_active(self.editing && index == 0);
        self.url
            .set_input_active(self.editing && index == 1 && !self.official());
        self.key.set_input_active(self.editing && index == 2);
    }
    fn next(&mut self) {
        let fields = self.editable();
        let position = fields
            .iter()
            .position(|index| *index == self.focus)
            .unwrap_or_default();
        self.focus(fields[(position + 1).min(fields.len() - 1)]);
    }
    fn move_focus(&mut self, direction: Direction) -> FieldFocusOutcome {
        let fields = self.editable();
        let position = fields
            .iter()
            .position(|index| *index == self.focus)
            .unwrap_or_default();
        let next = match direction {
            Direction::Previous => match position.checked_sub(1) {
                Some(next) => next,
                None => return FieldFocusOutcome::BeforeFirst,
            },
            Direction::Next if position + 1 < fields.len() => position + 1,
            Direction::Next => return FieldFocusOutcome::AfterLast,
        };
        self.focus(fields[next]);
        FieldFocusOutcome::Moved
    }
    fn field_mut(&mut self) -> Option<&mut SearchBoxState> {
        match self.focus {
            0 => Some(&mut self.name),
            1 if !self.official() => Some(&mut self.url),
            2 => Some(&mut self.key),
            _ => None,
        }
    }
    fn config(&self) -> Result<ProviderConfigDto, String> {
        let mut config = self.saved.clone();
        if self.official() {
            config.base_url = None;
            return Ok(config);
        }
        let name = if self.draft || self.focus == 0 {
            self.name.query().trim()
        } else {
            self.saved
                .custom
                .as_ref()
                .map_or("", |custom| custom.name.as_str())
        };
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err("Provider name must contain 1 to 80 characters".into());
        }
        let url = if self.draft || self.focus == 1 {
            self.url.query().trim().trim_end_matches('/')
        } else {
            self.saved.base_url.as_deref().unwrap_or_default()
        };
        let parsed = url::Url::parse(url).map_err(|_| "Enter a valid HTTP or HTTPS base URL")?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err("Use an HTTP or HTTPS URL without credentials, query or fragment".into());
        }
        config.base_url = Some(url.into());
        let protocol = if self.draft || self.focus == 3 {
            self.protocol
        } else {
            self.saved
                .custom
                .as_ref()
                .map_or(CustomProviderProtocolDto::Responses, |custom| {
                    custom.protocol
                })
        };
        config.custom = Some(CustomProviderConfigDto {
            name: name.into(),
            protocol,
        });
        Ok(config)
    }
    fn cancel_edit(&mut self) {
        match self.focus {
            0 => {
                self.name = input(
                    self.saved
                        .custom
                        .as_ref()
                        .map_or("", |custom| custom.name.as_str()),
                    "Provider name",
                    InputVisibility::Visible,
                )
            }
            1 => {
                self.url = input(
                    self.saved.base_url.as_deref().unwrap_or_default(),
                    "https://your-service.example/v1",
                    InputVisibility::Visible,
                )
            }
            2 => {
                self.key = input(
                    &self.confirmed_key,
                    if self.key_saved {
                        "Key saved · Enter a new key to replace"
                    } else {
                        "API key"
                    },
                    InputVisibility::Hidden,
                )
            }
            3 => {
                self.protocol = self
                    .saved
                    .custom
                    .as_ref()
                    .map_or(CustomProviderProtocolDto::Responses, |custom| {
                        custom.protocol
                    })
            }
            _ => {}
        }
        self.editing = false;
        self.name.set_input_active(false);
        self.url.set_input_active(false);
        self.key.set_input_active(false);
    }
}

#[derive(Debug)]
pub(crate) struct Panel {
    settings: Settings,
    tabs: TabListState<Tab>,
    forms: BTreeMap<String, Form>,
    draft_id: String,
    focus: PanelFocus,
    subscription: ListSelection<ConfigSelectionAction>,
    pending: Option<Request>,
}

impl Panel {
    pub(crate) fn key_hints(&self) -> &str {
        if self.focus == PanelFocus::Tabs {
            return TAB_HINTS.text();
        }
        let Some(form) = self.form() else {
            return self.subscription.key_hints();
        };
        if form.focus == 4 {
            return if form.draft {
                CREATE_HINTS.text()
            } else {
                FETCH_HINTS.text()
            };
        }
        if form.editing {
            EDIT_HINTS.text()
        } else {
            SELECT_HINTS.text()
        }
    }
    pub(crate) fn new(settings: Settings) -> Self {
        let choices = super::Subscription::default().choices();
        let draft_id = new_provider_id();
        let mut panel = Self {
            settings: settings.clone(),
            tabs: TabListState::new(vec![Tab {
                id: "openai".into(),
                label: "Official API key".into(),
            }]),
            forms: BTreeMap::new(),
            draft_id,
            focus: PanelFocus::Content,
            subscription: ListSelection::new(choices.model.without_tab_bar(), choices.actions),
            pending: None,
        };
        panel.replace(settings);
        panel
    }
    pub(crate) fn replace(&mut self, settings: Settings) {
        self.settings = settings;
        let mut configs = self.settings.configs.clone();
        configs
            .entry("openai".into())
            .or_insert_with(|| empty_config("openai".into()));
        let mut tabs = vec![
            Tab {
                id: "openai".into(),
                label: "Official API key".into(),
            },
            Tab {
                id: "chatgpt".into(),
                label: "ChatGPT subscription".into(),
            },
        ];
        let mut configs = configs.into_iter().collect::<Vec<_>>();
        configs.sort_by_key(|(id, _)| (id != "openai-compatible", id.clone()));
        for (id, mut config) in configs {
            if id != "openai" && config.custom.is_none() && id != "openai-compatible" {
                continue;
            }
            if id == "openai-compatible" && config.custom.is_none() {
                if config.base_url.as_ref().is_none_or(|url| url.is_empty()) {
                    continue;
                }
                config.custom = Some(CustomProviderConfigDto {
                    name: "Custom service".into(),
                    protocol: CustomProviderProtocolDto::ChatCompletions,
                });
            }
            if let Some(custom) = &config.custom {
                tabs.push(Tab {
                    id: id.clone(),
                    label: custom.name.clone(),
                });
            }
            let form = self.forms.entry(id.clone()).or_insert_with(|| {
                Form::new(
                    config.clone(),
                    FormKind::Saved {
                        key_saved: self.settings.keys.contains(&id),
                    },
                )
            });
            if form.saved == config {
                form.revision = self.settings.revision;
            } else {
                form.message =
                    "Configuration changed elsewhere · Reopen this panel to reload".into();
            }
        }
        self.forms
            .entry(self.draft_id.clone())
            .or_insert_with(|| Form::new(empty_config(self.draft_id.clone()), FormKind::Draft));
        self.forms
            .get_mut(&self.draft_id)
            .expect("draft exists")
            .revision = self.settings.revision;
        tabs.push(Tab {
            id: self.draft_id.clone(),
            label: "New custom provider".into(),
        });
        let active = self.tabs.active_tab().id.clone();
        self.tabs.replace_tabs(tabs);
        if let Some(index) = self.tabs.tabs().iter().position(|tab| tab.id == active) {
            self.tabs.select(index);
        }
    }
    pub(crate) fn update_subscription(&mut self, choices: ConfigChoices) {
        self.subscription
            .replace(choices.model.without_tab_bar(), choices.actions);
    }
    fn form(&self) -> Option<&Form> {
        self.forms.get(&self.tabs.active_tab().id)
    }
    fn form_mut(&mut self) -> Option<&mut Form> {
        self.forms.get_mut(&self.tabs.active_tab().id)
    }
    pub(crate) fn select_tab(&mut self, index: usize) -> ConfigEditorOutcome {
        self.tabs.select(index);
        if self.tabs.active_index() == 1 {
            ConfigEditorOutcome::Action(ConfigSelectionAction::OpenSubscription)
        } else {
            ConfigEditorOutcome::Consumed
        }
    }
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ConfigEditorOutcome {
        if key.kind != KeyEventKind::Press {
            return ConfigEditorOutcome::Consumed;
        }
        let mut key = key;
        if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT) {
            key.code = KeyCode::BackTab;
        }
        if key.modifiers.contains(KeyModifiers::ALT)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            self.tabs
                .handle_key(KeyEvent::new(key.code, KeyModifiers::NONE));
            return self.select_tab(self.tabs.active_index());
        }
        if self.focus == PanelFocus::Tabs {
            match self.tabs.handle_focused_key(key) {
                FocusedTabListInputOutcome::ActiveChanged => {
                    return self.select_tab(self.tabs.active_index());
                }
                FocusedTabListInputOutcome::EnterContent
                | FocusedTabListInputOutcome::FocusNext => self.focus = PanelFocus::Content,
                FocusedTabListInputOutcome::Consumed => return ConfigEditorOutcome::Consumed,
                FocusedTabListInputOutcome::Unhandled if key.code == KeyCode::Esc => {
                    return ConfigEditorOutcome::Dismiss;
                }
                FocusedTabListInputOutcome::Unhandled => {}
            }
            return ConfigEditorOutcome::Consumed;
        }
        if self.tabs.active_index() == 1 {
            if key.code == KeyCode::Esc || key.code == KeyCode::BackTab {
                self.focus = PanelFocus::Tabs;
                return ConfigEditorOutcome::Consumed;
            }
            return match self.subscription.handle_key(key) {
                ListSelectionOutcome::Activate(action) => ConfigEditorOutcome::Action(action),
                ListSelectionOutcome::FocusPrevious => {
                    self.focus = PanelFocus::Tabs;
                    ConfigEditorOutcome::Consumed
                }
                _ => ConfigEditorOutcome::Consumed,
            };
        }
        if self.pending.is_some() {
            if key.code == KeyCode::Esc {
                self.focus = PanelFocus::Tabs;
            }
            return ConfigEditorOutcome::Consumed;
        }
        let form = self.form_mut().expect("API tab owns a form");
        match key.code {
            KeyCode::Esc if form.editing => form.cancel_edit(),
            KeyCode::Esc => self.focus = PanelFocus::Tabs,
            KeyCode::BackTab => {
                if form.move_focus(Direction::Previous) == FieldFocusOutcome::BeforeFirst {
                    self.focus = PanelFocus::Tabs;
                }
            }
            KeyCode::Tab => {
                if form.move_focus(Direction::Next) == FieldFocusOutcome::AfterLast {
                    self.focus = PanelFocus::Tabs;
                }
            }
            KeyCode::Up if !form.editing => {
                if form.move_focus(Direction::Previous) == FieldFocusOutcome::BeforeFirst {
                    self.focus = PanelFocus::Tabs;
                }
            }
            KeyCode::Down if !form.editing => {
                form.move_focus(Direction::Next);
            }
            KeyCode::Left | KeyCode::Right if form.focus == 3 => {
                form.protocol = match form.protocol {
                    CustomProviderProtocolDto::Responses => {
                        CustomProviderProtocolDto::ChatCompletions
                    }
                    CustomProviderProtocolDto::ChatCompletions => {
                        CustomProviderProtocolDto::Responses
                    }
                };
            }
            KeyCode::Enter => return self.confirm(),
            _ if form.editing => {
                if let Some(field) = form.field_mut() {
                    field.handle_key(key);
                }
            }
            _ => {}
        }
        ConfigEditorOutcome::Consumed
    }
    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if self.pending.is_none() && self.focus == PanelFocus::Content {
            if let Some(form) = self.form_mut() {
                if form.editing {
                    if let Some(field) = form.field_mut() {
                        field.handle_paste(pasted);
                    }
                }
            }
        }
    }
    fn confirm(&mut self) -> ConfigEditorOutcome {
        let form = self.form_mut().expect("API tab owns a form");
        let revision = form.revision;
        if form.focus < 4 && !form.editing {
            form.focus(form.focus);
            return ConfigEditorOutcome::Consumed;
        }
        if form.draft && form.focus < 4 {
            // Enter confirms one draft field; creation is a separate explicit action.
            match form.focus {
                0 => {
                    if form.name.query().trim().is_empty() {
                        form.message = "Provider name is required".into();
                        return ConfigEditorOutcome::Consumed;
                    }
                    form.saved.custom = Some(CustomProviderConfigDto {
                        name: form.name.query().trim().into(),
                        protocol: form.protocol,
                    });
                }
                1 => {
                    if let Err(message) = form.config() {
                        form.message = message;
                        return ConfigEditorOutcome::Consumed;
                    }
                    form.saved.base_url =
                        Some(form.url.query().trim().trim_end_matches('/').into());
                }
                2 => *form.confirmed_key = form.key.query().into(),
                3 => {
                    if let Some(custom) = &mut form.saved.custom {
                        custom.protocol = form.protocol;
                    }
                }
                _ => {}
            }
            form.message.clear();
            form.next();
            return ConfigEditorOutcome::Consumed;
        }
        let config = match form.config() {
            Ok(config) => config,
            Err(message) => {
                form.message = message;
                return ConfigEditorOutcome::Consumed;
            }
        };
        let key = ((form.draft || form.focus == 2) && !form.key.query().is_empty())
            .then(|| ProviderApiKeyEdit::new(config.provider.clone(), form.key.query().into()));
        if !form.draft && form.focus < 4 && config == form.saved && key.is_none() {
            form.next();
            return ConfigEditorOutcome::Consumed;
        }
        let operation = if form.focus == 4 && !form.draft {
            Operation::FetchModels
        } else {
            Operation::Save
        };
        form.message = if operation == Operation::FetchModels {
            "Fetching models…"
        } else {
            "Saving…"
        }
        .into();
        let request = Request {
            id: new_command_id("connection"),
            revision,
            config,
            key,
            operation,
        };
        self.pending = Some(request.clone());
        ConfigEditorOutcome::Action(ConfigSelectionAction::Connection(request))
    }
    pub(crate) fn complete(&mut self, reply: Reply) {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.id != reply.id)
        {
            return;
        }
        let request = self.pending.take().expect("matched pending request");
        let id = request.config.provider.clone();
        match reply.result {
            Err(message) => {
                if let Some(form) = self.forms.get_mut(&id) {
                    form.message = message;
                    form.focus(form.focus);
                }
            }
            Ok((choices, models)) => {
                let was_draft = self.forms.get(&id).is_some_and(|form| form.draft);
                if let Some(form) = self.forms.get_mut(&id) {
                    let changed = form.saved != request.config || request.key.is_some();
                    form.saved = request.config;
                    form.draft = false;
                    if request.key.is_some() {
                        form.key_saved = true;
                        form.key = input(
                            "",
                            "Key saved · Enter a new key to replace",
                            InputVisibility::Hidden,
                        );
                    }
                    if changed {
                        form.models.clear();
                    }
                    form.message = match models {
                        Some(Ok(models)) => {
                            let message = format!(
                                "{} models fetched · Invocation not verified",
                                models.len()
                            );
                            form.models = models;
                            message
                        }
                        Some(Err(message)) => message,
                        None => "Saved · Connection not verified".into(),
                    };
                    if was_draft {
                        form.confirmed_key.clear();
                    }
                    if request.operation == Operation::Save {
                        form.next();
                    }
                }
                if was_draft {
                    self.draft_id = new_provider_id();
                }
                if let Some(ConfigSelectionAction::OpenOpenAi(settings)) = choices
                    .actions
                    .values()
                    .find(|action| matches!(action, ConfigSelectionAction::OpenOpenAi(_)))
                {
                    self.replace(settings.clone());
                }
            }
        }
    }
    pub(crate) fn tab_rows(&self, width: u16) -> u16 {
        tab_list::desired_height(self.tabs.tabs(), width)
    }
    pub(crate) fn body_rows(&self) -> u16 {
        self.form().map_or_else(
            || self.subscription.state().body_rows(),
            |form| {
                (form.fields().len() as u16 - 1) * FIELD_ROWS + 4 + form.models.len().min(5) as u16
            },
        )
    }
    pub(crate) fn draw_tabs(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        hovered: Option<usize>,
        pressed: Option<usize>,
        context: RenderContext<'_>,
    ) {
        tab_list::draw(
            frame,
            area,
            &self.tabs,
            self.focus == PanelFocus::Tabs,
            hovered,
            pressed,
            context,
        );
    }
    fn visible_fields(&self, height: u16) -> Vec<(usize, u16, u16)> {
        let Some(form) = self.form() else {
            return Vec::new();
        };
        let fields = form.fields();
        let focus = fields
            .iter()
            .position(|index| *index == form.focus)
            .unwrap_or_default();
        let capacity = usize::from(height.saturating_sub(2) / FIELD_ROWS).max(1);
        let total = (fields.len().saturating_sub(1) as u16) * FIELD_ROWS + 2;
        let start = if total <= height {
            0
        } else {
            focus.saturating_add(1).saturating_sub(capacity)
        };
        let mut y: u16 = 0;
        fields
            .into_iter()
            .skip(start)
            .filter_map(|index| {
                let h = if index == 4 { 1 } else { FIELD_ROWS };
                let result = (y + h <= height.saturating_sub(1)).then_some((index, y, h));
                y += h;
                result
            })
            .collect()
    }
    pub(crate) fn draw_body(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let Some(form) = self.form() else {
            list_selection::draw_body_with_pointer(
                frame,
                area,
                self.subscription.state(),
                false,
                false,
                None,
                None,
                context,
            );
            return;
        };
        let mut bottom = area.y;
        for (index, y, h) in self.visible_fields(area.height) {
            let label = match index {
                0 => "Provider name",
                1 if form.official() => "Base URL (read-only)",
                1 => "Base URL",
                2 => "API key",
                3 => "API protocol",
                _ if form.draft => "Create provider",
                _ => "Fetch model list",
            };
            let focused = self.focus == PanelFocus::Content && form.focus == index;
            let style = Style::default().fg(if focused {
                context.focus()
            } else {
                context.foreground()
            });
            frame.render_widget(
                Paragraph::new(label).style(style),
                Rect::new(area.x, area.y + y, area.width, 1),
            );
            let field_area = Rect::new(area.x, area.y + y + 1, area.width, h.saturating_sub(1));
            match index {
                0 | 1 | 2 => {
                    let mut input = match index {
                        0 => form.name.clone(),
                        1 => form.url.clone(),
                        _ => form.key.clone(),
                    };
                    input.set_input_active(
                        focused && form.editing && !(index == 1 && form.official()),
                    );
                    search_box::draw(frame, field_area, &input, focused, false, context);
                }
                3 => {
                    let value = match form.protocol {
                        CustomProviderProtocolDto::Responses => "Responses   ← / → to change",
                        CustomProviderProtocolDto::ChatCompletions => {
                            "Chat Completions   ← / → to change"
                        }
                    };
                    let input = input(value, "", InputVisibility::Visible);
                    search_box::draw(frame, field_area, &input, focused, false, context);
                }
                _ => {}
            }
            bottom = area.y + y + h;
        }
        if bottom < area.bottom() {
            frame.render_widget(
                Paragraph::new(form.message.as_str()).style(Style::default().fg(context.muted())),
                Rect::new(area.x, bottom, area.width, 1),
            );
            bottom += 1;
        }
        for model in &form.models {
            if bottom >= area.bottom() {
                break;
            }
            frame.render_widget(
                Paragraph::new(model.as_str()),
                Rect::new(area.x, bottom, area.width, 1),
            );
            bottom += 1;
        }
    }
}

#[cfg(test)]
#[path = "openai_tests.rs"]
mod tests;
