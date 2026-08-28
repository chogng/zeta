use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use zeta_app_server_protocol::protocol::language::LanguageCloseParams;
use zeta_app_server_protocol::protocol::language::LanguageCompletionsParams;
use zeta_app_server_protocol::protocol::language::LanguageDocumentDto;
use zeta_app_server_protocol::protocol::language::LanguageHoverParams;
use zeta_app_server_protocol::protocol::language::LanguageLocationKindDto;
use zeta_app_server_protocol::protocol::language::LanguageLocationsParams;
use zeta_app_server_protocol::protocol::language::LanguageSynchronizeParams;
use zeta_lsp_manager::LanguageRequestKind;
use zui::app::AppProxy;

use super::remote::RemoteLanguageEvent;
use crate::app_server::{
    AppServerEvent, AppServerEvents, AppServerHost, AppServerRequestHandle, ClientError,
    ServerNotification,
};
use crate::product_event::ProductEvent;

const COMMAND_QUEUE_CAPACITY: usize = 64;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const RECONNECT_WINDOW: Duration = Duration::from_secs(30);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(250);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(2);
const REMOTE_LANGUAGE_UNAVAILABLE: &str =
    "Remote language service is not connected; the request was not sent";

enum RemoteLanguageCommand {
    Synchronize(LanguageDocumentDto),
    Close(PathBuf),
    Hover {
        request_id: u64,
        params: LanguageHoverParams,
    },
    Completions {
        request_id: u64,
        params: LanguageCompletionsParams,
    },
    Locations {
        request_id: u64,
        params: LanguageLocationsParams,
    },
    Shutdown,
}

struct RemoteLanguageFailure {
    error: anyhow::Error,
    connection_was_ready: bool,
}

impl RemoteLanguageFailure {
    fn connecting(error: anyhow::Error) -> Self {
        Self {
            error,
            connection_was_ready: false,
        }
    }

    fn disconnected(error: anyhow::Error) -> Self {
        Self {
            error,
            connection_was_ready: true,
        }
    }
}

#[derive(Debug)]
struct ProductEventLoopUnavailable;

impl fmt::Display for ProductEventLoopUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("desktop event loop is unavailable")
    }
}

impl std::error::Error for ProductEventLoopUnavailable {}

/// Owns the Remote App Server connection used by app language features.
///
/// The dedicated connection prevents a slow language-server response from blocking Agent and
/// filesystem requests. SSH credentials and reconnect policy remain in the desktop product host;
/// the Remote App Server remains the only language and Workspace authority.
pub(crate) struct RemoteLanguageSession {
    available: Arc<AtomicBool>,
    closing: Arc<AtomicBool>,
    commands: SyncSender<RemoteLanguageCommand>,
    next_request_id: AtomicU64,
    worker: Option<JoinHandle<()>>,
}

impl RemoteLanguageSession {
    pub(crate) fn spawn(
        event_proxy: AppProxy<ProductEvent>,
        target: AppServerHost,
    ) -> Result<Self> {
        if !target.is_remote() {
            return Err(anyhow!("Remote language session requires an SSH target"));
        }
        let (commands, receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
        let available = Arc::new(AtomicBool::new(false));
        let closing = Arc::new(AtomicBool::new(false));
        let worker_availability = Arc::clone(&available);
        let worker_closing = Arc::clone(&closing);
        let worker = thread::Builder::new()
            .name("app-remote-language".into())
            .spawn(move || {
                run_remote_language_session(
                    event_proxy,
                    receiver,
                    target,
                    worker_availability,
                    worker_closing,
                )
            })
            .context("could not start Remote language session worker")?;
        Ok(Self {
            available,
            closing,
            commands,
            next_request_id: AtomicU64::new(1),
            worker: Some(worker),
        })
    }

    pub(crate) fn synchronize(&self, document: LanguageDocumentDto) -> Result<()> {
        self.try_send(RemoteLanguageCommand::Synchronize(document))
    }

    pub(crate) fn close(&self, path: PathBuf) -> Result<()> {
        self.try_send(RemoteLanguageCommand::Close(path))
    }

    pub(crate) fn hover(&self, params: LanguageHoverParams) -> Result<u64> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        self.try_send(RemoteLanguageCommand::Hover { request_id, params })?;
        Ok(request_id)
    }

