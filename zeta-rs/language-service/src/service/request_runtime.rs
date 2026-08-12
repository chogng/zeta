//! Capability-gated request execution and revision-fresh result delivery.

use super::*;
use crate::requests::{
    project_call_hierarchy_items, project_code_actions, project_completions, project_hover,
    project_incoming_calls, project_locations, project_outgoing_calls, project_references,
    project_rename_preparation, project_resolved_code_action, project_type_hierarchy_items,
    project_workspace_edit, project_workspace_symbols, protocol_call_hierarchy_item,
    protocol_code_action, protocol_position, protocol_type_hierarchy_item,
};
use crate::{
    LanguageDiagnostic, LanguageDiagnosticSeverity, LanguageHierarchyItem, LanguageHierarchyKind,
    LanguageLocationKind,
};
use zeta_lsp::lsp_types::request::{
    CallHierarchyIncomingCalls, CallHierarchyOutgoingCalls, CallHierarchyPrepare,
    CodeActionRequest, CodeActionResolveRequest, Completion, GotoDeclaration, GotoDefinition,
    GotoImplementation, GotoTypeDefinition, HoverRequest, PrepareRenameRequest, References, Rename,
    TypeHierarchyPrepare, TypeHierarchySubtypes, TypeHierarchySupertypes, WorkspaceSymbolRequest,
};
use zeta_lsp::lsp_types::{
    CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CallHierarchyServerCapability, CodeActionContext, CodeActionKind, CodeActionParams,
    CodeActionProviderCapability, CodeActionTriggerKind, CompletionParams, CompletionTriggerKind,
    DeclarationCapability, Diagnostic, DiagnosticSeverity, GotoDefinitionParams, HoverParams,
    ImplementationProviderCapability, NumberOrString, OneOf, PartialResultParams,
    PositionEncodingKind, ReferenceContext, ReferenceParams, RenameParams, TextDocumentIdentifier,
    TextDocumentPositionParams, TypeDefinitionProviderCapability, TypeHierarchyPrepareParams,
    TypeHierarchySubtypesParams, TypeHierarchySupertypesParams, WorkDoneProgressParams,
    WorkspaceSymbolParams,
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
    Declaration {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    Implementation {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    TypeDefinition {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    References {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        include_declaration: bool,
    },
    PrepareCallHierarchy {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    IncomingCalls {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    },
    OutgoingCalls {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    },
    PrepareTypeHierarchy {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    Supertypes {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    },
    Subtypes {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        item: LanguageHierarchyItem,
    },
    PrepareRename {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
    },
    Rename {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        position: LanguageDocumentPosition,
        new_name: String,
    },
    CodeActions {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        range: LanguageLocationRange,
        diagnostics: Vec<LanguageDiagnostic>,
        only: Vec<String>,
    },
    ResolveCodeAction {
        id: LanguageRequestId,
        path: PathBuf,
        revision: LanguageDocumentRevision,
        provider_data: serde_json::Value,
    },
}

impl PendingLanguageRequest {
    pub(super) const fn id(&self) -> LanguageRequestId {
        match self {
            Self::Hover { id, .. }
            | Self::Completion { id, .. }
            | Self::Declaration { id, .. }
            | Self::Definition { id, .. }
            | Self::Implementation { id, .. }
            | Self::TypeDefinition { id, .. }
            | Self::References { id, .. }
            | Self::PrepareCallHierarchy { id, .. }
            | Self::IncomingCalls { id, .. }
            | Self::OutgoingCalls { id, .. }
            | Self::PrepareTypeHierarchy { id, .. }
            | Self::Supertypes { id, .. }
            | Self::Subtypes { id, .. } => *id,
            Self::PrepareRename { id, .. }
            | Self::Rename { id, .. }
            | Self::CodeActions { id, .. }
            | Self::ResolveCodeAction { id, .. } => *id,
        }
    }

    const fn kind(&self) -> LanguageRequestKind {
        match self {
            Self::Hover { .. } => LanguageRequestKind::Hover,
            Self::Completion { .. } => LanguageRequestKind::Completion,
            Self::Declaration { .. } => LanguageRequestKind::Declaration,
            Self::Definition { .. } => LanguageRequestKind::Definition,
            Self::Implementation { .. } => LanguageRequestKind::Implementation,
            Self::TypeDefinition { .. } => LanguageRequestKind::TypeDefinition,
            Self::References { .. } => LanguageRequestKind::References,
            Self::PrepareCallHierarchy { .. } => LanguageRequestKind::PrepareCallHierarchy,
            Self::IncomingCalls { .. } => LanguageRequestKind::IncomingCalls,
            Self::OutgoingCalls { .. } => LanguageRequestKind::OutgoingCalls,
            Self::PrepareTypeHierarchy { .. } => LanguageRequestKind::PrepareTypeHierarchy,
            Self::Supertypes { .. } => LanguageRequestKind::Supertypes,
            Self::Subtypes { .. } => LanguageRequestKind::Subtypes,
            Self::PrepareRename { .. } => LanguageRequestKind::PrepareRename,
            Self::Rename { .. } => LanguageRequestKind::Rename,
            Self::CodeActions { .. } => LanguageRequestKind::CodeActions,
            Self::ResolveCodeAction { .. } => LanguageRequestKind::ResolveCodeAction,
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Hover { path, .. }
            | Self::Completion { path, .. }
            | Self::Declaration { path, .. }
            | Self::Definition { path, .. }
            | Self::Implementation { path, .. }
            | Self::TypeDefinition { path, .. }
            | Self::References { path, .. }
            | Self::PrepareCallHierarchy { path, .. }
            | Self::IncomingCalls { path, .. }
            | Self::OutgoingCalls { path, .. }
            | Self::PrepareTypeHierarchy { path, .. }
            | Self::Supertypes { path, .. }
            | Self::Subtypes { path, .. } => path,
            Self::PrepareRename { path, .. }
            | Self::Rename { path, .. }
            | Self::CodeActions { path, .. }
            | Self::ResolveCodeAction { path, .. } => path,
        }
    }

    const fn revision(&self) -> LanguageDocumentRevision {
        match self {
            Self::Hover { revision, .. }
            | Self::Completion { revision, .. }
            | Self::Declaration { revision, .. }
            | Self::Definition { revision, .. }
            | Self::Implementation { revision, .. }
            | Self::TypeDefinition { revision, .. }
            | Self::References { revision, .. }
            | Self::PrepareCallHierarchy { revision, .. }
            | Self::IncomingCalls { revision, .. }
            | Self::OutgoingCalls { revision, .. }
            | Self::PrepareTypeHierarchy { revision, .. }
            | Self::Supertypes { revision, .. }
            | Self::Subtypes { revision, .. } => *revision,
            Self::PrepareRename { revision, .. }
            | Self::Rename { revision, .. }
            | Self::CodeActions { revision, .. }
            | Self::ResolveCodeAction { revision, .. } => *revision,
        }
    }

    const fn position(&self) -> Option<LanguageDocumentPosition> {
        match self {
            Self::Hover { position, .. }
            | Self::Completion { position, .. }
            | Self::Declaration { position, .. }
            | Self::Definition { position, .. }
            | Self::Implementation { position, .. }
            | Self::TypeDefinition { position, .. }
            | Self::References { position, .. }
            | Self::PrepareCallHierarchy { position, .. }
            | Self::PrepareTypeHierarchy { position, .. } => Some(*position),
            Self::PrepareRename { position, .. } | Self::Rename { position, .. } => Some(*position),
            Self::IncomingCalls { .. }
            | Self::OutgoingCalls { .. }
            | Self::Supertypes { .. }
            | Self::Subtypes { .. } => None,
            Self::CodeActions { .. } | Self::ResolveCodeAction { .. } => None,
        }
    }
}

pub(super) enum CompletedLanguageRequest {
    Hover(LanguageHover),
    Completions(LanguageCompletions),
    Locations(LanguageLocations),
    Hierarchy(LanguageHierarchyResult),
    RenamePreparation(LanguageRenamePreparation),
    WorkspaceEdit(LanguageWorkspaceEditResult),
    CodeActions(LanguageCodeActions),
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
            Self::Locations(result) => &result.source_path,
            Self::Hierarchy(result) => &result.source_path,
            Self::RenamePreparation(result) => &result.source_path,
            Self::WorkspaceEdit(result) => &result.source_path,
            Self::CodeActions(result) => &result.source_path,
            Self::Empty { path, .. } | Self::Failed { path, .. } => path,
        }
    }

    const fn revision(&self) -> LanguageDocumentRevision {
        match self {
            Self::Hover(result) => result.revision,
            Self::Completions(result) => result.revision,
            Self::Locations(result) => result.source_revision,
            Self::Hierarchy(result) => result.source_revision,
            Self::RenamePreparation(result) => result.source_revision,
            Self::WorkspaceEdit(result) => result.source_revision,
            Self::CodeActions(result) => result.source_revision,
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
        let position = match request.position() {
            Some(position) => {
                match protocol_position(document.document.text(), position, &encoding) {
                    Some(position) => Some(position),
                    None => {
                        self.emit_request_failure(
                            &request,
                            "request position is outside the document snapshot",
                        );
                        return;
                    }
                }
            }
            None => None,
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
            CompletedLanguageRequest::Locations(result) => {
                self.emit(LanguageServiceEvent::Locations(result))
            }
            CompletedLanguageRequest::Hierarchy(result) => {
                self.emit(LanguageServiceEvent::Hierarchy(result))
            }
            CompletedLanguageRequest::RenamePreparation(result) => {
                self.emit(LanguageServiceEvent::RenamePreparation(result))
            }
            CompletedLanguageRequest::WorkspaceEdit(result) => {
                self.emit(LanguageServiceEvent::WorkspaceEdit(result))
            }
            CompletedLanguageRequest::CodeActions(result) => {
                self.emit(LanguageServiceEvent::CodeActions(result))
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
    position: Option<zeta_lsp::lsp_types::Position>,
    text: String,
    encoding: PositionEncodingKind,
) -> Result<CompletedLanguageRequest, String> {
    match request {
        PendingLanguageRequest::Hover {
            id, path, revision, ..
        } => {
            let text_document_position = text_document_position(uri, position)?;
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
            let text_document_position = text_document_position(uri, position)?;
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
            let text_document_position = text_document_position(uri, position)?;
            let response = client
                .request::<GotoDefinition>(GotoDefinitionParams {
                    text_document_position_params: text_document_position,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            let result = project_locations(
                id,
                LanguageLocationKind::Definition,
                path.clone(),
                revision,
                &encoding,
                response,
            );
            if result.targets.is_empty() {
                Ok(CompletedLanguageRequest::Empty {
                    id,
                    kind: LanguageRequestKind::Definition,
                    path,
                    revision,
                })
            } else {
                Ok(CompletedLanguageRequest::Locations(result))
            }
        }
        PendingLanguageRequest::Declaration {
            id, path, revision, ..
        } => {
            execute_locations::<GotoDeclaration>(
                client,
                text_document_position(uri, position)?,
                id,
                LanguageRequestKind::Declaration,
                LanguageLocationKind::Declaration,
                path,
                revision,
                encoding,
            )
            .await
        }
        PendingLanguageRequest::Implementation {
            id, path, revision, ..
        } => {
            execute_locations::<GotoImplementation>(
                client,
                text_document_position(uri, position)?,
                id,
                LanguageRequestKind::Implementation,
                LanguageLocationKind::Implementation,
                path,
                revision,
                encoding,
            )
            .await
        }
        PendingLanguageRequest::TypeDefinition {
            id, path, revision, ..
        } => {
            execute_locations::<GotoTypeDefinition>(
                client,
                text_document_position(uri, position)?,
                id,
                LanguageRequestKind::TypeDefinition,
                LanguageLocationKind::TypeDefinition,
                path,
                revision,
                encoding,
            )
            .await
        }
        PendingLanguageRequest::References {
            id,
            path,
            revision,
            include_declaration,
            ..
        } => {
            let text_document_position = text_document_position(uri, position)?;
            let response = client
                .request::<References>(ReferenceParams {
                    text_document_position,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                    context: ReferenceContext {
                        include_declaration,
                    },
                })
                .await
                .map_err(|error| error.to_string())?;
            let result = project_references(id, path.clone(), revision, &encoding, response);
            if result.targets.is_empty() {
                Ok(CompletedLanguageRequest::Empty {
                    id,
                    kind: LanguageRequestKind::References,
                    path,
                    revision,
                })
            } else {
                Ok(CompletedLanguageRequest::Locations(result))
            }
        }
        PendingLanguageRequest::PrepareCallHierarchy {
            id, path, revision, ..
        } => {
            let response = client
                .request::<CallHierarchyPrepare>(CallHierarchyPrepareParams {
                    text_document_position_params: text_document_position(uri, position)?,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_call_hierarchy_items(
                    id,
                    LanguageHierarchyKind::PrepareCall,
                    path.clone(),
                    revision,
                    &encoding,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::PrepareCallHierarchy,
                path,
                revision,
            )
        }
        PendingLanguageRequest::IncomingCalls {
            id,
            path,
            revision,
            item,
        } => {
            let item = protocol_call_hierarchy_item(item)
                .ok_or_else(|| "invalid call hierarchy item".to_owned())?;
            let response = client
                .request::<CallHierarchyIncomingCalls>(CallHierarchyIncomingCallsParams {
                    item,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_incoming_calls(
                    id,
                    path.clone(),
                    revision,
                    &encoding,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::IncomingCalls,
                path,
                revision,
            )
        }
        PendingLanguageRequest::OutgoingCalls {
            id,
            path,
            revision,
            item,
        } => {
            let from_path = item.path.clone();
            let item = protocol_call_hierarchy_item(item)
                .ok_or_else(|| "invalid call hierarchy item".to_owned())?;
            let response = client
                .request::<CallHierarchyOutgoingCalls>(CallHierarchyOutgoingCallsParams {
                    item,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_outgoing_calls(
                    id,
                    path.clone(),
                    revision,
                    &encoding,
                    from_path,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::OutgoingCalls,
                path,
                revision,
            )
        }
        PendingLanguageRequest::PrepareTypeHierarchy {
            id, path, revision, ..
        } => {
            let response = client
                .request::<TypeHierarchyPrepare>(TypeHierarchyPrepareParams {
                    text_document_position_params: text_document_position(uri, position)?,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_type_hierarchy_items(
                    id,
                    LanguageHierarchyKind::PrepareType,
                    path.clone(),
                    revision,
                    &encoding,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::PrepareTypeHierarchy,
                path,
                revision,
            )
        }
        PendingLanguageRequest::Supertypes {
            id,
            path,
            revision,
            item,
        } => {
            let item = protocol_type_hierarchy_item(item)
                .ok_or_else(|| "invalid type hierarchy item".to_owned())?;
            let response = client
                .request::<TypeHierarchySupertypes>(TypeHierarchySupertypesParams {
                    item,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_type_hierarchy_items(
                    id,
                    LanguageHierarchyKind::Supertypes,
                    path.clone(),
                    revision,
                    &encoding,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::Supertypes,
                path,
                revision,
            )
        }
        PendingLanguageRequest::Subtypes {
            id,
            path,
            revision,
            item,
        } => {
            let item = protocol_type_hierarchy_item(item)
                .ok_or_else(|| "invalid type hierarchy item".to_owned())?;
            let response = client
                .request::<TypeHierarchySubtypes>(TypeHierarchySubtypesParams {
                    item,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            hierarchy_result(
                project_type_hierarchy_items(
                    id,
                    LanguageHierarchyKind::Subtypes,
                    path.clone(),
                    revision,
                    &encoding,
                    response.unwrap_or_default(),
                ),
                id,
                LanguageRequestKind::Subtypes,
                path,
                revision,
            )
        }
        PendingLanguageRequest::PrepareRename {
            id, path, revision, ..
        } => {
            let response = client
                .request::<PrepareRenameRequest>(text_document_position(uri, position)?)
                .await
                .map_err(|error| error.to_string())?;
            Ok(
                project_rename_preparation(id, path.clone(), revision, &text, &encoding, response)
                    .map(CompletedLanguageRequest::RenamePreparation)
                    .unwrap_or(CompletedLanguageRequest::Empty {
                        id,
                        kind: LanguageRequestKind::PrepareRename,
                        path,
                        revision,
                    }),
            )
        }
        PendingLanguageRequest::Rename {
            id,
            path,
            revision,
            new_name,
            ..
        } => {
            let response = client
                .request::<Rename>(RenameParams {
                    text_document_position: text_document_position(uri, position)?,
                    new_name,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            project_workspace_edit(id, path, revision, &encoding, response)
                .map(CompletedLanguageRequest::WorkspaceEdit)
        }
        PendingLanguageRequest::CodeActions {
            id,
            path,
            revision,
            range,
            diagnostics,
            only,
        } => {
            let can_resolve = matches!(client.initialization().capabilities.code_action_provider, Some(CodeActionProviderCapability::Options(ref options)) if options.resolve_provider == Some(true));
            let response = client
                .request::<CodeActionRequest>(CodeActionParams {
                    text_document: TextDocumentIdentifier::new(uri),
                    range: protocol_location_range(range),
                    context: CodeActionContext {
                        diagnostics: diagnostics
                            .into_iter()
                            .filter_map(|diagnostic| {
                                protocol_diagnostic(&text, &encoding, diagnostic)
                            })
                            .collect(),
                        only: (!only.is_empty())
                            .then(|| only.into_iter().map(CodeActionKind::from).collect()),
                        trigger_kind: Some(CodeActionTriggerKind::INVOKED),
                    },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())?;
            Ok(CompletedLanguageRequest::CodeActions(project_code_actions(
                id,
                path,
                revision,
                &encoding,
                can_resolve,
                response,
            )))
        }
        PendingLanguageRequest::ResolveCodeAction {
            id,
            path,
            revision,
            provider_data,
        } => {
            let response = client
                .request::<CodeActionResolveRequest>(protocol_code_action(provider_data)?)
                .await
                .map_err(|error| error.to_string())?;
            project_resolved_code_action(id, path, revision, &encoding, response)
                .map(CompletedLanguageRequest::CodeActions)
        }
    }
}

fn protocol_location_range(range: LanguageLocationRange) -> zeta_lsp::lsp_types::Range {
    zeta_lsp::lsp_types::Range::new(
        zeta_lsp::lsp_types::Position::new(range.start.row, range.start.character),
        zeta_lsp::lsp_types::Position::new(range.end.row, range.end.character),
    )
}

fn protocol_diagnostic(
    text: &str,
    encoding: &PositionEncodingKind,
    diagnostic: LanguageDiagnostic,
) -> Option<Diagnostic> {
    let range = protocol_byte_range(text, diagnostic.range.byte_range(), encoding)?;
    Some(Diagnostic {
        range,
        severity: Some(match diagnostic.severity {
            LanguageDiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            LanguageDiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
            LanguageDiagnosticSeverity::Information => DiagnosticSeverity::INFORMATION,
            LanguageDiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: diagnostic.code.map(NumberOrString::String),
        source: diagnostic.source,
        message: diagnostic.message,
        ..Diagnostic::default()
    })
}

fn protocol_byte_range(
    text: &str,
    range: std::ops::Range<usize>,
    encoding: &PositionEncodingKind,
) -> Option<zeta_lsp::lsp_types::Range> {
    Some(zeta_lsp::lsp_types::Range::new(
        protocol_byte_offset(text, range.start, encoding)?,
        protocol_byte_offset(text, range.end, encoding)?,
    ))
}

fn protocol_byte_offset(
    text: &str,
    offset: usize,
    encoding: &PositionEncodingKind,
) -> Option<zeta_lsp::lsp_types::Position> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let prefix = &text[..offset];
    let row = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line_prefix = &text[line_start..offset];
    let character = if *encoding == PositionEncodingKind::UTF8 {
        line_prefix.len()
    } else {
        line_prefix.encode_utf16().count()
    };
    Some(zeta_lsp::lsp_types::Position::new(
        u32::try_from(row).ok()?,
        u32::try_from(character).ok()?,
    ))
}

impl Supervisor {
    pub(super) fn begin_workspace_symbols(
        &mut self,
        id: LanguageRequestId,
        language_id: String,
        query: String,
    ) {
        let Some((server, server_epoch)) = self.server_for_language(&language_id) else {
            self.emit(LanguageServiceEvent::WorkspaceSymbols(
                LanguageWorkspaceSymbols {
                    request_id: id,
                    query,
                    symbols: Vec::new(),
                },
            ));
            return;
        };
        let Ok(client) = self.router.client_for_language(&language_id).cloned() else {
            self.emit(LanguageServiceEvent::WorkspaceSymbols(
                LanguageWorkspaceSymbols {
                    request_id: id,
                    query,
                    symbols: Vec::new(),
                },
            ));
            return;
        };
        if !matches!(
            client
                .initialization()
                .capabilities
                .workspace_symbol_provider,
            Some(OneOf::Left(true)) | Some(OneOf::Right(_))
        ) {
            self.emit(LanguageServiceEvent::WorkspaceSymbols(
                LanguageWorkspaceSymbols {
                    request_id: id,
                    query,
                    symbols: Vec::new(),
                },
            ));
            return;
        }
        let encoding = client.initialization().position_encoding.clone();
        let generation = self.generation;
        let commands = self.commands.clone();
        let completion_query = query.clone();
        tokio::spawn(async move {
            let result = client
                .request::<WorkspaceSymbolRequest>(WorkspaceSymbolParams {
                    query: query.clone(),
                    partial_result_params: PartialResultParams::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .map(|response| project_workspace_symbols(id, query, &encoding, response))
                .map_err(|error| error.to_string());
            let _ = commands.send(SupervisorCommand::WorkspaceSymbolsCompleted {
                id,
                query: completion_query,
                server,
                generation,
                server_epoch,
                result,
            });
        });
    }

    pub(super) fn complete_workspace_symbols(
        &self,
        id: LanguageRequestId,
        query: String,
        server: LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<LanguageWorkspaceSymbols, String>,
    ) {
        if generation != self.generation
            || !self.servers.get(&server).is_some_and(|managed| {
                managed.epoch == server_epoch && managed.phase == ManagedServerPhase::Ready
            })
        {
            return;
        }
        match result {
            Ok(result) => self.emit(LanguageServiceEvent::WorkspaceSymbols(result)),
            Err(message) => {
                self.emit(LanguageServiceEvent::ServerMessage {
                    server: server.to_string(),
                    message,
                });
                self.emit(LanguageServiceEvent::WorkspaceSymbols(
                    LanguageWorkspaceSymbols {
                        request_id: id,
                        query,
                        symbols: Vec::new(),
                    },
                ));
            }
        }
    }
}

fn text_document_position(
    uri: Uri,
    position: Option<zeta_lsp::lsp_types::Position>,
) -> Result<TextDocumentPositionParams, String> {
    Ok(TextDocumentPositionParams::new(
        TextDocumentIdentifier::new(uri),
        position.ok_or_else(|| "language request requires a document position".to_owned())?,
    ))
}

fn hierarchy_result(
    result: LanguageHierarchyResult,
    id: LanguageRequestId,
    kind: LanguageRequestKind,
    path: PathBuf,
    revision: LanguageDocumentRevision,
) -> Result<CompletedLanguageRequest, String> {
    if result.entries.is_empty() {
        Ok(CompletedLanguageRequest::Empty {
            id,
            kind,
            path,
            revision,
        })
    } else {
        Ok(CompletedLanguageRequest::Hierarchy(result))
    }
}

async fn execute_locations<R>(
    client: LanguageServerClient,
    text_document_position_params: TextDocumentPositionParams,
    id: LanguageRequestId,
    request_kind: LanguageRequestKind,
    location_kind: LanguageLocationKind,
    path: PathBuf,
    revision: LanguageDocumentRevision,
    encoding: PositionEncodingKind,
) -> Result<CompletedLanguageRequest, String>
where
    R: zeta_lsp::lsp_types::request::Request<
            Params = GotoDefinitionParams,
            Result = Option<zeta_lsp::lsp_types::GotoDefinitionResponse>,
        >,
{
    let response = client
        .request::<R>(GotoDefinitionParams {
            text_document_position_params,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .map_err(|error| error.to_string())?;
    let result = project_locations(
        id,
        location_kind,
        path.clone(),
        revision,
        &encoding,
        response,
    );
    if result.targets.is_empty() {
        Ok(CompletedLanguageRequest::Empty {
            id,
            kind: request_kind,
            path,
            revision,
        })
    } else {
        Ok(CompletedLanguageRequest::Locations(result))
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
        LanguageRequestKind::Declaration => matches!(
            capabilities.declaration_provider,
            Some(DeclarationCapability::Simple(true))
                | Some(DeclarationCapability::RegistrationOptions(_))
                | Some(DeclarationCapability::Options(_))
        ),
        LanguageRequestKind::Implementation => matches!(
            capabilities.implementation_provider,
            Some(ImplementationProviderCapability::Simple(true))
                | Some(ImplementationProviderCapability::Options(_))
        ),
        LanguageRequestKind::TypeDefinition => matches!(
            capabilities.type_definition_provider,
            Some(TypeDefinitionProviderCapability::Simple(true))
                | Some(TypeDefinitionProviderCapability::Options(_))
        ),
        LanguageRequestKind::References => matches!(
            capabilities.references_provider,
            Some(OneOf::Left(true)) | Some(OneOf::Right(_))
        ),
        LanguageRequestKind::PrepareCallHierarchy
        | LanguageRequestKind::IncomingCalls
        | LanguageRequestKind::OutgoingCalls => matches!(
            capabilities.call_hierarchy_provider,
            Some(CallHierarchyServerCapability::Simple(true))
                | Some(CallHierarchyServerCapability::Options(_))
        ),
        LanguageRequestKind::PrepareTypeHierarchy
        | LanguageRequestKind::Supertypes
        | LanguageRequestKind::Subtypes => true,
        LanguageRequestKind::WorkspaceSymbols => matches!(
            capabilities.workspace_symbol_provider,
            Some(OneOf::Left(true)) | Some(OneOf::Right(_))
        ),
        LanguageRequestKind::PrepareRename | LanguageRequestKind::Rename => matches!(
            capabilities.rename_provider,
            Some(OneOf::Left(true)) | Some(OneOf::Right(_))
        ),
        LanguageRequestKind::CodeActions => matches!(
            capabilities.code_action_provider,
            Some(CodeActionProviderCapability::Simple(true))
                | Some(CodeActionProviderCapability::Options(_))
        ),
        LanguageRequestKind::ResolveCodeAction => matches!(
            capabilities.code_action_provider,
            Some(CodeActionProviderCapability::Options(ref options)) if options.resolve_provider == Some(true)
        ),
    }
}
