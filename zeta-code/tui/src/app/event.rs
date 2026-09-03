use crate::components::detail_list::DetailList;
#[cfg(test)]
use crate::components::list_selection::ListSelectionModel;
use crate::components::steer::SteerId;
use crate::features::approval::Approval;
use crate::features::config::ConfigChoices;
use crate::features::config::ConfigEditResult;
use crate::features::config::TerminalSettings;
use crate::features::connectors::ConnectorChoices;
use crate::features::dirs::DirChoices;
use crate::features::keymap::KeymapEditorUpdate;
use crate::features::keymap::KeymapSettings;
use crate::features::mcp::McpChoices;
use crate::features::models::ModelChoices;
use crate::features::query::Query;
use crate::features::queue::QueueId;
use crate::features::rewind::RewindChoices;
use crate::features::sessions::SessionChoices;
use crate::features::skills::SkillChoices;
use crate::features::status_line::StatusLineEditorUpdate;
use crate::features::status_line::StatusLineSettings;
use crate::features::theme::ThemeChoices;
use crate::features::thread::ThreadRequestIdentity;
use crate::features::thread::TurnActivity;
use crate::render::RenderTheme;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_app_server_protocol::protocol::skills::SkillDiagnosticDto;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_file_search::PathSearchSnapshot;
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
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted {
        command: String,
        result: String,
    },
    ConfigSettingsReceived(TerminalSettings),
    ConfigUpdated(ConfigEditResult),
    ConfigEditorOpened(ConfigChoices),
    ConfigEditorUpdated(ConfigChoices),
    ConfigApiKeySaved {
        provider: String,
        choices: ConfigChoices,
    },
    ConnectorPickerOpened(ConnectorChoices),
    ConnectorPickerUpdated(ConnectorChoices),
    PreferredModelReceived(Option<ModelRefDto>),
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
    ThreadGoalChanged(Option<ThreadGoal>),
    StatusOverlayOpened(DetailList),
    ComposerModeClosed,
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
