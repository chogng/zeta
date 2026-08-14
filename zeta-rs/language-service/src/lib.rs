//! Product-level language-service coordination above the protocol-only `zeta-lsp` runtime.
//!
//! This crate owns enablement, resolved server definition consumption, document snapshot routing,
//! diagnostic freshness, position conversion, and service-thread lifecycle. It does not own
//! editor text, filesystem access, UI presentation, executable discovery, installation, or trust.

mod capabilities;
mod configuration;
mod diagnostics;
mod document;
mod document_features;
mod error;
mod metrics;
mod projection;
mod requests;
mod restart;
mod semantic_tokens;
mod service;
mod workspace_diagnostics;

pub use capabilities::{LanguageServerCapabilities, LanguageServerFeature};
pub use configuration::{LanguageServiceConfiguration, LanguageServiceEnablement};
pub use diagnostics::{
    LanguageDiagnostic, LanguageDiagnosticSeverity, LanguageDiagnostics, LanguageTextRange,
};
pub use document::{LanguageDocumentRevision, LanguageServiceDocument};
pub use document_features::{
    LanguageCodeLens, LanguageCodeLenses, LanguageColor, LanguageColorPresentation,
    LanguageColorPresentations, LanguageCommand, LanguageDocumentColor, LanguageDocumentColors,
    LanguageDocumentLink, LanguageDocumentLinks, LanguageDocumentSymbol, LanguageDocumentSymbols,
    LanguageFoldingRange, LanguageFoldingRangeKind, LanguageFoldingRanges,
};
pub use error::LanguageServiceError;
pub use metrics::LanguageRequestMetric;
pub use metrics::LanguageRequestMetricOutcome;
pub use metrics::LanguageServiceMetricsSink;
pub use requests::{
    LanguageCodeAction, LanguageCodeActions, LanguageCommandResult, LanguageCompletionDetails,
    LanguageCompletionInsertTextFormat, LanguageCompletionItem, LanguageCompletionItemKind,
    LanguageCompletionTrigger, LanguageCompletions, LanguageDeleteMode, LanguageDocumentPosition,
    LanguageExistingTargetBehavior, LanguageFormattingEdits, LanguageFormattingOptions,
    LanguageHierarchyEntry, LanguageHierarchyItem, LanguageHierarchyKind, LanguageHierarchyResult,
    LanguageHover, LanguageInlayHint, LanguageInlayHintKind, LanguageInlayHints,
    LanguageLinkedEditingRanges, LanguageLocationKind, LanguageLocationPosition,
    LanguageLocationRange, LanguageLocationTarget, LanguageLocations,
    LanguageMissingTargetBehavior, LanguageParameterInformation, LanguagePositionEncoding,
    LanguagePulledDiagnosticReport, LanguagePulledDiagnostics, LanguageRenamePreparation,
    LanguageRequestId, LanguageRequestKind, LanguageSignatureHelp, LanguageSignatureHelpTrigger,
    LanguageSignatureInformation, LanguageTextEdit, LanguageWorkspaceDocumentEdit,
    LanguageWorkspaceEdit, LanguageWorkspaceEditEntry, LanguageWorkspaceEditResult,
    LanguageWorkspaceSymbol, LanguageWorkspaceSymbols, LanguageWorkspaceTextEdit,
};
pub use restart::LanguageServerRestartPolicy;
pub use semantic_tokens::{LanguageSemanticToken, LanguageSemanticTokens};
pub use service::{
    LanguageServerMessageSeverity, LanguageServerProgress, LanguageServerState, LanguageService,
    LanguageServiceDocumentOperation, LanguageServiceEvent, LanguageServiceEventSink,
    NoopLanguageServiceEventSink,
};
pub use workspace_diagnostics::{LanguageWorkspaceDiagnostic, LanguageWorkspaceDiagnostics};
pub use zeta_language_server_catalog::LanguageServerDefinition;
