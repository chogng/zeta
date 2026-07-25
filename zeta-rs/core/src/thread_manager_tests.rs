use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use zeta_protocol::AgentEvent;

#[test]
fn completion_is_durable_before_snapshot_exposes_it() {
    let journal = Arc::new(InMemoryJournal::default());
    let threads = ThreadManager::with_journal(journal.clone());
    let thread = threads.start_thread("test").unwrap();
    let turn = threads.start_turn(&thread).unwrap();
    threads.complete_turn(&thread, &turn).unwrap();
    assert_eq!(
        threads.read_thread(&thread).unwrap().turns,
        vec![(turn, TurnStatus::Completed)]
    );
    assert_eq!(journal.events().last().unwrap().kind, "turn.completed");
}

#[test]
fn terminal_turn_cannot_restart() {
    assert!(
        TurnStatus::Completed
            .transition(TurnStatus::Running)
            .is_err()
    );
}

#[test]
fn item_and_tool_terminal_states_are_distinct() {
    assert_eq!(
        ItemStatus::Created
            .transition(ItemStatus::InProgress)
            .unwrap(),
        ItemStatus::InProgress
    );
    assert_eq!(
        ToolCallStatus::AwaitingApproval
            .transition(ToolCallStatus::Declined)
            .unwrap(),
        ToolCallStatus::Declined
    );
    assert!(
        ToolCallStatus::Declined
            .transition(ToolCallStatus::Running)
            .is_err()
    );
    assert!(
        ItemStatus::Cancelled
            .transition(ItemStatus::Completed)
            .is_err()
    );
}

struct ToggleJournal {
    reject_writes: AtomicBool,
}

impl EventJournal for ToggleJournal {
    fn append(&self, _: &AgentEvent) -> Result<(), CoreError> {
        if self.reject_writes.load(Ordering::Relaxed) {
            Err(CoreError::Journal("simulated write failure".into()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn failed_durable_write_does_not_update_turn_projection() {
    let journal = Arc::new(ToggleJournal {
        reject_writes: AtomicBool::new(false),
    });
    let threads = ThreadManager::with_journal(journal.clone());
    let thread = threads.start_thread("test").unwrap();
    let turn = threads.start_turn(&thread).unwrap();
    journal.reject_writes.store(true, Ordering::Relaxed);
    assert!(threads.complete_turn(&thread, &turn).is_err());
    assert_eq!(
        threads.read_thread(&thread).unwrap().turns,
        vec![(turn, TurnStatus::Running)]
    );
}

#[test]
fn recovery_interrupts_non_terminal_turns() {
    let journal = Arc::new(InMemoryJournal::default());
    let original = ThreadManager::with_journal(journal.clone());
    let thread = original.start_thread("recover me").unwrap();
    let turn = original.start_turn(&thread).unwrap();
    let recovered = ThreadManager::with_journal(journal.clone());
    let snapshot = recovered.recover_thread(journal.events()).unwrap();
    assert_eq!(snapshot.title, "recover me");
    assert_eq!(snapshot.turns, vec![(turn, TurnStatus::Interrupted)]);
    assert_eq!(journal.events().last().unwrap().kind, "turn.interrupted");
}
