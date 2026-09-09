use super::Command;
use super::manager::SessionManagerState;
use crate::keymap::bindings;
use crate::thread::preview::ConversationPreview;
use crate::widgets::navigation::Navigation;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use std::collections::BTreeMap;
use std::time::Instant;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadReadResult;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TerminalScreen {
    Manager,
    Session(SessionId),
}

/// The manager handles local interaction and returns work for the application to coordinate.
#[derive(Debug)]
pub(crate) enum SessionManagerInputOutcome {
    Unhandled,
    Consumed,
    Command(Command),
    DetailsRequested,
}

#[derive(Debug, Default)]
pub(crate) struct SessionsState {
    pub(crate) details: Option<super::details::SessionDetails>,
    details_generation: u64,
    pub(crate) preview: Option<ConversationPreview>,
    preview_generation: u64,
    screen: Option<TerminalScreen>,
    active_session_id: Option<SessionId>,
    catalog: Vec<Session>,
    last_viewed_thread: BTreeMap<SessionId, ThreadId>,
    manager: SessionManagerState,
}

impl SessionsState {
    pub(crate) fn handle_manager_key(&mut self, key: KeyEvent) -> SessionManagerInputOutcome {
        use SessionManagerInputOutcome as Outcome;

        if !matches!(self.screen(), Some(TerminalScreen::Manager))
            || !self.manager.focused()
            || key.kind == KeyEventKind::Release
            || (key.kind == KeyEventKind::Repeat && Navigation::from_key(key).is_none())
        {
            return Outcome::Unhandled;
        }
        if let Some(navigation) = Navigation::from_key(key) {
            self.manager.navigate(&self.catalog, navigation);
            return Outcome::Consumed;
        }
        if (self.manager.selected_is_archived() && bindings::SESSION_DELETE.matches(key))
            || (!self.manager.selected_is_archived() && bindings::SESSION_ARCHIVE.matches(key))
        {
            let command = if self.manager.selected_is_archived() {
                self.manager
                    .selected_session()
                    .cloned()
                    .map(|session_id| Command::Delete { session_id })
            } else {
                let session_ids = self.manager.selected_archive_ids(&self.catalog);
                (!session_ids.is_empty()).then_some(Command::Archive { session_ids })
            };
            return command.map_or(Outcome::Consumed, Outcome::Command);
        }
        if (self.manager.selected_group().is_some()
            && if self.manager.selected_group_expanded() {
                bindings::GROUP_COLLAPSE.matches(key)
            } else {
                bindings::GROUP_EXPAND.matches(key)
            })
            || (self.manager.selected_group().is_none()
                && if self.manager.selected_is_archived() {
                    bindings::SESSION_RESTORE.matches(key)
                } else {
                    bindings::SESSION_OPEN.matches(key)
                })
        {
            if self.manager.selected_group().is_some() {
                self.manager.toggle_selected_group();
                return Outcome::Consumed;
            }
            let command = self.manager.selected_session().map(|session_id| {
                if self.manager.selected_is_archived() {
                    Command::Restore {
                        session_id: session_id.clone(),
                    }
                } else {
                    Command::Resume {
                        session_id: session_id.to_string(),
                        preferred_thread_id: self.remembered_thread(session_id).cloned(),
                    }
                }
            });
            return command.map_or(Outcome::Consumed, Outcome::Command);
        }
        if bindings::SESSION_PREVIEW.matches(key) {
            if self.manager.selected_group().is_some() {
                self.manager.toggle_selected_group();
                return Outcome::Consumed;
            }
            let session_id = self.manager.selected_session().cloned();
            return session_id
                .and_then(|id| self.open_preview(&id))
                .map_or(Outcome::Consumed, Outcome::Command);
        }
        if bindings::SESSION_DETAILS.matches(key) {
            return Outcome::DetailsRequested;
        }
        if (bindings::LEFT.matches(key) || bindings::RIGHT.matches(key))
            && self.manager.selected_group().is_some()
        {
            if bindings::RIGHT.matches(key) {
                self.manager.expand_selected_group();
            } else {
                self.manager.collapse_selected_group();
            }
            return Outcome::Consumed;
        }
        if bindings::SESSION_PIN.matches(key) {
            self.manager.toggle_selected_pin();
            return Outcome::Consumed;
        }
        if bindings::RETURN_INPUT.matches(key) {
            self.manager.blur();
            return Outcome::Consumed;
        }
        Outcome::Unhandled
    }

    pub(crate) fn open_details(&mut self) {
        let Some(session) = self.manager.selected_session().and_then(|id| {
            self.catalog
                .iter()
                .find(|session| &session.session_id == id)
        }) else {
            return;
        };
        self.details_generation = self.details_generation.wrapping_add(1);
        self.details = Some(super::details::SessionDetails::new(
            session,
            self.details_generation,
        ));
    }

    pub(crate) fn finish_details(
        &mut self,
        generation: u64,
        result: Result<zeta_app_server_protocol::protocol::session::SessionResult, String>,
    ) {
        if let Some(details) = self
            .details
            .as_mut()
            .filter(|details| details.generation == generation)
        {
            details.install(result);
        }
    }

