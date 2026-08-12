use super::update_broker::UpdateBroker;
use super::update_broker::unix_time_millis;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_core::CancelTurnInteractionRequest;
use zeta_core::ThreadController;
use zeta_protocol::InteractionCancelReason;
use zeta_protocol::StableTurnError;

const DEADLINE_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Enforces durable Agent interaction deadlines at the App Server delivery boundary.
pub(super) struct InteractionDeadlineWatcher {
    shutdown: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl InteractionDeadlineWatcher {
    pub(super) fn start(
        threads: Arc<ThreadController>,
        updates: Arc<UpdateBroker>,
        mutation_gate: Arc<Mutex<()>>,
    ) -> Self {
        let (shutdown, shutdown_receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-agent-interaction-deadlines".into())
            .spawn(move || {
                loop {
                    match shutdown_receiver.recv_timeout(DEADLINE_POLL_INTERVAL) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            expire_deadlines(&threads, &updates, &mutation_gate);
                        }
                    }
                }
            })
            .ok();
        Self {
            shutdown: Some(shutdown),
            thread,
        }
    }
}

impl Drop for InteractionDeadlineWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn expire_deadlines(threads: &ThreadController, updates: &UpdateBroker, mutation_gate: &Mutex<()>) {
    for request in updates.expired_agent_requests(unix_time_millis()) {
        let Ok(_mutation) = mutation_gate.lock() else {
            return;
        };
        let Ok(snapshot) = threads.read_thread(&request.thread_id) else {
            continue;
        };
        let still_expired = snapshot
            .turns
            .iter()
            .find(|turn| turn.turn_id == request.turn_id)
            .and_then(|turn| turn.pending_interaction.as_ref())
            .filter(|interaction| interaction.request_id == request.interaction.request_id)
            .and_then(|interaction| interaction.deadline)
            .is_some_and(|deadline| deadline.expires_at_unix_ms <= unix_time_millis());
        if !still_expired {
            continue;
        }
        let before_sequence = snapshot.sequence;
        if threads
            .cancel_turn_interaction(
                &request.thread_id,
                CancelTurnInteractionRequest {
                    turn_id: request.turn_id.clone(),
                    request_id: request.interaction.request_id.clone(),
                    reason: InteractionCancelReason::DeadlineElapsed,
                },
            )
            .is_err()
        {
            continue;
        }
        updates.retire_agent_request(&request.interaction.request_id);
        let _ = threads.fail_turn(
            &request.thread_id,
            &request.turn_id,
            StableTurnError::interaction_deadline_elapsed(),
        );
        if let Ok(published) = threads.thread_updates_after(&request.thread_id, before_sequence) {
            updates.publish_thread(&request.thread_id, &published);
        }
    }
}
