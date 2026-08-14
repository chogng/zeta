use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Context;
use anyhow::Result;
use zeta_protocol::SessionId;
use zeta_terminal::GridSize;
use zeta_winit::EventLoopProxy;

use crate::agent_session_target::AgentSessionTarget;
use crate::native_event::NativeEvent;
use crate::session_switch_trace;
use crate::terminal_session::TerminalSession;
use crate::terminal_session::TerminalSessionEvent;
use crate::terminal_session::TerminalSessionKey;
use crate::terminal_session::TerminalSessionReady;

/// Owns the native TerminalSession runtimes associated with App Server Sessions.
///
/// The first PTY is created before the asynchronous App Server Session snapshot arrives, so this
/// layer assigns a process-local key first and binds it to the authoritative `SessionId` later.
/// It keeps inactive PTYs alive while the host renders another Session Tab.
pub(crate) struct TerminalWorkspace {
    event_proxy: EventLoopProxy<NativeEvent>,
    target: AgentSessionTarget,
    state: TerminalWorkspaceState,
    active: Option<(TerminalSessionKey, TerminalSession)>,
    inactive: HashMap<TerminalSessionKey, TerminalSession>,
    pending_events: HashMap<TerminalSessionKey, Vec<TerminalSessionEvent>>,
    requested_size: GridSize,
}

#[derive(Debug, Eq, PartialEq)]
enum PendingTerminalReservation {
    Existing(TerminalSessionKey),
    Start(TerminalSessionKey),
}

#[derive(Debug, Eq, PartialEq)]
enum EnsureReservation {
    Ready(TerminalSessionKey),
    Pending(PendingTerminalReservation),
}

#[derive(Debug, Eq, PartialEq)]
enum ActivationDecision {
    Missing,
    AlreadyActive,
    Pending(TerminalSessionKey),
    Ready(TerminalSessionKey),
}

#[derive(Debug, Eq, PartialEq)]
enum TerminalReadyPlacement {
    Active,
    Inactive,
}

#[derive(Default)]
struct TerminalWorkspaceState {
    next_key: u64,
    initial_key: Option<TerminalSessionKey>,
    active_key: Option<TerminalSessionKey>,
    desired_active_key: Option<TerminalSessionKey>,
    pending_keys: HashSet<TerminalSessionKey>,
    session_terminal_keys: HashMap<SessionId, TerminalSessionKey>,
}

impl TerminalWorkspaceState {
    fn reserve_initial(&mut self) -> Option<TerminalSessionKey> {
        if self.active_key.is_some() || self.initial_key.is_some() {
            return None;
        }
        let key = self.allocate_key();
        self.initial_key = Some(key);
        self.pending_keys.insert(key);
        Some(key)
    }

    fn ensure_for_session(&mut self, session_id: &SessionId) -> EnsureReservation {
        if let Some(&key) = self.session_terminal_keys.get(session_id) {
            return self.reservation_for_key(key);
        }

        if self.session_terminal_keys.is_empty() {
            if let Some(key) = self.initial_key {
                self.session_terminal_keys.insert(session_id.clone(), key);
                return EnsureReservation::Pending(PendingTerminalReservation::Existing(key));
            }
            if let Some(key) = self.active_key {
                self.session_terminal_keys.insert(session_id.clone(), key);
                return EnsureReservation::Ready(key);
            }
        }

        let key = self.allocate_key();
        self.pending_keys.insert(key);
        self.session_terminal_keys.insert(session_id.clone(), key);
        EnsureReservation::Pending(PendingTerminalReservation::Start(key))
    }

    fn reservation_for_key(&self, key: TerminalSessionKey) -> EnsureReservation {
        if self.pending_keys.contains(&key) {
            EnsureReservation::Pending(PendingTerminalReservation::Existing(key))
        } else {
            EnsureReservation::Ready(key)
        }
    }

    fn activation_for_session(&mut self, session_id: &SessionId) -> ActivationDecision {
        let Some(&target_key) = self.session_terminal_keys.get(session_id) else {
            return ActivationDecision::Missing;
        };
        if self.active_key == Some(target_key) {
            self.desired_active_key = None;
            return ActivationDecision::AlreadyActive;
        }
        if self.pending_keys.contains(&target_key) {
            self.desired_active_key = Some(target_key);
            return ActivationDecision::Pending(target_key);
        }
        ActivationDecision::Ready(target_key)
    }

