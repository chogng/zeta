//! Capability-gated request execution and revision-fresh result delivery.

use super::*;
use crate::requests::{project_completions, project_definitions, project_hover, protocol_position};
use zeta_lsp::lsp_types::request::{Completion, GotoDefinition, HoverRequest};
use zeta_lsp::lsp_types::{
    CompletionParams, CompletionTriggerKind, GotoDefinitionParams, HoverParams, OneOf,
    PartialResultParams, PositionEncodingKind, TextDocumentIdentifier, TextDocumentPositionParams,
    WorkDoneProgressParams,
};

pub(super) enum PendingLanguageRequest {
    Hover {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    Completion {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    Definition {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
}

impl PendingLanguageRequest {
    pub(super) const fn id(&self) -> LanguageRequestId {
        match self {
            Self::Hover { id, .. } | Self::Completion { id, .. } | Self::Definition { id, .. } => {
                *id
            }
        }
    }

    const fn kind(&self) -> LanguageRequestKind {
        match self {
            Self::Hover { .. } => LanguageRequestKind::Hover,
            Self::Completion { .. } => LanguageRequestKind::Completion,
            Self::Definition { .. } => LanguageRequestKind::Definition,
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Hover { path, .. }
            | Self::Completion { path, .. }
            | Self::Definition { path, .. } => path,
        }
    }

    const fn revision(&self) -> LanguageDocumentRevision {
        match self {
            Self::Hover { revision, .. }
            | Self::Completion { revision, .. }
            | Self::Definition { revision, .. } => *revision,
        }
    }

    const fn position(&self) -> LanguageDocumentPosition {
        match self {
            Self::Hover { position, .. }
            | Self::Completion { position, .. }
            | Self::Definition { position, .. } => *position,
        }
    }
}

pub(super) enum CompletedLanguageRequest {
    Hover(LanguageHover),
    Completions(LanguageCompletions),
    Definitions(LanguageDefinitions),
    Empty {
        id: LanguageRequestId,
        kind: LanguageRequestKind,
        path: PathBuf,
        revision: LanguageDocumentRevision,
    },
    Failed {
        id: LanguageRequestId,
        kind: LanguageRequestKind,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        message: String,
    },
}

impl CompletedLanguageRequest {
    fn path(&self) -> &Path {
        match self {
            Self::Hover(result) => &result.path,
            Self::Completions(result) => &result.path,
            Self::Definitions(result) => &result.source_path,
            Self::Empty { path, .. } | Self::Failed { path, .. } => path,
        }
    }

    const fn revision(&self) -> LanguageDocumentRevision {
        match self {
            Self::Hover(result) => result.revision,
            Self::Completions(result) => result.revision,
            Self::Definitions(result) => result.source_revision,
            Self::Empty { revision, .. } | Self::Failed { revision, .. } => *revision,
        }
    }
}

impl Supervisor {
    pub(super) fn begin_language_request(&mut self, request: PendingLanguageRequest) {
        let Some(document) = self.documents.get(request.path()) else {
            self.emit_request_failure(&request, "document is not open in the language service");
            return;
        };
        if document.document.revision() != request.revision() || !document.routed {
            self.emit_request_failure(&request, "document revision is stale or not routed");
            return;
        }
        let Some((server_name, server_epoch)) =
            self.server_for_language(document.document.language_id())
        else {
            self.emit_request_failure(&request, "no ready language server supports this document");
            return;
        };
        let Ok(client) = self.router.client_for_document(&document.uri).cloned() else {
            self.emit_request_failure(&request, "document has no active language-server route");
            return;
        };
        if !supports_request(&client, request.kind()) {
            self.emit_request_failure(
                &request,
                "language server does not advertise this capability",
            );
            return;
        }
        let encoding = client.initialization().position_encoding.clone();
        let Some(position) =
            protocol_position(document.document.text(), request.position(), &encoding)
        else {
            self.emit_request_failure(
                &request,
                "request position is outside the document snapshot",
            );
            return;
        };
        let uri = document.uri.clone();
        let text = document.document.text().to_owned();
        let generation = self.generation;
        let commands = self.commands.clone();
        let failure_id = request.id();
        let failure_kind = request.kind();
        let failure_path = request.path().to_path_buf();
        let failure_revision = request.revision();
        tokio::spawn(async move {
            let result = execute_request(client, request, uri, position, text, encoding)
                .await
                .or_else(|message| {
                    Ok(CompletedLanguageRequest::Failed {
                        id: failure_id,
                        kind: failure_kind,
                        path: failure_path,
                        revision: failure_revision,
                        message,
                    })
                });
            let _ = commands.send(SupervisorCommand::LanguageRequestCompleted {
                server: server_name,
                generation,
                server_epoch,
                result,
            });
        });
    }

    pub(super) fn complete_language_request(
        &self,
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<CompletedLanguageRequest, String>,
    ) {
        if generation != self.generation
            || !self.servers.get(&server).is_some_and(|managed| {
                managed.epoch == server_epoch && managed.phase == ManagedServerPhase::Ready
            })
        {
            return;
        }
        let result = match result {
            Ok(result) => result,
            Err(message) => {
                self.emit(LanguageServiceEvent::ServerMessage {
                    server: server.to_string(),
                    message,
                });
                return;
            }
        };
        let fresh = self.documents.get(result.path()).is_some_and(|document| {
            document.routed && document.document.revision() == result.revision()
        });
        if !fresh {
            return;
        }
        match result {
            CompletedLanguageRequest::Hover(result) => {
                self.emit(LanguageServiceEvent::Hover(result))
            }
            CompletedLanguageRequest::Completions(result) => {
                self.emit(LanguageServiceEvent::Completions(result))
            }
            CompletedLanguageRequest::Definitions(result) => {
                self.emit(LanguageServiceEvent::Definitions(result))
            }
            CompletedLanguageRequest::Empty {
                id,
                kind,
                path,
                revision,
            } => self.emit(LanguageServiceEvent::RequestFailed {
                request_id: id,
                kind,
                path,
                revision,
                message: "language server returned no result".into(),
            }),
            CompletedLanguageRequest::Failed {
                id,
                kind,
                path,
                revision,
                message,
            } => self.emit(LanguageServiceEvent::RequestFailed {
                request_id: id,
                kind,
                path,
                revision,
                message,
            }),
        }
    }

    fn server_for_language(&self, language_id: &str) -> Option<(LanguageServerName, u64)> {
        self.servers.iter().find_map(|(name, server)| {
            (server.phase == ManagedServerPhase::Ready
                && server
                    .definition
                    .language_ids()
                    .any(|language| language == language_id))
            .then(|| (name.clone(), server.epoch))
        })
    }

    fn emit_request_failure(&self, request: &PendingLanguageRequest, message: &str) {
        self.emit(LanguageServiceEvent::RequestFailed {
            request_id: request.id(),
            kind: request.kind(),
            path: request.path().to_path_buf(),
            revision: request.revision(),
            message: message.into(),
        });
    }
}

async fn execute_request(
    client: LanguageServerClient,
    request: PendingLanguageRequest,
    uri: Uri,
    position: zeta_lsp::lsp_types::Position,
    text: String,
    encoding: PositionEncodingKind,
) -> Result<CompletedLanguageRequest, String> {
    let text_document_position =
        TextDocumentPositionParams::new(TextDocumentIdentifier::new(uri), position);
    match request {
        PendingLanguageRequest::Hover {
            id, path, revision, ..
        } => {
            let response = client
                .request::<HoverRequest>(HoverParams {
                    text_document_position_params: text_document_position,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            Ok(
                project_hover(id, path.clone(), revision, &text, &encoding, response)
                    .map(CompletedLanguageRequest::Hover)
                    .unwrap_or(CompletedLanguageRequest::Empty {
                        id,
                        kind: LanguageRequestKind::Hover,
                        path,
                        revision,
                    }),
            )
        }
        PendingLanguageRequest::Completion {
            id,
            path,
            revision,
            position: request_position,
        } => {
            let response = client
                .request::<Completion>(CompletionParams {
                    text_document_position,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: Some(zeta_lsp::lsp_types::CompletionContext {
                        trigger_kind: CompletionTriggerKind::INVOKED,
                        trigger_character: None,
                    }),
                })
                .await
                .map_err(|error| error.to_string())?;
            Ok(CompletedLanguageRequest::Completions(project_completions(
                id,
                path,
                revision,
                request_position,
                &text,
                &encoding,
                response,
            )))
        }
        PendingLanguageRequest::Definition {
            id, path, revision, ..
        } => {
            let response = client
                .request::<GotoDefinition>(GotoDefinitionParams {
                    text_document_position_params: text_document_position,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            let result = project_definitions(id, path.clone(), revision, &encoding, response);
            if result.targets.is_empty() {
                Ok(CompletedLanguageRequest::Empty {
                    id,
                    kind: LanguageRequestKind::Definition,
                    path,
                    revision,
                })
            } else {
                Ok(CompletedLanguageRequest::Definitions(result))
            }
        }
    }
}

fn supports_request(client: &LanguageServerClient, kind: LanguageRequestKind) -> bool {
    let capabilities = &client.initialization().capabilities;
    match kind {
        LanguageRequestKind::Hover => matches!(
            capabilities.hover_provider,
            Some(zeta_lsp::lsp_types::HoverProviderCapability::Simple(true))
                | Some(zeta_lsp::lsp_types::HoverProviderCapability::Options(_))
        ),
        LanguageRequestKind::Completion => capabilities.completion_provider.is_some(),
        LanguageRequestKind::Definition => matches!(
            capabilities.definition_provider,
            Some(OneOf::Left(true)) | Some(OneOf::Right(_))
        ),
    }
}