    pub(crate) fn open_preview(&mut self, session_id: &SessionId) -> Option<super::Command> {
        let session = self
            .catalog
            .iter()
            .find(|session| &session.session_id == session_id)?;
        let thread = self
            .remembered_thread(session_id)
            .and_then(|id| {
                session
                    .threads
                    .iter()
                    .find(|thread| &thread.thread_id == id)
            })
            .or_else(|| {
                session
                    .threads
                    .iter()
                    .find(|thread| thread.thread_id.as_str() == session_id.as_str())
            })?;
        let params = SessionThreadReadParams {
            session_id: session_id.clone(),
            thread_id: thread.thread_id.clone(),
            history: Some(ThreadSnapshotHistory::Latest { turn_limit: 50 }),
        };
        self.preview_generation = self.preview_generation.wrapping_add(1);
        self.preview = Some(ConversationPreview::new(
            self.preview_generation,
            session.title.clone(),
            params.clone(),
        ));
        Some(super::Command::Preview {
            generation: self.preview_generation,
            params,
        })
    }

    pub(crate) fn finish_preview(
        &mut self,
        generation: u64,
        result: Result<SessionThreadReadResult, String>,
    ) {
        if let Some(preview) = self
            .preview
            .as_mut()
            .filter(|preview| preview.generation == generation)
        {
            preview.install(result);
        }
    }
    #[cfg(test)]
    pub(crate) fn install_catalog(
        &mut self,
        catalog: Vec<Session>,
        active_session_id: SessionId,
        viewed_thread_id: ThreadId,
    ) {
        self.last_viewed_thread
            .insert(active_session_id.clone(), viewed_thread_id);
        self.active_session_id = Some(active_session_id.clone());
        self.screen = Some(TerminalScreen::Session(active_session_id));
        self.catalog = catalog;
        self.manager.reconcile(&self.catalog);
    }

    pub(crate) fn refresh_catalog(&mut self, catalog: Vec<Session>) {
        self.catalog = catalog;
        if let Some(details) = self.details.as_mut() {
            if self
                .catalog
                .iter()
                .any(|session| session.session_id == details.session_id)
            {
                details.invalidate();
            } else {
                self.details = None;
            }
        }
        if self.preview.as_ref().is_some_and(|preview| {
            !self
                .catalog
                .iter()
                .any(|session| &session.session_id == preview.session_id())
        }) {
            self.preview = None;
        }
        self.manager.reconcile(&self.catalog);
        if self.active_session_id.as_ref().is_some_and(|session_id| {
            !self
                .catalog
                .iter()
                .any(|session| &session.session_id == session_id)
        }) {
            self.active_session_id = None;
        }
        if let Some(TerminalScreen::Session(session_id)) = self.screen.as_ref()
            && !self
                .catalog
                .iter()
                .any(|session| &session.session_id == session_id)
        {
            self.screen = Some(TerminalScreen::Manager);
        }
    }

    pub(crate) fn screen(&self) -> Option<&TerminalScreen> {
        self.screen.as_ref()
    }

    pub(crate) fn show_manager(&mut self) {
        self.preview = None;
        self.screen = Some(TerminalScreen::Manager);
        self.manager.reconcile(&self.catalog);
    }

    pub(crate) fn show_session(&mut self, session_id: SessionId, viewed_thread_id: ThreadId) {
        self.preview = None;
        self.manager.blur();
        self.last_viewed_thread
            .insert(session_id.clone(), viewed_thread_id);
        self.active_session_id = Some(session_id.clone());
        self.screen = Some(TerminalScreen::Session(session_id));
    }

    pub(crate) fn activate_context(&mut self, session_id: SessionId, thread_id: ThreadId) {
        self.show_session(session_id, thread_id);
    }

    #[cfg(test)]
    pub(crate) fn remember_viewed_thread(&mut self, session_id: SessionId, thread_id: ThreadId) {
        self.last_viewed_thread.insert(session_id, thread_id);
    }

    pub(crate) fn remembered_thread(&self, session_id: &SessionId) -> Option<&ThreadId> {
        self.last_viewed_thread.get(session_id)
    }

    pub(crate) fn active_session_id(&self) -> Option<&SessionId> {
        self.active_session_id.as_ref()
    }

    pub(crate) fn restorable_thread(&self, session_id: &SessionId) -> Option<ThreadId> {
        let session = self
            .catalog
            .iter()
            .find(|session| &session.session_id == session_id)?;
        self.remembered_thread(session_id)
            .filter(|remembered| {
                session.threads.iter().any(|thread| {
                    &thread.thread_id == *remembered
                        && thread.status == ThreadStatus::Active
                        && thread.forked_from_id.is_none()
                })
            })
            .cloned()
            .or_else(|| {
                session
                    .threads
                    .iter()
                    .find(|thread| {
                        thread.thread_id.as_str() == session.session_id.as_str()
                            && thread.status == ThreadStatus::Active
                    })
                    .or_else(|| {
                        session.threads.iter().find(|thread| {
                            thread.status == ThreadStatus::Active && thread.forked_from_id.is_none()
                        })
                    })
                    .map(|thread| thread.thread_id.clone())
            })
    }

    pub(crate) fn manager(&self) -> &SessionManagerState {
        &self.manager
    }

    pub(crate) fn manager_mut(&mut self) -> &mut SessionManagerState {
        &mut self.manager
    }

    pub(crate) fn catalog(&self) -> &[Session] {
        &self.catalog
    }

    pub(crate) fn refresh_manager_time(&mut self, now: Instant) -> bool {
        self.manager.refresh_time(now, &self.catalog)
    }

    pub(crate) fn previous_screen(&self) -> Option<TerminalScreen> {
        match self.screen()? {
            TerminalScreen::Manager => None,
            TerminalScreen::Session(_) => Some(TerminalScreen::Manager),
        }
    }

    pub(crate) fn next_screen(&self) -> Option<TerminalScreen> {
        match self.screen()? {
            TerminalScreen::Manager => self.active_session_id.clone().map(TerminalScreen::Session),
            TerminalScreen::Session(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