    fn mark_active(&mut self, key: TerminalSessionKey) {
        self.active_key = Some(key);
        self.desired_active_key = None;
    }

    fn finish_pending(&mut self, key: TerminalSessionKey) -> Option<TerminalReadyPlacement> {
        if !self.pending_keys.remove(&key) {
            return None;
        }
        let is_initial = self.initial_key == Some(key);
        let should_activate = self.desired_active_key == Some(key)
            || (is_initial && self.active_key.is_none() && self.desired_active_key.is_none());
        if is_initial {
            self.initial_key = None;
        }
        if should_activate {
            self.mark_active(key);
            Some(TerminalReadyPlacement::Active)
        } else {
            Some(TerminalReadyPlacement::Inactive)
        }
    }

    fn fail_pending(&mut self, key: TerminalSessionKey) -> bool {
        if !self.pending_keys.remove(&key) {
            return false;
        }
        self.session_terminal_keys
            .retain(|_, terminal_key| *terminal_key != key);
        if self.initial_key == Some(key) {
            self.initial_key = None;
        }
        if self.desired_active_key == Some(key) {
            self.desired_active_key = None;
        }
        true
    }

    fn is_pending(&self, key: TerminalSessionKey) -> bool {
        self.pending_keys.contains(&key)
    }

    fn session_id_for_key(&self, key: TerminalSessionKey) -> Option<SessionId> {
        self.session_terminal_keys
            .iter()
            .find_map(|(session_id, terminal_key)| {
                (*terminal_key == key).then_some(session_id.clone())
            })
    }

    fn allocate_key(&mut self) -> TerminalSessionKey {
        let key = TerminalSessionKey::new(self.next_key);
        self.next_key = self
            .next_key
            .checked_add(1)
            .expect("terminal session key space exhausted");
        key
    }
}

pub(crate) enum TerminalReadyOutcome {
    Active {
        key: TerminalSessionKey,
        buffered_events: Vec<TerminalSessionEvent>,
    },
    Inactive {
        key: TerminalSessionKey,
        buffered_events: Vec<TerminalSessionEvent>,
    },
    Failed {
        key: TerminalSessionKey,
        error: String,
    },
    Ignored {
        key: TerminalSessionKey,
    },
}

impl TerminalWorkspace {
    pub(crate) fn new(
        event_proxy: EventLoopProxy<NativeEvent>,
        target: AgentSessionTarget,
    ) -> Self {
        Self {
            event_proxy,
            target,
            state: TerminalWorkspaceState::default(),
            active: None,
            inactive: HashMap::new(),
            pending_events: HashMap::new(),
            requested_size: GridSize::default(),
        }
    }

