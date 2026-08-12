use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::fs::{
    FsDeleteMode, FsExistingTargetBehavior, FsMissingTargetBehavior,
};
use zeta_app_server_protocol::protocol::language::LanguageCodeActionDto;
use zeta_app_server_protocol::protocol::language::LanguageCodeActionsParams;
use zeta_app_server_protocol::protocol::language::LanguageCodeActionsResult;
use zeta_app_server_protocol::protocol::language::LanguageDiagnosticSeverityDto;
use zeta_app_server_protocol::protocol::language::LanguageHierarchyEntryDto;
use zeta_app_server_protocol::protocol::language::LanguageHierarchyItemDto;
use zeta_app_server_protocol::protocol::language::LanguageHierarchyKindDto;
use zeta_app_server_protocol::protocol::language::LanguageHierarchyParams;
use zeta_app_server_protocol::protocol::language::LanguageHierarchyResultDto;
use zeta_app_server_protocol::protocol::language::LanguageLocationDto;
use zeta_app_server_protocol::protocol::language::LanguageLocationKindDto;
use zeta_app_server_protocol::protocol::language::LanguageLocationsParams;
use zeta_app_server_protocol::protocol::language::LanguageLocationsResult;
use zeta_app_server_protocol::protocol::language::LanguagePositionDto;
use zeta_app_server_protocol::protocol::language::LanguagePrepareRenameParams;
use zeta_app_server_protocol::protocol::language::LanguagePrepareRenameResult;
use zeta_app_server_protocol::protocol::language::LanguageRangeDto;
use zeta_app_server_protocol::protocol::language::LanguageRenameParams;
use zeta_app_server_protocol::protocol::language::LanguageRenamePreparationDto;
use zeta_app_server_protocol::protocol::language::LanguageResolveCodeActionParams;
use zeta_app_server_protocol::protocol::language::LanguageTextDocumentEditDto;
use zeta_app_server_protocol::protocol::language::LanguageTextEditDto;
use zeta_app_server_protocol::protocol::language::LanguageWorkspaceEditDto;
use zeta_app_server_protocol::protocol::language::LanguageWorkspaceEditEntryDto;
use zeta_app_server_protocol::protocol::language::LanguageWorkspaceSymbolDto;
use zeta_app_server_protocol::protocol::language::LanguageWorkspaceSymbolsParams;
use zeta_app_server_protocol::protocol::language::LanguageWorkspaceSymbolsResult;
use zeta_language_service::LanguageCodeAction;
use zeta_language_service::LanguageDiagnostic;
use zeta_language_service::LanguageDiagnosticSeverity;
use zeta_language_service::LanguageDocumentPosition;
use zeta_language_service::LanguageDocumentRevision;
use zeta_language_service::LanguageHierarchyEntry;
use zeta_language_service::LanguageHierarchyItem;
use zeta_language_service::LanguageLocationPosition;
use zeta_language_service::LanguageLocationRange;
use zeta_language_service::LanguageLocationTarget;
use zeta_language_service::LanguagePositionEncoding;
use zeta_language_service::LanguageServiceDocument;
use zeta_language_service::LanguageServiceEvent;
use zeta_language_service::LanguageTextRange;
use zeta_language_service::LanguageWorkspaceEdit;
use zeta_language_service::{
    LanguageDeleteMode, LanguageExistingTargetBehavior, LanguageMissingTargetBehavior,
    LanguageWorkspaceEditEntry,
};

use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;

const MAX_LANGUAGE_TARGET_BYTES: usize = 10 * 1024 * 1024;

