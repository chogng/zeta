//! Host-filesystem path comparison, canonical containment, symlink-aware write targeting,
//! and atomic writes.

mod canonical_root;
mod comparison;
mod environment;
mod persistence;

pub use canonical_root::CanonicalContainmentError;
pub use canonical_root::CanonicalPathRoot;
pub use comparison::normalize_for_native_workdir;
pub use comparison::normalize_for_path_comparison;
pub use comparison::paths_match_after_normalization;
pub use environment::is_wsl;
pub use persistence::SymlinkWritePaths;
pub use persistence::resolve_symlink_write_paths;
pub use persistence::write_atomically;
pub use persistence::write_text_atomically;

#[cfg(test)]
#[path = "path_utils_tests.rs"]
mod tests;
