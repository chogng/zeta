/// Stable entry kind shared across local and future remote filesystems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    Directory,
    File,
    SymbolicLink,
    Other,
}

/// Metadata for one existing workspace path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    pub file_type: FileType,
    pub size_bytes: u64,
    pub readonly: bool,
    pub modified_at_millis: Option<u64>,
}

/// One direct child returned by a directory read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    pub name: String,
    pub file_type: FileType,
}
