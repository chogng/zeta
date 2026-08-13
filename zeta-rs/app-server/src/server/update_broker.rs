use crate::server::notification_queue::NotificationQueue;
use crate::server::notification_queue::NotificationQueueHandle;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Mutex;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationPresenceSnapshot;
use zeta_app_server_protocol::protocol::collaboration::DocumentCollaborationUpdate;
use zeta_app_server_protocol::protocol::common::AgentInteractionCapability;
use zeta_app_server_protocol::protocol::config::ConfigChanged;
use zeta_app_server_protocol::protocol::connectors::ConnectorsChanged;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_app_server_protocol::protocol::git::{GitStatusChanged, GitStatusResult};
use zeta_app_server_protocol::protocol::language::LanguageDiagnosticsNotification;
use zeta_app_server_protocol::protocol::plugins::PluginsChanged;
use zeta_app_server_protocol::protocol::registry::ServerNotificationMethod;
use zeta_app_server_protocol::protocol::skills::SkillsChanged;
use zeta_app_server_protocol::rpc::JsonRpcNotification;
use zeta_config::ConfigChange;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::RequestId;
use zeta_protocol::SessionEvent;
use zeta_protocol::SessionId;
use zeta_protocol::SessionUpdate;
use zeta_protocol::SessionUpdateEnvelope;
use zeta_protocol::ThreadEvent;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdate;
use zeta_protocol::ThreadUpdateEnvelope;

pub(super) fn unix_time_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

pub(super) struct UpdateBroker {
    state: Mutex<BrokerState>,
}

#[derive(Default)]
struct BrokerState {
    subscribers: BTreeMap<u64, Subscriber>,
    interaction_assignments: BTreeMap<RequestId, u64>,
    pending_interactions: BTreeMap<RequestId, AgentRequestEnvelope>,
}

struct Subscriber {
    queue: NotificationQueueHandle,
    agent_interactions: Option<AgentInteractionCapability>,
    collaboration_rooms: BTreeSet<String>,
    sessions: BTreeMap<SessionId, u64>,
    threads: BTreeMap<ThreadId, ThreadSubscription>,
}

#[derive(Default)]
struct ThreadSubscription {
    sequence: u64,
    session_owners: BTreeSet<SessionId>,
}

impl Default for UpdateBroker {
    fn default() -> Self {
        Self {
            state: Mutex::new(BrokerState::default()),
        }
    }
}

impl UpdateBroker {
    pub(super) fn register(&self, connection_id: u64, queue: &NotificationQueue) {
        if let Ok(mut state) = self.state.lock() {
            state.subscribers.insert(
                connection_id,
                Subscriber {
                    queue: queue.downgrade(),
                    agent_interactions: None,
                    collaboration_rooms: BTreeSet::new(),
                    sessions: BTreeMap::new(),
                    threads: BTreeMap::new(),
                },
            );
        }
    }

    pub(super) fn unregister(&self, connection_id: u64) -> Vec<AgentRequestEnvelope> {
        if let Ok(mut state) = self.state.lock() {
            let lost = take_owned_dynamic_interactions(&mut state, connection_id, |_| true);
            state.subscribers.remove(&connection_id);
            state
                .interaction_assignments
                .retain(|_, owner| *owner != connection_id);
            reconcile_interaction_assignments(&mut state);
            return lost;
        }
        Vec::new()
    }

