use super::BackendInner;
use super::ORPHAN_CAPACITY;
use super::RouteKey;
use crate::CodexTurnEvent;
use std::sync::Weak;
use std::sync::mpsc::Receiver;

pub(super) fn pump_events(inner: Weak<BackendInner>, events: Receiver<CodexTurnEvent>) {
    while let Ok(event) = events.recv() {
        let Some(inner) = inner.upgrade() else {
            return;
        };
        if matches!(event, CodexTurnEvent::ProtocolError { .. }) {
            let routes = inner
                .state
                .lock()
                .map(|mut state| {
                    state.runtime_closed = true;
                    inner.state_changed.notify_all();
                    state.routes.values().cloned().collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for route in routes {
                let _ = route.send(event.clone());
            }
            return;
        }
        let Some(key) = event_route_key(&event) else {
            continue;
        };
        let (route, overflowed) = {
            let Ok(mut state) = inner.state.lock() else {
                return;
            };
            match state.routes.get(&key).cloned() {
                Some(route) => (Some(route), Vec::new()),
                None if state.orphans.len() < ORPHAN_CAPACITY => {
                    state.orphans.push_back(event.clone());
                    (None, Vec::new())
                }
                None => {
                    state.runtime_closed = true;
                    inner.state_changed.notify_all();
                    (None, state.routes.values().cloned().collect::<Vec<_>>())
                }
            }
        };
        if !overflowed.is_empty() {
            let failure = CodexTurnEvent::ProtocolError {
                method: "runtime/orphanOverflow".into(),
            };
            for route in overflowed {
                let _ = route.send(failure.clone());
            }
            return;
        }
        if let Some(route) = route {
            let _ = route.send(event);
        }
    }
}

pub(super) fn event_route_key(event: &CodexTurnEvent) -> Option<RouteKey> {
    let (thread_id, turn_id) = match event {
        CodexTurnEvent::Started { thread_id, turn_id }
        | CodexTurnEvent::AgentMessageDelta {
            thread_id, turn_id, ..
        }
        | CodexTurnEvent::ReasoningSummaryDelta {
            thread_id, turn_id, ..
        }
        | CodexTurnEvent::ReasoningDelta {
            thread_id, turn_id, ..
        }
        | CodexTurnEvent::DiffUpdated {
            thread_id, turn_id, ..
        }
        | CodexTurnEvent::Completed {
            thread_id, turn_id, ..
        } => (thread_id, turn_id),
        CodexTurnEvent::CommandApprovalRequested(request) => (&request.thread_id, &request.turn_id),
        CodexTurnEvent::FileChangeApprovalRequested(request) => {
            (&request.thread_id, &request.turn_id)
        }
        CodexTurnEvent::UserInputRequested(request) => (&request.thread_id, &request.turn_id),
        CodexTurnEvent::ProtocolError { .. } => return None,
    };
    Some(RouteKey {
        thread_id: thread_id.clone(),
        turn_id: turn_id.clone(),
    })
}
