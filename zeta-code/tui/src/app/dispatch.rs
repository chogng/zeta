//! Built-in product command dispatch for the active Session and Thread.

use crate::app::AppEvent;
use crate::dirs;
use crate::mcp;
use crate::models;
use crate::sessions;
use crate::sessions::ActiveConversation;
use crate::sessions::ConversationChange;
use crate::sessions::ResumeOutcome;
use crate::skills::load_selection;
use crate::status;
use crate::thread::composer::ChatInputItem;
use crate::thread::composer::SlashCommandInvocation;
use crate::thread::composer::TuiSlashCommandAction;
use crate::thread::rewind;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto;
use zeta_protocol::TurnId;

pub(crate) struct ProductCommandOutput {
    pub(crate) conversation: ActiveConversation,
    pub(crate) command: String,
    pub(crate) events: Vec<AppEvent>,
    pub(crate) conversation_change: Option<ConversationChange>,
}

pub(crate) fn execute_product_command<T>(
    mut conversation: ActiveConversation,
    client: &mut AppServerClient<T>,
    invocation: SlashCommandInvocation,
) -> Result<ProductCommandOutput, String>
where
    T: JsonRpcTransport,
{
    let command = invocation.display_text();
    dispatch(&mut conversation, client, invocation)
        .map(|output| ProductCommandOutput {
            conversation,
            command,
            events: output.events,
            conversation_change: output.conversation_change,
        })
        .map_err(|error| error.to_string())
}

fn dispatch<T>(
    conversation: &mut ActiveConversation,
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
        TuiSlashCommandAction::Pr
        | TuiSlashCommandAction::Issue
        | TuiSlashCommandAction::Sessions
        | TuiSlashCommandAction::Agents
        | TuiSlashCommandAction::Subagents => {
            return Err(CommandExecutionError(format!(
                "/{} must be handled by the TUI navigation layer",
                command.command()
            )));
        }
        TuiSlashCommandAction::Status => {
            output.events.push(
                status::Event::PanelOpened(status::load_status_panel(
                    client,
                    status::StatusRequestScope {
                        session_id: conversation.session_id(),
                        thread_id: conversation.thread_id(),
                    },
                )?)
                .into(),
            );
        }
        TuiSlashCommandAction::Skills => {
            output.events.push(
                crate::skills::Event::SettingsOpened(load_selection(
                    client,
                    conversation.session_id(),
                    SkillCatalogReloadDto::Refresh,
                )?)
                .into(),
            );
        }
        TuiSlashCommandAction::Mcp => {
            output
                .events
                .push(mcp::Event::SettingsOpened(mcp::load_selection(client)?).into());
        }
        TuiSlashCommandAction::Connectors => {
            output.events.push(
                crate::connectors::Event::PickerOpened(crate::connectors::load_selection(client)?)
                    .into(),
            );
        }
        TuiSlashCommandAction::Resume => {
            if arguments.is_empty() {
                output.events.push(
                    sessions::Event::PickerOpened(sessions::load_selection(
                        client,
                        conversation.session_id().as_str(),
                    )?)
                    .into(),
                );
            } else {
                match conversation
                    .resume_session(client, &arguments, None)
                    .map_err(session_error)?
                {
                    ResumeOutcome::Listed(notice) => {
                        output
                            .events
                            .push(crate::thread::Event::ProductNotice(notice).into());
                    }
                    ResumeOutcome::Changed(change) => {
                        output.conversation_change = Some(change);
                    }
                }
            }
        }
        TuiSlashCommandAction::Archive => {
            output.conversation_change = Some(
                conversation
                    .archive_and_replace(client)
                    .map_err(session_error)?,
            );
        }
        TuiSlashCommandAction::Rewind => {
            if arguments.is_empty() {
                output.events.push(
                    crate::thread::Event::RewindPickerOpened(rewind::load_selection(
                        client,
                        conversation.session_id(),
                        conversation.thread_id(),
                    )?)
                    .into(),
                );
            } else {
                let before_turn_id = TurnId::new(&arguments).map_err(|error| {
                    CommandExecutionError(format!(
                        "invalid rewind checkpoint '{arguments}': {error}"
                    ))
                })?;
                output.conversation_change = Some(
                    conversation
                        .rewind_active_thread(client, before_turn_id, &arguments)
                        .map_err(session_error)?,
                );
            }
        }
        TuiSlashCommandAction::New => {
            output.conversation_change = Some(
                conversation
                    .replace_with_new(client, &arguments)
                    .map_err(session_error)?,
            );
        }
        TuiSlashCommandAction::AddDir => {
            if arguments.is_empty() {
                output.events.push(
                    dirs::Event::PickerOpened(dirs::load_selection(
                        client,
                        conversation.session_id(),
                    )?)
                    .into(),
                );
            } else {
                let command = format!("/add-dir {arguments}");
                let update = dirs::add(
                    client,
                    conversation.session_id(),
                    std::path::PathBuf::from(&arguments),
                )
                .map_err(CommandExecutionError)?;
                let result = if update.already_present {
                    format!("Directory already added: {arguments}")
                } else {
                    format!(
                        "Added directory {arguments} with no permissions; use /config to grant access"
                    )
                };
                output
                    .events
                    .push(crate::thread::Event::CommandStarted(command.clone()).into());
                output
                    .events
                    .push(crate::thread::Event::CommandCompleted { command, result }.into());
            }
        }
        TuiSlashCommandAction::Fork => {
            output.conversation_change = Some(
                conversation
                    .fork_active_thread(client, &arguments)
                    .map_err(session_error)?,
            );
        }
        TuiSlashCommandAction::Config
        | TuiSlashCommandAction::Export
        | TuiSlashCommandAction::Help
        | TuiSlashCommandAction::Shortcuts
        | TuiSlashCommandAction::Startup
        | TuiSlashCommandAction::StatusLine => {
            return Err(CommandExecutionError(
                "host command reached the App Server dispatcher".into(),
            ));
        }
        TuiSlashCommandAction::Model => {
            if arguments.is_empty() {
                output.events.push(
                    models::Event::PickerOpened(
                        models::load_selection(client)
                            .map_err(|error| CommandExecutionError(error.to_string()))?,
                    )
                    .into(),
                );
            } else {
                let update = models::set_preferred_model(client, &arguments)
                    .map_err(|error| CommandExecutionError(error.to_string()))?;
                output
                    .events
                    .push(models::Event::SummaryReceived(update.summary).into());
                output
                    .events
                    .push(crate::thread::Event::ProductNotice(update.notice).into());
            }
        }
        TuiSlashCommandAction::Theme => unreachable!("theme commands are handled locally"),
        TuiSlashCommandAction::Quit => {
            return Err(CommandExecutionError(
                "quit command reached the product dispatcher".into(),
            ));
        }
    }
    Ok(output)
}

#[derive(Default)]
struct CommandOutput {
    events: Vec<AppEvent>,
    conversation_change: Option<ConversationChange>,
}

fn text_arguments(arguments: &[ChatInputItem]) -> Result<String, CommandExecutionError> {
    if arguments.iter().any(|argument| {
        matches!(
            argument,
            ChatInputItem::Image { .. } | ChatInputItem::Skill { .. }
        )
    }) {
        return Err(CommandExecutionError(
            "product commands do not accept image arguments or Skill selections".into(),
        ));
    }
    Ok(arguments
        .iter()
        .filter_map(|argument| match argument {
            ChatInputItem::Text(text) => Some(text.as_str()),
            ChatInputItem::Image { .. } | ChatInputItem::Skill { .. } => None,
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
