//! Interactive terminal client for Zeta's App Server product boundary.

mod app;
mod chatwidget;
mod clipboard;
mod file_search;
mod render;
mod terminal;
mod toppane;

use app::Action;
use app::App;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use crossterm::event::MouseButton;
use crossterm::event::MouseEventKind;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use toppane::ComposerInput;
use toppane::ComposerSubmission;
use toppane::DynamicSlashCommand;
use toppane::SlashCommandArgumentMode;
use toppane::SlashCommandRegistry;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::common::{SessionId, ThreadId};
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_app_server_protocol::protocol::turn::{InputItem, TurnInterruptParams, TurnStartParams};
use zeta_protocol::{
    CommandId, StableTurnError, StableTurnErrorCode, ThreadItem, TurnId, TurnStatus,
};

/// Startup values owned by the CLI host rather than by the terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiOptions {
    thread_title: String,
    workspace_root: PathBuf,
}

impl TuiOptions {
    pub fn new(thread_title: impl Into<String>) -> Self {
        Self {
            thread_title: thread_title.into(),
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Uses `workspace_root` as the bounded source for `@file` mention candidates.
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = workspace_root.into();
        self
    }
}

/// Describes why the interactive terminal returned control to its host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TuiExit {
    UserRequested,
}

/// Failure to start or operate an interactive terminal session.
#[derive(Debug)]
pub enum TuiError {
    Client(ClientError),
    Terminal(std::io::Error),
}

impl fmt::Display for TuiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(error) => write!(formatter, "{error}"),
            Self::Terminal(error) => write!(formatter, "terminal error: {error}"),
        }
    }
}

impl std::error::Error for TuiError {}

impl From<ClientError> for TuiError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

impl From<std::io::Error> for TuiError {
    fn from(error: std::io::Error) -> Self {
        Self::Terminal(error)
    }
}

/// Runs one interactive terminal session over an initialized App Server client.
///
/// Turn acceptance is asynchronous. The UI polls the product Thread while it is working so an
/// embedded client can collect notifications without owning execution state.
pub fn run<T>(mut client: AppServerClient<T>, options: TuiOptions) -> Result<TuiExit, TuiError>
where
    T: JsonRpcTransport,
{
    let TuiOptions {
        thread_title,
        workspace_root,
    } = options;
    let slash_commands = slash_command_registry(&client.initialization()?.slash_commands)?;
    let session = client.create_session(SessionCreateParams {
        command_id: CommandId::new(request_key("session"))
            .expect("generated command ID is non-empty"),
        title: thread_title.clone(),
    })?;
    let thread = client.create_session_thread(SessionThreadCreateParams {
        command_id: CommandId::new(request_key("thread"))
            .expect("generated command ID is non-empty"),
        session_id: session.session.session_id.clone(),
        expected_sequence: session.session.sequence,
        title: thread_title,
    })?;
    let mut thread_sequence = 1;
    let mut active_turn = None;
    let mut terminal = terminal::TerminalSession::open()?;
    let mut app = App::for_workspace_with_slash_commands(&workspace_root, slash_commands);

    loop {
        app.poll_background_events();
        if matches!(
            app.status(),
            app::Status::Working
                | app::Status::WaitingForApproval
                | app::Status::WaitingForUserInput
                | app::Status::WaitingForCapability
                | app::Status::Cancelling
        ) {
            refresh_turn(
                &mut client,
                &thread.thread_id,
                &mut thread_sequence,
                &mut active_turn,
                &mut app,
            );
        }
        terminal.draw(|frame| render::draw(frame, &app))?;
        if !event::poll(Duration::from_millis(25))? {
            continue;
        }
        let action = match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let terminal_area = terminal.area()?;
                if let Some(index) =
                    render::mention_index_at(&app, terminal_area, mouse.column, mouse.row)
                {
                    app.activate_mention(index);
                    None
                } else {
                    render::slash_command_index_at(&app, terminal_area, mouse.column, mouse.row)
                        .and_then(|index| app.activate_slash_command(index))
                }
            }
            Event::Paste(text) => {
                app.handle_paste(text);
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            match action {
                Action::Quit => return Ok(TuiExit::UserRequested),
                Action::Interrupt => {
                    refresh_turn(
                        &mut client,
                        &thread.thread_id,
                        &mut thread_sequence,
                        &mut active_turn,
                        &mut app,
                    );
                    if let Some(turn_id) = active_turn.clone()
                        && !matches!(app.status(), app::Status::Error)
                    {
                        interrupt_turn(
                            &mut client,
                            &session.session.session_id,
                            &thread.thread_id,
                            &mut thread_sequence,
                            &turn_id,
                            &mut active_turn,
                            &mut app,
                        );
                    } else if !matches!(app.status(), app::Status::Ready) {
                        app.record_interrupt_failure("the active turn is not available".into());
                    }
                }
                Action::PasteImage => match clipboard::read_image() {
                    Ok(image) => app.attach_image_bytes(image.png),
                    Err(error) => app.record_clipboard_error(error),
                },
                Action::Submit(prompt) => {
                    terminal.draw(|frame| render::draw(frame, &app))?;
                    active_turn = submit_prompt(
                        &mut client,
                        &session.session.session_id,
                        &thread.thread_id,
                        &mut thread_sequence,
                        prompt,
                        &mut app,
                    );
                }
            }
        }
    }
}