impl AppServer {
    fn prepare_position_request(
        &self,
        document: &zeta_app_server_protocol::protocol::language::LanguageDocumentDto,
        position: LanguagePositionDto,
    ) -> Result<
        (
            zeta_workspace::WorkspaceRoot,
            std::path::PathBuf,
            LanguageDocumentRevision,
            LanguageDocumentPosition,
            std::sync::MutexGuard<'_, super::language_runtime::AppServerLanguageRuntime>,
        ),
        RpcError,
    > {
        let workspace = self.language_workspace_root()?;
        let source_path = workspace
            .resolve_existing(&document.path)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let revision = LanguageDocumentRevision::new(document.revision);
        let position = utf8_position(&document.text, position)
            .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let runtime = self.prepare_document_runtime(&workspace, &source_path, document)?;
        Ok((workspace, source_path, revision, position, runtime))
    }

    fn prepare_document_runtime(
        &self,
        workspace: &zeta_workspace::WorkspaceRoot,
        source_path: &Path,
        document: &zeta_app_server_protocol::protocol::language::LanguageDocumentDto,
    ) -> Result<
        std::sync::MutexGuard<'_, super::language_runtime::AppServerLanguageRuntime>,
        RpcError,
    > {
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?
            .read_snapshot()
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let revision = LanguageDocumentRevision::new(document.revision);
        let service_document = LanguageServiceDocument::new(
            source_path,
            language_service_id(&document.language_id),
            revision,
            &document.text,
        )
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let mut runtime = self
            .language
            .lock()
            .map_err(|_| language_error(AppServerErrorName::ServerOverloaded))?;
        let service = runtime
            .ensure(
                workspace.canonical_path(),
                snapshot.generation.get(),
                &snapshot.values.language_servers,
                language_service_id(&document.language_id),
            )
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        service
            .synchronize_document(service_document)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        Ok(runtime)
    }

