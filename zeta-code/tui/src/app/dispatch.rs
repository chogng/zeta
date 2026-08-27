//! Built-in product command dispatch for the active Session and Thread.

use crate::TuiWorkspaceReconnect;
use crate::app::AppEvent;
use crate::app::help_selection_view;
use crate::components::composer::ComposerInput;
use crate::components::composer::SlashCommandInvocation;
use crate::components::composer::TuiSlashCommandAction;
use crate::features::config;
use crate::features::mcp;
use crate::features::models;
use crate::features::rewind;
use crate::features::sessions;
use crate::features::sessions::ActiveConversation;
use crate::features::sessions::ConversationChange;
use crate::features::sessions::NewConversationKind;
use crate::features::sessions::ResumeOutcome;
use crate::features::sessions::ThreadSelectionPurpose;
use crate::features::skills::load_selection;
use crate::features::status::status_view;
use crate::features::theme::theme_selection_view;
use crate::ui;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_protocol::TurnId;

#[cfg(test)]
use crate::app::App;
#[cfg(test)]
use crate::features::sessions::ConversationTranscript;
#[cfg(test)]
use crate::features::thread::read_thread;
#[cfg(test)]
use zeta_protocol::Thread;

pub(crate) struct ProductCommandOutput {
    pub(crate) conversation: ActiveConversation,
    pub(crate) events: Vec<AppEvent>,
    pub(crate) conversation_change: Option<ConversationChange>,
    pub(crate) workspace_reconnect: Option<TuiWorkspaceReconnect>,
}

pub(crate) fn execute_product_command<T>(
    mut conversation: ActiveConversation,
    client: &mut AppServerClient<T>,
    invocation: SlashCommandInvocation,
) -> Result<ProductCommandOutput, String>
where
    T: JsonRpcTransport,
{
    conversation
        .try_execute(client, invocation)
        .map(|output| ProductCommandOutput {
            conversation,
            events: output.events,
            conversation_change: output.conversation_change,
            workspace_reconnect: output.workspace_reconnect,
        })
        .map_err(|error| error.to_string())
}

