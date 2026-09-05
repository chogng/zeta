use super::manager::SessionManagerState;
use crate::thread::preview::ConversationPreview;
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

#[derive(Debug, Default)]
pub(crate) struct SessionsState {
    pub(crate) preview: Option<ConversationPreview>,
    preview_generation: u64,
    screen: Option<TerminalScreen>,
    active_session_id: Option<SessionId>,
    catalog: Vec<Session>,
    last_viewed_thread: BTreeMap<SessionId, ThreadId>,
    manager: SessionManagerState,
}

impl SessionsState {
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
