use super::LatestThreadSnapshot;
use super::OlderThreadHistoryPage;
use super::ThreadRequestIdentity;
use super::ThreadRequestResponse;
use super::ThreadRequestScope;
use super::composer::ChatSubmission;
use super::composer::SteerId;
use super::composer::SteerSource;
use super::interrupt_turn;
use super::queue::QueueId;
use super::read_thread_history;
use super::resolve_interaction;
use super::steer_prompt;
use super::submit_prompt;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_app_server_protocol::protocol::turn::TurnSteerResult;
use zeta_protocol::ApprovalMode;
use zeta_protocol::TurnId;

pub(crate) enum TurnStartCompletion {
    Rejected(ClientError),
    Accepted {
        start: TurnStartResult,
        snapshot: Box<Result<LatestThreadSnapshot, ClientError>>,
    },
}

/// Result of one asynchronous operation against the active Thread.
pub(crate) enum ThreadCompletion {
    RequestResolved {
        scope: ThreadRequestScope,
        request: ThreadRequestIdentity,
        result: Result<LatestThreadSnapshot, ClientError>,
    },
    Refreshed {
        scope: ThreadRequestScope,
        result: Result<LatestThreadSnapshot, ClientError>,
    },
    HistoryPage {
        scope: ThreadRequestScope,
        result: Result<OlderThreadHistoryPage, ClientError>,
    },
    Interrupted {
        scope: ThreadRequestScope,
        result: Result<LatestThreadSnapshot, ClientError>,
    },
    Steered {
        scope: ThreadRequestScope,
        source: SteerSource,
        steer_id: SteerId,
        result: Result<(TurnSteerResult, LatestThreadSnapshot), ClientError>,
    },
    Started {
        scope: ThreadRequestScope,
        result: TurnStartCompletion,
    },
    QueuedTurnStarted {
        scope: ThreadRequestScope,
        queue_id: QueueId,
        result: TurnStartCompletion,
    },
}

impl ThreadCompletion {
    pub(crate) fn scope(&self) -> &ThreadRequestScope {
        match self {
            Self::RequestResolved { scope, .. }
            | Self::Refreshed { scope, .. }
            | Self::HistoryPage { scope, .. }
            | Self::Interrupted { scope, .. }
            | Self::Steered { scope, .. }
            | Self::Started { scope, .. }
            | Self::QueuedTurnStarted { scope, .. } => scope,
        }
    }
}

pub(crate) fn resolve_request_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    response: ThreadRequestResponse,
    history: ThreadSnapshotHistory,
) -> Result<LatestThreadSnapshot, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    resolve_interaction(
        &mut client,
        scope,
        response.turn_id,
        response.request_id,
        response.response,
    )?;
    read_thread_history(&mut client, &session_id, &thread_id, history)
}

pub(crate) fn interrupt_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    turn_id: TurnId,
    history: ThreadSnapshotHistory,
) -> Result<LatestThreadSnapshot, ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    interrupt_turn(&mut client, scope, &turn_id)?;
    read_thread_history(&mut client, &session_id, &thread_id, history)
}

pub(crate) fn start_turn_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    submission: ChatSubmission,
    approval_mode: ApprovalMode,
    history: ThreadSnapshotHistory,
) -> TurnStartCompletion {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    let start = match submit_prompt(&mut client, scope, submission, approval_mode) {
        Ok(start) => start,
        Err(error) => return TurnStartCompletion::Rejected(error),
    };
    let snapshot = read_thread_history(&mut client, &session_id, &thread_id, history);
    TurnStartCompletion::Accepted {
        start,
        snapshot: Box::new(snapshot),
    }
}

pub(crate) fn steer_turn_and_read(
    mut client: AppServerRequestHandle,
    scope: ThreadRequestScope,
    turn_id: TurnId,
    submission: ChatSubmission,
    history: ThreadSnapshotHistory,
) -> Result<(TurnSteerResult, LatestThreadSnapshot), ClientError> {
    let session_id = scope.session_id().clone();
    let thread_id = scope.thread_id().clone();
    let steer = steer_prompt(&mut client, scope, turn_id, submission)?;
    let snapshot = read_thread_history(&mut client, &session_id, &thread_id, history)?;
    Ok((steer, snapshot))
}
