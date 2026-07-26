//! Interactive terminal client for Zeta's App Server product boundary.

mod app;
mod render;
mod terminal;

use app::Action;
use app::App;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use std::fmt;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::protocol::common::{SessionId, ThreadId};
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::thread::ThreadReadParams;
use zeta_app_server_protocol::protocol::turn::{InputItem, InputItemKind, TurnStartParams};
use zeta_protocol::{CommandId, ThreadEvent, ThreadItem, ThreadUpdate};

/// Startup values owned by the CLI host rather than by the terminal UI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TuiOptions {
    thread_title: String,
}

impl TuiOptions {
    pub fn new(thread_title: impl Into<String>) -> Self {
        Self {
            thread_title: thread_title.into(),
        }
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
    let session = client.create_session(SessionCreateParams {
        command_id: CommandId::new(request_key("session"))
            .expect("generated command ID is non-empty"),
        title: options.thread_title.clone(),
    })?;
    let thread = client.create_session_thread(SessionThreadCreateParams {
        command_id: CommandId::new(request_key("thread"))
            .expect("generated command ID is non-empty"),
        session_id: session.session.session_id.clone(),
        expected_sequence: session.session.sequence,
        title: options.thread_title,
    })?;
    let mut thread_sequence = 1;
    let mut terminal = terminal::TerminalSession::open()?;
    let mut app = App::new();

    loop {
        if matches!(app.status(), app::Status::Working) {
            refresh_turn(
                &mut client,
                &thread.thread_id,
                &mut thread_sequence,
                &mut app,
            );
        }
        terminal.draw(|frame| render::draw(frame, &app))?;
        if !event::poll(Duration::from_millis(25))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if let Some(action) = app.handle_key(key) {
                    match action {
                        Action::Quit => return Ok(TuiExit::UserRequested),
                        Action::Submit(prompt) => {
                            terminal.draw(|frame| render::draw(frame, &app))?;
                            submit_prompt(
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
            Event::Paste(text) => app.insert_text(&text),
            _ => {}
        }
    }
}

fn submit_prompt<T>(
    client: &mut AppServerClient<T>,
    session_id: &SessionId,
    thread_id: &ThreadId,
    thread_sequence: &mut u64,
    prompt: String,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    let turn = client.start_turn(TurnStartParams {
        command_id: CommandId::new(request_key("turn")).expect("generated command ID is non-empty"),
        session_id: session_id.clone(),
        thread_id: thread_id.clone(),
        expected_sequence: *thread_sequence,
        input: vec![InputItem {
            kind: InputItemKind::Text,
            text: prompt,
        }],
    });
    if let Err(error) = turn {
        app.record_error(error.to_string());
        return;
    }

    refresh_turn(client, thread_id, thread_sequence, app);
}

fn refresh_turn<T>(
    client: &mut AppServerClient<T>,
    thread_id: &ThreadId,
    thread_sequence: &mut u64,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    match client.read_thread(ThreadReadParams {
        thread_id: thread_id.clone(),
    }) {
        Ok(snapshot) => *thread_sequence = snapshot.thread.sequence,
        Err(error) => app.record_error(error.to_string()),
    }
    match client.drain_notifications() {
        Ok(notifications) => apply_notifications(app, notifications),
        Err(error) => app.record_error(error.to_string()),
    }
}

fn apply_notifications(app: &mut App, notifications: Vec<ServerNotification>) {
    for notification in notifications {
        match notification {
            ServerNotification::ThreadUpdate(update) => match update.update {
                ThreadUpdate::Committed {
                    event:
                        ThreadEvent::ItemCompleted {
                            item: ThreadItem::AgentMessage { text, .. },
                            ..
                        },
                } => {
                    app.record_response(text);
                }
                ThreadUpdate::Committed {
                    event: ThreadEvent::TurnInterrupted { .. },
                } => {
                    app.record_error("turn interrupted".into());
                    return;
                }
                ThreadUpdate::Committed {
                    event: ThreadEvent::TurnFailed { .. },
                } => {
                    app.record_error("turn failed".into());
                    return;
                }
                _ => {}
            },
            ServerNotification::SessionUpdate(_) => {}
            ServerNotification::Unknown { .. } => {}
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
