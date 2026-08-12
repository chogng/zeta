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
    SessionCreateParams, SessionListResult, SessionReadParams, SessionRequest,
    SessionRequestParams, SessionRequestResult, SessionResult, SessionSubscribeParams,
    SessionSubscribeResult, SessionThreadProjection, SessionThreadReadParams,
    SessionThreadReadResult, SessionThreadResult, SessionThreadSubscribeParams,
    SessionThreadSubscribeResult, SessionThreadUnsubscribeParams, SessionUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::turn::{
    InputItem, TurnInteractionResolveResult, TurnInterruptResult, TurnStartResult,
};
use zeta_app_server_protocol::schema_hash;
use zeta_core::{
    ArchiveSessionThreadRequest, CreateSessionRequest, CreateSessionThreadRequest,
    ForkSessionThreadRequest, InterruptTurnRequest, ResolveTurnInteractionRequest,
    RewindSessionThreadRequest, SequenceExpectation, SessionLifecycleRequest,
    SetSessionModelRequest, ShellTurnInvocation, StartShellTurnRequest, StartTurnDisposition,
    StartTurnRequest, ThreadSnapshot, TurnStatus,
};
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::ModelRef;
use zeta_protocol::SessionStatus;
use zeta_protocol::UserInput;
use zeta_typst::{
    TypstCompileError, TypstCompileOutcome, TypstDiagnostic, TypstDiagnosticSeverity,
};

struct SessionMutation {
    command_id: zeta_protocol::CommandId,
    session_id: zeta_protocol::SessionId,
    expected_sequence: u64,
}

