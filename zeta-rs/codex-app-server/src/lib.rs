//! Local adapter for upstream Codex App Server account and Turn contracts.
//!
//! The adapter owns only child-process and JSON-RPC coordination. Upstream
//! Codex continues to own OAuth, callbacks, token persistence, and refresh.

mod login_driver;
mod model_catalog;
mod options;
mod process;
mod runtime;
mod turn_backend;
mod turn_driver;

pub use login_driver::CodexAppServerLoginDriver;
pub use model_catalog::CodexCatalogModel;
pub use model_catalog::CodexModelCatalog;
pub use model_catalog::CodexModelCatalogError;
pub use options::CodexAppServerOptions;
pub use runtime::CodexAppServerRuntime;
pub use turn_backend::CodexTurnExecutionBackend;
pub use turn_backend::CodexTurnExecutionBackendOptions;
pub use turn_backend::CodexTurnWorkspace;
pub use turn_backend::CodexTurnWorkspaceSource;
pub use turn_driver::CodexApprovalDecision;
pub use turn_driver::CodexCommandApprovalRequest;
pub use turn_driver::CodexFileChangeApprovalRequest;
pub use turn_driver::CodexServerRequestId;
pub use turn_driver::CodexThreadAccess;
pub use turn_driver::CodexThreadId;
pub use turn_driver::CodexTurnDriver;
pub use turn_driver::CodexTurnError;
pub use turn_driver::CodexTurnErrorKind;
pub use turn_driver::CodexTurnEvent;
pub use turn_driver::CodexTurnId;
pub use turn_driver::CodexTurnStatus;
pub use turn_driver::CodexUserInputAnswers;
pub use turn_driver::CodexUserInputOption;
pub use turn_driver::CodexUserInputQuestion;
pub use turn_driver::CodexUserInputRequest;
pub use turn_driver::StartCodexThread;
pub use turn_driver::StartCodexTurn;
#[cfg(test)]
#[path = "login_driver_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "model_catalog_tests.rs"]
mod model_catalog_tests;

#[cfg(test)]
#[path = "turn_driver_tests.rs"]
mod turn_driver_tests;

#[cfg(test)]
#[path = "turn_backend_tests.rs"]
mod turn_backend_tests;
