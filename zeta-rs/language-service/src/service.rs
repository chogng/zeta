use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc as std_mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use zeta_lsp::lsp_types::{
    MessageType, NumberOrString, ProgressParamsValue, PublishDiagnosticsParams, Uri,
    WorkDoneProgress,
};
use zeta_lsp::{
    EditorDocumentRevision, LanguageDocumentSnapshot, LanguageServerClient,
    LanguageServerDocumentRouter, LanguageServerEvent, LanguageServerHost, LanguageServerName,
    LanguageServerOptions, LanguageServerRoute,
};

use crate::projection::project_diagnostic;
use crate::restart::{RestartDecision, ServerRestartTracker};
use crate::{
    LanguageCodeActions, LanguageCodeLens, LanguageCodeLenses, LanguageColor,
    LanguageColorPresentations, LanguageCommand, LanguageCommandResult, LanguageCompletionDetails,
    LanguageCompletionTrigger, LanguageCompletions, LanguageDiagnostic, LanguageDiagnostics,
    LanguageDocumentColors, LanguageDocumentLink, LanguageDocumentLinks, LanguageDocumentPosition,
    LanguageDocumentRevision, LanguageDocumentSymbols, LanguageFoldingRanges,
    LanguageFormattingEdits, LanguageFormattingOptions, LanguageHierarchyItem,
    LanguageHierarchyResult, LanguageHover, LanguageInlayHints, LanguageLinkedEditingRanges,
    LanguageLocationRange, LanguageLocations, LanguagePulledDiagnostics, LanguageRenamePreparation,
    LanguageRequestId, LanguageRequestKind, LanguageSemanticTokens, LanguageServerCapabilities,
    LanguageServerDefinition, LanguageServiceConfiguration, LanguageServiceDocument,
    LanguageServiceEnablement, LanguageServiceError, LanguageSignatureHelp,
    LanguageSignatureHelpTrigger, LanguageTextRange, LanguageWorkspaceDiagnostics,
    LanguageWorkspaceEditResult, LanguageWorkspaceSymbols,
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

mod lifecycle;
mod request_runtime;
mod workspace_diagnostic_request;

use request_runtime::{CompletedLanguageRequest, PendingLanguageRequest};

/// Product-visible lifecycle state of one configured language server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageServerState {
    Starting,
    Ready,
    BackingOff {
        attempt: u32,
        retry_after: Duration,
    },
    CrashLoop {
        restart_attempts: u32,
        message: String,
    },
    Failed(String),
    Stopped,
}

/// Document operation that failed after crossing the asynchronous service boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageServiceDocumentOperation {
    Synchronize,
    Save,
    Close,
}

/// Presentation-neutral severity for a language-server message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageServerMessageSeverity {
    Error,
    Warning,
    Information,
    Log,
}

/// Product-visible work-done progress state for one server-owned token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageServerProgress {
    pub server: String,
    pub token: String,
    pub title: Option<String>,
    pub message: Option<String>,
    pub percentage: Option<u32>,
    pub done: bool,
}

/// Product-level events emitted after protocol details and stale results have been resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageServiceEvent {
    ServerStateChanged {
        server: String,
        state: LanguageServerState,
    },
    Diagnostics(LanguageDiagnostics),
    PulledDiagnostics(LanguagePulledDiagnostics),
    ServerMessage {
        server: String,
        severity: LanguageServerMessageSeverity,
        show: bool,
        message: String,
    },
    ServerProgress(LanguageServerProgress),
    CapabilitiesChanged {
        server: String,
        capabilities: LanguageServerCapabilities,
    },
    DocumentOperationFailed {
        path: PathBuf,
        operation: LanguageServiceDocumentOperation,
        message: String,
    },
    Hover(LanguageHover),
    Completions(LanguageCompletions),
    CompletionDetails(LanguageCompletionDetails),
    CommandResult(LanguageCommandResult),
    Locations(LanguageLocations),
    Hierarchy(LanguageHierarchyResult),
    WorkspaceSymbols(LanguageWorkspaceSymbols),
    WorkspaceDiagnostics(LanguageWorkspaceDiagnostics),
    RenamePreparation(LanguageRenamePreparation),
    WorkspaceEdit(LanguageWorkspaceEditResult),
    CodeActions(LanguageCodeActions),
    FormattingEdits(LanguageFormattingEdits),
    SignatureHelp(LanguageSignatureHelp),
    InlayHints(LanguageInlayHints),
    LinkedEditingRanges(LanguageLinkedEditingRanges),
    SemanticTokens(LanguageSemanticTokens),
    DocumentSymbols(LanguageDocumentSymbols),
    CodeLenses(LanguageCodeLenses),
    DocumentLinks(LanguageDocumentLinks),
    DocumentColors(LanguageDocumentColors),
    ColorPresentations(LanguageColorPresentations),
    FoldingRanges(LanguageFoldingRanges),
    RequestFailed {
        request_id: LanguageRequestId,
        kind: LanguageRequestKind,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        message: String,
    },
}

