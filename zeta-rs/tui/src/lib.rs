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
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::common::ThreadId;
use zeta_app_server_protocol::v1::thread::ThreadStartParams;
use zeta_app_server_protocol::v1::turn::InputItem;
use zeta_app_server_protocol::v1::turn::InputItemKind;
use zeta_app_server_protocol::v1::turn::TurnStartParams;

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
/// The current client contract performs a complete request synchronously, so keyboard input is
/// paused while a turn is executing. The UI owns only presentation state; authoritative Thread
/// and Turn state remains behind the App Server protocol.
pub fn run<T>(mut client: AppServerClient<T>, options: TuiOptions) -> Result<TuiExit, TuiError>
where
    T: JsonRpcTransport,
{
    let thread = client.start_thread(ThreadStartParams {
        idempotency_key: request_key("thread"),
        title: options.thread_title,
    })?;
    let mut terminal = terminal::TerminalSession::open()?;
    let mut app = App::new();

    loop {
        terminal.draw(|frame| render::draw(frame, &app))?;
        match event::read()? {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if let Some(action) = app.handle_key(key) {
                    match action {
                        Action::Quit => return Ok(TuiExit::UserRequested),
                        Action::Submit(prompt) => {
                            terminal.draw(|frame| render::draw(frame, &app))?;
                            submit_prompt(&mut client, &thread.thread_id, prompt, &mut app);
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
    thread_id: &ThreadId,
    prompt: String,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    let turn = client.start_turn(TurnStartParams {
        idempotency_key: request_key("turn"),
        thread_id: thread_id.clone(),
        input: vec![InputItem {
            kind: InputItemKind::Text,
            text: prompt,
        }],
    });
    if let Err(error) = turn {
        app.record_error(error.to_string());
        return;
    }

    match client.drain_notifications() {
        Ok(notifications) => apply_notifications(app, notifications),
        Err(error) => app.record_error(error.to_string()),
    }
}

fn apply_notifications(app: &mut App, notifications: Vec<ServerNotification>) {
    let mut received_response = false;
    for notification in notifications {
        match notification {
            ServerNotification::AgentMessageCompleted(message) => {
                app.record_response(message.text);
                received_response = true;
            }
            ServerNotification::TurnInterrupted(_) => {
                app.record_error("turn interrupted".into());
                return;
            }
            _ => {}
        }
    }
    if !received_response {
        app.record_error("turn completed without an agent message".into());
    }
}

fn request_key(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{timestamp}", std::process::id())
}
