use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use zeta_core::{CoreError, LeaseGuard, ThreadWriterLease};
use zeta_protocol::ThreadId;

pub struct ThreadLeaseDirectory {
    root: PathBuf,
}
pub struct FileThreadLease {
    path: PathBuf,
}

impl ThreadLeaseDirectory {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| CoreError::Journal(error.to_string()))?;
        Ok(Self { root })
    }
}

impl ThreadWriterLease for ThreadLeaseDirectory {
    fn acquire(&self, thread_id: &ThreadId) -> Result<Box<dyn LeaseGuard>, CoreError> {
        let path = self.root.join(format!("{}.lease", thread_id.as_str()));
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                CoreError::Journal(format!("thread writer lease unavailable: {error}"))
            })?;
        Ok(Box::new(FileThreadLease { path }))
    }
}

impl LeaseGuard for FileThreadLease {}
impl Drop for FileThreadLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
