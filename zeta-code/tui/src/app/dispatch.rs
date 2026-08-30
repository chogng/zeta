//! Built-in product command dispatch for the active Session and Thread.

use crate::app::AppEvent;
use crate::app::help_pane_spec;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::chat_input::TuiSlashCommandAction;
use crate::features::config;
use crate::features::dirs;
use crate::features::mcp;
use crate::features::models;
use crate::features::rewind;
use crate::features::sessions;
use crate::features::sessions::ActiveConversation;
use crate::features::sessions::ConversationChange;
use crate::features::sessions::NewConversationKind;
use crate::features::sessions::ResumeOutcome;
use crate::features::skills::load_selection;
use crate::features::status;
use crate::features::theme::ThemeResource;
use crate::features::theme::theme_pane_spec;
use std::fmt;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirMutationDto;
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
    pub(crate) exit_requested: bool,
}

pub(crate) fn execute_product_command<T>(
    mut conversation: ActiveConversation,
    client: &mut AppServerClient<T>,
    invocation: SlashCommandInvocation,
    dir_permissions: Vec<PermissionDto>,
    theme_resource: &ThemeResource,
) -> Result<ProductCommandOutput, String>
where
    T: JsonRpcTransport,
{
    conversation
        .try_execute(client, invocation, &dir_permissions, theme_resource)
        .map(|output| ProductCommandOutput {
            conversation,
            events: output.events,
            conversation_change: output.conversation_change,
            exit_requested: output.exit_requested,
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
        let theme_resource = ThemeResource::for_test(
            std::env::temp_dir().join(format!("zeta-tui-dispatch-theme-{}", std::process::id())),
            zeta_terminal_detection::ColorLevel::TrueColor,
            zeta_theme::ColorScheme::Dark,
        );
        match execute_product_command(
            self.clone(),
            client,
            invocation,
            config::TerminalSettings::default().dir_permissions(),
            &theme_resource,
        ) {
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
        dir_permissions: &[PermissionDto],
        theme_resource: &ThemeResource,
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
            TuiSlashCommandAction::Sessions
            | TuiSlashCommandAction::Agents
            | TuiSlashCommandAction::Subagents
            | TuiSlashCommandAction::Queue => {
                return Err(CommandExecutionError(format!(
                    "/{} must be handled by the TUI navigation layer",
                    command.command()
                )));
            }
            TuiSlashCommandAction::Status => {
                output.events.push(AppEvent::StatusQuickViewOpened(
                    status::load_status_pane_spec(
                        client,
                        status::StatusRequestScope {
                            session_id: self.session_id(),
                            thread_id: self.thread_id(),
                        },
                    )?,
                ));
            }
            TuiSlashCommandAction::Skills => {
                output
                    .events
                    .push(AppEvent::SkillsPaneOpened(load_selection(
                        client,
                        self.session_id(),
                        SkillCatalogReloadDto::Refresh,
                    )?));
            }
            TuiSlashCommandAction::Mcp => {
                output
                    .events
                    .push(AppEvent::McpPaneOpened(mcp::load_selection(client)?));
            }
            TuiSlashCommandAction::Connectors => {
                output.events.push(AppEvent::ConnectorPaneOpened(
                    crate::features::connectors::load_selection(client)?,
                ));
            }
            TuiSlashCommandAction::Resume => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::SessionPaneOpened(sessions::load_selection(
                            client,
                            self.session_id().as_str(),
                        )?));
                } else {
                    match self
                        .resume_session(client, &arguments, None)
                        .map_err(session_error)?
                    {
                        ResumeOutcome::Listed(notice) => {
                            output.events.push(AppEvent::ProductNotice(notice));
                        }
                        ResumeOutcome::Changed(change) => {
                            output.conversation_change = Some(change);
                        }
                    }
                }
            }
            TuiSlashCommandAction::Archive => {
                let archived_id = self.archive_session(client).map_err(session_error)?;
                output.events.push(AppEvent::ProductNotice(format!(
                    "Archived session {archived_id}."
                )));
                output.exit_requested = true;
            }
            TuiSlashCommandAction::Rewind => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::RewindPaneOpened(rewind::load_selection(
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
            TuiSlashCommandAction::AddDir => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::DirsPaneOpened(dirs::load_selection(
                            client,
                            self.session_id(),
                        )?));
                } else {
                    let command = format!("/add-dir {arguments}");
                    let update = dirs::add(
                        client,
                        self.session_id(),
                        std::path::PathBuf::from(&arguments),
                        dir_permissions.to_vec(),
                    )?;
                    let result = match update.mutation {
                        SessionDirMutationDto::Added => {
                            format!("Added directory {arguments}")
                        }
                        SessionDirMutationDto::AlreadyPresent => {
                            format!("Directory already added: {arguments}")
                        }
                        SessionDirMutationDto::Updated
                        | SessionDirMutationDto::Removed
                        | SessionDirMutationDto::NotPresent => {
                            return Err(CommandExecutionError(
                                "add-dir returned an invalid mutation result".into(),
                            ));
                        }
                    };
                    output
                        .events
                        .push(AppEvent::CommandStarted(command.clone()));
                    output
                        .events
                        .push(AppEvent::CommandCompleted { command, result });
                }
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
                    .push(AppEvent::ListSelectionPaneOpened(help_pane_spec()));
            }
            TuiSlashCommandAction::Copy
            | TuiSlashCommandAction::Config
            | TuiSlashCommandAction::Export
            | TuiSlashCommandAction::Shortcuts
            | TuiSlashCommandAction::StatusLine => {
                return Err(CommandExecutionError(
                    "host command reached the App Server dispatcher".into(),
                ));
            }
            TuiSlashCommandAction::Model => {
                if arguments.is_empty() {
                    output
                        .events
                        .push(AppEvent::ModelPaneOpened(models::load_selection(client)?));
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
                    let catalog = theme_resource.catalog().map_err(CommandExecutionError)?;
                    output
                        .events
                        .push(AppEvent::ThemePaneOpened(theme_pane_spec(&catalog)));
                } else {
                    let command = format!("/theme {arguments}");
                    let selection = theme_resource
                        .select(&arguments)
                        .map_err(CommandExecutionError)?;
                    output
                        .events
                        .push(AppEvent::CommandStarted(command.clone()));
                    output
                        .events
                        .push(AppEvent::RenderThemeChanged(selection.theme));
                    output.events.push(AppEvent::CommandCompleted {
                        command,
                        result: format!("Theme set to {}", selection.label),
                    });
                }
            }
            TuiSlashCommandAction::Quit => {
                return Err(CommandExecutionError(
                    "quit command reached the product dispatcher".into(),
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
    exit_requested: bool,
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
