use crate::LocalStateError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeta_attachments::FileImageAttachmentStore;
use zeta_attachments::ImageAttachments;
use zeta_core::{SessionCoordinator, ThreadController, ThreadStore, WriterLease};
use zeta_protocol::SessionId;
use zeta_session_store::SessionStore;
use zeta_storage::{LeaseDirectory, SqliteSessionStore, SqliteThreadStore};

/// Opens and recovers local authoritative Session and Thread state under one profile root.
///
/// A repository provides the typed store ports needed by consumers that must inspect durable
/// history. New coordinators must be obtained through [`Self::recover_coordinator`] so every
/// Thread is recovered before Session saga reconciliation begins.
pub struct LocalStateRepository {
    database_path: PathBuf,
    session_store: Arc<SqliteSessionStore>,
    thread_store: Arc<SqliteThreadStore>,
    writer_lease: Arc<LeaseDirectory>,
    image_attachments: Arc<ImageAttachments>,
}

impl LocalStateRepository {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, LocalStateError> {
        let root = root.into();
        let database_path = local_database_path(&root);
        let image_store = FileImageAttachmentStore::open(root.join("attachments"))
            .map_err(|error| zeta_core::CoreError::Journal(error.to_string()))?;
        Ok(Self {
            session_store: Arc::new(SqliteSessionStore::open(&database_path)?),
            thread_store: Arc::new(SqliteThreadStore::open(&database_path)?),
            writer_lease: Arc::new(LeaseDirectory::open(root.join("leases"))?),
            image_attachments: Arc::new(ImageAttachments::new(Arc::new(image_store))),
            database_path,
        })
    }

    /// Returns the shared SQLite database used by the repository's local authorities.
    pub fn database_path(&self) -> &Path {
        &self.database_path
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
    pub fn recover_coordinator(&self) -> Result<Arc<SessionCoordinator>, LocalStateError> {
        self.recover_coordinator_with_image_attachments(Arc::clone(&self.image_attachments))
    }

    /// Recovers state with the exact attachment service used by the owning product host.
    pub fn recover_coordinator_with_image_attachments(
        &self,
        image_attachments: Arc<ImageAttachments>,
    ) -> Result<Arc<SessionCoordinator>, LocalStateError> {
        let thread_store: Arc<dyn ThreadStore> = self.thread_store.clone();
        let thread_lease: Arc<dyn WriterLease<zeta_protocol::ThreadId>> = self.writer_lease.clone();
        let threads = Arc::new(ThreadController::with_store_lease_and_image_attachments(
            thread_store,
            thread_lease,
            image_attachments,
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

fn local_database_path(root: &Path) -> PathBuf {
    root.join("state.sqlite3")
}
