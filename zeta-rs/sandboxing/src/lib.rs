//! Cross-platform sandbox policy and backend coordination.
//!
//! Owns policy, directory scopes and process lifecycle. Implementations are injected by the host.

mod dir;
mod error;
mod manager;
mod model;
mod process;
mod scope;
pub use model::HostAclChanges;
pub use model::SandboxLaunch;
pub use process::ProcessHandle;
pub use process::SandboxProcess;

pub use dir::PROTECTED_DIR_METADATA_NAMES;
pub use error::SandboxError;
pub use manager::{SandboxBackend, SandboxManager};
pub use model::{
    FileSystemAccess, ManagedNetworkAccess, NetworkAccess, PreparedCommand, SandboxCommand,
    SandboxDenialTiming, SandboxKind, SandboxPolicy, SandboxProcessDenial,
    SandboxProcessExitStatus,
};
pub use scope::SandboxDirAccess;
pub use scope::SandboxDirGrant;
pub use scope::SandboxScope;
