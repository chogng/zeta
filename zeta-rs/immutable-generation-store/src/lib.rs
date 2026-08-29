//! Immutable base generations with atomically selected change-layer snapshots.

#![deny(unsafe_code)]

mod layout;
mod mapped_file;
mod store;

pub use mapped_file::MappedGenerationFile;
pub use mapped_file::OpenGenerationFile;
pub use store::GenerationFile;
pub use store::ImmutableGenerationStore;
pub use store::PublishedSnapshot;
