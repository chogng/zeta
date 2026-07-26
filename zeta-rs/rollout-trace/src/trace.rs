use crate::RolloutTraceError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use zeta_protocol::{SessionEvent, SessionId, ThreadId};
use zeta_session_store::{SessionStore, StoredSessionEvent};
use zeta_thread_store::{StoredEvent, ThreadStore};

/// Version of the self-contained trace artifact format.
pub const ROLLOUT_TRACE_FORMAT_VERSION: u32 = 1;

/// A read-only trace of one Session topology stream and every Thread stream it planned.
///
/// `session_events` and every child `events` vector retain their source aggregate sequence. There
/// is intentionally no global event sequence because Session and Thread writes have independent
/// ordering and concurrency boundaries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolloutTrace {
    pub format_version: u32,
    pub session_id: SessionId,
    pub session_events: Vec<StoredSessionEvent>,
    pub threads: Vec<ThreadRolloutTrace>,
}

/// One planned Thread's durable history within a [`RolloutTrace`].
///
/// An empty `events` vector is meaningful: it records a Session plan that was durable but whose
/// child Thread had not yet been created when the trace was captured.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadRolloutTrace {
    pub thread_id: ThreadId,
    pub events: Vec<StoredEvent>,
}

/// Captures the durable history belonging to one product Session without mutating either store.
///
/// The Session stream determines which Thread identities belong in the trace. A Thread is
/// included as soon as its creation is planned, so an interrupted create/fork saga remains
/// observable rather than being silently omitted.
pub fn capture_session_trace(
    session_store: &dyn SessionStore,
    thread_store: &dyn ThreadStore,
    session_id: &SessionId,
) -> Result<RolloutTrace, RolloutTraceError> {
    let session_events = session_store.load(session_id)?;
    if session_events.is_empty() {
        return Err(RolloutTraceError::SessionNotFound(session_id.clone()));
    }

    let mut seen_thread_ids = BTreeSet::new();
    let mut threads = Vec::new();
    for event in &session_events {
        let SessionEvent::ThreadCreationPlanned { thread, .. } = &event.event else {
            continue;
        };
        if !seen_thread_ids.insert(thread.thread_id.clone()) {
            continue;
        }
        let thread_id = thread.thread_id.clone();
        let events =
            thread_store
                .load(&thread_id)
                .map_err(|source| RolloutTraceError::ThreadStore {
                    thread_id: thread_id.clone(),
                    source,
                })?;
        threads.push(ThreadRolloutTrace { thread_id, events });
    }

    Ok(RolloutTrace {
        format_version: ROLLOUT_TRACE_FORMAT_VERSION,
        session_id: session_id.clone(),
        session_events,
        threads,
    })
}
