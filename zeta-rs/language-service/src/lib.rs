//! Product-level language-service coordination above the protocol-only `zeta-lsp` runtime.
//!
//! This crate owns enablement, resolved server definition consumption, document snapshot routing,
//! diagnostic freshness, position conversion, and service-thread lifecycle. It does not own
//! editor text, filesystem access, UI presentation, executable discovery, installation, or trust.

mod configuration;
mod diagnostics;
mod document;
mod error;
mod projection;
mod requests;
mod restart;
mod service;

pub use configuration::{LanguageServiceConfiguration, LanguageServiceEnablement};
pub use diagnostics::{
    LanguageDiagnostic, LanguageDiagnosticSeverity, LanguageDiagnostics, LanguageTextRange,
};
pub use document::{LanguageDocumentRevision, LanguageServiceDocument};
pub use error::LanguageServiceError;
pub use requests::{
    LanguageCodeAction, LanguageCodeActions, LanguageCompletionItem, LanguageCompletions,
    LanguageDeleteMode, LanguageDocumentPosition, LanguageExistingTargetBehavior,
    LanguageHierarchyEntry, LanguageHierarchyItem, LanguageHierarchyKind, LanguageHierarchyResult,
    LanguageHover, LanguageLocationKind, LanguageLocationPosition, LanguageLocationRange,
    LanguageLocationTarget, LanguageLocations, LanguageMissingTargetBehavior,
    LanguagePositionEncoding, LanguageRenamePreparation, LanguageRequestId, LanguageRequestKind,
    LanguageTextEdit, LanguageWorkspaceDocumentEdit, LanguageWorkspaceEdit,
    LanguageWorkspaceEditEntry, LanguageWorkspaceEditResult, LanguageWorkspaceSymbol,
    LanguageWorkspaceSymbols, LanguageWorkspaceTextEdit,
};
pub use restart::LanguageServerRestartPolicy;
pub use service::{
    LanguageServerState, LanguageService, LanguageServiceDocumentOperation, LanguageServiceEvent,
    LanguageServiceEventSink, NoopLanguageServiceEventSink,
};
pub use zeta_language_server_catalog::LanguageServerDefinition;
