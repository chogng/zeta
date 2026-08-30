use crate::LocalStateError;
use crate::lease::LeaseDirectory;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeta_attachments::FileImageAttachmentStore;
use zeta_attachments::ImageAttachments;
use zeta_core::{ThreadController, ThreadStore, WriterLease};
use zeta_state::{SqliteThreadStore, StateRuntime};

/// Opens and recovers local authoritative Thread state under one profile root.
///
/// A repository provides the typed store ports needed by consumers that must inspect durable
/// history. New controllers must be obtained through [`Self::recover_threads`] so every durable
/// Thread is available before product services resume work.
pub struct LocalStateRepository {
    database_path: PathBuf,
    thread_store: Arc<SqliteThreadStore>,
    writer_lease: Arc<LeaseDirectory>,
    image_attachments: Arc<ImageAttachments>,
}

impl LocalStateRepository {
    pub fn open(state: &StateRuntime) -> Result<Self, LocalStateError> {
        let root = state.profile_root();
        let database_path = state.database_path().to_path_buf();
        let image_store = FileImageAttachmentStore::open(root.join("attachments"))
            .map_err(|error| zeta_core::CoreError::Journal(error.to_string()))?;
        Ok(Self {
            thread_store: Arc::new(SqliteThreadStore::open(&database_path)?),
            writer_lease: Arc::new(LeaseDirectory::open(state.writer_leases_root())?),
            image_attachments: Arc::new(ImageAttachments::new(Arc::new(image_store))),
            database_path,
        })
    }

    /// Returns the shared SQLite database used by the repository's local authorities.
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Returns the read/write Thread history port for this repository.
    pub fn thread_store(&self) -> Arc<dyn ThreadStore> {
        self.thread_store.clone()
    }

    /// Recovers the Core Thread authority from this repository's durable history.
    pub fn recover_threads(&self) -> Result<Arc<ThreadController>, LocalStateError> {
        self.recover_threads_with_image_attachments(Arc::clone(&self.image_attachments))
    }

    /// Recovers state with the exact attachment service used by the owning product host.
    pub fn recover_threads_with_image_attachments(
        &self,
        image_attachments: Arc<ImageAttachments>,
    ) -> Result<Arc<ThreadController>, LocalStateError> {
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

        Ok(threads)
    }
}