    pub(super) fn language_locations(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageLocationsParams = decode(params)?;
        let workspace = self.language_workspace_root()?;
        let source_path = workspace
            .resolve_existing(&params.document.path)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let position = utf8_position(&params.document.text, params.position)
            .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?
            .read_snapshot()
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let revision = LanguageDocumentRevision::new(params.document.revision);
        let document = LanguageServiceDocument::new(
            &source_path,
            language_service_id(&params.document.language_id),
            revision,
            &params.document.text,
        )
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let mut runtime = self
            .language
            .lock()
            .map_err(|_| language_error(AppServerErrorName::ServerOverloaded))?;
        let service = runtime
            .ensure(
                workspace.canonical_path(),
                snapshot.generation.get(),
                &snapshot.values.language_servers,
                language_service_id(&params.document.language_id),
            )
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        service
            .synchronize_document(document)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let request_id = match params.kind {
            LanguageLocationKindDto::Declaration => {
                service.request_declaration(&source_path, revision, position)
            }
            LanguageLocationKindDto::Definition => {
                service.request_definition(&source_path, revision, position)
            }
            LanguageLocationKindDto::Implementation => {
                service.request_implementation(&source_path, revision, position)
            }
            LanguageLocationKindDto::TypeDefinition => {
                service.request_type_definition(&source_path, revision, position)
            }
            LanguageLocationKindDto::References => service.request_references(
                &source_path,
                revision,
                position,
                params.include_declaration,
            ),
        }
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let locations = match event {
            LanguageServiceEvent::Locations(locations) => locations,
            LanguageServiceEvent::RequestFailed { .. } => {
                return Err(language_error(AppServerErrorName::LanguageRequestFailed));
            }
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        let file_system = self.file_system_service()?;
        let projected = locations
            .targets
            .into_iter()
            .filter_map(|target| {
                let relative = target
                    .path
                    .strip_prefix(workspace.canonical_path())
                    .ok()?
                    .to_path_buf();
                let text = if target.path == source_path {
                    params.document.text.clone()
                } else {
                    String::from_utf8(
                        file_system
                            .read_file(&relative, MAX_LANGUAGE_TARGET_BYTES)
                            .ok()?,
                    )
                    .ok()?
                };
                target_location(&relative, &text, target)
            })
            .collect();
        result(&LanguageLocationsResult {
            revision: params.document.revision,
            locations: projected,
        })
    }

    pub(super) fn language_hierarchy(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageHierarchyParams = decode(params)?;
        let workspace = self.language_workspace_root()?;
        let source_path = workspace
            .resolve_existing(&params.document.path)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let position = params
            .position
            .map(|position| {
                utf8_position(&params.document.text, position)
                    .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))
            })
            .transpose()?;
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?
            .read_snapshot()
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let revision = LanguageDocumentRevision::new(params.document.revision);
        let document = LanguageServiceDocument::new(
            &source_path,
            language_service_id(&params.document.language_id),
            revision,
            &params.document.text,
        )
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let item = params
            .item
            .map(|item| hierarchy_item_from_dto(&workspace, item))
            .transpose()?;
        let mut runtime = self
            .language
            .lock()
            .map_err(|_| language_error(AppServerErrorName::ServerOverloaded))?;
        let service = runtime
            .ensure(
                workspace.canonical_path(),
                snapshot.generation.get(),
                &snapshot.values.language_servers,
                language_service_id(&params.document.language_id),
            )
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        service
            .synchronize_document(document)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let request_id = match (params.kind, position, item) {
            (LanguageHierarchyKindDto::PrepareCall, Some(position), None) => {
                service.request_prepare_call_hierarchy(&source_path, revision, position)
            }
            (LanguageHierarchyKindDto::IncomingCalls, None, Some(item)) => {
                service.request_incoming_calls(&source_path, revision, item)
            }
            (LanguageHierarchyKindDto::OutgoingCalls, None, Some(item)) => {
                service.request_outgoing_calls(&source_path, revision, item)
            }
            (LanguageHierarchyKindDto::PrepareType, Some(position), None) => {
                service.request_prepare_type_hierarchy(&source_path, revision, position)
            }
            (LanguageHierarchyKindDto::Supertypes, None, Some(item)) => {
                service.request_supertypes(&source_path, revision, item)
            }
            (LanguageHierarchyKindDto::Subtypes, None, Some(item)) => {
                service.request_subtypes(&source_path, revision, item)
            }
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        }
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let hierarchy = match event {
            LanguageServiceEvent::Hierarchy(hierarchy) => hierarchy,
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        let file_system = self.file_system_service()?;
        let entries = hierarchy
            .entries
            .into_iter()
            .filter_map(|entry| {
                hierarchy_entry_to_dto(
                    workspace.canonical_path(),
                    &source_path,
                    &params.document.text,
                    file_system.as_ref(),
                    entry,
                )
            })
            .collect();
        result(&LanguageHierarchyResultDto {
            revision: params.document.revision,
            entries,
        })
    }

