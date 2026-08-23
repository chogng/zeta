//! Handle-derived, domain-neutral identity and hard-link information for open files.

#![deny(unsafe_code)]

use std::fs::File;
use std::io;
use std::path::Path;

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

/// Filesystem identity used to compare observations within one validation operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    volume: u64,
    object: [u8; 16],
}

/// Identity and hard-link count captured from one open file handle.
#[derive(Clone, Copy, Debug)]
pub struct FileInformation {
    identity: FileIdentity,
    number_of_links: u64,
}

impl FileInformation {
    /// Captures identity and link count from an already-open file without reopening its path.
    pub fn from_file(file: &File) -> io::Result<Self> {
        platform::inspect(file)
    }

    /// Opens a path and captures identity and link count from the resulting handle.
    ///
    /// This follows symbolic links and represents only this point-in-time observation. Callers
    /// that make a trust decision about later reads must inspect the handle used for that read
    /// with [`Self::from_file`] and compare the two observations.
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_file(&File::open(path)?)
    }

    /// Returns whether both contemporaneous observations identify the same filesystem object.
    ///
    /// This is not a durable object key: callers must not persist the result across deletion,
    /// replacement, filesystem remounts, or process restarts.
    pub fn same_file_as(self, other: Self) -> bool {
        self.identity == other.identity
    }

    /// Returns whether the observed filesystem object has more than one hard link.
    pub fn has_multiple_links(self) -> bool {
        self.number_of_links > 1
    }

    pub(crate) fn new(volume: u64, object: [u8; 16], number_of_links: u64) -> Self {
        Self {
            identity: FileIdentity { volume, object },
            number_of_links,
        }
    }
}

#[cfg(test)]
#[path = "file_identity_tests.rs"]
mod tests;
