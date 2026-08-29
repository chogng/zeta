use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileStamp {
    pub(crate) length: u64,
    pub(crate) modified_nanos: u64,
    pub(crate) change_nanos: u64,
}

impl FileStamp {
    pub(crate) fn read(path: &Path) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified_nanos = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64;
        Ok(Self {
            length: metadata.len(),
            modified_nanos,
            change_nanos: change_nanos(&metadata),
        })
    }
}

#[cfg(unix)]
fn change_nanos(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    let seconds = metadata.ctime().max(0) as u64;
    let nanos = metadata.ctime_nsec().max(0) as u64;
    seconds.saturating_mul(1_000_000_000).saturating_add(nanos)
}

#[cfg(not(unix))]
fn change_nanos(_metadata: &fs::Metadata) -> u64 {
    0
}