    pub(super) fn language_workspace_symbols(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageWorkspaceSymbolsParams = decode(params)?;
        let workspace = self.language_workspace_root()?;
        let snapshot = self
            .config
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?
            .read_snapshot()
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let language_id = language_service_id(&params.language_id);
        let mut runtime = self
            .language
            .lock()
            .map_err(|_| language_error(AppServerErrorName::ServerOverloaded))?;
        let service = runtime
            .ensure(
                workspace.canonical_path(),
                snapshot.generation.get(),
                &snapshot.values.language_servers,
                language_id,
            )
            .map_err(|_| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let request_id = service
            .request_workspace_symbols(language_id, &params.query)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let response = match event {
            LanguageServiceEvent::WorkspaceSymbols(response) => response,
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        let file_system = self.file_system_service()?;
        let symbols = response
            .symbols
            .into_iter()
            .filter_map(|symbol| {
                let relative = symbol
                    .path
                    .strip_prefix(workspace.canonical_path())
                    .ok()?
                    .to_path_buf();
                let text = String::from_utf8(
                    file_system
                        .read_file(&relative, MAX_LANGUAGE_TARGET_BYTES)
                        .ok()?,
                )
                .ok()?;
                Some(LanguageWorkspaceSymbolDto {
                    name: symbol.name,
                    symbol_kind: symbol.symbol_kind,
                    container_name: symbol.container_name,
                    path: relative,
                    range: utf16_range(&text, symbol.range, symbol.encoding)?,
                })
            })
            .collect();
        result(&LanguageWorkspaceSymbolsResult { symbols })
    }

    pub(super) fn language_prepare_rename(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguagePrepareRenameParams = decode(params)?;
        let (workspace, source_path, revision, position, mut runtime) =
            self.prepare_position_request(&params.document, params.position)?;
        let service = runtime
            .service
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let request_id = service
            .request_prepare_rename(&source_path, revision, position)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let preparation = match event {
            LanguageServiceEvent::RenamePreparation(preparation) => preparation,
            LanguageServiceEvent::RequestFailed { .. } => {
                return result(&LanguagePrepareRenameResult { preparation: None });
            }
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        let range = preparation
            .range
            .and_then(|range| byte_range_to_utf16(&params.document.text, range.byte_range()));
        let preparation = range.map(|range| LanguageRenamePreparationDto {
            range,
            placeholder: preparation.placeholder.unwrap_or_else(|| {
                text_for_utf16_range(&params.document.text, range).unwrap_or_default()
            }),
        });
        let _ = workspace;
        result(&LanguagePrepareRenameResult { preparation })
    }

    pub(super) fn language_rename(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageRenameParams = decode(params)?;
        let (workspace, source_path, revision, position, mut runtime) =
            self.prepare_position_request(&params.document, params.position)?;
        let service = runtime
            .service
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let request_id = service
            .request_rename(&source_path, revision, position, params.new_name)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let edit = match event {
            LanguageServiceEvent::WorkspaceEdit(result) => result.edit,
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        result(&workspace_edit_to_dto(
            &workspace,
            &source_path,
            &params.document.text,
            self.file_system_service()?.as_ref(),
            edit,
        )?)
    }

    pub(super) fn language_code_actions(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageCodeActionsParams = decode(params)?;
        let workspace = self.language_workspace_root()?;
        let source_path = workspace
            .resolve_existing(&params.document.path)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let revision = LanguageDocumentRevision::new(params.document.revision);
        let mut runtime =
            self.prepare_document_runtime(&workspace, &source_path, &params.document)?;
        let diagnostics = params
            .diagnostics
            .into_iter()
            .filter_map(|diagnostic| {
                Some(LanguageDiagnostic {
                    range: LanguageTextRange::new(utf8_byte_range(
                        &params.document.text,
                        diagnostic.range,
                    )?),
                    severity: match diagnostic.severity {
                        LanguageDiagnosticSeverityDto::Error => LanguageDiagnosticSeverity::Error,
                        LanguageDiagnosticSeverityDto::Warning => {
                            LanguageDiagnosticSeverity::Warning
                        }
                        LanguageDiagnosticSeverityDto::Information => {
                            LanguageDiagnosticSeverity::Information
                        }
                        LanguageDiagnosticSeverityDto::Hint => LanguageDiagnosticSeverity::Hint,
                    },
                    message: diagnostic.message,
                    source: diagnostic.source,
                    code: diagnostic.code.map(|code| {
                        code.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| code.to_string())
                    }),
                })
            })
            .collect();
        let range = service_range(params.range);
        let service = runtime
            .service
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let request_id = service
            .request_code_actions(&source_path, revision, range, diagnostics, params.only)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let actions = match event {
            LanguageServiceEvent::CodeActions(actions) => actions.actions,
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        let file_system = self.file_system_service()?;
        result(&LanguageCodeActionsResult {
            actions: actions
                .into_iter()
                .filter_map(|action| {
                    code_action_to_dto(
                        &workspace,
                        &source_path,
                        &params.document.text,
                        file_system.as_ref(),
                        action,
                    )
                    .ok()
                })
                .collect(),
        })
    }

    pub(super) fn language_resolve_code_action(&self, params: &Value) -> Result<Value, RpcError> {
        let params: LanguageResolveCodeActionParams = decode(params)?;
        let workspace = self.language_workspace_root()?;
        let source_path = workspace
            .resolve_existing(&params.document.path)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let revision = LanguageDocumentRevision::new(params.document.revision);
        let mut runtime =
            self.prepare_document_runtime(&workspace, &source_path, &params.document)?;
        let service = runtime
            .service
            .as_ref()
            .ok_or_else(|| language_error(AppServerErrorName::LanguageServiceUnavailable))?;
        let request_id = service
            .request_resolve_code_action(&source_path, revision, params.provider_data)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let event = runtime
            .wait_for_request(request_id)
            .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
        let action = match event {
            LanguageServiceEvent::CodeActions(actions) => actions
                .actions
                .into_iter()
                .next()
                .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))?,
            _ => return Err(language_error(AppServerErrorName::LanguageRequestFailed)),
        };
        result(&code_action_to_dto(
            &workspace,
            &source_path,
            &params.document.text,
            self.file_system_service()?.as_ref(),
            action,
        )?)
    }
}

fn hierarchy_item_from_dto(
    workspace: &zeta_workspace::WorkspaceRoot,
    item: LanguageHierarchyItemDto,
) -> Result<LanguageHierarchyItem, RpcError> {
    let path = workspace
        .resolve_existing(&item.path)
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))?;
    Ok(LanguageHierarchyItem {
        name: item.name,
        symbol_kind: item.symbol_kind,
        detail: item.detail,
        path,
        range: service_range(item.range),
        selection_range: service_range(item.selection_range),
        encoding: LanguagePositionEncoding::Utf16,
        data: item.data,
    })
}