fn slash_command_registry(
    definitions: &[SlashCommandDefinition],
) -> Result<SlashCommandRegistry, ClientError> {
    let commands = definitions.iter().map(|definition| DynamicSlashCommand {
        name: definition.name.clone(),
        description: definition.description.clone(),
        argument_mode: match definition.argument_mode {
            SlashCommandArgumentModeDto::None => SlashCommandArgumentMode::None,
            SlashCommandArgumentModeDto::Optional => SlashCommandArgumentMode::Optional,
        },
    });
    SlashCommandRegistry::with_dynamic_commands(commands).map_err(|error| {
        ClientError::Protocol(format!(
            "App Server advertised an invalid slash command snapshot: {error}"
        ))
    })
}

fn submit_prompt<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
    thread_sequence: &mut u64,
    submission: ComposerSubmission,
    app: &mut App,
) -> Option<TurnId>
where
    T: JsonRpcTransport,
{
    let turn = client.start_turn(TurnStartParams {
        command_id: CommandId::new(request_key("turn")).expect("generated command ID is non-empty"),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        expected_sequence: *thread_sequence,
        input: submission
            .input
            .into_iter()
            .map(|input| match input {
                ComposerInput::Text(text) => InputItem::Text { text },
                ComposerInput::Image { url } => InputItem::Image { url },
            })
            .collect(),
    });
    match turn {
        Ok(start) => {
            *thread_sequence = start.sequence;
            Some(start.turn_id)
        }
        Err(error) => {
            app.record_error(error.to_string());
            None
        }
    }
}

fn refresh_turn<T>(
    client: &mut AppServerClient<T>,
    thread_id: &ThreadId,
    thread_sequence: &mut u64,
    active_turn: &mut Option<TurnId>,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    match client.read_thread(ThreadReadParams {
        thread_id: thread_id.clone(),
    }) {
        Ok(snapshot) => {
            *thread_sequence = snapshot.thread.sequence;
            apply_active_turn_snapshot(app, active_turn, &snapshot.thread.turns);
        }
        Err(error) => app.record_error(error.to_string()),
    }
    if let Err(error) = client.drain_notifications() {
        app.record_error(error.to_string());
    }
}

fn interrupt_turn<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
    thread_sequence: &mut u64,
    turn_id: &TurnId,
    active_turn: &mut Option<TurnId>,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    match client.interrupt_turn(TurnInterruptParams {
        command_id: CommandId::new(request_key("interrupt"))
            .expect("generated command ID is non-empty"),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        turn_id: turn_id.clone(),
        expected_sequence: *thread_sequence,
    }) {
        Ok(result) => {
            *thread_sequence = result.sequence;
            refresh_turn(client, thread_id, thread_sequence, active_turn, app);
        }
        Err(error) => app.record_interrupt_failure(error.to_string()),
    }
}

fn apply_active_turn_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    turns: &[zeta_protocol::Turn],
) {
    let Some(turn_id) = active_turn.as_ref() else {
        return;
    };
    let Some(turn) = turns.iter().find(|turn| &turn.turn_id == turn_id) else {
        return;
    };

    match turn.status {
        TurnStatus::Completed => {
            let response = turn.items.iter().rev().find_map(|item| match item {
                ThreadItem::AgentMessage { text, .. } => Some(text.clone()),
                _ => None,
            });
            *active_turn = None;
            match response {
                Some(response) => app.record_response(response),
                None => app.record_error("turn completed without an agent message".into()),
            }
        }
        TurnStatus::Failed => {
            *active_turn = None;
            let detail = turn
                .error
                .as_ref()
                .map(present_turn_error)
                .unwrap_or_else(|| {
                    "The request stopped before Zeta could finish. Please try again.".into()
                });
            app.record_error(detail);
        }
        TurnStatus::Interrupted => {
            *active_turn = None;
            app.record_interrupted();
        }
        TurnStatus::WaitingForApproval => app.wait_for_approval(),
        TurnStatus::WaitingForUserInput => app.wait_for_user_input(),
        TurnStatus::WaitingForCapability => app.wait_for_capability(),
        TurnStatus::Created | TurnStatus::Running => app.record_working(),
        TurnStatus::Cancelling => app.record_cancelling(),
    }
}

fn present_turn_error(error: &StableTurnError) -> String {
    match error.code {
        StableTurnErrorCode::ModelInvocationFailed => {
            "Zeta couldn't reach the configured model. Check the model provider and credentials, \
             then try again."
                .into()
        }
        StableTurnErrorCode::CompletionPersistenceFailed => {
            "Zeta generated a response but couldn't save it. Please try again.".into()
        }
    }
}

fn request_key(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{timestamp}", std::process::id())
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
