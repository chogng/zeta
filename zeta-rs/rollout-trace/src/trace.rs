use crate::RolloutTraceError;
use serde::{Deserialize, Serialize};
use zeta_history::StoredEvent;
use zeta_protocol::{SessionId, ThreadEvent, ThreadId};
use zeta_thread_store::ThreadStore;

/// Version of the self-contained trace artifact format.
pub const ROLLOUT_TRACE_FORMAT_VERSION: u32 = 2;

/// A read-only trace of every durable Thread carrying one Session tree identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolloutTrace {
    pub format_version: u32,
    pub session_id: SessionId,
    pub threads: Vec<ThreadRolloutTrace>,
}

/// One durable Thread history within a [`RolloutTrace`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRolloutTrace {
    pub thread_id: ThreadId,
    pub events: Vec<StoredEvent>,
}

/// Captures the durable Thread histories grouped by one `session_id` without mutating the store.
pub fn capture_session_trace(
    thread_store: &dyn ThreadStore,
    session_id: &SessionId,
) -> Result<RolloutTrace, RolloutTraceError> {
    let mut threads = Vec::new();
    for thread_id in thread_store
        .list_thread_ids()
        .map_err(RolloutTraceError::ThreadList)?
    {
        let events =
            thread_store
                .load(&thread_id)
                .map_err(|source| RolloutTraceError::ThreadStore {
                    thread_id: thread_id.clone(),
                    source,
                })?;
        let belongs_to_session = events.first().is_some_and(|event| {
            matches!(
                &event.event,
                ThreadEvent::ThreadCreated {
                    session_id: event_session_id,
                    ..
                } if event_session_id == session_id
            )
        });
        if !belongs_to_session {
            continue;
        }
        threads.push(ThreadRolloutTrace { thread_id, events });
    }
    if threads.is_empty() {
        return Err(RolloutTraceError::SessionNotFound(session_id.clone()));
    }

    Ok(RolloutTrace {
        format_version: ROLLOUT_TRACE_FORMAT_VERSION,
        session_id: session_id.clone(),
        threads,
    })
}
