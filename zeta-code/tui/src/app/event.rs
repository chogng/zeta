use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionViewModel;
use crate::features::additional_directories::AdditionalDirectorySelectionView;
use crate::features::config::ConfigSelectionView;
use crate::features::config::TerminalSettings;
use crate::features::connectors::ConnectorSelectionView;
use crate::features::interactions::InteractionSelectionView;
use crate::features::keymap::KeymapView;
use crate::features::mcp::McpSelectionView;
use crate::features::models::ModelSelectionView;
use crate::features::rewind::RewindSelectionView;
use crate::features::sessions::SessionSelectionView;
use crate::features::skills::SkillSelectionView;
use crate::features::status_line::StatusLineSelectionView;
use crate::features::status_line::StatusLineSettings;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::TurnActivity;
use zeta_app_server_protocol::protocol::config::ModelRefDto;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptSnapshot;
use zeta_app_server_protocol::protocol::transcript::ThreadTranscriptUpdateEnvelope;
use zeta_file_search::PathSearchSnapshot;

/// A fact delivered to the single writer of TUI presentation state.
pub(crate) enum AppEvent {
    AdditionalDirectoriesViewOpened(AdditionalDirectorySelectionView),
    AdditionalDirectoryRemoved {
        root: std::path::PathBuf,
        view: AdditionalDirectorySelectionView,
    },
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted {
        command: String,
        result: String,
    },
    ConfigSettingsReceived(TerminalSettings),
    ConfigViewOpened(ConfigSelectionView),
    ConfigViewReplaced(ConfigSelectionView),
    ConfigApiKeySaved {
        provider: String,
        view: ConfigSelectionView,
    },
    ConnectorViewOpened(ConnectorSelectionView),
    ConnectorViewReplaced(ConnectorSelectionView),
    PreferredModelReceived(Option<ModelRefDto>),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    GitStatusReceived(GitStatusResult),
    HostOperationCompleted(Result<String, String>),
    InterruptFailed(String),
    ProductNotice(String),
    InteractionViewOpened(InteractionSelectionView),
    KeymapViewOpened(KeymapView),
    KeymapViewsClosed,
    StatusLineSettingsReceived(StatusLineSettings),
    StatusLineViewOpened(StatusLineSelectionView),
    StatusLineViewReplaced(StatusLineSelectionView),
    McpViewOpened(McpSelectionView),
    McpViewReplaced(McpSelectionView),
    ModelViewOpened(ModelSelectionView),
    RewindViewOpened(RewindSelectionView),
    SessionViewOpened(SessionSelectionView),
    SelectionViewClosed,
    SelectionViewOpened(PaneViewModel<SelectionViewModel>),
    SkillsViewOpened(SkillSelectionView),
    SkillsViewReplaced(SkillSelectionView),
    ThemeViewClosed,
    ThemeViewOpened(ThemeSelectionView),
    ThreadTranscriptSnapshotReceived(ThreadTranscriptSnapshot),
    ThreadTranscriptHistoryPageReceived(ThreadTranscriptSnapshot),
    ThreadTranscriptUpdateReceived(Box<ThreadTranscriptUpdateEnvelope>),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnCompleted,
    TurnInterrupted,
}