    pub(super) fn set_agent_interaction_capability(
        &self,
        connection_id: u64,
        capability: Option<AgentInteractionCapability>,
    ) {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            subscriber.agent_interactions = capability;
            reconcile_interaction_assignments(&mut state);
        }
    }

    pub(super) fn offer_agent_request(&self, request: AgentRequestEnvelope) {
        if let Ok(mut state) = self.state.lock() {
            state
                .pending_interactions
                .insert(request.interaction.request_id.clone(), request);
            reconcile_interaction_assignments(&mut state);
        }
    }

    pub(super) fn is_agent_interaction_owner(
        &self,
        connection_id: u64,
        request_id: &RequestId,
    ) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.interaction_assignments.get(request_id).copied())
            == Some(connection_id)
    }

    pub(super) fn is_agent_interaction_expired(
        &self,
        request_id: &RequestId,
        now_unix_ms: u64,
    ) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.pending_interactions.get(request_id).cloned())
            .and_then(|request| request.interaction.deadline)
            .is_some_and(|deadline| deadline.expires_at_unix_ms <= now_unix_ms)
    }

    pub(super) fn expired_agent_requests(&self, now_unix_ms: u64) -> Vec<AgentRequestEnvelope> {
        self.state
            .lock()
            .map(|state| {
                state
                    .pending_interactions
                    .values()
                    .filter(|request| {
                        request
                            .interaction
                            .deadline
                            .is_some_and(|deadline| deadline.expires_at_unix_ms <= now_unix_ms)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn retire_agent_request(&self, request_id: &RequestId) {
        if let Ok(mut state) = self.state.lock() {
            state.pending_interactions.remove(request_id);
            state.interaction_assignments.remove(request_id);
        }
    }

    pub(super) fn subscribe_session(
        &self,
        connection_id: u64,
        session_id: SessionId,
        sequence: u64,
    ) {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            subscriber.sessions.insert(session_id, sequence);
            reconcile_interaction_assignments(&mut state);
        }
    }

    pub(super) fn unsubscribe_session(
        &self,
        connection_id: u64,
        session_id: &SessionId,
    ) -> Vec<AgentRequestEnvelope> {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            subscriber.sessions.remove(session_id);
            subscriber.threads.retain(|_, subscription| {
                subscription.session_owners.remove(session_id);
                !subscription.session_owners.is_empty()
            });
            let lost = take_owned_dynamic_interactions(&mut state, connection_id, |request| {
                &request.session_id == session_id
            });
            reconcile_interaction_assignments(&mut state);
            return lost;
        }
        Vec::new()
    }

    pub(super) fn subscribe_document_collaboration(&self, connection_id: u64, room_id: String) {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            subscriber.collaboration_rooms.insert(room_id);
        }
    }

    pub(super) fn publish_document_collaboration(&self, update: DocumentCollaborationUpdate) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            if subscriber.collaboration_rooms.contains(&update.room_id) {
                queue.push(notification(
                    ServerNotificationMethod::DocumentCollaborationUpdate,
                    &update,
                ));
            }
            true
        });
    }

    pub(super) fn publish_document_collaboration_presence(
        &self,
        snapshot: DocumentCollaborationPresenceSnapshot,
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            if subscriber.collaboration_rooms.contains(&snapshot.room_id) {
                queue.push(notification(
                    ServerNotificationMethod::DocumentCollaborationPresence,
                    &snapshot,
                ));
            }
            true
        });
    }

    pub(super) fn subscribe_session_thread(
        &self,
        connection_id: u64,
        session_id: SessionId,
        thread_id: ThreadId,
        sequence: u64,
    ) {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            let subscription = subscriber.threads.entry(thread_id).or_default();
            subscription.sequence = subscription.sequence.max(sequence);
            subscription.session_owners.insert(session_id);
            reconcile_interaction_assignments(&mut state);
        }
    }

    pub(super) fn unsubscribe_session_thread(
        &self,
        connection_id: u64,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Vec<AgentRequestEnvelope> {
        if let Ok(mut state) = self.state.lock()
            && let Some(subscriber) = state.subscribers.get_mut(&connection_id)
        {
            if let Some(subscription) = subscriber.threads.get_mut(thread_id) {
                subscription.session_owners.remove(session_id);
                if subscription.session_owners.is_empty() {
                    subscriber.threads.remove(thread_id);
                }
            }
            let lost = take_owned_dynamic_interactions(&mut state, connection_id, |request| {
                &request.session_id == session_id && &request.thread_id == thread_id
            });
            reconcile_interaction_assignments(&mut state);
            return lost;
        }
        Vec::new()
    }

    pub(super) fn publish_session(
        &self,
        session_id: &SessionId,
        updates: &[SessionUpdateEnvelope],
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            for update in updates {
                let SessionUpdate::Committed { event } = &update.update;
                match event {
                    SessionEvent::ThreadCreationPlanned { thread, .. } => {
                        subscribe_session_thread_locked(
                            subscriber,
                            session_id,
                            thread.thread_id.clone(),
                            0,
                        );
                    }
                    SessionEvent::ThreadAttached { thread_id, .. } => {
                        subscribe_session_thread_locked(
                            subscriber,
                            session_id,
                            thread_id.clone(),
                            0,
                        );
                    }
                    SessionEvent::ThreadArchived { thread_id, .. } => {
                        if let Some(subscription) = subscriber.threads.get_mut(thread_id) {
                            subscription.session_owners.remove(session_id);
                        }
                        subscriber
                            .threads
                            .retain(|_, subscription| !subscription.session_owners.is_empty());
                    }
                    _ => {}
                }
            }
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
        reconcile_interaction_assignments(&mut state);
    }

    pub(super) fn publish_thread(&self, thread_id: &ThreadId, updates: &[ThreadUpdateEnvelope]) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        for update in updates {
            let ThreadUpdate::Committed { event } = &update.update else {
                continue;
            };
            match event {
                ThreadEvent::InteractionRequested {
                    turn_id,
                    interaction,
                    ..
                } => {
                    state.pending_interactions.insert(
                        interaction.request_id.clone(),
                        AgentRequestEnvelope {
                            session_id: update.session_id.clone(),
                            thread_id: update.thread_id.clone(),
                            turn_id: turn_id.clone(),
                            interaction: interaction.clone(),
                        },
                    );
                }
                ThreadEvent::InteractionResolved { request_id, .. } => {
                    state.pending_interactions.remove(request_id);
                }
                ThreadEvent::InteractionCancelled { request_id, .. } => {
                    state.pending_interactions.remove(request_id);
                    state.interaction_assignments.remove(request_id);
                }
                _ => {}
            }
        }
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            let Some(subscription) = subscriber.threads.get_mut(thread_id) else {
                return true;
            };
            let session_pending = updates
                .iter()
                .filter(|update| update.durable_sequence > subscription.sequence)
                .map(|update| notification(ServerNotificationMethod::SessionThreadUpdate, update))
                .collect::<Vec<_>>();
            if let Some(last) = updates.last() {
                subscription.sequence = subscription.sequence.max(last.durable_sequence);
            }
            if !subscription.session_owners.is_empty() {
                queue.extend(session_pending);
            }
            true
        });
        reconcile_interaction_assignments(&mut state);
    }

    pub(super) fn publish_thread_update(&self, update: ThreadUpdateEnvelope) {
        match update.update {
            ThreadUpdate::Committed { .. } => {
                self.publish_thread(&update.thread_id, std::slice::from_ref(&update));
            }
            ThreadUpdate::ItemStarted { .. }
            | ThreadUpdate::ItemDelta { .. }
            | ThreadUpdate::PlanUpdated { .. }
            | ThreadUpdate::ToolOutputDelta { .. } => self.publish_thread_transient(&update),
        }
    }

    pub(super) fn publish_skills_changed(&self, generation: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::SkillsChanged,
                &SkillsChanged { generation },
            ));
            true
        });
    }

    pub(super) fn publish_config_changed(&self, change: ConfigChange) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::ConfigChanged,
                &ConfigChanged {
                    revision: change.revision.get(),
                    generation: change.generation.get(),
                },
            ));
            true
        });
    }

    pub(super) fn publish_connectors_changed(&self, generation: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::ConnectorsChanged,
                &ConnectorsChanged { generation },
            ));
            true
        });
    }

    pub(super) fn publish_plugins_changed(&self, revision: u64, activation_generation: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::PluginsChanged,
                &PluginsChanged {
                    revision,
                    activation_generation,
                },
            ));
            true
        });
    }

    pub(super) fn publish_git_status_changed(&self, status: GitStatusResult) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
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
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(ServerNotificationMethod::FsChanged, &changed));
            true
        });
    }

    pub(super) fn publish_language_diagnostics(
        &self,
        diagnostics: LanguageDiagnosticsNotification,
    ) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            queue.push(notification(
                ServerNotificationMethod::LanguageDiagnostics,
                &diagnostics,
            ));
            true
        });
    }

    fn publish_thread_transient(&self, update: &ThreadUpdateEnvelope) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.subscribers.retain(|_, subscriber| {
            let Some(queue) = subscriber.queue.upgrade() else {
                return false;
            };
            let Some(subscription) = subscriber.threads.get(&update.thread_id) else {
                return true;
            };
            if !subscription.session_owners.is_empty() {
                queue.push(notification(
                    ServerNotificationMethod::SessionThreadUpdate,
                    update,
                ));
            }
            true
        });
    }
}

