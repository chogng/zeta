use crate::CoreError;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use zeta_async_utils::{CancellationSource, CancellationToken};
use zeta_protocol::{ThreadId, TurnId};

const DEFAULT_EXECUTION_MAILBOX_CAPACITY: NonZeroUsize =
    NonZeroUsize::new(8).expect("execution mailbox capacity is non-zero");

type ExecutionTask = Box<dyn FnOnce(CancellationToken) + Send + 'static>;

struct QueuedExecution {
    thread_id: ThreadId,
    turn_id: TurnId,
    cancellation: CancellationSource,
    task: ExecutionTask,
}

/// Owns the bounded, FIFO execution lanes for loaded Threads.
///
/// A lane runs long model and tool work outside the Thread projection lock. The controller still
/// owns every durable mutation; cancelling a Turn signals its queued or active lane immediately.
pub(crate) struct ThreadExecutionMailboxes {
    capacity: NonZeroUsize,
    senders: Mutex<BTreeMap<ThreadId, SyncSender<QueuedExecution>>>,
    active: Arc<Mutex<BTreeMap<(ThreadId, TurnId), CancellationSource>>>,
}

impl Default for ThreadExecutionMailboxes {
    fn default() -> Self {
        Self::new(DEFAULT_EXECUTION_MAILBOX_CAPACITY)
    }
}

impl ThreadExecutionMailboxes {
    pub(crate) fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            senders: Mutex::new(BTreeMap::new()),
            active: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub(crate) fn enqueue(
        &self,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        task: impl FnOnce(CancellationToken) + Send + 'static,
    ) -> Result<(), CoreError> {
        let cancellation = CancellationSource::new();
        {
            let mut active = self.active.lock().map_err(|_| {
                CoreError::Execution("execution mailbox state lock poisoned".into())
            })?;
            active.insert((thread_id.clone(), turn_id.clone()), cancellation.clone());
        }

        let queued = QueuedExecution {
            thread_id: thread_id.clone(),
            turn_id: turn_id.clone(),
            cancellation,
            task: Box::new(task),
        };
        let sender = self.sender_for(thread_id)?;
        match sender.try_send(queued) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.clear_if_current(thread_id, turn_id);
                Err(match error {
                    TrySendError::Full(_) => CoreError::Execution(format!(
                        "Thread execution mailbox is full: {thread_id}"
                    )),
                    TrySendError::Disconnected(_) => CoreError::Execution(format!(
                        "Thread execution mailbox stopped: {thread_id}"
                    )),
                })
            }
        }
    }

    pub(crate) fn cancel(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        let cancellation = self
            .active
            .lock()
            .ok()
            .and_then(|active| active.get(&(thread_id.clone(), turn_id.clone())).cloned());
        if let Some(cancellation) = cancellation {
            cancellation.cancel();
        }
    }

    fn sender_for(&self, thread_id: &ThreadId) -> Result<SyncSender<QueuedExecution>, CoreError> {
        let mut senders = self
            .senders
            .lock()
            .map_err(|_| CoreError::Execution("execution mailbox registry lock poisoned".into()))?;
        if let Some(sender) = senders.get(thread_id) {
            return Ok(sender.clone());
        }
        let (sender, receiver) = sync_channel(self.capacity.get());
        let active = self.active.clone();
        let name = format!("zeta-thread-{}", thread_id.as_str());
        thread::Builder::new()
            .name(name)
            .spawn(move || {
                while let Ok(queued) = receiver.recv() {
                    let QueuedExecution {
                        thread_id,
                        turn_id,
                        cancellation,
                        task,
                    } = queued;
                    task(cancellation.token());
                    clear_active(&active, &thread_id, &turn_id);
                }
            })
            .map_err(|error| {
                CoreError::Execution(format!("failed to start Thread mailbox: {error}"))
            })?;
        senders.insert(thread_id.clone(), sender.clone());
        Ok(sender)
    }

    fn clear_if_current(&self, thread_id: &ThreadId, turn_id: &TurnId) {
        clear_active(&self.active, thread_id, turn_id);
    }
}

fn clear_active(
    active: &Mutex<BTreeMap<(ThreadId, TurnId), CancellationSource>>,
    thread_id: &ThreadId,
    turn_id: &TurnId,
) {
    if let Ok(mut active) = active.lock() {
        active.remove(&(thread_id.clone(), turn_id.clone()));
    }
}
