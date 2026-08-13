use zeta_lsp::lsp_types::PartialResultParams;
use zeta_lsp::lsp_types::WorkDoneProgressParams;
use zeta_lsp::lsp_types::WorkspaceDiagnosticParams;
use zeta_lsp::lsp_types::request::WorkspaceDiagnosticRequest;

use super::Supervisor;
use super::SupervisorCommand;
use super::request_runtime::supports_request;
use crate::LanguageRequestId;
use crate::LanguageRequestKind;
use crate::LanguageServiceEvent;
use crate::LanguageWorkspaceDiagnostics;
use crate::workspace_diagnostics::project_workspace_diagnostics;

impl Supervisor {
    pub(super) fn begin_workspace_diagnostics(
        &mut self,
        id: LanguageRequestId,
        language_id: String,
    ) {
        let Some((server, server_epoch)) = self.server_for_language(&language_id) else {
            self.emit_unsupported_workspace_diagnostics(id, language_id);
            return;
        };
        let Ok(client) = self.router.client_for_language(&language_id).cloned() else {
            self.emit_unsupported_workspace_diagnostics(id, language_id);
            return;
        };
        if !supports_request(&client, LanguageRequestKind::WorkspaceDiagnostics) {
            self.emit_unsupported_workspace_diagnostics(id, language_id);
            return;
        }
        let encoding = client.initialization().position_encoding.clone();
        let generation = self.generation;
        let commands = self.commands.clone();
        let completion_language_id = language_id.clone();
        tokio::spawn(async move {
            let result = client
                .request::<WorkspaceDiagnosticRequest>(WorkspaceDiagnosticParams {
                    identifier: None,
                    previous_result_ids: Vec::new(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|response| {
                    project_workspace_diagnostics(id, language_id, &encoding, response)
                });
            let _ = commands.send(SupervisorCommand::WorkspaceDiagnosticsCompleted {
                id,
                language_id: completion_language_id,
                server,
                generation,
                server_epoch,
                result,
            });
        });
    }

    pub(super) fn complete_workspace_diagnostics(
        &self,
        id: LanguageRequestId,
        language_id: String,
        server: zeta_lsp::LanguageServerName,
        generation: u64,
        server_epoch: u64,
        result: Result<LanguageWorkspaceDiagnostics, String>,
    ) {
        if generation != self.generation
            || !self.servers.get(&server).is_some_and(|managed| {
                managed.epoch == server_epoch && managed.phase == super::ManagedServerPhase::Ready
            })
        {
            return;
        }
        match result {
            Ok(result) => self.emit(LanguageServiceEvent::WorkspaceDiagnostics(result)),
            Err(message) => {
                self.emit(LanguageServiceEvent::ServerMessage {
                    server: server.to_string(),
                    severity: super::LanguageServerMessageSeverity::Error,
                    show: false,
                    message,
                });
                self.emit_unsupported_workspace_diagnostics(id, language_id);
            }
        }
    }

    fn emit_unsupported_workspace_diagnostics(
        &self,
        request_id: LanguageRequestId,
        language_id: String,
    ) {
        self.emit(LanguageServiceEvent::WorkspaceDiagnostics(
            LanguageWorkspaceDiagnostics {
                request_id,
                language_id,
                supported: false,
                diagnostics: Vec::new(),
            },
        ));
    }
}