fn hierarchy_entry_to_dto(
    workspace: &Path,
    source_path: &Path,
    source_text: &str,
    file_system: &dyn zeta_file_system::WorkspaceFileSystem,
    entry: LanguageHierarchyEntry,
) -> Option<LanguageHierarchyEntryDto> {
    let item_relative = entry.item.path.strip_prefix(workspace).ok()?.to_path_buf();
    let item_text = workspace_text(
        source_path,
        source_text,
        file_system,
        &entry.item.path,
        &item_relative,
    )?;
    let from_path = entry.from_path.as_ref();
    let (from_relative, from_text) = match from_path {
        Some(path) => {
            let relative = path.strip_prefix(workspace).ok()?.to_path_buf();
            let text = workspace_text(source_path, source_text, file_system, path, &relative)?;
            (Some(relative), Some(text))
        }
        None => (None, None),
    };
    let from_ranges = entry
        .from_ranges
        .into_iter()
        .map(|range| utf16_range(from_text.as_deref()?, range, entry.item.encoding))
        .collect::<Option<Vec<_>>>()?;
    Some(LanguageHierarchyEntryDto {
        item: LanguageHierarchyItemDto {
            name: entry.item.name,
            symbol_kind: entry.item.symbol_kind,
            detail: entry.item.detail,
            path: item_relative,
            range: utf16_range(&item_text, entry.item.range, entry.item.encoding)?,
            selection_range: utf16_range(
                &item_text,
                entry.item.selection_range,
                entry.item.encoding,
            )?,
            data: entry.item.data,
        },
        from_path: from_relative,
        from_ranges,
    })
}

fn workspace_text(
    source_path: &Path,
    source_text: &str,
    file_system: &dyn zeta_file_system::WorkspaceFileSystem,
    absolute: &Path,
    relative: &Path,
) -> Option<String> {
    if absolute == source_path {
        Some(source_text.to_owned())
    } else {
        String::from_utf8(
            file_system
                .read_file(relative, MAX_LANGUAGE_TARGET_BYTES)
                .ok()?,
        )
        .ok()
    }
}

