use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionViewModel;
use crate::features::mcp::McpSelectionView;
use crate::features::models::ModelSelectionView;
use crate::features::rewind::RewindSelectionView;
use crate::features::sessions::SessionSelectionView;
use crate::features::skills::SkillSelectionView;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::TurnActivity;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::Thread;

/// A fact delivered to the single writer of TUI presentation state.
pub(crate) enum AppEvent {
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted { command: String, result: String },
    ConfigSnapshotReceived(ConfigReadResult),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    InterruptFailed(String),
    ProductNotice(String),
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
    ThreadSnapshotReceived(Thread),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnCompleted,
    TurnInterrupted,
}