fn take_owned_dynamic_interactions(
    state: &mut BrokerState,
    connection_id: u64,
    matches_scope: impl Fn(&AgentRequestEnvelope) -> bool,
) -> Vec<AgentRequestEnvelope> {
    let request_ids = state
        .interaction_assignments
        .iter()
        .filter_map(|(request_id, owner)| {
            if *owner != connection_id {
                return None;
            }
            let request = state.pending_interactions.get(request_id)?;
            (matches!(
                &request.interaction.request,
                zeta_protocol::AgentRequest::DynamicTool { .. }
            ) && matches_scope(request))
            .then(|| request_id.clone())
        })
        .collect::<Vec<_>>();
    request_ids
        .into_iter()
        .filter_map(|request_id| {
            state.interaction_assignments.remove(&request_id);
            state.pending_interactions.remove(&request_id)
        })
        .collect()
}

impl zeta_skills_extension::SkillRuntimeEventSink for UpdateBroker {
    fn skills_changed(&self, generation: u64) {
        self.publish_skills_changed(generation);
    }
}

fn reconcile_interaction_assignments(state: &mut BrokerState) {
    let invalid_assignments = state
        .interaction_assignments
        .iter()
        .filter_map(|(request_id, connection_id)| {
            let request = state.pending_interactions.get(request_id)?;
            let valid = state
                .subscribers
                .get(connection_id)
                .is_some_and(|subscriber| interaction_owner_matches(subscriber, request));
            (!valid).then_some(request_id.clone())
        })
        .collect::<Vec<_>>();
    for request_id in invalid_assignments {
        state.interaction_assignments.remove(&request_id);
    }

    let unassigned = state
        .pending_interactions
        .keys()
        .filter(|request_id| !state.interaction_assignments.contains_key(*request_id))
        .cloned()
        .collect::<Vec<_>>();
    for request_id in unassigned {
        let Some(request) = state.pending_interactions.get(&request_id) else {
            continue;
        };
        let owner = state
            .subscribers
            .iter()
            .find_map(|(connection_id, subscriber)| {
                interaction_owner_matches(subscriber, request)
                    .then(|| subscriber.queue.upgrade())
                    .flatten()
                    .map(|queue| (*connection_id, queue))
            });
        if let Some((connection_id, queue)) = owner {
            queue.push(notification(
                ServerNotificationMethod::AgentRequest,
                request,
            ));
            state
                .interaction_assignments
                .insert(request_id, connection_id);
        }
    }
}

fn interaction_owner_matches(subscriber: &Subscriber, request: &AgentRequestEnvelope) -> bool {
    let supports_kind = subscriber
        .agent_interactions
        .as_ref()
        .is_some_and(|capability| {
            let supports_exact_dynamic_tool = match &request.interaction.request {
                zeta_protocol::AgentRequest::DynamicTool { call } => capability
                    .dynamic_tools
                    .as_ref()
                    .is_some_and(|tools| tools.contains(&call.name)),
                _ => true,
            };
            capability.version == 1
                && capability
                    .kinds
                    .contains(&request.interaction.request.kind())
                && supports_exact_dynamic_tool
        });
    supports_kind
        && subscriber
            .threads
            .get(&request.thread_id)
            .is_some_and(|subscription| subscription.session_owners.contains(&request.session_id))
}

fn subscribe_session_thread_locked(
    subscriber: &mut Subscriber,
    session_id: &SessionId,
    thread_id: ThreadId,
    sequence: u64,
) {
    if !subscriber.sessions.contains_key(session_id) {
        return;
    }
    let subscription = subscriber.threads.entry(thread_id).or_default();
    subscription.sequence = subscription.sequence.max(sequence);
    subscription.session_owners.insert(session_id.clone());
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
