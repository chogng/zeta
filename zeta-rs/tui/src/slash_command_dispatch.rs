use crate::app::App;
use crate::request_key;
use crate::toppane::ComposerInput;
use crate::toppane::SlashCommand;
use crate::toppane::SlashCommandInvocation;
use crate::toppane::SlashCommandItem;
use crate::toppane::built_in_slash_commands;
use std::fmt;
use zeta_app_server_client::{AppServerClient, ClientError, JsonRpcTransport};
use zeta_app_server_protocol::protocol::config::{
    ConfigReadResult, ConfigUpdateParams, McpServerEnablementDto, ModelRefDto,
    SkillSourceEnablementDto,
};
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionReadParams, SessionThreadCreateParams, SessionThreadForkParams,
};
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_protocol::{CommandId, Patch, Session, SessionId, SessionThreadStatus, ThreadId};

/// Mutable Session/Thread selection used by one TUI conversation.
pub(crate) struct ActiveConversation {
    session: Session,
    thread_id: ThreadId,
    thread_sequence: u64,
}

impl ActiveConversation {
    pub(crate) fn start<T>(
        client: &mut AppServerClient<T>,
        title: String,
    ) -> Result<Self, ClientError>
    where
        T: JsonRpcTransport,
    {
        create_conversation(client, title)
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session.session_id
    }

    pub(crate) fn thread_id(&self) -> &ThreadId {
        &self.thread_id
    }

    pub(crate) fn thread_sequence(&self) -> u64 {
        self.thread_sequence
    }

    pub(crate) fn set_thread_sequence(&mut self, sequence: u64) {
        self.thread_sequence = sequence;
    }

