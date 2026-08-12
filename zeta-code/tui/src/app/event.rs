use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionViewModel;
use crate::features::interactions::InteractionSelectionView;
use crate::features::mcp::McpSelectionView;
use crate::features::models::ModelSelectionView;
use crate::features::rewind::RewindSelectionView;
use crate::features::sessions::SessionSelectionView;
use crate::features::sessions::ThreadSelectionView;
use crate::features::skills::SkillSelectionView;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::TurnActivity;
use crate::features::workspace_files::FileSelectionView;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_app_server_protocol::protocol::git::GitStatusResult;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::Thread;
use zeta_protocol::ThreadUpdateEnvelope;

/// A fact delivered to the single writer of TUI presentation state.
pub(crate) enum AppEvent {
    ClipboardImageRead(Result<Vec<u8>, String>),
    CommandStarted(String),
    CommandCompleted { command: String, result: String },
    ConfigSnapshotReceived(ConfigReadResult),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    FileViewOpened(FileSelectionView),
    GitStatusReceived(GitStatusResult),
    HostOperationCompleted(Result<String, String>),
    InterruptFailed(String),
    ProductNotice(String),
    InteractionViewOpened(InteractionSelectionView),
    McpViewOpened(McpSelectionView),
    McpViewReplaced(McpSelectionView),
    ModelViewOpened(ModelSelectionView),
    RewindViewOpened(RewindSelectionView),
    SessionViewOpened(SessionSelectionView),
    ThreadViewOpened(ThreadSelectionView),
    SelectionViewClosed,
    SelectionViewOpened(PaneViewModel<SelectionViewModel>),
    SkillsViewOpened(SkillSelectionView),
    SkillsViewReplaced(SkillSelectionView),
    ThemeViewClosed,
    ThemeViewOpened(ThemeSelectionView),
    ThreadSnapshotReceived(Thread),
    ThreadHistoryPageReceived(Thread),
    TransientThreadStreamReset,
    TransientThreadUpdateReceived(Box<ThreadUpdateEnvelope>),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnCompleted,
    TurnInterrupted,
}
