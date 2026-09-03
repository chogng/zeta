use crate::config::ConfigChoices;
use crate::config::ConfigEditResult;
use crate::config::TerminalSettings;
use crate::connectors::ConnectorChoices;
use crate::dirs::DirChoices;
use crate::keymap::KeymapEditorUpdate;
use crate::keymap::KeymapSettings;
use crate::mcp::McpChoices;
use crate::models::ModelChoices;
use crate::models::ModelSummary;
use crate::render::RenderTheme;
use crate::sessions::SessionChoices;
use crate::skills::SkillChoices;
use crate::status::StatusLineEditorUpdate;
use crate::status::StatusLineSettings;
use crate::status::StatusPanel;
use crate::theme::ThemeChoices;
use crate::thread::ThreadRequestIdentity;
use crate::thread::TurnActivity;
use crate::thread::composer::SteerId;
use crate::thread::interaction::approval::Approval;
use crate::thread::interaction::query::Query;
use crate::thread::queue::QueueId;
use crate::thread::rewind::RewindChoices;
#[cfg(test)]
use crate::widgets::list_selection::ListSelectionModel;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_app_server_protocol::protocol::skills::SkillDiagnosticDto;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::ModelReferenceCostSummary;
use zeta_protocol::ModelUsageSummary;
use zeta_protocol::PlanUpdate;
use zeta_protocol::RequestId;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadGoal;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

/// A fact delivered to the single writer of TUI presentation state.
pub(crate) enum AppEvent {
    DirPickerOpened(DirChoices),
    DirRemoved {
        path: std::path::PathBuf,
        choices: DirChoices,
    },
    DirPermissionsUpdated(DirChoices),
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted {
        command: String,
        result: String,
    },
    ConfigSettingsReceived(TerminalSettings),
    ConfigUpdated(ConfigEditResult),
    ConfigEditorOpened(ConfigChoices),
    ConfigApiKeySaved {
        provider: String,
        choices: ConfigChoices,
    },
    ConnectorPickerOpened(ConnectorChoices),
    ConnectorPickerUpdated(ConnectorChoices),
    ModelSummaryReceived(ModelSummary),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    GitStatusReceived(GitStatusResult),
    HostOperationCompleted(Result<String, String>),
    TopTipNoticeShown(String),
    InterruptFailed(String),
    ProductNotice(String),
    ApprovalRequested(Approval),
    QueryRequested(Query),
    ThreadRequestResolved(ThreadRequestIdentity),
    ThreadRequestSubmissionFailed {
        request: ThreadRequestIdentity,
        error: String,
    },
    KeymapSettingsReceived(KeymapSettings),
    KeymapEditorOpened(KeymapEditorUpdate),
    StatusLineSettingsReceived(StatusLineSettings),
    StatusLineEditorOpened(StatusLineEditorUpdate),
    StatusLineEditorUpdated(StatusLineEditorUpdate),
    McpSettingsOpened(McpChoices),
    McpSettingsUpdated(McpChoices),
    ModelPickerOpened(ModelChoices),
    RewindPickerOpened(RewindChoices),
    SessionPickerOpened(SessionChoices),
    SessionCatalogReceived(Vec<Session>),
    ThreadContextChanged {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    ThreadAccountingChanged {
        usage: ModelUsageSummary,
        reference_cost: ModelReferenceCostSummary,
    },
    ThreadGoalChanged(Option<ThreadGoal>),
    StatusPanelOpened(StatusPanel),
    ComposerSlotClosed,
    #[cfg(test)]
    HelpOpened(ListSelectionModel),
    SkillSettingsOpened(SkillChoices),
    SkillSettingsUpdated(SkillChoices),
    SkillDiagnosticsReceived(Vec<SkillDiagnosticDto>),
    SteerCompleted(SteerId),
    SteerSubmissionFailed {
        steer_id: SteerId,
        error: String,
    },
    QueueSubmissionCompleted(QueueId),
    QueueSubmissionFailed {
        queue_id: QueueId,
        error: String,
    },
    ThemePickerOpened(ThemeChoices),
    RenderThemeChanged(RenderTheme),
    ThreadTranscriptSnapshotReceived(ThreadTranscriptSnapshot),
    ThreadTranscriptHistoryPageReceived(ThreadTranscriptSnapshot),
    ThreadTranscriptUpdateReceived(Box<ThreadTranscriptUpdateEnvelope>),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnPlanChanged(Option<PlanUpdate>),
    PendingInteractionChanged(Option<(TurnId, RequestId)>),
    TurnCompleted,
    TurnInterrupted,
}
