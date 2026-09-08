mod editor;
mod issues;
pub(crate) use issues::IssueConfigEdit;
pub(crate) mod openai;
mod request;
mod settings;
mod subscription;

pub(crate) use subscription::Subscription;
pub(crate) use subscription::SubscriptionCommand;
pub(crate) use subscription::SubscriptionEvent;

pub(crate) use editor::ConfigChoices;
pub(crate) use editor::ConfigEdit;
pub(crate) use editor::ConfigEditor;
pub(crate) use editor::ConfigEditorOutcome;
pub(crate) use editor::ConfigEditorPage;
pub(crate) use editor::ConfigSelectionAction;
pub(crate) use editor::LanguageServerEdit;
pub(crate) use editor::ProviderApiKeyEdit;
pub(crate) use editor::config_choices;
pub(crate) use request::execute;
#[cfg(test)]
pub(crate) use request::set_settings;
pub(crate) use settings::TerminalSettings;

pub(crate) struct ConfigEditResult {
    pub(crate) terminal: TerminalSettings,
    pub(crate) status_line: crate::status::StatusLineSettings,
    pub(crate) choices: ConfigChoices,
}

/// A completed configuration operation delivered to the TUI state owner.
pub(crate) enum Event {
    IssueModels {
        request_id: zeta_protocol::CommandId,
        result: Result<ConfigChoices, String>,
    },
    Connection(openai::Reply),
    Subscription(SubscriptionEvent),
    SettingsReceived(TerminalSettings),
    Updated(ConfigEditResult),
    EditorOpened(ConfigChoices),
    ApiKeySaved {
        provider: String,
        choices: ConfigChoices,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    SetIssues(IssueConfigEdit),
    LoadIssueModels {
        request_id: zeta_protocol::CommandId,
        expected_revision: u64,
    },
    Connection(openai::Request),
    Subscription(SubscriptionCommand),
    OpenEditor,
    Edit(ConfigEdit),
    SetLanguageServerMode(LanguageServerEdit),
    SetProviderApiKey(ProviderApiKeyEdit),
}
