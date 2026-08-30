use crate::components::detail_list::DetailList;
use crate::components::list_selection::ListSelectionModel;
use crate::components::pane::PaneSpec;
use crate::components::steer::SteerId;
use crate::features::approval::Approval;
use crate::features::config::ConfigPaneSpec;
use crate::features::config::TerminalSettings;
use crate::features::connectors::ConnectorPaneSpec;
use crate::features::dirs::DirPaneSpec;
use crate::features::keymap::KeymapPaneSpec;
use crate::features::mcp::McpPaneSpec;
use crate::features::models::ModelPaneSpec;
use crate::features::query::Query;
use crate::features::queue::QueueId;
use crate::features::rewind::RewindPaneSpec;
use crate::features::sessions::SessionPaneSpec;
use crate::features::skills::SkillPaneSpec;
use crate::features::status_line::StatusLinePaneSpec;
use crate::features::status_line::StatusLineSettings;
use crate::features::theme::ThemePaneSpec;
use crate::features::thread::ThreadRequestIdentity;
use crate::features::thread::TurnActivity;
use crate::render::RenderTheme;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
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
    DirsPaneOpened(DirPaneSpec),
    DirRemoved {
        path: std::path::PathBuf,
        pane_spec: DirPaneSpec,
    },
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted {
        command: String,
        result: String,
    },
    ConfigSettingsReceived(TerminalSettings),
    ConfigPaneOpened(ConfigPaneSpec),
    ConfigPaneReplaced(ConfigPaneSpec),
    ConfigApiKeySaved {
        provider: String,
        pane_spec: ConfigPaneSpec,
    },
    ConnectorPaneOpened(ConnectorPaneSpec),
    ConnectorPaneReplaced(ConnectorPaneSpec),
    PreferredModelReceived(Option<ModelRefDto>),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    GitStatusReceived(GitStatusResult),
    HostOperationCompleted(Result<String, String>),
    StatusNoticeShown(String),
    InterruptFailed(String),
    ProductNotice(String),
    ApprovalRequested(Approval),
    QueryRequested(Query),
    ThreadRequestResolved(ThreadRequestIdentity),
    ThreadRequestSubmissionFailed {
        request: ThreadRequestIdentity,
        error: String,
    },
    KeymapPaneOpened(KeymapPaneSpec),
    KeymapPanesClosed,
    StatusLineSettingsReceived(StatusLineSettings),
    StatusLinePaneOpened(StatusLinePaneSpec),
    StatusLinePaneReplaced(StatusLinePaneSpec),
    McpPaneOpened(McpPaneSpec),
    McpPaneReplaced(McpPaneSpec),
    ModelPaneOpened(ModelPaneSpec),
    RewindPaneOpened(RewindPaneSpec),
    SessionPaneOpened(SessionPaneSpec),
    SessionCatalogReceived(Vec<Session>),
    ThreadContextChanged {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    ThreadGoalChanged(Option<ThreadGoal>),
    StatusQuickViewOpened(PaneSpec<DetailList>),
    ListSelectionPaneClosed,
    ListSelectionPaneOpened(PaneSpec<ListSelectionModel>),
    SkillsPaneOpened(SkillPaneSpec),
    SkillsPaneReplaced(SkillPaneSpec),
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
    ThemePanesClosed,
    ThemePaneOpened(ThemePaneSpec),
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
