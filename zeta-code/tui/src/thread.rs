mod agent_switcher;
mod completion;
pub(crate) mod composer;
pub(crate) mod goal;
pub(crate) mod interaction;
pub(crate) mod plan;
mod presentation;
mod presentation_store;
pub(crate) mod queue;
mod request;
pub(crate) mod rewind;
mod state;
mod subscription;
pub(crate) mod transcript;
mod update;

pub(crate) use agent_switcher::AgentThreadSwitcher;
pub(crate) use agent_switcher::AgentThreadSwitcherView;
pub(crate) use agent_switcher::draw_agent_thread_switcher;
pub(crate) use completion::CommandActivity;
pub(crate) use completion::CommandPreparation;
pub(crate) use completion::CommandState;
pub(crate) use completion::ThreadCompletion;
pub(crate) use completion::TurnStartCompletion;
pub(crate) use completion::prepare_command;
pub(crate) use completion::start_turn_and_read;
pub(crate) use presentation::ActiveTurnUpdate;
pub(crate) use presentation::TurnActivity;
#[cfg(test)]
pub(crate) use presentation::present_turn_error;
pub(crate) use presentation_store::ThreadPresentationStore;
pub(crate) use request::LatestThreadSnapshot;
pub(crate) use request::OlderThreadHistoryPage;
pub(crate) use request::ThreadRequestIdentity;
pub(crate) use request::ThreadRequestKind;
pub(crate) use request::ThreadRequestResponse;
pub(crate) use request::ThreadRequestScope;
pub(crate) use request::interrupt_turn;
pub(crate) use request::read_older_thread_history;
#[cfg(test)]
pub(crate) use request::read_thread;
pub(crate) use request::read_thread_history;
pub(crate) use request::resolve_interaction;
pub(crate) use request::steer_prompt;
pub(crate) use request::submit_prompt;
pub(crate) use state::ThreadState;
pub(crate) use state::TurnApprovalModes;
pub(crate) use subscription::ThreadSubscription;
pub(crate) use subscription::ThreadSwitch;
pub(crate) use subscription::ThreadUpdateDisposition;
pub(crate) use subscription::TranscriptUpdateDisposition;
pub(crate) use transcript::TranscriptCellId;
pub(crate) use update::ThreadPresentationEvent;

/// A completed current-Thread operation delivered to the TUI state owner.
pub(crate) enum Event {
    CommandStarted(String),
    CommandCompleted {
        command: String,
        result: String,
    },
    FailureReported(String),
    ProductNotice(String),
    FileSearchSnapshotReceived(zeta_file_search::PathSearchSnapshot),
    InterruptFailed(String),
    ApprovalRequested(interaction::approval::Approval),
    QueryRequested(interaction::query::Query),
    RewindPickerOpened(rewind::RewindChoices),
    RequestResolved(ThreadRequestIdentity),
    RequestSubmissionFailed {
        request: ThreadRequestIdentity,
        error: String,
    },
    ContextChanged {
        session_id: zeta_protocol::SessionId,
        thread_id: zeta_protocol::ThreadId,
    },
    AccountingChanged {
        usage: zeta_protocol::ModelUsageSummary,
        reference_cost: zeta_protocol::ModelReferenceCostSummary,
    },
    GoalChanged(Option<zeta_protocol::ThreadGoal>),
    SteerCompleted {
        source: composer::SteerSource,
        steer_id: composer::SteerId,
    },
    SteerSubmissionFailed {
        source: composer::SteerSource,
        steer_id: composer::SteerId,
        error: String,
    },
    QueueSubmissionCompleted(queue::QueueId),
    QueueSubmissionFailed {
        queue_id: queue::QueueId,
        error: String,
    },
    TranscriptSnapshotReceived(
        zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot,
    ),
    TranscriptHistoryPageReceived(
        zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot,
    ),
    TranscriptUpdateReceived(
        Box<zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope>,
    ),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnPlanChanged(Option<zeta_protocol::PlanUpdate>),
    PendingInteractionChanged(Option<(zeta_protocol::TurnId, zeta_protocol::RequestId)>),
    TurnCompleted,
    TurnInterrupted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    ExecuteProductCommand(composer::SlashCommandInvocation),
    Interrupt,
    LoadOlderHistory,
    OpenRewindPicker,
    RewindToCheckpoint {
        before_turn_id: zeta_protocol::TurnId,
        checkpoint_label: String,
    },
    ResolveRequest(ThreadRequestResponse),
    CycleNextApprovalMode,
    SubmitTurn {
        submission: composer::ChatSubmission,
    },
    SubmitQueuedTurn {
        queue_id: queue::QueueId,
        submission: composer::ChatSubmission,
    },
    SteerTurn {
        source: composer::SteerSource,
        steer_id: composer::SteerId,
        submission: composer::ChatSubmission,
    },
}