    pub(crate) fn completions(&self, params: LanguageCompletionsParams) -> Result<u64> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        self.try_send(RemoteLanguageCommand::Completions { request_id, params })?;
        Ok(request_id)
    }

    pub(crate) fn locations(&self, params: LanguageLocationsParams) -> Result<u64> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        self.try_send(RemoteLanguageCommand::Locations { request_id, params })?;
        Ok(request_id)
    }

    fn try_send(&self, command: RemoteLanguageCommand) -> Result<()> {
        if !self.available.load(Ordering::Acquire) {
            return Err(anyhow!(REMOTE_LANGUAGE_UNAVAILABLE));
        }
        self.commands
            .try_send(command)
            .context("Remote language request queue is unavailable")
    }
}

impl Drop for RemoteLanguageSession {
    fn drop(&mut self) {
        self.available.store(false, Ordering::Release);
        self.closing.store(true, Ordering::Release);
        let _ = self.commands.try_send(RemoteLanguageCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_remote_language_session(
    event_proxy: AppProxy<ProductEvent>,
    commands: Receiver<RemoteLanguageCommand>,
    target: AppServerHost,
    available: Arc<AtomicBool>,
    closing: Arc<AtomicBool>,
) {
    let mut attempts = 0;
    let mut recovery_started = None;
    loop {
        if closing.load(Ordering::Acquire) {
            return;
        }
        match run_connection(&event_proxy, &commands, &target, &available, &closing) {
            Ok(()) => return,
            Err(failure) => {
                available.store(false, Ordering::Release);
                if failure
                    .error
                    .downcast_ref::<ProductEventLoopUnavailable>()
                    .is_some()
                {
                    return;
                }
                let _ = send_event(&event_proxy, RemoteLanguageEvent::ConnectionLost);
                if failure.connection_was_ready || recovery_started.is_none() {
                    attempts = 0;
                    recovery_started = Some(Instant::now());
                }
                let started = recovery_started.expect("language recovery has a start time");
                let Some(delay) = reconnect_delay_within_window(started.elapsed(), attempts) else {
                    let _ = send_event(
                        &event_proxy,
                        RemoteLanguageEvent::ConnectionError(format!(
                            "Remote language service did not recover within {} seconds after {attempts} attempts: {}",
                            RECONNECT_WINDOW.as_secs(),
                            failure.error
                        )),
                    );
                    return;
                };
                attempts += 1;
                let _ = send_event(
                    &event_proxy,
                    RemoteLanguageEvent::ConnectionError(format!(
                        "Remote language service disconnected; reconnecting (attempt {attempts})"
                    )),
                );
                if !wait_for_reconnect(&event_proxy, &commands, delay, &closing) {
                    return;
                }
            }
        }
    }
}

fn run_connection(
    event_proxy: &AppProxy<ProductEvent>,
    commands: &Receiver<RemoteLanguageCommand>,
    target: &AppServerHost,
    available: &AtomicBool,
    closing: &AtomicBool,
) -> std::result::Result<(), RemoteLanguageFailure> {
    available.store(false, Ordering::Release);
    let mut session = target.start().map_err(RemoteLanguageFailure::connecting)?;
    let mut client = session.client();
    let events = session
        .take_events()
        .map_err(|error| RemoteLanguageFailure::connecting(anyhow!(error.to_string())))?;
    available.store(true, Ordering::Release);
    send_event(event_proxy, RemoteLanguageEvent::ConnectionReady)
        .map_err(RemoteLanguageFailure::disconnected)?;
    let result = drive_connection(event_proxy, commands, &events, &mut client, closing);
    available.store(false, Ordering::Release);
    let _ = session.shutdown();
    result.map_err(RemoteLanguageFailure::disconnected)
}

fn drive_connection(
    event_proxy: &AppProxy<ProductEvent>,
    commands: &Receiver<RemoteLanguageCommand>,
    events: &AppServerEvents,
    client: &mut AppServerRequestHandle,
    closing: &AtomicBool,
) -> Result<()> {
    loop {
        if closing.load(Ordering::Acquire) {
            return Ok(());
        }
        loop {
            if closing.load(Ordering::Acquire) {
                return Ok(());
            }
            match commands.try_recv() {
                Ok(RemoteLanguageCommand::Shutdown) => return Ok(()),
                Ok(command) => drive_command(event_proxy, client, command)?,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }
        match events.recv_timeout(EVENT_POLL_INTERVAL) {
            Ok(AppServerEvent::Notification(ServerNotification::LanguageDiagnostics(
                diagnostics,
            ))) => send_event(event_proxy, RemoteLanguageEvent::Diagnostics(diagnostics))?,
            Ok(AppServerEvent::Notification(ServerNotification::LanguageServerMessage(
                message,
            ))) => send_event(event_proxy, RemoteLanguageEvent::ServerMessage(message))?,
            Ok(AppServerEvent::Notification(_)) => {}
            Ok(AppServerEvent::ConnectionClosed(reason)) => {
                return Err(anyhow!(
                    "Remote language App Server connection closed: {reason:?}"
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow!(
                    "Remote language App Server event stream disconnected"
                ));
            }
        }
    }
}

fn drive_command(
    event_proxy: &AppProxy<ProductEvent>,
    client: &mut AppServerRequestHandle,
    command: RemoteLanguageCommand,
) -> Result<()> {
    match command {
        RemoteLanguageCommand::Synchronize(document) => {
            let path = document.path.clone();
            match client.synchronize_language_document(LanguageSynchronizeParams { document }) {
                Ok(()) => Ok(()),
                Err(error) => document_failure(event_proxy, path, "synchronize", error),
            }
        }
        RemoteLanguageCommand::Close(path) => {
            match client.close_language_document(LanguageCloseParams {
                workspace_folder_id: None,
                session_directory: None,
                path: path.clone(),
            }) {
                Ok(()) => Ok(()),
                Err(error) => document_failure(event_proxy, path, "close", error),
            }
        }
        RemoteLanguageCommand::Hover { request_id, params } => {
            let path = params.document.path.clone();
            match client.language_hover(params) {
                Ok(result) => send_event(
                    event_proxy,
                    RemoteLanguageEvent::Hover {
                        request_id,
                        path,
                        result,
                    },
                ),
                Err(error) => request_failure(
                    event_proxy,
                    request_id,
                    LanguageRequestKind::Hover,
                    path,
                    error,
                ),
            }
        }
        RemoteLanguageCommand::Completions { request_id, params } => {
            let path = params.document.path.clone();
            match client.language_completions(params) {
                Ok(result) => send_event(
                    event_proxy,
                    RemoteLanguageEvent::Completions {
                        request_id,
                        path,
                        result,
                    },
                ),
                Err(error) => request_failure(
                    event_proxy,
                    request_id,
                    LanguageRequestKind::Completion,
                    path,
                    error,
                ),
            }
        }
        RemoteLanguageCommand::Locations { request_id, params } => {
            let path = params.document.path.clone();
            let kind = params.kind;
            match client.language_locations(params) {
                Ok(result) => send_event(
                    event_proxy,
                    RemoteLanguageEvent::Locations {
                        request_id,
                        path,
                        kind,
                        result,
                    },
                ),
                Err(error) => {
                    request_failure(event_proxy, request_id, request_kind(kind), path, error)
                }
            }
        }
        RemoteLanguageCommand::Shutdown => Ok(()),
    }
}

fn document_failure(
    event_proxy: &AppProxy<ProductEvent>,
    path: PathBuf,
    operation: &'static str,
    error: ClientError,
) -> Result<()> {
    match error {
        ClientError::Transport(message) => Err(anyhow!(message)),
        error => send_event(
            event_proxy,
            RemoteLanguageEvent::DocumentOperationFailed {
                path,
                operation,
                message: error.to_string(),
            },
        ),
    }
}

fn request_failure(
    event_proxy: &AppProxy<ProductEvent>,
    request_id: u64,
    kind: LanguageRequestKind,
    path: PathBuf,
    error: ClientError,
) -> Result<()> {
    match error {
        ClientError::Transport(message) => Err(anyhow!(message)),
        error => send_event(
            event_proxy,
            RemoteLanguageEvent::RequestFailed {
                request_id,
                kind,
                path,
                message: error.to_string(),
            },
        ),
    }
}

fn wait_for_reconnect(
    event_proxy: &AppProxy<ProductEvent>,
    commands: &Receiver<RemoteLanguageCommand>,
    delay: Duration,
    closing: &AtomicBool,
) -> bool {
    let started = Instant::now();
    loop {
        if closing.load(Ordering::Acquire) {
            return false;
        }
        let Some(remaining) = delay.checked_sub(started.elapsed()) else {
            return true;
        };
        match commands.recv_timeout(remaining) {
            Ok(RemoteLanguageCommand::Shutdown) => return false,
            Ok(command) => {
                if let Some(event) = disconnected_event(command) {
                    let _ = send_event(event_proxy, event);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return true,
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }
}

fn disconnected_event(command: RemoteLanguageCommand) -> Option<RemoteLanguageEvent> {
    let message = REMOTE_LANGUAGE_UNAVAILABLE.to_owned();
    match command {
        RemoteLanguageCommand::Synchronize(document) => {
            Some(RemoteLanguageEvent::DocumentOperationFailed {
                path: document.path,
                operation: "synchronize",
                message,
            })
        }
        RemoteLanguageCommand::Close(path) => Some(RemoteLanguageEvent::DocumentOperationFailed {
            path,
            operation: "close",
            message,
        }),
        RemoteLanguageCommand::Hover { request_id, params } => {
            Some(RemoteLanguageEvent::RequestFailed {
                request_id,
                kind: LanguageRequestKind::Hover,
                path: params.document.path,
                message,
            })
        }
        RemoteLanguageCommand::Completions { request_id, params } => {
            Some(RemoteLanguageEvent::RequestFailed {
                request_id,
                kind: LanguageRequestKind::Completion,
                path: params.document.path,
                message,
            })
        }
        RemoteLanguageCommand::Locations { request_id, params } => {
            Some(RemoteLanguageEvent::RequestFailed {
                request_id,
                kind: request_kind(params.kind),
                path: params.document.path,
                message,
            })
        }
        RemoteLanguageCommand::Shutdown => None,
    }
}

fn request_kind(kind: LanguageLocationKindDto) -> LanguageRequestKind {
    match kind {
        LanguageLocationKindDto::Declaration => LanguageRequestKind::Declaration,
        LanguageLocationKindDto::Definition => LanguageRequestKind::Definition,
        LanguageLocationKindDto::Implementation => LanguageRequestKind::Implementation,
        LanguageLocationKindDto::TypeDefinition => LanguageRequestKind::TypeDefinition,
        LanguageLocationKindDto::References => LanguageRequestKind::References,
    }
}

fn reconnect_delay_within_window(elapsed: Duration, attempt: usize) -> Option<Duration> {
    let remaining = RECONNECT_WINDOW.checked_sub(elapsed)?;
    let delay = reconnect_delay(attempt);
    (delay <= remaining).then_some(delay)
}

fn reconnect_delay(attempt: usize) -> Duration {
    let multiplier = 1_u32 << (attempt.min(31) as u32);
    (INITIAL_RECONNECT_DELAY * multiplier).min(MAX_RECONNECT_DELAY)
}

fn send_event(event_proxy: &AppProxy<ProductEvent>, event: RemoteLanguageEvent) -> Result<()> {
    event_proxy
        .send_event(ProductEvent::RemoteLanguage(event))
        .map_err(|_| ProductEventLoopUnavailable.into())
}

#[cfg(test)]
#[path = "language_service_remote_session_tests.rs"]
mod tests;
