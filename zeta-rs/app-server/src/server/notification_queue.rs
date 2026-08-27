use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::Weak;

const MAX_NOTIFICATION_QUEUE_LEN: usize = 4_096;

#[derive(Clone, Debug, Default)]
pub(crate) struct NotificationQueue {
    inner: Arc<NotificationQueueInner>,
}

#[derive(Debug, Default)]
struct NotificationQueueInner {
    state: Mutex<NotificationQueueState>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct NotificationQueueState {
    values: VecDeque<Value>,
    closed: bool,
}

#[derive(Clone, Debug)]
pub(super) struct NotificationQueueHandle {
    inner: Weak<NotificationQueueInner>,
}

pub(crate) struct NotificationListener {
    queue: NotificationQueue,
}

impl NotificationQueue {
    pub(super) fn downgrade(&self) -> NotificationQueueHandle {
        NotificationQueueHandle {
            inner: Arc::downgrade(&self.inner),
        }
    }

    pub(crate) fn listener(&self) -> NotificationListener {
        NotificationListener {
            queue: self.clone(),
        }
    }

    pub(crate) fn push(&self, value: Value) {
        self.extend([value]);
    }

    pub(crate) fn extend(&self, values: impl IntoIterator<Item = Value>) {
        if let Ok(mut state) = self.inner.state.lock() {
            let was_empty = state.values.is_empty();
            for value in values {
                if state.closed {
                    break;
                }
                if state.values.len() >= MAX_NOTIFICATION_QUEUE_LEN {
                    let resets = transcript_resets_for_dropped_notifications(&state.values);
                    state
                        .values
                        .retain(|queued| !is_transient_notification(queued));
                    for reset in resets {
                        if state.values.len() >= MAX_NOTIFICATION_QUEUE_LEN {
                            break;
                        }
                        state.values.push_back(reset);
                    }
                }
                if state.values.len() >= MAX_NOTIFICATION_QUEUE_LEN {
                    state.closed = true;
                    break;
                }
                state.values.push_back(value);
            }
            if (was_empty && !state.values.is_empty()) || state.closed {
                self.inner.changed.notify_all();
            }
        }
    }

    pub(crate) fn drain(&self) -> Vec<Value> {
        self.inner
            .state
            .lock()
            .map(|mut state| state.values.drain(..).collect())
            .unwrap_or_default()
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.closed = true;
            self.inner.changed.notify_all();
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.inner
            .state
            .lock()
            .map(|state| state.values.len())
            .unwrap_or_default()
    }
}

impl NotificationQueueHandle {
    pub(super) fn upgrade(&self) -> Option<NotificationQueue> {
        self.inner
            .upgrade()
            .map(|inner| NotificationQueue { inner })
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

fn is_transient_notification(value: &Value) -> bool {
    let method = value.get("method").and_then(Value::as_str);
    method == Some("language/diagnostics")
        || method == Some("session/thread/transcript/update")
            && value.pointer("/params/streamCursor").is_some()
        || method == Some("session/thread/update")
            && value
                .pointer("/params/update/type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind != "committed")
}

fn transcript_resets_for_dropped_notifications(values: &VecDeque<Value>) -> Vec<Value> {
    let mut scopes = BTreeMap::<(String, String), u64>::new();
    for value in values {
        if value.get("method").and_then(Value::as_str) != Some("session/thread/transcript/update")
            || value.pointer("/params/streamCursor").is_none()
        {
            continue;
        }
        let Some(session_id) = value.pointer("/params/sessionId").and_then(Value::as_str) else {
            continue;
        };
        let Some(thread_id) = value.pointer("/params/threadId").and_then(Value::as_str) else {
            continue;
        };
        let durable_sequence = value
            .pointer("/params/durableSequence")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        scopes
            .entry((session_id.to_owned(), thread_id.to_owned()))
            .and_modify(|sequence| *sequence = (*sequence).max(durable_sequence))
            .or_insert(durable_sequence);
    }
    scopes
        .into_iter()
        .map(|((session_id, thread_id), durable_sequence)| {
            serde_json::json!({
                "jsonrpc": "2.0",
                "method": "session/thread/transcript/update",
                "params": {
                    "sessionId": session_id,
                    "threadId": thread_id,
                    "durableSequence": durable_sequence,
                    "changes": [{ "type": "clearTransient" }]
                }
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "notification_queue_tests.rs"]
mod tests;
