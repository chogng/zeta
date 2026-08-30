use super::manager::SessionManagerState;
use std::collections::BTreeMap;
use std::time::Instant;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RootTarget {
    Manager,
    Session(SessionId),
}

#[derive(Debug, Default)]
pub(crate) struct SessionsState {
    root: Option<RootTarget>,
    catalog: Vec<Session>,
    last_viewed_thread: BTreeMap<SessionId, ThreadId>,
    manager: SessionManagerState,
}

impl SessionsState {
    #[cfg(test)]
    pub(crate) fn install_catalog(
        &mut self,
        catalog: Vec<Session>,
        active_session_id: SessionId,
        viewed_thread_id: ThreadId,
    ) {
        self.last_viewed_thread
            .insert(active_session_id.clone(), viewed_thread_id);
        self.root = Some(RootTarget::Session(active_session_id));
        self.catalog = catalog;
        self.manager.reconcile(&self.catalog);
    }

    pub(crate) fn refresh_catalog(&mut self, catalog: Vec<Session>) {
        self.catalog = catalog;
        self.manager.reconcile(&self.catalog);
        if let Some(RootTarget::Session(session_id)) = self.root.as_ref()
            && !self
                .catalog
                .iter()
                .any(|session| &session.session_id == session_id)
        {
            self.root = Some(RootTarget::Manager);
        }
    }

    pub(crate) fn root(&self) -> Option<&RootTarget> {
        self.root.as_ref()
    }

    pub(crate) fn show_manager(&mut self) {
        self.root = Some(RootTarget::Manager);
        self.manager.reconcile(&self.catalog);
    }

    pub(crate) fn show_session(&mut self, session_id: SessionId, viewed_thread_id: ThreadId) {
        self.last_viewed_thread
            .insert(session_id.clone(), viewed_thread_id);
        self.root = Some(RootTarget::Session(session_id));
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

    pub(crate) fn previous_root(&self) -> Option<RootTarget> {
        match self.root()? {
            RootTarget::Manager => None,
            RootTarget::Session(session_id) => {
                let ordered = self.manager.ordered(&self.catalog);
                let index = ordered
                    .iter()
                    .position(|session| &session.session_id == session_id)?;
                if index == 0 {
                    Some(RootTarget::Manager)
                } else {
                    Some(RootTarget::Session(ordered[index - 1].session_id.clone()))
                }
            }
        }
    }

    pub(crate) fn next_root(&self) -> Option<RootTarget> {
        match self.root()? {
            RootTarget::Manager => self
                .manager
                .ordered(&self.catalog)
                .first()
                .map(|session| RootTarget::Session(session.session_id.clone())),
            RootTarget::Session(session_id) => {
                let ordered = self.manager.ordered(&self.catalog);
                let index = ordered
                    .iter()
                    .position(|session| &session.session_id == session_id)?;
                ordered
                    .get(index.saturating_add(1))
                    .map(|session| RootTarget::Session(session.session_id.clone()))
            }
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
