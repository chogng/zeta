use crate::components::selection::SelectionViewModel;
use crate::features::skills::SkillSelectionView;
use crate::features::thread::TurnActivity;
use zeta_app_server_protocol::protocol::config::ConfigReadResult;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::Thread;

/// A fact delivered to the single writer of TUI presentation state.
pub(crate) enum AppEvent {
    ClipboardImageRead(Result<Vec<u8>, String>),
    ConfigSnapshotReceived(ConfigReadResult),
    FailureReported(String),
    FileSearchSnapshotReceived(PathSearchSnapshot),
    InterruptFailed(String),
    ProductNotice(String),
    SelectionViewOpened(SelectionViewModel),
    SkillsViewOpened(SkillSelectionView),
    SkillsViewReplaced(SkillSelectionView),
    ThreadSnapshotReceived(Thread),
    TranscriptCleared,
    TurnActivityChanged(TurnActivity),
    TurnCompleted,
    TurnInterrupted,
}