impl ActiveConversation {
    #[cfg(test)]
    pub(crate) fn execute<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        invocation: SlashCommandInvocation,
        app: &mut App,
    ) where
        T: JsonRpcTransport,
    {
        match execute_product_command(self.clone(), client, invocation) {
            Ok(output) => {
                *self = output.conversation;
                for event in output.events {
                    app.update(event);
                }
                if let Some(change) = output.conversation_change {
                    match read_thread(client, self.session_id(), self.thread_id()) {
                        Ok(snapshot) => apply_conversation_change(app, change, snapshot),
                        Err(error) => app.update(AppEvent::FailureReported(error.to_string())),
                    }
                }
            }
            Err(error) => app.update(AppEvent::FailureReported(error)),
        }
    }

    fn try_execute<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        invocation: SlashCommandInvocation,
    ) -> Result<CommandOutput, CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        let command = invocation
            .command
            .name
            .parse::<TuiSlashCommandAction>()
            .map_err(|_| {
                CommandExecutionError("server command reached the TUI-local dispatcher".into())
            })?;
        let arguments = text_arguments(&invocation.arguments)?;
        let mut output = CommandOutput::default();

        match command {
            TuiSlashCommandAction::Status => {
                let config = client.read_config()?;
                output
                    .events
                    .push(AppEvent::SelectionViewOpened(status_view(
                        self.session_id().as_str(),
                        self.thread_id().as_str(),
                        self.thread_sequence(),
                        &config::preferred_model(config.preferred_model.as_ref()),
                    )));
            }
            TuiSlashCommandAction::Skills => {
                output
                    .events
                    .push(AppEvent::SkillsViewOpened(load_selection(
                        client,
                        SkillCatalogReloadDto::Refresh,
                    )?));
            }
            TuiSlashCommandAction::Mcp => {
                output
                    .events
                    .push(AppEvent::McpViewOpened(mcp::load_selection(client)?));
            }
            TuiSlashCommandAction::Connectors => {
                output.events.push(AppEvent::ConnectorViewOpened(
                    crate::features::connectors::load_selection(client)?,
                ));
            }
            TuiSlashCommandAction::Resume => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::SessionViewOpened(sessions::load_selection(
                            client,
                            self.session_id().as_str(),
                        )?));
                } else {
                    match self
                        .resume_session(client, &arguments)
                        .map_err(session_error)?
                    {
                        ResumeOutcome::Listed(notice) => {
                            output.events.push(AppEvent::ProductNotice(notice));
                        }
                        ResumeOutcome::Changed(change) => {
                            output.conversation_change = Some(change);
                        }
                        ResumeOutcome::WorkspaceReconnect(reconnect) => {
                            output.workspace_reconnect = Some(reconnect);
                        }
                    }
                }
            }
            TuiSlashCommandAction::Thread => {
                if arguments.is_empty() {
                    output.events.push(AppEvent::ThreadViewOpened(
                        sessions::load_thread_selection(
                            client,
                            self.session_id(),
                            self.thread_id(),
                            ThreadSelectionPurpose::Switch,
                        )?,
                    ));
                } else {
                    let thread_id = zeta_protocol::ThreadId::new(&arguments).map_err(|error| {
                        CommandExecutionError(format!("invalid thread ID '{arguments}': {error}"))
                    })?;
                    output.conversation_change = Some(
                        self.switch_thread(client, thread_id)
                            .map_err(session_error)?,
                    );
                }
            }
            TuiSlashCommandAction::ArchiveThread => {
                if arguments.is_empty() {
                    output.events.push(AppEvent::ThreadViewOpened(
                        sessions::load_thread_selection(
                            client,
                            self.session_id(),
                            self.thread_id(),
                            ThreadSelectionPurpose::Archive,
                        )?,
                    ));
                } else {
                    let thread_id = zeta_protocol::ThreadId::new(&arguments).map_err(|error| {
                        CommandExecutionError(format!("invalid thread ID '{arguments}': {error}"))
                    })?;
                    output.conversation_change = Some(
                        self.archive_thread(client, thread_id)
                            .map_err(session_error)?,
                    );
                }
            }
            TuiSlashCommandAction::ArchiveSession => {
                output.conversation_change = Some(
                    self.archive_session_and_replace(client)
                        .map_err(session_error)?,
                );
            }
            TuiSlashCommandAction::Rewind => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::RewindViewOpened(rewind::load_selection(
                            client,
                            self.session_id(),
                            self.thread_id(),
                        )?));
                } else {
                    let before_turn_id = TurnId::new(&arguments).map_err(|error| {
                        CommandExecutionError(format!(
                            "invalid rewind checkpoint '{arguments}': {error}"
                        ))
                    })?;
                    output.conversation_change = Some(
                        self.rewind_active_thread(client, before_turn_id, &arguments)
                            .map_err(session_error)?,
                    );
                }
            }
            TuiSlashCommandAction::Clear | TuiSlashCommandAction::New => {
                let kind = match command {
                    TuiSlashCommandAction::Clear => NewConversationKind::Clear,
                    TuiSlashCommandAction::New => NewConversationKind::New,
                    _ => unreachable!("only new-chat commands reach this branch"),
                };
                output.conversation_change = Some(
                    self.replace_with_new(client, kind, &arguments)
                        .map_err(session_error)?,
                );
            }
            TuiSlashCommandAction::Config => {
                let config = client.read_config()?;
                output
                    .events
                    .push(AppEvent::SelectionViewOpened(config::config_view(&config)));
            }
            TuiSlashCommandAction::Files => {
                output.events.push(AppEvent::FileViewOpened(
                    crate::features::workspace_files::load_directory(
                        client,
                        std::path::PathBuf::from(arguments),
                    )?,
                ));
            }
            TuiSlashCommandAction::Fork => {
                output.conversation_change = Some(
                    self.fork_active_thread(client, &arguments)
                        .map_err(session_error)?,
                );
            }
            TuiSlashCommandAction::Help => {
                output
                    .events
                    .push(AppEvent::SelectionViewOpened(help_selection_view()));
            }
            TuiSlashCommandAction::Copy
            | TuiSlashCommandAction::Export
            | TuiSlashCommandAction::Keymap => {
                return Err(CommandExecutionError(
                    "host command reached the App Server dispatcher".into(),
                ));
            }
            TuiSlashCommandAction::Model => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::ModelViewOpened(models::load_selection(client)?));
                } else {
                    let update = config::set_preferred_model(client, &arguments)
                        .map_err(|error| CommandExecutionError(error.to_string()))?;
                    output
                        .events
                        .push(AppEvent::PreferredModelReceived(update.preferred_model));
                    output.events.push(AppEvent::ProductNotice(update.notice));
                }
            }
            TuiSlashCommandAction::Theme => {
                if arguments.is_empty() {
                    let catalog = ui::theme_catalog().map_err(CommandExecutionError)?;
                    output
                        .events
                        .push(AppEvent::ThemeViewOpened(theme_selection_view(&catalog)));
                } else {
                    let command = format!("/theme {arguments}");
                    let label = ui::select_theme(&arguments).map_err(CommandExecutionError)?;
                    output
                        .events
                        .push(AppEvent::CommandStarted(command.clone()));
                    output.events.push(AppEvent::CommandCompleted {
                        command,
                        result: format!("Theme set to {label}"),
                    });
                }
            }
            TuiSlashCommandAction::Quit | TuiSlashCommandAction::Exit => {
                return Err(CommandExecutionError(
                    "exit command reached the product dispatcher".into(),
                ));
            }
        }
        Ok(output)
    }
}

#[derive(Default)]
struct CommandOutput {
    events: Vec<AppEvent>,
    conversation_change: Option<ConversationChange>,
    workspace_reconnect: Option<TuiWorkspaceReconnect>,
}

#[cfg(test)]
fn apply_conversation_change(app: &mut App, change: ConversationChange, snapshot: Thread) {
    if matches!(change.transcript, ConversationTranscript::Clear) {
        app.update(AppEvent::TranscriptCleared);
    }
    app.update(AppEvent::ThreadTranscriptSnapshotReceived(
        zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot::from_thread(
            &snapshot,
        ),
    ));
    app.update(AppEvent::ProductNotice(change.notice));
}

fn text_arguments(arguments: &[ComposerInput]) -> Result<String, CommandExecutionError> {
    if arguments.iter().any(|argument| {
        matches!(
            argument,
            ComposerInput::Image { .. } | ComposerInput::Skill { .. }
        )
    }) {
        return Err(CommandExecutionError(
            "product commands do not accept image arguments or Skill selections".into(),
        ));
    }
    Ok(arguments
        .iter()
        .filter_map(|argument| match argument {
            ComposerInput::Text(text) => Some(text.as_str()),
            ComposerInput::Image { .. } | ComposerInput::Skill { .. } => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned())
}

fn session_error(error: impl fmt::Display) -> CommandExecutionError {
    CommandExecutionError(error.to_string())
}

#[derive(Debug)]
struct CommandExecutionError(String);

impl fmt::Display for CommandExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<ClientError> for CommandExecutionError {
    fn from(error: ClientError) -> Self {
        Self(error.to_string())
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