enum SessionLifecycleAction {
    Complete,
    Archive,
}

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
        if params
            .capabilities
            .agent_interactions
            .as_ref()
            .is_some_and(|capability| capability.version != 1 || capability.kinds.is_empty())
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        self.updates.set_agent_interaction_capability(
            connection.connection_id,
            params.capabilities.agent_interactions,
        );
        connection.initialized = true;
        let (file_system, git, workspace_search, code_index, cloud_code_index, terminal) =
            self.workspace_features();
        let extensions = self
            .extensions
            .lock()
            .map(|catalog| catalog.is_available())
            .unwrap_or(false);
        result(&InitializeResult {
            server_info: ServerInfo {
                name: "zeta-app-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            schema_hash: SchemaHash(schema_hash()),
            capabilities: ServerCapabilities {
                agent_interactions: true,
                document_collaboration: true,
                sessions: true,
                threads: true,
                turns: true,
                resources: true,
                file_system,
                git,
                workspace_search,
                code_index,
                cloud_code_index,
                terminal,
                typst: true,
                update_replay: true,
                extensions,
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

    fn set_session_model_request(
        &self,
        mutation: SessionMutation,
        model: ModelRef,
    ) -> Result<SessionResult, RpcError> {
        self.model_catalog.validate(&model).map_err(core_error)?;
        self.sessions
            .set_model(SetSessionModelRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                model,
            })
            .map_err(core_error)?;
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        Ok(SessionResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
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
        let thread_snapshots = session
            .threads
            .iter()
            .map(|membership| {
                self.sessions
                    .threads()
                    .read_thread(&membership.membership.thread_id)
                    .map_err(core_error)
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        let thread_projections = thread_snapshots
            .iter()
            .map(|thread| {
                let updates = self
                    .sessions
                    .threads()
                    .thread_updates_after(&thread.thread_id, 0)
                    .map_err(core_error)?;
                Ok(SessionThreadProjection {
                    thread: thread.public_thread(),
                    updates,
                })
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        self.updates.subscribe_session(
            connection.connection_id,
            params.session_id.clone(),
            session.sequence,
        );
        for projection in &thread_projections {
            self.updates.subscribe_session_thread(
                connection.connection_id,
                params.session_id.clone(),
                projection.thread.thread_id.clone(),
                projection.thread.sequence,
            );
        }
        for snapshot in &thread_snapshots {
            self.offer_pending_interactions(snapshot);
        }
        result(&SessionSubscribeResult {
            session: session.public_session(),
            updates,
            thread_projections,
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

    /// Routes one canonical mutation through the owning Session aggregate.
    pub(super) fn session_request(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let SessionRequestParams {
            command_id,
            session_id,
            expected_sequence,
            request,
        } = decode(params)?;
        let mutation = SessionMutation {
            command_id: command_id.clone(),
            session_id: session_id.clone(),
            expected_sequence,
        };

        match request {
            SessionRequest::Complete => result(&SessionRequestResult::Session(
                self.complete_session_request(mutation)?,
            )),
            SessionRequest::Archive => result(&SessionRequestResult::Session(
                self.archive_session_request(mutation)?,
            )),
            SessionRequest::Stop => result(&SessionRequestResult::Session(
                self.stop_session_request(mutation)?,
            )),
            SessionRequest::SetModel { model } => result(&SessionRequestResult::Session(
                self.set_session_model_request(mutation, model)?,
            )),
            SessionRequest::CreateThread { title } => result(&SessionRequestResult::Thread(
                self.create_session_thread_request(connection.connection_id, mutation, title)?,
            )),
            SessionRequest::ForkThread {
                parent_thread_id,
                title,
            } => result(&SessionRequestResult::Thread(
                self.fork_session_thread_request(
                    connection.connection_id,
                    mutation,
                    parent_thread_id,
                    title,
                )?,
            )),
            SessionRequest::RewindThread {
                parent_thread_id,
                before_turn_id,
                title,
            } => result(&SessionRequestResult::Thread(
                self.rewind_session_thread_request(
                    connection.connection_id,
                    mutation,
                    parent_thread_id,
                    before_turn_id,
                    title,
                )?,
            )),
            SessionRequest::ArchiveThread { thread_id } => result(&SessionRequestResult::Session(
                self.archive_session_thread_request(mutation, thread_id)?,
            )),
            SessionRequest::StartTurn { thread_id, input } => result(&SessionRequestResult::Turn(
                self.start_turn_request(mutation, thread_id, input)?,
            )),
            SessionRequest::StartShellTurn {
                thread_id,
                command,
                working_directory,
            } => result(&SessionRequestResult::Turn(self.start_shell_turn_request(
                mutation,
                thread_id,
                command,
                working_directory,
            )?)),
            SessionRequest::InterruptTurn { thread_id, turn_id } => {
                result(&SessionRequestResult::TurnInterrupt(
                    self.interrupt_turn_request(mutation, thread_id, turn_id)?,
                ))
            }
            SessionRequest::ResolveInteraction {
                thread_id,
                turn_id,
                request_id,
                response,
            } => result(&SessionRequestResult::Interaction(
                self.resolve_turn_interaction_request(
                    connection.connection_id,
                    mutation,
                    thread_id,
                    turn_id,
                    request_id,
                    response,
                )?,
            )),
        }
    }

    fn create_session_thread_request(
        &self,
        connection_id: u64,
        mutation: SessionMutation,
        title: String,
    ) -> Result<SessionThreadResult, RpcError> {
        let created = self
            .sessions
            .create_thread(CreateSessionThreadRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                title,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            created.thread_id.clone(),
            0,
        );
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        self.notify_thread_updates(&created.thread_id, 0)?;
        Ok(SessionThreadResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: created.thread_id,
        })
    }

    fn fork_session_thread_request(
        &self,
        connection_id: u64,
        mutation: SessionMutation,
        parent_thread_id: zeta_protocol::ThreadId,
        title: String,
    ) -> Result<SessionThreadResult, RpcError> {
        let forked = self
            .sessions
            .fork_thread(ForkSessionThreadRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                parent_thread_id,
                title,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            forked.thread_id.clone(),
            0,
        );
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        self.notify_thread_updates(&forked.thread_id, 0)?;
        Ok(SessionThreadResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: forked.thread_id,
        })
    }

    fn rewind_session_thread_request(
        &self,
        connection_id: u64,
        mutation: SessionMutation,
        parent_thread_id: zeta_protocol::ThreadId,
        before_turn_id: zeta_protocol::TurnId,
        title: String,
    ) -> Result<SessionThreadResult, RpcError> {
        let rewound = self
            .sessions
            .rewind_thread(RewindSessionThreadRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                parent_thread_id,
                before_turn_id,
                title,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            rewound.thread_id.clone(),
            0,
        );
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        self.notify_thread_updates(&rewound.thread_id, 0)?;
        Ok(SessionThreadResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
            thread_id: rewound.thread_id,
        })
    }

    fn archive_session_thread_request(
        &self,
        mutation: SessionMutation,
        thread_id: zeta_protocol::ThreadId,
    ) -> Result<SessionResult, RpcError> {
        self.sessions
            .archive_thread(ArchiveSessionThreadRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                thread_id,
            })
            .map_err(core_error)?;
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        Ok(SessionResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    fn complete_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        self.lifecycle_request(mutation, SessionLifecycleAction::Complete)
    }

    fn archive_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        self.lifecycle_request(mutation, SessionLifecycleAction::Archive)
    }

    fn stop_session_request(&self, mutation: SessionMutation) -> Result<SessionResult, RpcError> {
        let session_before = self
            .sessions
            .read_session(&mutation.session_id)
            .map_err(core_error)?;
        let thread_sequences = session_before
            .threads
            .iter()
            .map(|thread| {
                self.sessions
                    .threads()
                    .read_thread(&thread.membership.thread_id)
                    .map(|snapshot| (thread.membership.thread_id.clone(), snapshot.sequence))
                    .map_err(core_error)
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        self.sessions
            .stop(SessionLifecycleRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
            })
            .map_err(core_error)?;
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        for (thread_id, sequence) in thread_sequences {
            self.notify_thread_updates(&thread_id, sequence)?;
        }
        Ok(SessionResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    fn lifecycle_request(
        &self,
        mutation: SessionMutation,
        action: SessionLifecycleAction,
    ) -> Result<SessionResult, RpcError> {
        let request = SessionLifecycleRequest {
            command_id: mutation.command_id,
            session_id: mutation.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
        };
        match action {
            SessionLifecycleAction::Complete => self.sessions.complete(request),
            SessionLifecycleAction::Archive => self.sessions.archive(request),
        }
        .map_err(core_error)?;
        self.notify_session_updates(&mutation.session_id, mutation.expected_sequence)?;
        Ok(SessionResult {
            session: self
                .sessions
                .read_session(&mutation.session_id)
                .map_err(core_error)?
                .public_session(),
        })
    }

    pub(super) fn session_thread_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SessionThreadReadParams = decode(params)?;
        let snapshot = self.read_session_thread_snapshot(&params.session_id, &params.thread_id)?;
        result(&SessionThreadReadResult {
            thread: snapshot.public_thread(),
        })
    }

    pub(super) fn session_thread_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadSubscribeParams = decode(params)?;
        let snapshot = self.read_session_thread_snapshot(&params.session_id, &params.thread_id)?;
        let thread = snapshot.public_thread();
        let updates = self
            .sessions
            .threads()
            .thread_updates_after(&params.thread_id, params.after_sequence)
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection.connection_id,
            params.session_id.clone(),
            params.thread_id,
            thread.sequence,
        );
        self.offer_pending_interactions(&snapshot);
        result(&SessionThreadSubscribeResult { thread, updates })
    }

    pub(super) fn session_thread_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadUnsubscribeParams = decode(params)?;
        self.updates.unsubscribe_session_thread(
            connection.connection_id,
            &params.session_id,
            &params.thread_id,
        );
        Ok(Value::Null)
    }

    fn read_session_thread(
        &self,
        session_id: &zeta_protocol::SessionId,
        thread_id: &zeta_protocol::ThreadId,
    ) -> Result<zeta_protocol::Thread, RpcError> {
        self.read_session_thread_snapshot(session_id, thread_id)
            .map(|thread| thread.public_thread())
    }

    fn read_session_thread_snapshot(
        &self,
        session_id: &zeta_protocol::SessionId,
        thread_id: &zeta_protocol::ThreadId,
    ) -> Result<ThreadSnapshot, RpcError> {
        let thread = self
            .sessions
            .threads()
            .read_thread(thread_id)
            .map_err(core_error)?;
        if thread.session_id != *session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        Ok(thread)
    }

    fn offer_pending_interactions(&self, thread: &ThreadSnapshot) {
        for turn in &thread.turns {
            if let Some(interaction) = &turn.pending_interaction {
                self.updates.offer_agent_request(AgentRequestEnvelope {
                    session_id: thread.session_id.clone(),
                    thread_id: thread.thread_id.clone(),
                    turn_id: turn.turn_id.clone(),
                    interaction: interaction.clone(),
                });
            }
        }
    }

    fn start_turn_request(
        &self,
        mutation: SessionMutation,
        thread_id: zeta_protocol::ThreadId,
        input: Vec<InputItem>,
    ) -> Result<TurnStartResult, RpcError> {
        let thread_before = self
            .sessions
            .threads()
            .read_thread(&thread_id)
            .map_err(core_error)?;
        if thread_before.session_id != mutation.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let session = self
            .sessions
            .read_session(&mutation.session_id)
            .map_err(core_error)?;
        if session.status != SessionStatus::Active {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let input = input
            .into_iter()
            .map(|item| match item {
                InputItem::Text { text } => UserInput::Text { text },
                InputItem::Image { url } => UserInput::Image { url },
                InputItem::Skill { skill } => UserInput::Skill { skill },
            })
            .collect::<Vec<_>>();
        if let Some(replayed) =
            super::start_turn::replayed_result(&thread_before, &mutation.command_id, &input)?
        {
            return Ok(replayed);
        }
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
        let turn_executor = self.turn_executor_snapshot();
        let policy_revision = turn_executor.policy_revision();
        let activated_skills = input
            .iter()
            .filter_map(|item| match item {
                UserInput::Skill { skill } => Some(skill),
                UserInput::Text { .. }
                | UserInput::Image { .. }
                | UserInput::LocalImage { .. }
                | UserInput::Mention { .. } => None,
            })
            .map(|skill| {
                let runtime = self
                    .skills
                    .as_ref()
                    .ok_or_else(|| RpcError::new(-32050, AppServerErrorName::SkillsUnavailable))?;
                runtime
                    .activate_explicit(skill)
                    .map_err(|_| RpcError::new(-32051, AppServerErrorName::SkillOperationFailed))
            })
            .map(|activated| activated.map(|skill| skill.activation().clone()))
            .collect::<Result<Vec<_>, RpcError>>()?;
        let command_id = mutation.command_id.clone();
        let replay_input = input.clone();
        let start = self
            .sessions
            .start_turn(
                &mutation.session_id,
                &thread_id,
                StartTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    model,
                    policy_revision,
                    activated_skills,
                    input,
                },
            )
            .map_err(core_error)?;
        let turn_id = start.turn_id;
        if start.disposition == StartTurnDisposition::Replayed {
            let snapshot = self
                .sessions
                .threads()
                .read_thread(&thread_id)
                .map_err(core_error)?;
            return super::start_turn::replayed_result(&snapshot, &command_id, &replay_input)?
                .ok_or_else(|| RpcError::new(-32000, AppServerErrorName::InternalError));
        }
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        turn_executor
            .start(&thread_id, &turn_id)
            .map_err(core_error)?;
        Ok(TurnStartResult {
            turn_id,
            sequence: start.sequence,
        })
    }

    fn start_shell_turn_request(
        &self,
        mutation: SessionMutation,
        thread_id: zeta_protocol::ThreadId,
        command: String,
        working_directory: String,
    ) -> Result<TurnStartResult, RpcError> {
        let thread_before = self.read_session_thread(&mutation.session_id, &thread_id)?;
        if self
            .sessions
            .read_session(&mutation.session_id)
            .map_err(core_error)?
            .status
            != SessionStatus::Active
        {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let turn_executor = self.turn_executor_snapshot();
        let policy_revision = turn_executor.policy_revision();
        let start = self
            .sessions
            .start_shell_turn(
                &mutation.session_id,
                &thread_id,
                StartShellTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    policy_revision,
                    invocation: ShellTurnInvocation {
                        command,
                        shell_program: "/bin/sh".into(),
                        working_directory,
                    },
                },
            )
            .map_err(core_error)?;
        let turn_id = start.turn_id;
        if start.disposition == StartTurnDisposition::Replayed {
            let turn = thread_before
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
                | TurnStatus::Completed => Ok(TurnStartResult {
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
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        turn_executor
            .start_shell(&thread_id, &turn_id)
            .map_err(core_error)?;
        Ok(TurnStartResult {
            turn_id,
            sequence: start.sequence,
        })
    }

    fn interrupt_turn_request(
        &self,
        mutation: SessionMutation,
        thread_id: zeta_protocol::ThreadId,
        turn_id: zeta_protocol::TurnId,
    ) -> Result<TurnInterruptResult, RpcError> {
        self.read_session_thread(&mutation.session_id, &thread_id)?;
        let interrupted = self
            .sessions
            .threads()
            .interrupt_turn(
                &thread_id,
                InterruptTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    turn_id,
                },
            )
            .map_err(core_error)?;
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        Ok(TurnInterruptResult {
            sequence: interrupted.sequence,
        })
    }

    fn resolve_turn_interaction_request(
        &self,
        connection_id: u64,
        mutation: SessionMutation,
        thread_id: zeta_protocol::ThreadId,
        turn_id: zeta_protocol::TurnId,
        request_id: zeta_protocol::RequestId,
        response: zeta_protocol::AgentResponse,
    ) -> Result<TurnInteractionResolveResult, RpcError> {
        if !self
            .updates
            .is_agent_interaction_owner(connection_id, &request_id)
        {
            return Err(RpcError::new(
                -32030,
                AppServerErrorName::AgentInteractionNotOwner,
            ));
        }
        let _workspace_authority = self
            .workspace_authority_gate
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?;
        if self
            .updates
            .is_agent_interaction_expired(&request_id, super::update_broker::unix_time_millis())
        {
            return Err(RpcError::new(
                -32031,
                AppServerErrorName::AgentInteractionExpired,
            ));
        }
        let before = self
            .sessions
            .threads()
            .read_thread(&thread_id)
            .map_err(core_error)?;
        if before.session_id != mutation.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let approval_response = matches!(&response, zeta_protocol::AgentResponse::Approval { .. });
        let resumes_tool = approval_response
            && before
                .turns
                .iter()
                .find(|turn| turn.turn_id == turn_id)
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
        let turn_id_for_resume = turn_id.clone();
        let resolved = self
            .sessions
            .threads()
            .resolve_turn_interaction(
                &thread_id,
                ResolveTurnInteractionRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    turn_id,
                    request_id,
                    response,
                },
            )
            .map_err(core_error)?;
        if resumes_tool
            && resolved.disposition == zeta_core::ResolveTurnInteractionDisposition::Resolved
        {
            self.turn_executor_snapshot()
                .resume(&thread_id, &turn_id_for_resume)
                .map_err(core_error)?;
        }
        self.notify_thread_updates(&thread_id, before.sequence)?;
        Ok(TurnInteractionResolveResult {
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

pub(super) fn resource_rpc_error(error: crate::resource_store::ResourceError) -> RpcError {
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
