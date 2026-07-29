//! Stable, domain-neutral identity and hard-link information for open files.

use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

/// Stable filesystem identity used to determine whether two handles name the same file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileIdentity {
    device: u64,
    file: u64,
}

/// Identity and hard-link count captured from one open file handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileInformation {
    identity: FileIdentity,
    number_of_links: u64,
}

impl FileInformation {
    /// Captures identity and link count from an already-open file without reopening its path.
    pub fn from_file(file: &File) -> io::Result<Self> {
        platform::inspect(file)
    }

    /// Opens a controlled path and captures identity and link count from the resulting handle.
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_file(&File::open(path)?)
    }

    /// Returns the stable identity associated with the inspected handle.
    pub fn identity(self) -> FileIdentity {
        self.identity
    }

    /// Returns the filesystem hard-link count associated with the inspected handle.
    pub fn number_of_links(self) -> u64 {
        self.number_of_links
    }

    pub(crate) fn new(device: u64, file: u64, number_of_links: u64) -> Self {
        Self {
            identity: FileIdentity { device, file },
            number_of_links,
        }
    }
}

#[cfg(test)]
#[path = "file_identity_tests.rs"]
mod tests;