fn service_range(range: LanguageRangeDto) -> LanguageLocationRange {
    LanguageLocationRange {
        start: LanguageLocationPosition {
            row: range.start.line_index,
            character: range.start.column_index,
        },
        end: LanguageLocationPosition {
            row: range.end.line_index,
            character: range.end.column_index,
        },
    }
}

fn workspace_edit_to_dto(
    workspace: &zeta_workspace::WorkspaceRoot,
    source_path: &Path,
    source_text: &str,
    file_system: &dyn zeta_file_system::WorkspaceFileSystem,
    edit: LanguageWorkspaceEdit,
) -> Result<LanguageWorkspaceEditDto, RpcError> {
    let mut entries = Vec::with_capacity(edit.entries.len());
    let mut virtual_text = HashMap::<std::path::PathBuf, Option<String>>::new();
    virtual_text.insert(source_path.to_path_buf(), Some(source_text.to_owned()));
    for entry in edit.entries {
        let entry = match entry {
            LanguageWorkspaceEditEntry::TextDocument(document) => {
                let relative = relative_workspace_path(workspace, &document.path)?;
                let text = virtual_workspace_text(
                    &mut virtual_text,
                    source_path,
                    source_text,
                    file_system,
                    &document.path,
                    &relative,
                )
                .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))?;
                let edits = document
                    .edits
                    .into_iter()
                    .map(|text_edit| {
                        Ok(LanguageTextEditDto {
                            range: utf16_range(&text, text_edit.range, edit.encoding).ok_or_else(
                                || language_error(AppServerErrorName::LanguageRequestFailed),
                            )?,
                            new_text: text_edit.new_text,
                        })
                    })
                    .collect::<Result<Vec<_>, RpcError>>()?;
                let next_text = apply_utf16_text_edits(&text, &edits)
                    .ok_or_else(|| language_error(AppServerErrorName::LanguageRequestFailed))?;
                virtual_text.insert(document.path.clone(), Some(next_text));
                LanguageWorkspaceEditEntryDto::TextDocument {
                    document: LanguageTextDocumentEditDto {
                        path: relative,
                        expected_text: text,
                        edits,
                    },
                }
            }
            LanguageWorkspaceEditEntry::Create { path, existing } => {
                let relative = relative_workspace_path(workspace, &path)?;
                let current = virtual_workspace_text(
                    &mut virtual_text,
                    source_path,
                    source_text,
                    file_system,
                    &path,
                    &relative,
                );
                if existing != LanguageExistingTargetBehavior::Ignore || current.is_none() {
                    virtual_text.insert(path.clone(), Some(String::new()));
                }
                LanguageWorkspaceEditEntryDto::Create {
                    path: relative,
                    existing: existing_target_behavior(existing),
                }
            }
            LanguageWorkspaceEditEntry::Rename {
                source,
                target,
                existing,
            } => {
                let source_relative = relative_workspace_path(workspace, &source)?;
                let target_relative = relative_workspace_path(workspace, &target)?;
                let source_content = virtual_workspace_text(
                    &mut virtual_text,
                    source_path,
                    source_text,
                    file_system,
                    &source,
                    &source_relative,
                );
                let target_content = virtual_workspace_text(
                    &mut virtual_text,
                    source_path,
                    source_text,
                    file_system,
                    &target,
                    &target_relative,
                );
                if existing != LanguageExistingTargetBehavior::Ignore || target_content.is_none() {
                    virtual_text.insert(source.clone(), None);
                    virtual_text.insert(target.clone(), source_content);
                }
                LanguageWorkspaceEditEntryDto::Rename {
                    source: source_relative,
                    target: target_relative,
                    existing: existing_target_behavior(existing),
                }
            }
            LanguageWorkspaceEditEntry::Delete {
                path,
                missing,
                mode,
            } => {
                virtual_text.insert(path.clone(), None);
                LanguageWorkspaceEditEntryDto::Delete {
                    path: relative_workspace_path(workspace, &path)?,
                    missing: match missing {
                        LanguageMissingTargetBehavior::Error => FsMissingTargetBehavior::Error,
                        LanguageMissingTargetBehavior::Ignore => FsMissingTargetBehavior::Ignore,
                    },
                    mode: match mode {
                        LanguageDeleteMode::FileOrEmptyDirectory => {
                            FsDeleteMode::FileOrEmptyDirectory
                        }
                        LanguageDeleteMode::Recursive => FsDeleteMode::Recursive,
                    },
                }
            }
        };
        entries.push(entry);
    }
    Ok(LanguageWorkspaceEditDto { entries })
}

