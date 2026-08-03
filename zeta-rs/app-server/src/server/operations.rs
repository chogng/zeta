use super::{AppServer, ConnectionState, RpcError, core_error, decode, result};
use base64::Engine;
use serde_json::Value;
use std::time::Duration;
use zeta_app_server_protocol::protocol::common::{SchemaHash, ServerInfo};
use zeta_app_server_protocol::protocol::document::{
    TypstCompileParams, TypstCompileResult, TypstDiagnosticDto, TypstDiagnosticSeverityDto,
    TypstSourceRangeDto,
};
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::initialize::{
    InitializeParams, InitializeResult, ServerCapabilities,
};
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_app_server_protocol::protocol::resources::{
    ResourceMetadataParams, ResourceMetadataResult, ResourceReadParams, ResourceReadResult,
    ResourceReleaseParams,
};
use zeta_app_server_protocol::protocol::session::{
    SessionCommandParams, SessionCreateParams, SessionListResult, SessionModelSetParams,
    SessionReadParams, SessionResult, SessionSubscribeParams, SessionSubscribeResult,
    SessionThreadArchiveParams, SessionThreadCreateParams, SessionThreadForkParams,
    SessionThreadResult, SessionThreadRewindParams, SessionUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::thread::{
    ThreadReadParams, ThreadReadResult, ThreadSubscribeParams, ThreadSubscribeResult,
    ThreadUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::turn::{
    InputItem, ShellTurnStartParams, TurnInteractionResolveParams, TurnInteractionResolveResult,
    TurnInterruptParams, TurnInterruptResult, TurnStartParams, TurnStartResult,
};
use zeta_app_server_protocol::schema_hash;
use zeta_core::{
    ArchiveSessionThreadRequest, CreateSessionRequest, CreateSessionThreadRequest,
    ForkSessionThreadRequest, InterruptTurnRequest, ResolveTurnInteractionRequest,
    RewindSessionThreadRequest, SequenceExpectation, SessionLifecycleRequest,
    SetSessionModelRequest, ShellTurnInvocation, StartShellTurnRequest, StartTurnDisposition,
    StartTurnRequest, TurnStatus,
};
use zeta_protocol::UserInput;
use zeta_typst::{
    TypstCompileError, TypstCompileOutcome, TypstDiagnostic, TypstDiagnosticSeverity,
};

impl AppServer {
    pub(super) fn initialize(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        if connection.initialized {
            return Err(RpcError::new(
                -32002,
                AppServerErrorName::AlreadyInitialized,
            ));
        }
        let params: InitializeParams = decode(params)?;
        if params.client_info.name.trim().is_empty() || params.client_info.version.trim().is_empty()
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        connection.initialized = true;
        let (file_system, git, workspace_search, terminal) = self.workspace_features();
        result(&InitializeResult {
            server_info: ServerInfo {
                name: "zeta-app-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            schema_hash: SchemaHash(schema_hash()),
            capabilities: ServerCapabilities {
                sessions: true,
                threads: true,
                turns: true,
                resources: true,
                file_system,
                git,
                workspace_search,
                terminal,
                typst: true,
                update_replay: true,
            },
            slash_commands: self.slash_commands.commands().to_vec(),
        })
    }

    pub(super) fn session_create(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionCreateParams = decode(params)?;
        let created = self
            .sessions
            .create_session(CreateSessionRequest {
                command_id: params.command_id,
                title: params.title,
                model: self
                    .model_catalog
                    .configured_default()
                    .map_err(core_error)?,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session(
            connection.connection_id,
            created.session_id.clone(),
            created.sequence,
        );
        result(&SessionResult {
            session: self
                .sessions
                .read_session(&created.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn session_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SessionReadParams = decode(params)?;
        result(&SessionResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn session_list(&self) -> Result<Value, RpcError> {
        result(&SessionListResult {
            sessions: self
                .sessions
                .list_sessions()
                .map_err(core_error)?
                .into_iter()
                .map(|session| session.public_session())
                .collect(),
        })
    }

    pub(super) fn model_list(&self) -> Result<Value, RpcError> {
        result(&ModelListResult {
            models: self.model_catalog.list().map_err(core_error)?,
        })
    }

    pub(super) fn session_model_set(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionModelSetParams = decode(params)?;
        self.model_catalog
            .validate(&params.model)
            .map_err(core_error)?;
        self.sessions
            .set_model(SetSessionModelRequest {
                command_id: params.command_id,
                session_id: params.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                model: params.model,
            })
            .map_err(core_error)?;
        self.notify_session_updates(&params.session_id, params.expected_sequence)?;
        result(&SessionResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn session_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionSubscribeParams = decode(params)?;
        let session = self
            .sessions
            .read_session(&params.session_id)
            .map_err(core_error)?;
        let updates = self
            .sessions
            .session_updates_after(&params.session_id, params.after_sequence)
            .map_err(core_error)?;
        self.updates.subscribe_session(
            connection.connection_id,
            params.session_id,
            session.sequence,
        );
        result(&SessionSubscribeResult {
            session: session.public_session(),
            updates,
        })
    }

    pub(super) fn session_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionUnsubscribeParams = decode(params)?;
        self.updates
            .unsubscribe_session(connection.connection_id, &params.session_id);
        Ok(Value::Null)
    }

    pub(super) fn session_thread_create(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadCreateParams = decode(params)?;
        let previous_sequence = params.expected_sequence;
        let created = self
            .sessions
            .create_thread(CreateSessionThreadRequest {
                command_id: params.command_id,
                session_id: params.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                title: params.title,
            })
            .map_err(core_error)?;
        self.updates
            .subscribe_thread(connection.connection_id, created.thread_id.clone(), 0);
        self.notify_session_updates(&params.session_id, previous_sequence)?;
        self.notify_thread_updates(&created.thread_id, 0)?;
        result(&SessionThreadResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: created.thread_id,
        })
    }

    pub(super) fn session_thread_fork(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadForkParams = decode(params)?;
        let previous_sequence = params.expected_sequence;
        let forked = self
            .sessions
            .fork_thread(ForkSessionThreadRequest {
                command_id: params.command_id,
                session_id: params.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                parent_thread_id: params.parent_thread_id,
                title: params.title,
            })
            .map_err(core_error)?;
        self.updates
            .subscribe_thread(connection.connection_id, forked.thread_id.clone(), 0);
        self.notify_session_updates(&params.session_id, previous_sequence)?;
        self.notify_thread_updates(&forked.thread_id, 0)?;
        result(&SessionThreadResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: forked.thread_id,
        })
    }

    pub(super) fn session_thread_rewind(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadRewindParams = decode(params)?;
        let previous_sequence = params.expected_sequence;
        let rewound = self
            .sessions
            .rewind_thread(RewindSessionThreadRequest {
                command_id: params.command_id,
                session_id: params.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                parent_thread_id: params.parent_thread_id,
                before_turn_id: params.before_turn_id,
                title: params.title,
            })
            .map_err(core_error)?;
        self.updates
            .subscribe_thread(connection.connection_id, rewound.thread_id.clone(), 0);
        self.notify_session_updates(&params.session_id, previous_sequence)?;
        self.notify_thread_updates(&rewound.thread_id, 0)?;
        result(&SessionThreadResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: rewound.thread_id,
        })
    }

    pub(super) fn session_thread_archive(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadArchiveParams = decode(params)?;
        self.sessions
            .archive_thread(ArchiveSessionThreadRequest {
                command_id: params.command_id,
                session_id: params.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                thread_id: params.thread_id,
            })
            .map_err(core_error)?;
        self.notify_session_updates(&params.session_id, params.expected_sequence)?;
        result(&SessionResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn session_complete(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        self.session_lifecycle(params, true)
    }

    pub(super) fn session_archive(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        self.session_lifecycle(params, false)
    }

    fn session_lifecycle(&self, params: &Value, complete: bool) -> Result<Value, RpcError> {
        let params: SessionCommandParams = decode(params)?;
        let request = SessionLifecycleRequest {
            command_id: params.command_id,
            session_id: params.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
        };
        if complete {
            self.sessions.complete(request)
        } else {
            self.sessions.archive(request)
        }
        .map_err(core_error)?;
        self.notify_session_updates(&params.session_id, params.expected_sequence)?;
        result(&SessionResult {
            session: self
                .sessions
                .read_session(&params.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn thread_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ThreadReadParams = decode(params)?;
        result(&ThreadReadResult {
            thread: self
                .sessions
                .threads()
                .read_thread(&params.thread_id)
                .map_err(core_error)?
                .public_thread(),
        })
    }

    pub(super) fn thread_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ThreadSubscribeParams = decode(params)?;
        let thread = self
            .sessions
            .threads()
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        let updates = self
            .sessions
            .threads()
            .thread_updates_after(&params.thread_id, params.after_sequence)
            .map_err(core_error)?;
        self.updates
            .subscribe_thread(connection.connection_id, params.thread_id, thread.sequence);
        result(&ThreadSubscribeResult {
            thread: thread.public_thread(),
            updates,
        })
    }

    pub(super) fn thread_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ThreadUnsubscribeParams = decode(params)?;
        self.updates
            .unsubscribe_thread(connection.connection_id, &params.thread_id);
        Ok(Value::Null)
    }

    pub(super) fn turn_start(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TurnStartParams = decode(params)?;
        let thread_before = self
            .sessions
            .threads()
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        if thread_before.session_id != params.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let session = self
            .sessions
            .read_session(&params.session_id)
            .map_err(core_error)?;
        let model = match session.model {
            Some(model) => Some(model),
            None => self
                .model_catalog
                .configured_default()
                .map_err(core_error)?,
        };
        let _workspace_authority = self
            .workspace_authority_gate
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?;
        let start = self
            .sessions
            .threads()
            .start_turn(
                &params.thread_id,
                StartTurnRequest {
                    command_id: params.command_id,
                    expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                    model,
                    input: params
                        .input
                        .into_iter()
                        .map(|item| match item {
                            InputItem::Text { text } => UserInput::Text { text },
                            InputItem::Image { url } => UserInput::Image { url },
                        })
                        .collect(),
                },
            )
            .map_err(core_error)?;
        let turn_id = start.turn_id;
        if start.disposition == StartTurnDisposition::Replayed {
            let snapshot = self
                .sessions
                .threads()
                .read_thread(&params.thread_id)
                .map_err(core_error)?;
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == turn_id)
                .ok_or_else(|| RpcError::new(-32000, AppServerErrorName::InternalError))?;
            return match turn.status {
                TurnStatus::Created
                | TurnStatus::Running
                | TurnStatus::WaitingForApproval
                | TurnStatus::WaitingForUserInput
                | TurnStatus::WaitingForCapability
                | TurnStatus::Completed => result(&TurnStartResult {
                    turn_id,
                    sequence: start.sequence,
                }),
                TurnStatus::Failed | TurnStatus::Interrupted => Err(RpcError::new(
                    -32010,
                    AppServerErrorName::CoreOperationFailed,
                )),
                TurnStatus::Cancelling => {
                    Err(RpcError::new(-32000, AppServerErrorName::ServerOverloaded))
                }
            };
        }
        self.notify_thread_updates(&params.thread_id, params.expected_sequence)?;
        self.turn_executor_snapshot()
            .start(&params.thread_id, &turn_id)
            .map_err(core_error)?;
        result(&TurnStartResult {
            turn_id,
            sequence: start.sequence,
        })
    }

    pub(super) fn shell_turn_start(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ShellTurnStartParams = decode(params)?;
        let thread_before = self
            .sessions
            .threads()
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        if thread_before.session_id != params.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let start = self
            .sessions
            .threads()
            .start_shell_turn(
                &params.thread_id,
                StartShellTurnRequest {
                    command_id: params.command_id,
                    expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                    invocation: ShellTurnInvocation {
                        command: params.command,
                        shell_program: "/bin/sh".into(),
                        working_directory: params.working_directory,
                    },
                },
            )
            .map_err(core_error)?;
        let turn_id = start.turn_id;
        if start.disposition == StartTurnDisposition::Replayed {
            let snapshot = self
                .sessions
                .threads()
                .read_thread(&params.thread_id)
                .map_err(core_error)?;
            let turn = snapshot
                .turns
                .iter()
                .find(|turn| turn.turn_id == turn_id)
                .ok_or_else(|| RpcError::new(-32000, AppServerErrorName::InternalError))?;
            return match turn.status {
                TurnStatus::Created
                | TurnStatus::Running
                | TurnStatus::WaitingForApproval
                | TurnStatus::WaitingForUserInput
                | TurnStatus::WaitingForCapability
                | TurnStatus::Completed => result(&TurnStartResult {
                    turn_id,
                    sequence: start.sequence,
                }),
                TurnStatus::Failed | TurnStatus::Interrupted => Err(RpcError::new(
                    -32010,
                    AppServerErrorName::CoreOperationFailed,
                )),
                TurnStatus::Cancelling => {
                    Err(RpcError::new(-32000, AppServerErrorName::ServerOverloaded))
                }
            };
        }
        self.notify_thread_updates(&params.thread_id, params.expected_sequence)?;
        self.turn_executor_snapshot()
            .start_shell(&params.thread_id, &turn_id)
            .map_err(core_error)?;
        result(&TurnStartResult {
            turn_id,
            sequence: start.sequence,
        })
    }

    pub(super) fn turn_interrupt(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TurnInterruptParams = decode(params)?;
        let before = self
            .sessions
            .threads()
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        if before.session_id != params.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let interrupted = self
            .sessions
            .threads()
            .interrupt_turn(
                &params.thread_id,
                InterruptTurnRequest {
                    command_id: params.command_id,
                    expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                    turn_id: params.turn_id,
                },
            )
            .map_err(core_error)?;
        self.notify_thread_updates(&params.thread_id, params.expected_sequence)?;
        result(&TurnInterruptResult {
            sequence: interrupted.sequence,
        })
    }

    pub(super) fn turn_interaction_resolve(
        &self,
        _connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TurnInteractionResolveParams = decode(params)?;
        let _workspace_authority = self
            .workspace_authority_gate
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?;
        let before = self
            .sessions
            .threads()
            .read_thread(&params.thread_id)
            .map_err(core_error)?;
        if before.session_id != params.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let approval_response = matches!(
            &params.response,
            zeta_protocol::AgentResponse::Approval { .. }
        );
        let resumes_tool = approval_response
            && before
                .turns
                .iter()
                .find(|turn| turn.turn_id == params.turn_id)
                .and_then(|turn| turn.pending_interaction.as_ref())
                .filter(|interaction| {
                    matches!(
                        &interaction.request,
                        zeta_protocol::AgentRequest::Approval { .. }
                    )
                })
                .and_then(|interaction| interaction.item_id.as_ref())
                .is_some_and(|item_id| {
                    before.items.iter().any(|item| {
                        matches!(
                            item,
                            zeta_protocol::ThreadItem::ToolCall {
                                item_id: call_item_id,
                                ..
                            } if call_item_id == item_id
                        )
                    })
                });
        let turn_id = params.turn_id.clone();
        let resolved = self
            .sessions
            .threads()
            .resolve_turn_interaction(
                &params.thread_id,
                ResolveTurnInteractionRequest {
                    command_id: params.command_id,
                    expected_sequence: SequenceExpectation::Exact(params.expected_sequence),
                    turn_id: params.turn_id,
                    request_id: params.request_id,
                    response: params.response,
                },
            )
            .map_err(core_error)?;
        if resumes_tool
            && resolved.disposition == zeta_core::ResolveTurnInteractionDisposition::Resolved
        {
            self.turn_executor_snapshot()
                .resume(&params.thread_id, &turn_id)
                .map_err(core_error)?;
        }
        self.notify_thread_updates(&params.thread_id, before.sequence)?;
        result(&TurnInteractionResolveResult {
            sequence: resolved.sequence,
        })
    }

    pub(super) fn resource_metadata(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceMetadataParams = decode(params)?;
        let metadata = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?
            .metadata(connection.connection_id, &params.resource_id)
            .map_err(resource_rpc_error)?;
        result(&ResourceMetadataResult {
            resource_id: metadata.resource_id,
            mime_type: metadata.mime_type,
            size: metadata.size,
            sha256: metadata.sha256,
        })
    }

    pub(super) fn typst_compile(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: TypstCompileParams = decode(params)?;
        let outcome = self
            .typst
            .compile(&params.source)
            .map_err(|error| match error {
                TypstCompileError::SourceTooLarge { .. } => {
                    RpcError::new(-32602, AppServerErrorName::InvalidParams)
                }
            })?;
        match outcome {
            TypstCompileOutcome::Success(success) => {
                let metadata = self
                    .resources
                    .lock()
                    .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?
                    .create(
                        connection.connection_id,
                        "application/pdf".into(),
                        success.pdf,
                        Duration::from_secs(300),
                    )
                    .map_err(resource_rpc_error)?;
                result(&TypstCompileResult::Success {
                    resource: ResourceMetadataResult {
                        resource_id: metadata.resource_id,
                        mime_type: metadata.mime_type,
                        size: metadata.size,
                        sha256: metadata.sha256,
                    },
                    warnings: success
                        .warnings
                        .into_iter()
                        .map(typst_diagnostic_dto)
                        .collect(),
                })
            }
            TypstCompileOutcome::Failed { diagnostics } => result(&TypstCompileResult::Failed {
                diagnostics: diagnostics.into_iter().map(typst_diagnostic_dto).collect(),
            }),
        }
    }

    pub(super) fn resource_read(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceReadParams = decode(params)?;
        let resource_id = params.resource_id.clone();
        let chunk = self
            .resources
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?
            .read(
                connection.connection_id,
                &params.resource_id,
                params.offset,
                params.max_bytes,
            )
            .map_err(resource_rpc_error)?;
        result(&ResourceReadResult {
            resource_id,
            offset: chunk.offset,
            data_base64: base64::engine::general_purpose::STANDARD.encode(&chunk.data),
            decoded_length: chunk.data.len(),
            eof: chunk.eof,
        })
    }

    pub(super) fn resource_release(
        &self,
        connection: &ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: ResourceReleaseParams = decode(params)?;
        self.resources
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?
            .release(connection.connection_id, &params.resource_id)
            .map_err(resource_rpc_error)?;
        Ok(Value::Null)
    }

    fn notify_session_updates(
        &self,
        session_id: &zeta_protocol::SessionId,
        after_sequence: u64,
    ) -> Result<(), RpcError> {
        let updates = self
            .sessions
            .session_updates_after(session_id, after_sequence)
            .map_err(core_error)?;
        self.updates.publish_session(session_id, &updates);
        Ok(())
    }

    fn notify_thread_updates(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        after_sequence: u64,
    ) -> Result<(), RpcError> {
        let updates = self
            .sessions
            .threads()
            .thread_updates_after(thread_id, after_sequence)
            .map_err(core_error)?;
        self.updates.publish_thread(thread_id, &updates);
        Ok(())
    }
}

fn resource_rpc_error(error: crate::resource_store::ResourceError) -> RpcError {
    use crate::resource_store::ResourceError;
    RpcError::new(
        -32020,
        match error {
            ResourceError::NotFound => AppServerErrorName::ResourceNotFound,
            ResourceError::NotOwner => AppServerErrorName::ResourceNotOwner,
            ResourceError::TooLarge => AppServerErrorName::ResourceTooLarge,
            ResourceError::InvalidChunkSize => AppServerErrorName::InvalidResourceChunkSize,
            ResourceError::InvalidOffset => AppServerErrorName::InvalidResourceOffset,
        },
    )
}

fn typst_diagnostic_dto(diagnostic: TypstDiagnostic) -> TypstDiagnosticDto {
    TypstDiagnosticDto {
        severity: match diagnostic.severity {
            TypstDiagnosticSeverity::Error => TypstDiagnosticSeverityDto::Error,
            TypstDiagnosticSeverity::Warning => TypstDiagnosticSeverityDto::Warning,
        },
        message: diagnostic.message,
        hints: diagnostic.hints,
        range: diagnostic.range.map(|range| TypstSourceRangeDto {
            start: range.start,
            end: range.end,
        }),
    }
}
