//! Built-in product command dispatch for the active Session and Thread.

use crate::app::App;
use crate::app::AppEvent;
use crate::app::help_selection_view;
use crate::components::composer::ComposerInput;
use crate::components::composer::SlashCommandInvocation;
use crate::components::composer::TuiSlashCommandAction;
use crate::features::config;
use crate::features::config::PreferredModelOutcome;
use crate::features::sessions::ActiveConversation;
use crate::features::sessions::ConversationChange;
use crate::features::sessions::ConversationTranscript;
use crate::features::sessions::NewConversationKind;
use crate::features::sessions::ResumeOutcome;
use crate::features::skills::load_selection;
use std::fmt;
use zeta_app_server_client::{AppServerClient, ClientError, JsonRpcTransport};
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;

impl ActiveConversation {
    pub(crate) fn execute<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        invocation: SlashCommandInvocation,
        app: &mut App,
    ) where
        T: JsonRpcTransport,
    {
        if let Err(error) = self.try_execute(client, invocation, app) {
            app.update(AppEvent::FailureReported(error.to_string()));
        }
    }

    fn try_execute<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        invocation: SlashCommandInvocation,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
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

        match command {
            TuiSlashCommandAction::Status => self.show_status(client, app),
            TuiSlashCommandAction::Skills => show_skills(client, app),
            TuiSlashCommandAction::Mcp => show_mcp(client, app),
            TuiSlashCommandAction::Resume => self.resume(client, &arguments, app),
            TuiSlashCommandAction::Clear | TuiSlashCommandAction::New => {
                self.start_new(client, command, &arguments, app)
            }
            TuiSlashCommandAction::Config => show_config(client, app),
            TuiSlashCommandAction::Fork => self.fork(client, &arguments, app),
            TuiSlashCommandAction::Help => {
                app.update(AppEvent::SelectionViewOpened(help_selection_view()));
                Ok(())
            }
            TuiSlashCommandAction::Model => set_or_show_model(client, &arguments, app),
            TuiSlashCommandAction::Quit | TuiSlashCommandAction::Exit => Err(
                CommandExecutionError("exit command reached the product dispatcher".into()),
            ),
        }
    }

    fn show_status<T>(
        &self,
        client: &mut AppServerClient<T>,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        let config = client.read_config()?;
        app.update(AppEvent::ProductNotice(format!(
            "Session: {}\nThread: {}\nThread sequence: {}\nModel: {}",
            self.session_id(),
            self.thread_id(),
            self.thread_sequence(),
            config::preferred_model(&config)
        )));
        Ok(())
    }

    fn start_new<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        command: TuiSlashCommandAction,
        arguments: &str,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        let kind = match command {
            TuiSlashCommandAction::Clear => NewConversationKind::Clear,
            TuiSlashCommandAction::New => NewConversationKind::New,
            _ => unreachable!("only new-chat commands call start_new"),
        };
        let change = self
            .replace_with_new(client, kind, arguments)
            .map_err(|error| CommandExecutionError(error.to_string()))?;
        apply_conversation_change(app, change);
        Ok(())
    }

    fn fork<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        arguments: &str,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        let change = self
            .fork_active_thread(client, arguments)
            .map_err(|error| CommandExecutionError(error.to_string()))?;
        apply_conversation_change(app, change);
        Ok(())
    }

    fn resume<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        arguments: &str,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        match self
            .resume_session(client, arguments)
            .map_err(|error| CommandExecutionError(error.to_string()))?
        {
            ResumeOutcome::Listed(notice) => app.update(AppEvent::ProductNotice(notice)),
            ResumeOutcome::Changed(change) => apply_conversation_change(app, change),
        }
        Ok(())
    }
}

fn apply_conversation_change(app: &mut App, change: ConversationChange) {
    if matches!(change.transcript, ConversationTranscript::Clear) {
        app.update(AppEvent::TranscriptCleared);
    }
    app.update(AppEvent::ThreadSnapshotReceived(change.snapshot));
    app.update(AppEvent::ProductNotice(change.notice));
}

fn show_config<T>(
    client: &mut AppServerClient<T>,
    app: &mut App,
) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    app.update(AppEvent::ProductNotice(config::config_summary(client)?));
    Ok(())
}

fn show_mcp<T>(client: &mut AppServerClient<T>, app: &mut App) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    app.update(AppEvent::ProductNotice(config::mcp_summary(client)?));
    Ok(())
}

fn show_skills<T>(
    client: &mut AppServerClient<T>,
    app: &mut App,
) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    app.update(AppEvent::SkillsViewOpened(load_selection(
        client,
        SkillCatalogReloadDto::Refresh,
    )?));
    Ok(())
}

fn set_or_show_model<T>(
    client: &mut AppServerClient<T>,
    arguments: &str,
    app: &mut App,
) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    match config::set_or_show_preferred_model(client, arguments)
        .map_err(|error| CommandExecutionError(error.to_string()))?
    {
        PreferredModelOutcome::Shown(notice) => app.update(AppEvent::ProductNotice(notice)),
        PreferredModelOutcome::Updated { config, notice } => {
            app.update(AppEvent::ConfigSnapshotReceived(config));
            app.update(AppEvent::ProductNotice(notice));
        }
    }
    Ok(())
}

fn text_arguments(arguments: &[ComposerInput]) -> Result<String, CommandExecutionError> {
    if arguments
        .iter()
        .any(|argument| matches!(argument, ComposerInput::Image { .. }))
    {
        return Err(CommandExecutionError(
            "product commands do not accept image arguments".into(),
        ));
    }
    Ok(arguments
        .iter()
        .filter_map(|argument| match argument {
            ComposerInput::Text(text) => Some(text.as_str()),
            ComposerInput::Image { .. } => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned())
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