    pub(crate) fn spawn_initial(&mut self, size: GridSize) -> Result<()> {
        if self.active.is_some() || self.state.initial_key.is_some() {
            return Ok(());
        }
        self.requested_size = size;
        let Some(key) = self.state.reserve_initial() else {
            return Ok(());
        };
        if let Err(error) = self.start_terminal(key, size) {
            self.state.fail_pending(key);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn active(&self) -> Option<&TerminalSession> {
        self.active.as_ref().map(|(_, terminal)| terminal)
    }

    pub(crate) fn active_mut(&mut self) -> Option<&mut TerminalSession> {
        self.active.as_mut().map(|(_, terminal)| terminal)
    }

    pub(crate) fn active_key(&self) -> Option<TerminalSessionKey> {
        self.active.as_ref().map(|(key, _)| *key)
    }

    pub(crate) fn inactive_mut(&mut self, key: TerminalSessionKey) -> Option<&mut TerminalSession> {
        self.inactive.get_mut(&key)
    }

    pub(crate) fn session_id_for_key(&self, key: TerminalSessionKey) -> Option<SessionId> {
        self.state.session_id_for_key(key)
    }

    pub(crate) fn ensure_for_session(
        &mut self,
        session_id: &SessionId,
        size: GridSize,
    ) -> Result<()> {
        let _trace = session_switch_trace::Span::new(None, "terminal-workspace-ensure");
        session_switch_trace::event(
            None,
            "terminal-ensure",
            format_args!(
                "session_id={session_id} known={}",
                self.state.session_terminal_keys.contains_key(session_id)
            ),
        );
        self.requested_size = size;
        let reservation = self.state.ensure_for_session(session_id);
        let pending = match reservation {
            EnsureReservation::Ready(key) => {
                session_switch_trace::event(
                    None,
                    "terminal-ensure-ready",
                    format_args!("session_id={session_id} key={key:?}"),
                );
                return Ok(());
            }
            EnsureReservation::Pending(reservation) => reservation,
        };
        let PendingTerminalReservation::Start(key) = pending else {
            return Ok(());
        };

        if let Err(error) = self.start_terminal(key, size) {
            self.state.fail_pending(key);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn activate_for_session(&mut self, session_id: &SessionId) -> bool {
        match self.state.activation_for_session(session_id) {
            ActivationDecision::Missing => false,
            ActivationDecision::AlreadyActive => true,
            ActivationDecision::Pending(key) => {
                session_switch_trace::event(
                    None,
                    "terminal-activation-pending",
                    format_args!("session_id={session_id} key={key:?}"),
                );
                false
            }
            ActivationDecision::Ready(target_key) => {
                let Some(next_terminal) = self.inactive.remove(&target_key) else {
                    return false;
                };
                if let Some((current_key, current_terminal)) = self.active.take() {
                    self.inactive.insert(current_key, current_terminal);
                }
                self.active = Some((target_key, next_terminal));
                self.state.mark_active(target_key);
                true
            }
        }
    }

    pub(crate) fn handle_ready(&mut self, ready: TerminalSessionReady) -> TerminalReadyOutcome {
        let TerminalSessionReady { key, result } = ready;
        let buffered_events = self.pending_events.remove(&key).unwrap_or_default();
        if !self.state.is_pending(key) {
            return TerminalReadyOutcome::Ignored { key };
        }

        let placement = match result {
            Ok(mut terminal) => {
                let placement = self
                    .state
                    .finish_pending(key)
                    .expect("pending terminal disappeared before completion");
                resize_terminal(&mut terminal, self.requested_size);
                match placement {
                    TerminalReadyPlacement::Active => {
                        session_switch_trace::event(
                            None,
                            "terminal-ready",
                            format_args!(
                                "key={key:?} placement=active buffered_events={}",
                                buffered_events.len()
                            ),
                        );
                        if let Some((current_key, current_terminal)) = self.active.take() {
                            self.inactive.insert(current_key, current_terminal);
                        }
                        self.active = Some((key, terminal));
                        TerminalReadyOutcome::Active {
                            key,
                            buffered_events,
                        }
                    }
                    TerminalReadyPlacement::Inactive => {
                        session_switch_trace::event(
                            None,
                            "terminal-ready",
                            format_args!(
                                "key={key:?} placement=inactive buffered_events={}",
                                buffered_events.len()
                            ),
                        );
                        self.inactive.insert(key, terminal);
                        TerminalReadyOutcome::Inactive {
                            key,
                            buffered_events,
                        }
                    }
                }
            }
            Err(error) => {
                self.state.fail_pending(key);
                TerminalReadyOutcome::Failed { key, error }
            }
        };
        placement
    }

    pub(crate) fn buffer_event_if_pending(
        &mut self,
        key: TerminalSessionKey,
        event: TerminalSessionEvent,
    ) -> bool {
        if !self.state.is_pending(key) {
            return false;
        }
        self.pending_events.entry(key).or_default().push(event);
        true
    }

    pub(crate) fn is_pending(&self, key: TerminalSessionKey) -> bool {
        self.state.is_pending(key)
    }

    pub(crate) fn resize_all(&mut self, size: GridSize) {
        self.requested_size = size;
        if let Some((_, terminal)) = self.active.as_mut() {
            resize_terminal(terminal, size);
        }
        for terminal in self.inactive.values_mut() {
            resize_terminal(terminal, size);
        }
    }

    fn start_terminal(&mut self, key: TerminalSessionKey, size: GridSize) -> Result<()> {
        TerminalSession::spawn_async(key, size, self.event_proxy.clone(), self.target.clone())
            .with_context(|| format!("could not queue terminal runtime {key:?} creation"))
    }
}

fn resize_terminal(terminal: &mut TerminalSession, size: GridSize) {
    if terminal.core().grid().size() != size
        && let Err(error) = terminal.resize(size)
    {
        eprintln!("could not resize terminal: {error}");
    }
}

#[cfg(test)]
#[path = "terminal_workspace_tests.rs"]
mod tests;
