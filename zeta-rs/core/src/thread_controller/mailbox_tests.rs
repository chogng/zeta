use super::*;
use crate::{CreateThreadRequest, InMemoryThreadStore, ThreadController, ThreadSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use zeta_protocol::{SessionId, ThreadId, TurnId};

#[test]
fn idle_lane_evicts_projection_and_a_later_load_gets_a_new_incarnation() {
    let threads = ThreadController::with_store(Arc::new(InMemoryThreadStore::default()));
    let thread_id = ThreadId::new("thread").unwrap();
    threads
        .create_thread(CreateThreadRequest {
            session_id: SessionId::new("session").unwrap(),
            thread_id: thread_id.clone(),
            title: "test".into(),
        })
        .unwrap();
    let loaded_threads = threads.loaded_threads.clone();
    let first_incarnation = loaded_threads
        .current_incarnation(&thread_id)
        .unwrap()
        .unwrap();
    let mailboxes = ThreadExecutionMailboxes::with_settings(
        loaded_threads.clone(),
        NonZeroUsize::new(2).unwrap(),
        Duration::from_millis(10),
    );
    let (completed_tx, completed_rx) = mpsc::channel();

    mailboxes
        .enqueue(&thread_id, &TurnId::new("turn").unwrap(), move |_| {
            completed_tx.send(()).unwrap()
        })
        .unwrap();
    completed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_until(Duration::from_secs(1), || {
        loaded_threads
            .current_incarnation(&thread_id)
            .unwrap()
            .is_none()
    });

    let second_incarnation = loaded_threads
        .ensure_loaded_incarnation(&thread_id)
        .unwrap();
    assert_ne!(first_incarnation, second_incarnation);
    assert_eq!(threads.read_thread(&thread_id).unwrap().title, "test");
}

#[test]
fn queued_work_from_a_stale_incarnation_is_rejected() {
    let loaded_threads = Arc::new(LoadedThreads::new(Arc::new(InMemoryThreadStore::default())));
    let thread_id = ThreadId::new("thread").unwrap();
    install(&loaded_threads, snapshot(&thread_id));
    let mailboxes = ThreadExecutionMailboxes::with_settings(
        loaded_threads.clone(),
        NonZeroUsize::new(2).unwrap(),
        Duration::from_secs(1),
    );
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (stale_context_tx, stale_context_rx) = mpsc::channel();
    let stale_ran = Arc::new(AtomicBool::new(false));

    mailboxes
        .enqueue(
            &thread_id,
            &TurnId::new("first").unwrap(),
            move |execution| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                stale_context_tx.send(execution.check_current()).unwrap();
            },
        )
        .unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let stale_ran_in_task = stale_ran.clone();
    mailboxes
        .enqueue(&thread_id, &TurnId::new("stale").unwrap(), move |_| {
            stale_ran_in_task.store(true, Ordering::Relaxed)
        })
        .unwrap();

    install(&loaded_threads, snapshot(&thread_id));
    release_tx.send(()).unwrap();
    assert!(
        stale_context_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .is_err()
    );
    wait_until(Duration::from_secs(1), || {
        !mailboxes.has_work(&thread_id).unwrap()
    });

    assert!(!stale_ran.load(Ordering::Relaxed));
}

#[test]
fn one_thread_is_fifo_and_rejects_work_beyond_its_bounded_backlog() {
    let loaded_threads = Arc::new(LoadedThreads::new(Arc::new(InMemoryThreadStore::default())));
    let thread_id = ThreadId::new("thread").unwrap();
    install(&loaded_threads, snapshot(&thread_id));
    let mailboxes = ThreadExecutionMailboxes::with_settings(
        loaded_threads,
        NonZeroUsize::new(1).unwrap(),
        Duration::from_secs(1),
    );
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let order = Arc::new(Mutex::new(Vec::new()));
    let first_order = order.clone();
    mailboxes
        .enqueue(&thread_id, &TurnId::new("first").unwrap(), move |_| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            first_order.lock().unwrap().push("first");
        })
        .unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let second_order = order.clone();
    mailboxes
        .enqueue(&thread_id, &TurnId::new("second").unwrap(), move |_| {
            second_order.lock().unwrap().push("second");
        })
        .unwrap();

    assert!(matches!(
        mailboxes.enqueue(&thread_id, &TurnId::new("third").unwrap(), |_| {}),
        Err(CoreError::Execution(message)) if message.contains("mailbox is full")
    ));
    release_tx.send(()).unwrap();
    wait_until(Duration::from_secs(1), || {
        !mailboxes.has_work(&thread_id).unwrap()
    });
    assert_eq!(*order.lock().unwrap(), ["first", "second"]);
}

fn install(loaded_threads: &LoadedThreads, snapshot: ThreadSnapshot) {
    let slot = loaded_threads.slot(&snapshot.thread_id).unwrap();
    *slot.loaded.lock().unwrap() = Some(loaded_threads.install(snapshot));
}

fn snapshot(thread_id: &ThreadId) -> ThreadSnapshot {
    ThreadSnapshot {
        session_id: SessionId::new("session").unwrap(),
        thread_id: thread_id.clone(),
        title: "test".into(),
        turn_execution_binding: None,
        sequence: 1,
        turns: Vec::new(),
        items: Vec::new(),
        context_checkpoints: Vec::new(),
        context_overflow_recoveries: BTreeMap::new(),
        item_sequences: BTreeMap::new(),
        event_digests: BTreeMap::new(),
        commands: Vec::new(),
        steer_deliveries: BTreeMap::new(),
        seen_interaction_ids: BTreeSet::new(),
        resolved_interactions: Vec::new(),
        started_tool_calls: BTreeSet::new(),
        tool_execution_starts: BTreeMap::new(),
        escalated_tool_calls: BTreeSet::new(),
        agent_context_seed: None,
        delegations: BTreeMap::new(),
        agent_cancellations_received: BTreeSet::new(),
        agent_joins: BTreeMap::new(),
        produced_delegation_results: BTreeMap::new(),
        received_delegation_results: BTreeMap::new(),
        sent_agent_messages: BTreeMap::new(),
        received_agent_messages: BTreeMap::new(),
    }
}

fn wait_until(timeout: Duration, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + timeout;
    while !condition() {
        assert!(Instant::now() < deadline, "condition was not met in time");
        thread::sleep(Duration::from_millis(1));
    }
}
