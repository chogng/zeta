//! Durable domain model for Git repository changes attributed to one Turn.

mod ledger;
mod model;
mod store;
mod watcher;

pub use model::CaptureState;
pub use model::ChangeFile;
pub use model::ChangeFileKind;
pub use model::ChangeSetId;
pub use model::CommitState;
pub use model::MessageState;
pub use model::TerminalTurnState;
pub use model::TurnChangeError;
pub use model::TurnChangeSet;
pub use model::TurnChangeSetDraft;
pub use store::TurnChangeStore;
pub use store::TurnChangeStoreError;

#[cfg(test)]
#[path = "turn_changes_tests.rs"]
mod tests;
pub use ledger::RepositoryCaptureTarget;
pub use ledger::ToolChangeScope;
pub use ledger::TurnChangeBeginRequest;
pub use ledger::TurnChangeLedger;
pub use ledger::TurnChangeLedgerError;
pub use ledger::TurnChangeSealRequest;
pub use watcher::GitTurnChangeWatcher;
pub use watcher::WriteLifecycleTracker;
