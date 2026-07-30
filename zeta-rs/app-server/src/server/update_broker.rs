use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, Weak};
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_app_server_protocol::protocol::git::{GitStatusChanged, GitStatusResult};
use zeta_app_server_protocol::protocol::registry::ServerNotificationMethod;
use zeta_app_server_protocol::protocol::skills::SkillsChanged;
use zeta_app_server_protocol::rpc::JsonRpcNotification;
use zeta_protocol::{
    SessionId, SessionUpdateEnvelope, ThreadId, ThreadUpdate, ThreadUpdateEnvelope,
};

#[derive(Clone, Debug, Default)]
pub(super) struct NotificationQueue {
    inner: Arc<NotificationQueueInner>,
}

#[derive(Debug, Default)]
struct NotificationQueueInner {
    state: Mutex<NotificationQueueState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct NotificationQueueState {
    values: Vec<Value>,
    closed: bool,
}

pub(crate) struct NotificationListener {
    queue: NotificationQueue,
}

impl NotificationQueue {
    fn downgrade(&self) -> Weak<NotificationQueueInner> {
        Arc::downgrade(&self.inner)
    }

    fn from_inner(inner: Arc<NotificationQueueInner>) -> Self {
        Self { inner }
    }

    pub(super) fn listener(&self) -> NotificationListener {
        NotificationListener {
            queue: self.clone(),
        }
    }

    pub(super) fn push(&self, value: Value) {
        self.extend([value]);
    }

    pub(super) fn extend(&self, values: impl IntoIterator<Item = Value>) {
        if let Ok(mut state) = self.inner.state.lock() {
            let was_empty = state.values.is_empty();
            state.values.extend(values);
            if was_empty && !state.values.is_empty() {
                self.inner.changed.notify_all();
            }
        }
    }

    pub(super) fn drain(&self) -> Vec<Value> {
        self.inner
            .state
            .lock()
            .map(|mut state| std::mem::take(&mut state.values))
            .unwrap_or_default()
    }

    pub(super) fn close(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.closed = true;
            self.inner.changed.notify_all();
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| state.values.len())
            .unwrap_or_default()
    }
}

impl NotificationListener {
    pub(crate) fn wait(&self) -> bool {
        let Ok(mut state) = self.queue.inner.state.lock() else {
            return false;
        };
        while state.values.is_empty() && !state.closed {
            let Ok(next) = self.queue.inner.changed.wait(state) else {
                return false;
            };
            state = next;
        }
        !state.values.is_empty()
    }

    pub(crate) fn drain(&self) -> Vec<Value> {
        self.queue.drain()
    }

    pub(crate) fn close(&self) {
        self.queue.close();
    }
}

#[derive(Default)]
pub(super) struct UpdateBroker {
    subscribers: Mutex<BTreeMap<u64, Subscriber>>,
}

struct Subscriber {
    queue: Weak<NotificationQueueInner>,
    sessions: BTreeMap<SessionId, u64>,
    threads: BTreeMap<ThreadId, u64>,
}