fn virtual_workspace_text(
    states: &mut HashMap<std::path::PathBuf, Option<String>>,
    source_path: &Path,
    source_text: &str,
    file_system: &dyn zeta_file_system::WorkspaceFileSystem,
    absolute: &Path,
    relative: &Path,
) -> Option<String> {
    if let Some(state) = states.get(absolute) {
        return state.clone();
    }
    let text = workspace_text(source_path, source_text, file_system, absolute, relative);
    states.insert(absolute.to_path_buf(), text.clone());
    text
}

fn apply_utf16_text_edits(text: &str, edits: &[LanguageTextEditDto]) -> Option<String> {
    let mut projected = edits
        .iter()
        .map(|edit| Some((utf8_byte_range(text, edit.range)?, edit.new_text.as_str())))
        .collect::<Option<Vec<_>>>()?;
    projected.sort_by(|left, right| {
        right
            .0
            .start
            .cmp(&left.0.start)
            .then_with(|| right.0.end.cmp(&left.0.end))
    });
    let mut previous_start = text.len();
    let mut result = text.to_owned();
    for (range, replacement) in projected {
        if range.end > previous_start {
            return None;
        }
        result.replace_range(range.clone(), replacement);
        previous_start = range.start;
    }
    Some(result)
}

fn relative_workspace_path(
    workspace: &zeta_workspace::WorkspaceRoot,
    path: &Path,
) -> Result<std::path::PathBuf, RpcError> {
    path.strip_prefix(workspace.canonical_path())
        .map(Path::to_path_buf)
        .map_err(|_| language_error(AppServerErrorName::LanguageRequestFailed))
}

fn existing_target_behavior(value: LanguageExistingTargetBehavior) -> FsExistingTargetBehavior {
    match value {
        LanguageExistingTargetBehavior::Error => FsExistingTargetBehavior::Error,
        LanguageExistingTargetBehavior::Overwrite => FsExistingTargetBehavior::Overwrite,
        LanguageExistingTargetBehavior::Ignore => FsExistingTargetBehavior::Ignore,
    }
}

fn code_action_to_dto(
    workspace: &zeta_workspace::WorkspaceRoot,
    source_path: &Path,
    source_text: &str,
    file_system: &dyn zeta_file_system::WorkspaceFileSystem,
    action: LanguageCodeAction,
) -> Result<LanguageCodeActionDto, RpcError> {
    Ok(LanguageCodeActionDto {
        title: action.title,
        kind: action.kind,
        is_preferred: action.is_preferred,
        disabled_reason: action.disabled_reason,
        edit: action
            .edit
            .map(|edit| {
                workspace_edit_to_dto(workspace, source_path, source_text, file_system, edit)
            })
            .transpose()?,
        provider_data: action.provider_data,
    })
}

fn utf8_byte_range(text: &str, range: LanguageRangeDto) -> Option<std::ops::Range<usize>> {
    let start = utf8_position(text, range.start)?;
    let end = utf8_position(text, range.end)?;
    let start = absolute_byte_offset(text, start)?;
    let end = absolute_byte_offset(text, end)?;
    (start <= end).then_some(start..end)
}