    pub(crate) fn execute<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        invocation: SlashCommandInvocation,
        app: &mut App,
    ) where
        T: JsonRpcTransport,
    {
        if let Err(error) = self.try_execute(client, invocation, app) {
            app.record_error(error.to_string());
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
        let SlashCommandItem::Builtin(command) = invocation.command else {
            return Err(CommandExecutionError(
                "dynamic command reached the built-in dispatcher".into(),
            ));
        };
        let arguments = text_arguments(&invocation.arguments)?;

        match command {
            SlashCommand::Status => self.show_status(client, app),
            SlashCommand::Skills => show_skills(client, app),
            SlashCommand::Mcp => show_mcp(client, app),
            SlashCommand::Resume => self.resume(client, &arguments, app),
            SlashCommand::Clear | SlashCommand::New => {
                self.start_new(client, command, &arguments, app)
            }
            SlashCommand::Config => show_config(client, app),
            SlashCommand::Fork => self.fork(client, &arguments, app),
            SlashCommand::Help => {
                app.record_notice(help_text());
                Ok(())
            }
            SlashCommand::Model => set_or_show_model(client, &arguments, app),
            SlashCommand::Quit | SlashCommand::Exit => Err(CommandExecutionError(
                "exit command reached the product dispatcher".into(),
            )),
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
        app.record_notice(format!(
            "Session: {}\nThread: {}\nThread sequence: {}\nModel: {}",
            self.session.session_id,
            self.thread_id,
            self.thread_sequence,
            preferred_model(&config)
        ));
        Ok(())
    }

    fn start_new<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        command: SlashCommand,
        arguments: &str,
        app: &mut App,
    ) -> Result<(), CommandExecutionError>
    where
        T: JsonRpcTransport,
    {
        let title = if arguments.is_empty() {
            match command {
                SlashCommand::Clear => "Cleared conversation",
                SlashCommand::New => "TUI conversation",
                _ => unreachable!("only new-chat commands call start_new"),
            }
            .to_owned()
        } else {
            arguments.to_owned()
        };
        *self = create_conversation(client, title)?;
        app.clear_messages();
        app.record_notice(format!(
            "Started session {} on thread {}.",
            self.session.session_id, self.thread_id
        ));
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
        let title = if arguments.is_empty() {
            format!("Fork of {}", self.session.title)
        } else {
            arguments.to_owned()
        };
        let result = client.fork_session_thread(SessionThreadForkParams {
            command_id: command_id("fork"),
            session_id: self.session.session_id.clone(),
            expected_sequence: self.session.sequence,
            parent_thread_id: self.thread_id.clone(),
            title,
        })?;
        let thread = client.read_thread(ThreadReadParams {
            thread_id: result.thread_id.clone(),
        })?;
        self.session = result.session;
        self.thread_id = result.thread_id;
        self.thread_sequence = thread.thread.sequence;
        app.load_thread(&thread.thread);
        app.record_notice(format!("Forked to thread {}.", self.thread_id));
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
        if arguments.is_empty() {
            let sessions = client.list_sessions()?.sessions;
            let text = if sessions.is_empty() {
                "No saved sessions.".into()
            } else {
                let lines = sessions
                    .into_iter()
                    .map(|session| {
                        format!(
                            "{}  {}  {:?}",
                            session.session_id, session.title, session.status
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Saved sessions:\n{lines}\nUse /resume <session-id>.")
            };
            app.record_notice(text);
            return Ok(());
        }

        let session_id = SessionId::new(arguments).map_err(|error| {
            CommandExecutionError(format!("invalid session ID '{arguments}': {error}"))
        })?;
        let session = client
            .read_session(SessionReadParams { session_id })?
            .session;
        let thread_id = session
            .threads
            .iter()
            .rev()
            .find(|thread| thread.status == SessionThreadStatus::Active)
            .map(|thread| thread.thread_id.clone())
            .ok_or_else(|| {
                CommandExecutionError(format!(
                    "session {} has no active thread",
                    session.session_id
                ))
            })?;
        let thread = client.read_thread(ThreadReadParams {
            thread_id: thread_id.clone(),
        })?;
        self.session = session;
        self.thread_id = thread_id;
        self.thread_sequence = thread.thread.sequence;
        app.load_thread(&thread.thread);
        app.record_notice(format!(
            "Resumed session {} on thread {}.",
            self.session.session_id, self.thread_id
        ));
        Ok(())
    }
}

fn create_conversation<T>(
    client: &mut AppServerClient<T>,
    title: String,
) -> Result<ActiveConversation, ClientError>
where
    T: JsonRpcTransport,
{
    let session = client.create_session(SessionCreateParams {
        command_id: command_id("session"),
        title: title.clone(),
    })?;
    let thread = client.create_session_thread(SessionThreadCreateParams {
        command_id: command_id("thread"),
        session_id: session.session.session_id.clone(),
        expected_sequence: session.session.sequence,
        title,
    })?;
    let thread_snapshot = client.read_thread(ThreadReadParams {
        thread_id: thread.thread_id.clone(),
    })?;
    Ok(ActiveConversation {
        session: thread.session,
        thread_id: thread.thread_id,
        thread_sequence: thread_snapshot.thread.sequence,
    })
}

fn show_config<T>(
    client: &mut AppServerClient<T>,
    app: &mut App,
) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    app.record_notice(format!(
        "Config revision: {}\nModel: {}\nProviders: {}\nMCP servers: {}\nSkill sources: {}",
        config.revision,
        preferred_model(&config),
        config.providers.len(),
        config.mcp_servers.len(),
        config.skill_sources.len()
    ));
    Ok(())
}

fn show_mcp<T>(client: &mut AppServerClient<T>, app: &mut App) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let text = if config.mcp_servers.is_empty() {
        "No MCP servers configured.".into()
    } else {
        config
            .mcp_servers
            .values()
            .map(|server| {
                let state = match server.enablement {
                    McpServerEnablementDto::Disabled => "disabled",
                    McpServerEnablementDto::Enabled => "enabled",
                };
                format!("{}  {}  {state}", server.id, server.display_name)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    app.record_notice(text);
    Ok(())
}

fn show_skills<T>(
    client: &mut AppServerClient<T>,
    app: &mut App,
) -> Result<(), CommandExecutionError>
where
    T: JsonRpcTransport,
{
    let config = client.read_config()?;
    let text = if config.skill_sources.is_empty() {
        "No skill sources configured.".into()
    } else {
        config
            .skill_sources
            .values()
            .map(|source| {
                let state = match source.enablement {
                    SkillSourceEnablementDto::Disabled => "disabled",
                    SkillSourceEnablementDto::Enabled => "enabled",
                };
                format!("{}  {}  {state}", source.id, source.root_reference)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    app.record_notice(text);
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
    let config = client.read_config()?;
    if arguments.is_empty() {
        app.record_notice(format!("Preferred model: {}", preferred_model(&config)));
        return Ok(());
    }

    let preferred_model_patch = if arguments == "clear" {
        Patch::Null
    } else {
        let (provider, model) = arguments.split_once('/').ok_or_else(|| {
            CommandExecutionError(
                "model must use <provider>/<model>; use /model clear to unset it".into(),
            )
        })?;
        if provider.trim().is_empty()
            || model.trim().is_empty()
            || provider.contains(char::is_whitespace)
            || model.contains(char::is_whitespace)
        {
            return Err(CommandExecutionError(
                "model must use non-empty <provider>/<model> without whitespace".into(),
            ));
        }
        if !config.providers.contains_key(provider) {
            return Err(CommandExecutionError(format!(
                "provider '{provider}' is not configured"
            )));
        }
        Patch::Value(ModelRefDto {
            provider: provider.into(),
            model: model.into(),
        })
    };

    client.update_config(ConfigUpdateParams {
        command_id: command_id("model"),
        expected_revision: config.revision,
        preferred_model: preferred_model_patch,
        approval_review_model: Patch::Missing,
        theme: Patch::Missing,
    })?;
    let updated = client.read_config()?;
    app.record_notice(format!("Preferred model: {}", preferred_model(&updated)));
    Ok(())
}

fn preferred_model(config: &ConfigReadResult) -> String {
    config
        .preferred_model
        .as_ref()
        .map(|model| format!("{}/{}", model.provider, model.model))
        .unwrap_or_else(|| "not configured".into())
}

fn help_text() -> String {
    built_in_slash_commands()
        .into_iter()
        .map(|(name, command)| format!("/{name}  {}", command.description()))
        .collect::<Vec<_>>()
        .join("\n")
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

fn command_id(prefix: &str) -> CommandId {
    CommandId::new(request_key(prefix)).expect("generated command ID is non-empty")
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
#[path = "slash_command_dispatch_tests.rs"]
mod tests;