/// Non-blocking destination for product-level language-service events.
///
/// Implementations are expected to enqueue the event into their UI or application event loop and
/// return immediately. They must not call blocking language-service methods from this callback.
pub trait LanguageServiceEventSink: Send + Sync + 'static {
    fn on_event(&self, event: LanguageServiceEvent);
}

/// Event sink used by hosts that intentionally ignore language-service output.
#[derive(Debug, Default)]
pub struct NoopLanguageServiceEventSink;

impl LanguageServiceEventSink for NoopLanguageServiceEventSink {
    fn on_event(&self, _event: LanguageServiceEvent) {}
}

/// Product-level language-service supervisor with a non-blocking document API.
///
/// The supervisor thread owns the Tokio runtime, `zeta-lsp` clients, router, and document bindings.
/// Callers retain authoritative text and send full snapshots whenever their revision changes.
pub struct LanguageService {
    commands: mpsc::UnboundedSender<SupervisorCommand>,
    thread: Option<JoinHandle<()>>,
    next_request_id: AtomicU64,
}

impl LanguageService {
    pub fn start(
        configuration: LanguageServiceConfiguration,
        events: Arc<dyn LanguageServiceEventSink>,
    ) -> Result<Self, LanguageServiceError> {
        validate_catalog(&configuration)?;
        let (commands, receiver) = mpsc::unbounded_channel();
        let (started_tx, started_rx) = std_mpsc::sync_channel(1);
        let thread_commands = commands.clone();
        let thread = std::thread::Builder::new()
            .name("zeta-language-service".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .thread_name("zeta-language-service-worker")
                    .build();
                match runtime {
                    Ok(runtime) => {
                        let _ = started_tx.send(Ok(()));
                        runtime.block_on(
                            Supervisor::new(configuration, events, thread_commands).run(receiver),
                        );
                    }
                    Err(error) => {
                        let _ = started_tx.send(Err(error));
                    }
                }
            })
            .map_err(LanguageServiceError::RuntimeStart)?;
        match started_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                commands,
                thread: Some(thread),
                next_request_id: AtomicU64::new(1),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(LanguageServiceError::RuntimeStart(error))
            }
            Err(_) => {
                let _ = thread.join();
                Err(LanguageServiceError::Closed)
            }
        }
    }

    pub fn synchronize_document(
        &self,
        document: LanguageServiceDocument,
    ) -> Result<(), LanguageServiceError> {
        self.send(SupervisorCommand::Synchronize(document))
    }

    pub fn save_document(&self, path: impl Into<PathBuf>) -> Result<(), LanguageServiceError> {
        self.send(SupervisorCommand::Save(path.into()))
    }

    pub fn close_document(&self, path: impl Into<PathBuf>) -> Result<(), LanguageServiceError> {
        self.send(SupervisorCommand::Close(path.into()))
    }

    pub fn set_enablement(
        &self,
        enablement: LanguageServiceEnablement,
    ) -> Result<(), LanguageServiceError> {
        self.send(SupervisorCommand::SetEnablement(enablement))
    }

    pub fn request_hover(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Hover {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_completions(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        trigger: LanguageCompletionTrigger,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Completion {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
            trigger,
        })
    }

    pub fn request_resolve_completion(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        provider_data: serde_json::Value,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ResolveCompletion {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            provider_data,
        })
    }

    pub fn request_execute_command(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        command: LanguageCommand,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ExecuteCommand {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            command,
        })
    }

    pub fn request_definition(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Definition {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_declaration(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Declaration {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_implementation(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Implementation {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_type_definition(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::TypeDefinition {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_references(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        include_declaration: bool,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::References {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
            include_declaration,
        })
    }

    pub fn request_prepare_call_hierarchy(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::PrepareCallHierarchy {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_incoming_calls(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::IncomingCalls {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            item,
        })
    }

    pub fn request_outgoing_calls(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::OutgoingCalls {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            item,
        })
    }

    pub fn request_prepare_type_hierarchy(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::PrepareTypeHierarchy {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_supertypes(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Supertypes {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            item,
        })
    }

    pub fn request_subtypes(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Subtypes {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            item,
        })
    }

    pub fn request_workspace_symbols(
        &self,
        language_id: impl Into<String>,
        query: impl Into<String>,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        let id = self.next_request_id();
        self.send(SupervisorCommand::WorkspaceSymbols {
            id,
            language_id: language_id.into(),
            query: query.into(),
        })?;
        Ok(id)
    }

    pub fn request_workspace_diagnostics(
        &self,
        language_id: impl Into<String>,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        let id = self.next_request_id();
        self.send(SupervisorCommand::WorkspaceDiagnostics {
            id,
            language_id: language_id.into(),
        })?;
        Ok(id)
    }

    pub fn request_prepare_rename(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::PrepareRename {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_rename(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        new_name: impl Into<String>,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::Rename {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
            new_name: new_name.into(),
        })
    }

    pub fn request_code_actions(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        range: LanguageLocationRange,
        diagnostics: Vec<LanguageDiagnostic>,
        only: Vec<String>,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::CodeActions {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            range,
            diagnostics,
            only,
        })
    }

    pub fn request_resolve_code_action(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        provider_data: serde_json::Value,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ResolveCodeAction {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            provider_data,
        })
    }

    pub fn request_document_formatting(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        options: LanguageFormattingOptions,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::DocumentFormatting {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            options,
        })
    }

    pub fn request_range_formatting(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        range: LanguageTextRange,
        options: LanguageFormattingOptions,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::RangeFormatting {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            range,
            options,
        })
    }

    pub fn request_signature_help(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        trigger: LanguageSignatureHelpTrigger,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::SignatureHelp {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
            trigger,
        })
    }

    pub fn request_inlay_hints(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        range: LanguageTextRange,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::InlayHints {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            range,
        })
    }

    pub fn request_linked_editing_ranges(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::LinkedEditingRanges {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            position,
        })
    }

    pub fn request_semantic_tokens(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::SemanticTokens {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn request_document_symbols(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::DocumentSymbols {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn request_code_lenses(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::CodeLenses {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn resolve_code_lens(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        lens: LanguageCodeLens,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ResolveCodeLens {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            lens,
        })
    }

    pub fn request_document_links(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::DocumentLinks {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn resolve_document_link(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        link: LanguageDocumentLink,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ResolveDocumentLink {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            link,
        })
    }

    pub fn request_document_colors(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::DocumentColors {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn request_color_presentations(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
        range: LanguageTextRange,
        color: LanguageColor,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::ColorPresentations {
            id: self.next_request_id(),
            path: path.into(),
            revision,
            range,
            color,
        })
    }

    pub fn request_folding_ranges(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::FoldingRanges {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn request_document_diagnostics(
        &self,
        path: impl Into<PathBuf>,
        revision: LanguageDocumentRevision,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        self.queue_request(PendingLanguageRequest::DocumentDiagnostics {
            id: self.next_request_id(),
            path: path.into(),
            revision,
        })
    }

    pub fn shutdown(mut self) -> Result<(), LanguageServiceError> {
        let (completion, response) = std_mpsc::sync_channel(1);
        self.send(SupervisorCommand::Shutdown { completion })?;
        response
            .recv_timeout(SHUTDOWN_TIMEOUT)
            .map_err(|_| LanguageServiceError::ShutdownTimeout)?;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        Ok(())
    }

    fn send(&self, command: SupervisorCommand) -> Result<(), LanguageServiceError> {
        self.commands
            .send(command)
            .map_err(|_| LanguageServiceError::Closed)
    }

    fn queue_request(
        &self,
        request: PendingLanguageRequest,
    ) -> Result<LanguageRequestId, LanguageServiceError> {
        let id = request.id();
        self.send(SupervisorCommand::LanguageRequest(request))?;
        Ok(id)
    }

    fn next_request_id(&self) -> LanguageRequestId {
        LanguageRequestId::new(self.next_request_id.fetch_add(1, Ordering::Relaxed))
    }
}

impl Drop for LanguageService {
    fn drop(&mut self) {
        if self.thread.is_some() {
            let (completion, _) = std_mpsc::sync_channel(1);
            let _ = self
                .commands
                .send(SupervisorCommand::Shutdown { completion });
        }
    }
}

enum SupervisorCommand {
    Synchronize(LanguageServiceDocument),
    Save(PathBuf),
    Close(PathBuf),
    SetEnablement(LanguageServiceEnablement),
    ProtocolEvent {
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        event: LanguageServerEvent,
    },
    ServerStarted {
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        route: LanguageServerRoute,
        result: Result<LanguageServerClient, String>,
    },
    RetryServer {
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
    },
    LanguageRequest(PendingLanguageRequest),
    WorkspaceSymbols {
        id: LanguageRequestId,
        language_id: String,
        query: String,
    },
    WorkspaceSymbolsCompleted {
        id: LanguageRequestId,
        query: String,
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<LanguageWorkspaceSymbols, String>,
    },
    WorkspaceDiagnostics {
        id: LanguageRequestId,
        language_id: String,
    },
    WorkspaceDiagnosticsCompleted {
        id: LanguageRequestId,
        language_id: String,
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<LanguageWorkspaceDiagnostics, String>,
    },
    LanguageRequestCompleted {
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<CompletedLanguageRequest, String>,
    },
    Shutdown {
        completion: std_mpsc::SyncSender<()>,
    },
}

struct DocumentState {
    document: LanguageServiceDocument,
    uri: Uri,
    routed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedServerPhase {
    Stopped,
    Starting,
    Ready,
    BackingOff,
    Terminal,
}

struct ManagedServer {
    definition: LanguageServerDefinition,
    epoch: u64,
    phase: ManagedServerPhase,
    restart: ServerRestartTracker,
}

struct Supervisor {
    configuration: LanguageServiceConfiguration,
    events: Arc<dyn LanguageServiceEventSink>,
    commands: mpsc::UnboundedSender<SupervisorCommand>,
    router: LanguageServerDocumentRouter,
    documents: BTreeMap<PathBuf, DocumentState>,
    uri_paths: HashMap<Uri, PathBuf>,
    servers: BTreeMap<LanguageServerName, ManagedServer>,
    launches: HashMap<LanguageServerName, tokio::task::JoinHandle<()>>,
    retry_tasks: HashMap<LanguageServerName, tokio::task::JoinHandle<()>>,
    generation: u64,
}

impl Supervisor {
    fn new(
        configuration: LanguageServiceConfiguration,
        events: Arc<dyn LanguageServiceEventSink>,
        commands: mpsc::UnboundedSender<SupervisorCommand>,
    ) -> Self {
        let servers = configuration
            .servers
            .iter()
            .cloned()
            .map(|definition| {
                (
                    definition.name().clone(),
                    ManagedServer {
                        definition,
                        epoch: 0,
                        phase: ManagedServerPhase::Stopped,
                        restart: ServerRestartTracker::default(),
                    },
                )
            })
            .collect();
        Self {
            configuration,
            events,
            commands,
            router: LanguageServerDocumentRouter::default(),
            documents: BTreeMap::new(),
            uri_paths: HashMap::new(),
            servers,
            launches: HashMap::new(),
            retry_tasks: HashMap::new(),
            generation: 0,
        }
    }

    async fn run(mut self, mut commands: mpsc::UnboundedReceiver<SupervisorCommand>) {
        if self.configuration.enablement == LanguageServiceEnablement::Enabled {
            self.enable().await;
        }
        while let Some(command) = commands.recv().await {
            match command {
                SupervisorCommand::Synchronize(document) => self.synchronize(document).await,
                SupervisorCommand::Save(path) => self.save(&path).await,
                SupervisorCommand::Close(path) => self.close(&path).await,
                SupervisorCommand::SetEnablement(enablement) => {
                    self.set_enablement(enablement).await;
                }
                SupervisorCommand::ProtocolEvent {
                    server,
                    generation,
                    server_epoch,
                    event,
                } if generation == self.generation => {
                    self.handle_protocol_event(server, server_epoch, event)
                        .await;
                }
                SupervisorCommand::ProtocolEvent { .. } => {}
                SupervisorCommand::ServerStarted {
                    server,
                    generation,
                    server_epoch,
                    route,
                    result,
                } => {
                    self.handle_server_started(server, generation, server_epoch, route, result)
                        .await;
                }
                SupervisorCommand::RetryServer {
                    server,
                    generation,
                    server_epoch,
                } if generation == self.generation => {
                    self.retry_server(&server, server_epoch);
                }
                SupervisorCommand::RetryServer { .. } => {}
                SupervisorCommand::LanguageRequest(request) => {
                    self.begin_language_request(request);
                }
                SupervisorCommand::WorkspaceSymbols {
                    id,
                    language_id,
                    query,
                } => {
                    self.begin_workspace_symbols(id, language_id, query);
                }
                SupervisorCommand::WorkspaceSymbolsCompleted {
                    id,
                    query,
                    server,
                    generation,
                    server_epoch,
                    result,
                } => {
                    self.complete_workspace_symbols(
                        id,
                        query,
                        server,
                        generation,
                        server_epoch,
                        result,
                    );
                }
                SupervisorCommand::WorkspaceDiagnostics { id, language_id } => {
                    self.begin_workspace_diagnostics(id, language_id);
                }
                SupervisorCommand::WorkspaceDiagnosticsCompleted {
                    id,
                    language_id,
                    server,
                    generation,
                    server_epoch,
                    result,
                } => {
                    self.complete_workspace_diagnostics(
                        id,
                        language_id,
                        server,
                        generation,
                        server_epoch,
                        result,
                    );
                }
                SupervisorCommand::LanguageRequestCompleted {
                    server,
                    generation,
                    server_epoch,
                    result,
                } => {
                    self.complete_language_request(server, generation, server_epoch, result);
                }
                SupervisorCommand::Shutdown { completion } => {
                    self.disable().await;
                    let _ = completion.send(());
                    return;
                }
            }
        }
        self.disable().await;
    }

    async fn set_enablement(&mut self, enablement: LanguageServiceEnablement) {
        if self.configuration.enablement == enablement {
            return;
        }
        self.configuration.enablement = enablement;
        match enablement {
            LanguageServiceEnablement::Disabled => self.disable().await,
            LanguageServiceEnablement::Enabled => self.enable().await,
        }
    }

    async fn synchronize(&mut self, document: LanguageServiceDocument) {
        let path = self.absolute_path(document.path());
        if let Some(current) = self.documents.get(&path)
            && document.revision() <= current.document.revision()
        {
            if document.revision() == current.document.revision()
                && document.text() == current.document.text()
                && document.language_id() == current.document.language_id()
            {
                return;
            }
            self.emit_document_failure(
                path,
                LanguageServiceDocumentOperation::Synchronize,
                "document revision did not advance".into(),
            );
            return;
        }
        let uri = match file_uri(&path) {
            Ok(uri) => uri,
            Err(error) => {
                self.emit_document_failure(
                    path,
                    LanguageServiceDocumentOperation::Synchronize,
                    error.to_string(),
                );
                return;
            }
        };
        let language_changed = self
            .documents
            .get(&path)
            .is_some_and(|current| current.document.language_id() != document.language_id());
        if language_changed {
            self.close_routed(&path).await;
        }
        self.uri_paths.insert(uri.clone(), path.clone());
        let was_routed = self
            .documents
            .get(&path)
            .is_some_and(|current| current.routed);
        self.documents.insert(
            path.clone(),
            DocumentState {
                document,
                uri,
                routed: was_routed && !language_changed,
            },
        );
        if self.configuration.enablement == LanguageServiceEnablement::Enabled {
            self.route_current_document(&path).await;
        }
    }

    async fn route_current_document(&mut self, path: &Path) {
        let Some(current) = self.documents.get(path) else {
            return;
        };
        if !self.supports_language(current.document.language_id()) {
            return;
        }
        let snapshot = match router_snapshot(current) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.emit_document_failure(
                    path.to_path_buf(),
                    LanguageServiceDocumentOperation::Synchronize,
                    error.to_string(),
                );
                return;
            }
        };
        let result = if current.routed {
            self.router.update_document(snapshot).await
        } else {
            self.router.open_document(snapshot).await
        };
        match result {
            Ok(_) => {
                if let Some(current) = self.documents.get_mut(path) {
                    current.routed = true;
                }
            }
            Err(error) => self.emit_document_failure(
                path.to_path_buf(),
                LanguageServiceDocumentOperation::Synchronize,
                error.to_string(),
            ),
        }
    }

    async fn save(&mut self, requested: &Path) {
        let path = self.absolute_path(requested);
        let Some(document) = self.documents.get(&path) else {
            return;
        };
        if !document.routed {
            return;
        }
        if let Err(error) = self.router.save_document(&document.uri).await {
            self.emit_document_failure(
                path,
                LanguageServiceDocumentOperation::Save,
                error.to_string(),
            );
        }
    }

    async fn close(&mut self, requested: &Path) {
        let path = self.absolute_path(requested);
        self.close_routed(&path).await;
        if let Some(document) = self.documents.remove(&path) {
            self.uri_paths.remove(&document.uri);
        }
    }

    async fn close_routed(&mut self, path: &Path) {
        let Some(document) = self.documents.get(path) else {
            return;
        };
        if !document.routed {
            return;
        }
        if let Err(error) = self.router.close_document(&document.uri).await {
            self.emit_document_failure(
                path.to_path_buf(),
                LanguageServiceDocumentOperation::Close,
                error.to_string(),
            );
        }
        if let Some(document) = self.documents.get_mut(path) {
            document.routed = false;
        }
    }

    async fn handle_protocol_event(
        &mut self,
        server: LanguageServerName,
        server_epoch: u64,
        event: LanguageServerEvent,
    ) {
        let Some(phase) = self
            .servers
            .get(&server)
            .filter(|managed| managed.epoch == server_epoch)
            .map(|managed| managed.phase)
        else {
            return;
        };
        match (phase, event) {
            (ManagedServerPhase::Starting, LanguageServerEvent::TransportClosed { message }) => {
                self.schedule_failure(&server, server_epoch, message)
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::TransportClosed { message }) => {
                self.handle_server_disconnect(&server, server_epoch, message)
                    .await;
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::Diagnostics(params)) => {
                self.publish_diagnostics(params);
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::LogMessage(message)) => {
                self.emit_server_message(server, message.typ, false, message.message)
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::ShowMessage(message)) => {
                self.emit_server_message(server, message.typ, true, message.message)
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::DynamicCapabilitiesChanged(_)) => {
                self.emit_server_capabilities(&server, server_epoch)
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::ServerStderr(message)) => {
                self.emit_server_message(server, MessageType::LOG, false, message)
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::Telemetry(_))
            | (ManagedServerPhase::Ready, LanguageServerEvent::WorkDoneProgressCreated(_))
            | (ManagedServerPhase::Ready, LanguageServerEvent::UnhandledNotification { .. })
            | (ManagedServerPhase::Ready, LanguageServerEvent::UnsupportedServerRequest { .. }) => {
            }
            (ManagedServerPhase::Ready, LanguageServerEvent::Progress(progress)) => {
                let ProgressParamsValue::WorkDone(progress_value) = progress.value;
                let (title, message, percentage, done) = match progress_value {
                    WorkDoneProgress::Begin(progress) => (
                        Some(progress.title),
                        progress.message,
                        progress.percentage,
                        false,
                    ),
                    WorkDoneProgress::Report(progress) => {
                        (None, progress.message, progress.percentage, false)
                    }
                    WorkDoneProgress::End(progress) => (None, progress.message, None, true),
                };
                self.emit(LanguageServiceEvent::ServerProgress(
                    LanguageServerProgress {
                        server: server.to_string(),
                        token: progress_token(progress.token),
                        title,
                        message,
                        percentage,
                        done,
                    },
                ));
            }
            _ => {}
        }
    }

    fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        let Some(path) = self.uri_paths.get(&params.uri) else {
            return;
        };
        let Some(document) = self.documents.get(path) else {
            return;
        };
        let Ok(binding) = self.router.document_version(&params.uri) else {
            return;
        };
        if params
            .version
            .is_some_and(|version| version != binding.server_version().value())
        {
            return;
        }
        let Ok(client) = self.router.client_for_document(&params.uri) else {
            return;
        };
        let encoding = &client.initialization().position_encoding;
        let diagnostics = params
            .diagnostics
            .into_iter()
            .filter_map(|diagnostic| {
                project_diagnostic(document.document.text(), diagnostic, encoding)
            })
            .collect();
        self.emit(LanguageServiceEvent::Diagnostics(LanguageDiagnostics::new(
            path.clone(),
            document.document.revision(),
            diagnostics,
        )));
    }

    fn absolute_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.configuration.workspace_root.join(path)
        }
    }

    fn supports_language(&self, language_id: &str) -> bool {
        self.servers.values().any(|server| {
            server.phase == ManagedServerPhase::Ready
                && server
                    .definition
                    .language_ids()
                    .any(|known| known == language_id)
        })
    }

    fn emit_server_state(&self, server: &LanguageServerName, state: LanguageServerState) {
        self.emit(LanguageServiceEvent::ServerStateChanged {
            server: server.to_string(),
            state,
        });
    }

    fn emit_server_message(
        &self,
        server: LanguageServerName,
        message_type: MessageType,
        show: bool,
        message: String,
    ) {
        self.emit(LanguageServiceEvent::ServerMessage {
            server: server.to_string(),
            severity: language_server_message_severity(message_type),
            show,
            message,
        });
    }

    fn emit_server_capabilities(&self, server: &LanguageServerName, server_epoch: u64) {
        let Ok(client) = self.router.client_for_server(server) else {
            return;
        };
        self.emit(LanguageServiceEvent::CapabilitiesChanged {
            server: server.to_string(),
            capabilities: request_runtime::capability_snapshot(client, server_epoch),
        });
    }

    fn emit_document_failure(
        &self,
        path: PathBuf,
        operation: LanguageServiceDocumentOperation,
        message: String,
    ) {
        self.emit(LanguageServiceEvent::DocumentOperationFailed {
            path,
            operation,
            message,
        });
    }

    fn emit(&self, event: LanguageServiceEvent) {
        self.events.on_event(event);
    }
}

fn language_server_message_severity(message_type: MessageType) -> LanguageServerMessageSeverity {
    if message_type == MessageType::ERROR {
        LanguageServerMessageSeverity::Error
    } else if message_type == MessageType::WARNING {
        LanguageServerMessageSeverity::Warning
    } else if message_type == MessageType::INFO {
        LanguageServerMessageSeverity::Information
    } else {
        LanguageServerMessageSeverity::Log
    }
}

fn progress_token(token: NumberOrString) -> String {
    match token {
        NumberOrString::Number(token) => token.to_string(),
        NumberOrString::String(token) => token,
    }
}

struct ProtocolEventBridge {
    server: LanguageServerName,
    generation: u64,
    server_epoch: u64,
    commands: mpsc::UnboundedSender<SupervisorCommand>,
}

impl LanguageServerHost for ProtocolEventBridge {
    fn on_event(&self, event: LanguageServerEvent) {
        let _ = self.commands.send(SupervisorCommand::ProtocolEvent {
            server: self.server.clone(),
            generation: self.generation,
            server_epoch: self.server_epoch,
            event,
        });
    }
}

fn validate_catalog(
    configuration: &LanguageServiceConfiguration,
) -> Result<(), LanguageServiceError> {
    let mut names = BTreeSet::new();
    let mut languages = BTreeSet::new();
    for definition in &configuration.servers {
        if !names.insert(definition.name().to_string()) {
            return Err(LanguageServiceError::DuplicateServer(
                definition.name().to_string(),
            ));
        }
        for language in definition.language_ids() {
            if !languages.insert(language.to_owned()) {
                return Err(LanguageServiceError::DuplicateLanguage(language.to_owned()));
            }
        }
    }
    file_uri(&configuration.workspace_root).map(|_| ())
}

fn router_snapshot(
    current: &DocumentState,
) -> Result<LanguageDocumentSnapshot, LanguageServiceError> {
    Ok(LanguageDocumentSnapshot::new(
        current.uri.clone(),
        current.document.language_id(),
        EditorDocumentRevision::new(current.document.revision().value()),
        current.document.text(),
    )?)
}

fn file_uri(path: &Path) -> Result<Uri, LanguageServiceError> {
    let url = url::Url::from_file_path(path).map_err(|_| {
        LanguageServiceError::InvalidDocumentUri(path.to_string_lossy().into_owned())
    })?;
    Uri::from_str(url.as_str())
        .map_err(|_| LanguageServiceError::InvalidDocumentUri(path.to_string_lossy().into_owned()))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