impl UpdateBroker {
    pub(super) fn register(&self, connection_id: u64, queue: &NotificationQueue) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.insert(
                connection_id,
                Subscriber {
                    queue: queue.downgrade(),
                    sessions: BTreeMap::new(),
                    threads: BTreeMap::new(),
                },
            );
        }
    }

    pub(super) fn unregister(&self, connection_id: u64) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.remove(&connection_id);
        }
    }

    pub(super) fn subscribe_session(
        &self,
        connection_id: u64,
        session_id: SessionId,
        sequence: u64,
    ) {
        if let Ok(mut subscribers) = self.subscribers.lock()
            && let Some(subscriber) = subscribers.get_mut(&connection_id)
        {
            subscriber.sessions.insert(session_id, sequence);
        }
    }

    pub(super) fn unsubscribe_session(&self, connection_id: u64, session_id: &SessionId) {
        if let Ok(mut subscribers) = self.subscribers.lock()
            && let Some(subscriber) = subscribers.get_mut(&connection_id)
        {
            subscriber.sessions.remove(session_id);
        }
    }

    pub(super) fn subscribe_thread(&self, connection_id: u64, thread_id: ThreadId, sequence: u64) {
        if let Ok(mut subscribers) = self.subscribers.lock()
            && let Some(subscriber) = subscribers.get_mut(&connection_id)
        {
            subscriber.threads.insert(thread_id, sequence);
        }
    }

    pub(super) fn unsubscribe_thread(&self, connection_id: u64, thread_id: &ThreadId) {
        if let Ok(mut subscribers) = self.subscribers.lock()
            && let Some(subscriber) = subscribers.get_mut(&connection_id)
        {
            subscriber.threads.remove(thread_id);
        }
    }

    pub(super) fn publish_session(
        &self,
        session_id: &SessionId,
        updates: &[SessionUpdateEnvelope],
    ) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            let Some(cursor) = subscriber.sessions.get_mut(session_id) else {
                return true;
            };
            let pending = updates
                .iter()
                .filter(|update| update.durable_sequence > *cursor)
                .map(|update| notification(ServerNotificationMethod::SessionUpdate, update))
                .collect::<Vec<_>>();
            if let Some(last) = updates.last() {
                *cursor = (*cursor).max(last.durable_sequence);
            }
            queue.extend(pending);
            true
        });
    }

    pub(super) fn publish_thread(&self, thread_id: &ThreadId, updates: &[ThreadUpdateEnvelope]) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            let Some(cursor) = subscriber.threads.get_mut(thread_id) else {
                return true;
            };
            let pending = updates
                .iter()
                .filter(|update| update.durable_sequence > *cursor)
                .map(|update| notification(ServerNotificationMethod::ThreadUpdate, update))
                .collect::<Vec<_>>();
            if let Some(last) = updates.last() {
                *cursor = (*cursor).max(last.durable_sequence);
            }
            queue.extend(pending);
            true
        });
    }

    pub(super) fn publish_thread_update(&self, update: ThreadUpdateEnvelope) {
        match update.update {
            ThreadUpdate::Committed { .. } => {
                self.publish_thread(&update.thread_id, std::slice::from_ref(&update));
            }
            ThreadUpdate::ItemStarted { .. }
            | ThreadUpdate::ItemDelta { .. }
            | ThreadUpdate::PlanUpdated { .. } => self.publish_thread_transient(&update),
        }
    }

    pub(super) fn publish_skills_changed(&self, generation: u64) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::SkillsChanged,
                &SkillsChanged { generation },
            ));
            true
        });
    }

    pub(super) fn publish_git_status_changed(&self, status: GitStatusResult) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::GitStatusChanged,
                &GitStatusChanged {
                    status: status.clone(),
                },
            ));
            true
        });
    }

    pub(super) fn publish_fs_changed(&self, changed: FsChanged) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            queue.push(notification(ServerNotificationMethod::FsChanged, &changed));
            true
        });
    }

    fn publish_thread_transient(&self, update: &ThreadUpdateEnvelope) {
        let Ok(mut subscribers) = self.subscribers.lock() else {
            return;
        };
        subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber
                .queue
                .upgrade()
                .map(NotificationQueue::from_inner)
            else {
                return false;
            };
            if !subscriber.threads.contains_key(&update.thread_id) {
                return true;
            }
            queue.push(notification(ServerNotificationMethod::ThreadUpdate, update));
            true
        });
    }
}

fn notification<T: Serialize>(method: ServerNotificationMethod, params: &T) -> Value {
    serde_json::to_value(JsonRpcNotification::new(
        method.as_str().into(),
        serde_json::to_value(params).expect("notification params must serialize"),
    ))
    .expect("JSON-RPC notification must serialize")
}

#[cfg(test)]
#[path = "update_broker_tests.rs"]
mod tests;
