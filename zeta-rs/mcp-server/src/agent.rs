mod invocation;
mod outcome;
mod progress;
mod service;

use crate::events::{AgentEvents, AgentProgress, InteractionResolution};
use crate::receipt::ReceiptStore;
pub(crate) use invocation::InvocationFingerprint;
#[cfg(test)]
pub(crate) use invocation::start_fingerprint;
use outcome::{terminal_outcome, waiting_outcome};
use progress::{TurnUpdate, project};
use serde::{Deserialize, Serialize};
use service::{command_id, interaction_command_id};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use zeta_app_server_client::{AppServerClient, JsonRpcTransport};
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionRequest, SessionRequestParams, SessionRequestResult,
    SessionThreadReadParams, SessionThreadResult, SessionThreadSubscribeParams,
};
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_protocol::{
    AgentRequestEnvelope, CommandId, SessionId, ThreadId, ThreadUpdateEnvelope, TurnId,
};

const CANCELLATION_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeLimits {
    pub(crate) default_turn_timeout: Duration,
    pub(crate) maximum_turn_timeout: Duration,
    pub(crate) poll_interval: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StartAgentRequest {
    pub(crate) invocation_id: String,
    pub(crate) prompt: String,
    pub(crate) timeout: Option<Duration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ReplyAgentRequest {
    pub(crate) invocation_id: String,
    pub(crate) thread_id: String,
    pub(crate) prompt: String,
    pub(crate) timeout: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AgentOutcomeStatus {
    Completed,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Failed,
    Interrupted,
    OutcomeUnknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentOutcome {
    pub(crate) invocation_id: String,
    pub(crate) session_id: SessionId,
    pub(crate) thread_id: ThreadId,
    pub(crate) turn_id: TurnId,
    pub(crate) status: AgentOutcomeStatus,
    pub(crate) content: String,
}

impl AgentOutcome {
    pub(crate) fn is_error(&self) -> bool {
        self.status != AgentOutcomeStatus::Completed
    }

    pub(crate) fn is_terminal(&self) -> bool {
        !matches!(
            self.status,
            AgentOutcomeStatus::WaitingForApproval
                | AgentOutcomeStatus::WaitingForUserInput
                | AgentOutcomeStatus::WaitingForCapability
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AgentCallError {
    InvalidInput(String),
    InvocationConflict,
    InvocationInProgress,
    CancelledBeforeStart,
    ThreadNotOwned,
    AppServer(String),
}

impl fmt::Display for AgentCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::InvocationConflict => {
                formatter.write_str("invocation ID was already used with different arguments")
            }
            Self::InvocationInProgress => {
                formatter.write_str("invocation with this ID is already running")
            }
            Self::CancelledBeforeStart => {
                formatter.write_str("invocation was cancelled before its Turn started")
            }
            Self::ThreadNotOwned => {
                formatter.write_str("thread is not authorized for this MCP principal")
            }
            Self::AppServer(message) => write!(formatter, "App Server error: {message}"),
        }
    }
}

/// Executes Agent turns while preserving App Server ownership of all product state.
///
/// Implementations must treat `invocation_id` as an idempotency identity, observe cancellation
/// without assuming transport disconnect completed a Turn, and return only bounded public output.
pub(crate) trait AgentService: Send + Sync {
    fn start(
        &self,
        request: StartAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError>;

    fn reply(
        &self,
        request: ReplyAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError>;
}

pub(crate) struct AppServerAgentService<T> {
    client: Mutex<AppServerClient<T>>,
    receipts: Arc<ReceiptStore>,
    principal: String,
    limits: RuntimeLimits,
}

struct TurnCall<'a> {
    invocation_id: &'a str,
    session_id: &'a SessionId,
    thread_id: &'a ThreadId,
    prompt: &'a str,
    timeout: Duration,
    cancellation: &'a AtomicBool,
    events: &'a dyn AgentEvents,
}

struct TurnWait<'a> {
    invocation_id: &'a str,
    session_id: &'a SessionId,
    thread_id: &'a ThreadId,
    turn_id: &'a TurnId,
    timeout: Duration,
    cancellation: &'a AtomicBool,
    events: &'a dyn AgentEvents,
}

fn expect_thread_result(
    result: SessionRequestResult,
) -> Result<SessionThreadResult, AgentCallError> {
    match result {
        SessionRequestResult::Thread(result) => Ok(result),
        other => Err(AgentCallError::AppServer(format!(
            "session request returned {other:?} for CreateThread"
        ))),
    }
}

fn expect_turn_result(
    result: SessionRequestResult,
) -> Result<zeta_app_server_protocol::protocol::turn::TurnStartResult, AgentCallError> {
    match result {
        SessionRequestResult::Turn(result) => Ok(result),
        other => Err(AgentCallError::AppServer(format!(
            "session request returned {other:?} for StartTurn"
        ))),
    }
}

fn expect_interaction_result(
    result: SessionRequestResult,
) -> Result<zeta_app_server_protocol::protocol::turn::TurnInteractionResolveResult, AgentCallError>
{
    match result {
        SessionRequestResult::Interaction(result) => Ok(result),
        other => Err(AgentCallError::AppServer(format!(
            "session request returned {other:?} for ResolveInteraction"
        ))),
    }
}

impl<T: JsonRpcTransport + Send> AppServerAgentService<T> {
    #[cfg(test)]
    pub(crate) fn new(client: AppServerClient<T>, limits: RuntimeLimits) -> Self {
        Self::with_receipts(
            client,
            limits,
            Arc::new(ReceiptStore::memory()),
            "test".into(),
        )
    }

    pub(crate) fn with_receipts(
        client: AppServerClient<T>,
        limits: RuntimeLimits,
        receipts: Arc<ReceiptStore>,
        principal: String,
    ) -> Self {
        Self {
            client: Mutex::new(client),
            receipts,
            principal,
            limits,
        }
    }

    fn start_inner(
        &self,
        request: &StartAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        if cancellation.load(Ordering::Acquire) {
            return Err(AgentCallError::CancelledBeforeStart);
        }
        let timeout = self.effective_timeout(request.timeout)?;
        let session_command = command_id(&self.principal, &request.invocation_id, "session")?;
        let session = self.with_client(|client| {
            client.create_session(SessionCreateParams {
                command_id: session_command,
                title: format!("MCP invocation {}", request.invocation_id),
            })
        })?;
        let session_id = session.session.session_id.clone();
        let thread_command = command_id(&self.principal, &request.invocation_id, "thread")?;
        let thread = self
            .with_client(|client| {
                client.request_session(SessionRequestParams {
                    command_id: thread_command,
                    session_id: session_id.clone(),
                    expected_sequence: session.session.sequence,
                    request: SessionRequest::CreateThread {
                        title: "MCP Agent".into(),
                    },
                })
            })
            .and_then(expect_thread_result)?;
        self.receipts.bind_thread(
            &self.principal,
            thread.thread_id.clone(),
            session_id.clone(),
        )?;
        self.start_and_wait(TurnCall {
            invocation_id: &request.invocation_id,
            session_id: &session_id,
            thread_id: &thread.thread_id,
            prompt: &request.prompt,
            timeout,
            cancellation,
            events,
        })
    }

    fn reply_inner(
        &self,
        request: &ReplyAgentRequest,
        cancellation: &AtomicBool,
        events: &dyn AgentEvents,
    ) -> Result<AgentOutcome, AgentCallError> {
        if cancellation.load(Ordering::Acquire) {
            return Err(AgentCallError::CancelledBeforeStart);
        }
        let timeout = self.effective_timeout(request.timeout)?;
        let thread_id = ThreadId::new(request.thread_id.clone())
            .map_err(|error| AgentCallError::InvalidInput(error.to_string()))?;
        let session_id = self
            .receipts
            .session_for_thread(&self.principal, &thread_id)?
            .ok_or(AgentCallError::ThreadNotOwned)?;
        self.start_and_wait(TurnCall {
            invocation_id: &request.invocation_id,
            session_id: &session_id,
            thread_id: &thread_id,
            prompt: &request.prompt,
            timeout,
            cancellation,
            events,
        })
    }

    fn start_and_wait(&self, call: TurnCall<'_>) -> Result<AgentOutcome, AgentCallError> {
        let TurnCall {
            invocation_id,
            session_id,
            thread_id,
            prompt,
            timeout,
            cancellation,
            events,
        } = call;
        let before = self.read_thread(session_id, thread_id)?;
        let subscription = self.with_client(|client| {
            client.subscribe_session_thread(SessionThreadSubscribeParams {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                after_sequence: 0,
                history: None,
            })
        })?;
        let turn_command = command_id(&self.principal, invocation_id, "turn")?;
        let started = self
            .with_client(|client| {
                client.request_session(SessionRequestParams {
                    command_id: turn_command,
                    session_id: session_id.clone(),
                    expected_sequence: before.thread.sequence,
                    request: SessionRequest::StartTurn {
                        thread_id: thread_id.clone(),
                        tool_mode: None,
                        input: vec![InputItem::Text {
                            text: prompt.to_string(),
                        }],
                    },
                })
            })
            .and_then(expect_turn_result)?;
        self.wait_for_turn(
            TurnWait {
                invocation_id,
                session_id,
                thread_id,
                turn_id: &started.turn_id,
                timeout,
                cancellation,
                events,
            },
            subscription.updates,
        )
    }

    fn wait_for_turn(
        &self,
        wait: TurnWait<'_>,
        mut updates: Vec<ThreadUpdateEnvelope>,
    ) -> Result<AgentOutcome, AgentCallError> {
        let TurnWait {
            invocation_id,
            session_id,
            thread_id,
            turn_id,
            timeout,
            cancellation,
            events,
        } = wait;
        let deadline = Instant::now() + timeout;
        let mut cancellation_deadline = None;
        loop {
            let (snapshot, notifications) = self.read_thread_and_updates(session_id, thread_id)?;
            updates.extend(notifications);
            let turn = snapshot
                .thread
                .turns
                .iter()
                .find(|turn| &turn.turn_id == turn_id)
                .ok_or_else(|| AgentCallError::AppServer("started Turn was not readable".into()))?;
            let mut interaction_resolved = false;
            for update in updates.drain(..) {
                match project(&update, turn_id) {
                    Some(TurnUpdate::Progress(message)) => {
                        events.progress(AgentProgress { message });
                    }
                    Some(TurnUpdate::Interaction(interaction))
                        if turn.pending_interaction.as_ref().is_some_and(|pending| {
                            pending.request_id == interaction.request_id
                        }) =>
                    {
                        let envelope = AgentRequestEnvelope {
                            session_id: session_id.clone(),
                            thread_id: thread_id.clone(),
                            turn_id: turn_id.clone(),
                            interaction: interaction.as_ref().clone(),
                        };
                        if let InteractionResolution::Respond(response) =
                            events.resolve_interaction(&envelope)
                        {
                            let expected_sequence =
                                self.read_thread(session_id, thread_id)?.thread.sequence;
                            let interaction_command = interaction_command_id(
                                &self.principal,
                                invocation_id,
                                &interaction.request_id,
                            )?;
                            self.with_client(|client| {
                                client.request_session(SessionRequestParams {
                                    command_id: interaction_command,
                                    session_id: session_id.clone(),
                                    expected_sequence,
                                    request: SessionRequest::ResolveInteraction {
                                        thread_id: thread_id.clone(),
                                        turn_id: turn_id.clone(),
                                        request_id: interaction.request_id,
                                        response,
                                    },
                                })
                            })
                            .and_then(expect_interaction_result)?;
                            interaction_resolved = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if interaction_resolved {
                continue;
            }
            if let Some(outcome) = terminal_outcome(invocation_id, session_id, thread_id, turn) {
                return Ok(outcome);
            }
            if let Some(outcome) = waiting_outcome(invocation_id, session_id, thread_id, turn) {
                return Ok(outcome);
            }

            let should_cancel = cancellation.load(Ordering::Acquire) || Instant::now() >= deadline;
            if should_cancel && cancellation_deadline.is_none() {
                let cancel_command = command_id(&self.principal, invocation_id, "cancel")?;
                let _ = self.with_client(|client| {
                    client.request_session(SessionRequestParams {
                        command_id: cancel_command,
                        session_id: session_id.clone(),
                        expected_sequence: snapshot.thread.sequence,
                        request: SessionRequest::InterruptTurn {
                            thread_id: thread_id.clone(),
                            turn_id: turn_id.clone(),
                        },
                    })
                });
                cancellation_deadline = Some(Instant::now() + CANCELLATION_GRACE);
            }
            if cancellation_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Ok(AgentOutcome {
                    invocation_id: invocation_id.into(),
                    session_id: session_id.clone(),
                    thread_id: thread_id.clone(),
                    turn_id: turn_id.clone(),
                    status: AgentOutcomeStatus::OutcomeUnknown,
                    content: "Turn cancellation did not reach a terminal state before the grace period elapsed".into(),
                });
            }
            thread::sleep(self.limits.poll_interval);
        }
    }

    fn read_thread(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<zeta_app_server_protocol::protocol::session::SessionThreadReadResult, AgentCallError>
    {
        self.with_client(|client| {
            client.read_session_thread(SessionThreadReadParams {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                history: None,
            })
        })
    }

    fn read_thread_and_updates(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<
        (
            zeta_app_server_protocol::protocol::session::SessionThreadReadResult,
            Vec<ThreadUpdateEnvelope>,
        ),
        AgentCallError,
    > {
        let mut client = self
            .client
            .lock()
            .map_err(|_| lock_error("App Server client"))?;
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                history: None,
            })
            .map_err(|error| AgentCallError::AppServer(error.to_string()))?;
        let updates = client
            .drain_notifications()
            .map_err(|error| AgentCallError::AppServer(error.to_string()))?
            .into_iter()
            .filter_map(|notification| match notification {
                zeta_app_server_client::ServerNotification::SessionThreadUpdate(update)
                    if &update.thread_id == thread_id =>
                {
                    Some(*update)
                }
                _ => None,
            })
            .collect();
        Ok((snapshot, updates))
    }

    fn effective_timeout(&self, requested: Option<Duration>) -> Result<Duration, AgentCallError> {
        let timeout = requested.unwrap_or(self.limits.default_turn_timeout);
        if timeout.is_zero() {
            return Err(AgentCallError::InvalidInput(
                "timeout must be greater than zero".into(),
            ));
        }
        if timeout > self.limits.maximum_turn_timeout {
            return Err(AgentCallError::InvalidInput(format!(
                "timeout exceeds the maximum of {} milliseconds",
                self.limits.maximum_turn_timeout.as_millis()
            )));
        }
        Ok(timeout)
    }

    fn with_client<R>(
        &self,
        operation: impl FnOnce(
            &mut AppServerClient<T>,
        ) -> Result<R, zeta_app_server_client::ClientError>,
    ) -> Result<R, AgentCallError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| lock_error("App Server client"))?;
        operation(&mut client).map_err(|error| AgentCallError::AppServer(error.to_string()))
    }
}

fn lock_error(name: &str) -> AgentCallError {
    AgentCallError::AppServer(format!("{name} lock poisoned"))
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