fn absolute_byte_offset(text: &str, position: LanguageDocumentPosition) -> Option<usize> {
    let mut offset = 0usize;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if index == usize::try_from(position.row).ok()? {
            return Some(offset + usize::try_from(position.byte_offset).ok()?);
        }
        offset += line.len();
    }
    if text.ends_with('\n')
        && usize::try_from(position.row).ok()? == text.lines().count()
        && position.byte_offset == 0
    {
        Some(text.len())
    } else {
        None
    }
}

fn byte_range_to_utf16(text: &str, range: std::ops::Range<usize>) -> Option<LanguageRangeDto> {
    Some(LanguageRangeDto {
        start: byte_offset_to_utf16(text, range.start)?,
        end: byte_offset_to_utf16(text, range.end)?,
    })
}

fn byte_offset_to_utf16(text: &str, offset: usize) -> Option<LanguagePositionDto> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let prefix = &text[..offset];
    let line_index = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count()).ok()?;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column_index = u32::try_from(text[line_start..offset].encode_utf16().count()).ok()?;
    Some(LanguagePositionDto {
        line_index,
        column_index,
    })
}

fn text_for_utf16_range(text: &str, range: LanguageRangeDto) -> Option<String> {
    let bytes = utf8_byte_range(text, range)?;
    Some(text[bytes].to_owned())
}

fn language_service_id(language_id: &str) -> &str {
    if language_id == "shell" {
        "shellscript"
    } else {
        language_id
    }
}

fn target_location(
    relative: &Path,
    text: &str,
    target: LanguageLocationTarget,
) -> Option<LanguageLocationDto> {
    Some(LanguageLocationDto {
        path: relative.to_path_buf(),
        range: utf16_range(text, target.range, target.encoding)?,
        selection_range: utf16_range(text, target.selection_range, target.encoding)?,
    })
}

fn utf8_position(text: &str, position: LanguagePositionDto) -> Option<LanguageDocumentPosition> {
    let line = source_line(text, position.line_index)?;
    let utf16_column = usize::try_from(position.column_index).ok()?;
    let mut utf16 = 0usize;
    let mut byte_offset = 0usize;
    for character in line.chars() {
        if utf16 == utf16_column {
            return Some(LanguageDocumentPosition::new(
                position.line_index,
                u32::try_from(byte_offset).ok()?,
            ));
        }
        let width = character.len_utf16();
        if utf16 + width > utf16_column {
            return None;
        }
        utf16 += width;
        byte_offset += character.len_utf8();
    }
    if utf16 != utf16_column {
        return None;
    }
    Some(LanguageDocumentPosition::new(
        position.line_index,
        u32::try_from(byte_offset).ok()?,
    ))
}

fn utf16_range(
    text: &str,
    range: LanguageLocationRange,
    encoding: LanguagePositionEncoding,
) -> Option<LanguageRangeDto> {
    Some(LanguageRangeDto {
        start: utf16_target_position(text, range.start, encoding)?,
        end: utf16_target_position(text, range.end, encoding)?,
    })
}

fn utf16_target_position(
    text: &str,
    position: LanguageLocationPosition,
    encoding: LanguagePositionEncoding,
) -> Option<LanguagePositionDto> {
    let line = source_line(text, position.row)?;
    let column_index = match encoding {
        LanguagePositionEncoding::Utf16 => position.character,
        LanguagePositionEncoding::Utf8 => {
            let byte_offset = usize::try_from(position.character).ok()?;
            if byte_offset > line.len() || !line.is_char_boundary(byte_offset) {
                return None;
            }
            u32::try_from(line[..byte_offset].encode_utf16().count()).ok()?
        }
    };
    Some(LanguagePositionDto {
        line_index: position.row,
        column_index,
    })
}

fn source_line(text: &str, requested: u32) -> Option<&str> {
    let line = text.split('\n').nth(usize::try_from(requested).ok()?)?;
    Some(line.strip_suffix('\r').unwrap_or(line))
}

fn language_error(name: AppServerErrorName) -> RpcError {
    RpcError::new(-32072, name)
}
