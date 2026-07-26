use crate::RolloutError;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_core::{SessionCoordinator, ThreadController, ThreadStore, WriterLease};
use zeta_protocol::SessionId;
use zeta_session_store::SessionStore;
use zeta_storage::{LeaseDirectory, SessionRolloutStore, ThreadRolloutStore};

/// Opens and recovers the local authoritative rollout for all Sessions and Threads under one root.
///
/// A repository provides the typed store ports needed by consumers that must inspect durable
/// history. New coordinators must be obtained through [`Self::recover_coordinator`] so every
/// Thread is recovered before Session saga reconciliation begins.
pub struct RolloutRepository {
    session_store: Arc<SessionRolloutStore>,
    thread_store: Arc<ThreadRolloutStore>,
    writer_lease: Arc<LeaseDirectory>,
}

impl RolloutRepository {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RolloutError> {
        let root = root.into();
        Ok(Self {
            session_store: Arc::new(SessionRolloutStore::open(&root)?),
            thread_store: Arc::new(ThreadRolloutStore::open(&root)?),
            writer_lease: Arc::new(LeaseDirectory::open(root.join("leases"))?),
        })
    }

    /// Returns the read/write Session history port for this repository.
    pub fn session_store(&self) -> Arc<dyn SessionStore> {
        self.session_store.clone()
    }

    /// Returns the read/write Thread history port for this repository.
    pub fn thread_store(&self) -> Arc<dyn ThreadStore> {
        self.thread_store.clone()
    }

    /// Recovers the Core coordinator from this repository's durable history.
    ///
    /// Thread recovery precedes Session recovery because Session reconciliation may finish a
    /// durable `ThreadCreationPlanned` saga by observing or creating its child Thread.
    pub fn recover_coordinator(&self) -> Result<Arc<SessionCoordinator>, RolloutError> {
        let thread_store: Arc<dyn ThreadStore> = self.thread_store.clone();
        let thread_lease: Arc<dyn WriterLease<zeta_protocol::ThreadId>> = self.writer_lease.clone();
        let threads = Arc::new(ThreadController::with_store_and_lease(
            thread_store,
            thread_lease,
        ));
        for thread_id in self.thread_store.list_thread_ids()? {
            threads.recover_thread(&thread_id)?;
        }

        let session_store: Arc<dyn SessionStore> = self.session_store.clone();
        let session_lease: Arc<dyn WriterLease<SessionId>> = self.writer_lease.clone();
        let sessions = Arc::new(SessionCoordinator::with_store_and_lease(
            session_store,
            threads,
            session_lease,
        ));
        for session_id in self.session_store.list_session_ids()? {
            sessions.recover_session(&session_id)?;
        }
        Ok(sessions)
    }
}
