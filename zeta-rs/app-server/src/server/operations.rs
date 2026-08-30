use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::core_error;
use super::decode;
use super::result;
use base64::Engine;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use zeta_app_server_protocol::protocol::common::SchemaHash;
use zeta_app_server_protocol::protocol::common::ServerInfo;
use zeta_app_server_protocol::protocol::document::TypstCompileParams;
use zeta_app_server_protocol::protocol::document::TypstCompileResult;
use zeta_app_server_protocol::protocol::document::TypstDiagnosticDto;
use zeta_app_server_protocol::protocol::document::TypstDiagnosticSeverityDto;
use zeta_app_server_protocol::protocol::document::TypstSourceRangeDto;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::goal::ThreadGoalClearParams;
use zeta_app_server_protocol::protocol::goal::ThreadGoalClearResponse;
use zeta_app_server_protocol::protocol::goal::ThreadGoalGetParams;
use zeta_app_server_protocol::protocol::goal::ThreadGoalGetResponse;
use zeta_app_server_protocol::protocol::goal::ThreadGoalSetParams;
use zeta_app_server_protocol::protocol::goal::ThreadGoalSetResponse;
use zeta_app_server_protocol::protocol::initialize::InitializeParams;
use zeta_app_server_protocol::protocol::initialize::InitializeResult;
use zeta_app_server_protocol::protocol::initialize::ProtocolVersion;
use zeta_app_server_protocol::protocol::initialize::ServerCapabilities;
use zeta_app_server_protocol::protocol::model::ModelListResult;
use zeta_app_server_protocol::protocol::resources::ResourceMetadataParams;
use zeta_app_server_protocol::protocol::resources::ResourceMetadataResult;
use zeta_app_server_protocol::protocol::resources::ResourceReadParams;
use zeta_app_server_protocol::protocol::resources::ResourceReadResult;
use zeta_app_server_protocol::protocol::resources::ResourceReleaseParams;
use zeta_app_server_protocol::protocol::session::MAX_THREAD_SNAPSHOT_TURNS;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_app_server_protocol::protocol::session::SessionListResult;
use zeta_app_server_protocol::protocol::session::SessionReadParams;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_app_server_protocol::protocol::session::SessionRequestResult;
use zeta_app_server_protocol::protocol::session::SessionResult;
use zeta_app_server_protocol::protocol::session::SessionRewriteResult;
use zeta_app_server_protocol::protocol::session::SessionSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadProjection;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadReadResult;
use zeta_app_server_protocol::protocol::session::SessionThreadResult;
use zeta_app_server_protocol::protocol::session::SessionThreadSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionThreadSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadUnsubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionUnsubscribeParams;
use zeta_app_server_protocol::protocol::session::ThreadHistoryBoundary;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_app_server_protocol::protocol::turn::TurnInteractionResolveResult;
use zeta_app_server_protocol::protocol::turn::TurnInterruptResult;
use zeta_app_server_protocol::protocol::turn::TurnStartResult;
use zeta_app_server_protocol::protocol::turn::TurnSteerResult;
use zeta_app_server_protocol::schema_hash;
use zeta_core::CreateBranchRequest;
use zeta_core::ForkThreadRequest;
use zeta_core::InterruptTurnRequest;
use zeta_core::ResolveTurnInteractionRequest;
use zeta_core::RewindThreadRequest;
use zeta_core::SequenceExpectation;
use zeta_core::ShellTurnInvocation;
use zeta_core::StartContextCompactionRequest;
use zeta_core::StartShellTurnRequest;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnDisposition;
use zeta_core::StartTurnRequest;
use zeta_core::SteerTurnDisposition;
use zeta_core::SteerTurnRequest;
use zeta_core::ThreadSnapshot;
use zeta_core::TurnExecutionBackend;
use zeta_core::TurnStatus;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::ModelAccess;
use zeta_protocol::Session;
use zeta_protocol::SessionManagerActivity;
use zeta_protocol::SessionManagerInfo;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThread;
use zeta_protocol::StableTurnError;
use zeta_protocol::ThreadArchiveReason;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::UserInput;
use zeta_typst::TypstCompileError;
use zeta_typst::TypstCompileOutcome;
use zeta_typst::TypstDiagnostic;
use zeta_typst::TypstDiagnosticSeverity;

struct SessionMutation {
    command_id: zeta_protocol::CommandId,
    session_id: zeta_protocol::SessionId,
}

struct ThreadMutation {
    command_id: zeta_protocol::CommandId,
    session_id: zeta_protocol::SessionId,
    expected_sequence: u64,
}

struct RewriteSessionMutation {
    parent_thread_id: zeta_protocol::ThreadId,
    before_turn_id: zeta_protocol::TurnId,
    title: String,
    tool_mode: Option<zeta_protocol::ToolMode>,
    input: Vec<InputItem>,
}

enum TurnToolModeSelection {
    ConfiguredDefault,
    Explicit(zeta_protocol::ToolMode),
}

enum RewritePhase {
    Rewind,
    Start,
}

impl RewritePhase {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Rewind => "rewind",
            Self::Start => "start",
        }
    }
}

impl AppServer {
    pub(super) fn initialize(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        if connection.is_initialized() {
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
            .is_some_and(|capability| {
                let supports_dynamic = capability
                    .kinds
                    .contains(&zeta_protocol::AgentInteractionKind::DynamicTool);
                let dynamic_tools = capability.dynamic_tools.as_deref().unwrap_or_default();
                let unique_dynamic_tools = dynamic_tools
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == dynamic_tools.len();
                capability.version != 1
                    || capability.kinds.is_empty()
                    || !unique_dynamic_tools
                    || supports_dynamic != !dynamic_tools.is_empty()
            })
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        if params
            .capabilities
            .browser
            .as_ref()
            .is_some_and(|capability| {
                capability.version != 1 || (!capability.observe && !capability.input)
            })
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        if params
            .capabilities
            .dir_permissions_host
            .as_ref()
            .is_some_and(|capability| capability.version != 1)
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        if params
            .capabilities
            .work_coordination_host
            .as_ref()
            .is_some_and(|capability| capability.version != 1)
        {
            return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
        }
        if (params.capabilities.dir_permissions_host.is_some()
            || params.capabilities.work_coordination_host.is_some())
            && !connection.allows_product_host_capabilities()
        {
            return Err(RpcError::new(
                -32073,
                AppServerErrorName::PermissionRequired,
            ));
        }
        connection.set_dir_permissions_host(params.capabilities.dir_permissions_host.is_some());
        connection.set_work_coordination_host(params.capabilities.work_coordination_host.is_some());
        self.updates.set_work_coordination_host(
            connection.connection_id,
            params.capabilities.work_coordination_host.is_some(),
        );
        self.updates.set_agent_interaction_capability(
            connection.connection_id,
            params.capabilities.agent_interactions,
        );
        if let Some(capability) = params.capabilities.browser {
            self.browser_host.register(
                connection.connection_id,
                capability,
                connection.outbound_notifications.clone(),
            );
            if self.synchronize_browser_tool_availability().is_err() {
                self.browser_host.unregister(connection.connection_id);
                return Err(RpcError::new(-32603, AppServerErrorName::InternalError));
            }
        }
        connection.set_initialized();
        let (file_system, git, content_search, codebase, cloud_codebase, terminal, debug_adapter) =
            self.env_features();
        let extensions = self
            .extensions
            .lock()
            .map(|catalog| catalog.is_available())
            .unwrap_or(false);
        let mut capabilities = ServerCapabilities {
            agent_interactions: true,
            document_collaboration: true,
            sessions: true,
            threads: true,
            turns: true,
            work_coordination: self.work_coordination.is_some(),
            projects: self.projects.is_some(),
            resources: true,
            attachments: true,
            file_system,
            git,
            content_search,
            codebase,
            cloud_codebase,
            terminal,
            debug_adapter,
            typst: true,
            update_replay: true,
            extensions,
            extension_host: self.extension_hosts.is_some(),
            connectors: self.connectors.is_some(),
            plugins: self.plugins.is_some(),
            marketplace: self.marketplace_manager_client.is_some(),
            mcp: self.config.is_some(),
            mcp_oauth: self.mcp_oauth.is_some(),
            contracts: Default::default(),
        };
        capabilities.advertise_contracts();
        result(&InitializeResult {
            server_info: ServerInfo {
                name: "zeta-app-server".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            protocol_version: ProtocolVersion::current(),
            schema_hash: SchemaHash(schema_hash()),
            capabilities,
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
            .threads
            .start_thread(StartThreadRequest {
                command_id: params.command_id,
                title: params.title,
            })
            .map_err(core_error)?;
        self.updates.bind_session_scope(created.session_id.clone());
        self.threads
            .install_session_extensions(
                created.session_id.clone(),
                Arc::clone(&self.agent_extensions),
            )
            .map_err(core_error)?;
        self.updates
            .subscribe_session(connection.connection_id, created.session_id.clone());
        result(&SessionResult {
            session: self.session_view(&created.session_id)?,
        })
    }

    pub(super) fn session_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SessionReadParams = decode(params)?;
        result(&SessionResult {
            session: self.session_view(&params.session_id)?,
        })
    }

    pub(super) fn session_list(&self) -> Result<Value, RpcError> {
        result(&SessionListResult {
            sessions: self.session_views()?,
        })
    }

    pub(super) fn model_list(&self) -> Result<Value, RpcError> {
        result(&ModelListResult {
            models: self.model_catalog.list().map_err(core_error)?,
        })
    }

    pub(super) fn session_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionSubscribeParams = decode(params)?;
        let session = self.session_view(&params.session_id)?;
        let thread_snapshots = self
            .threads
            .list_session_threads(&params.session_id)
            .map_err(core_error)?;
        let thread_projections = thread_snapshots
            .iter()
            .map(|thread| {
                let thread = thread.public_thread();
                let updates = self
                    .threads
                    .thread_updates_after(&thread.thread_id, 0)
                    .map_err(core_error)?;
                Ok(SessionThreadProjection {
                    transcript: self.updates.thread_transcript_snapshot(&thread, true),
                    thread,
                    updates,
                })
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        self.updates
            .subscribe_session(connection.connection_id, params.session_id.clone());
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
            agent_tree: zeta_core::project_agent_tree(&thread_snapshots),
            session,
            thread_projections,
        })
    }

    pub(super) fn session_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionUnsubscribeParams = decode(params)?;
        let lost_dynamic_tools = self
            .updates
            .unsubscribe_session(connection.connection_id, &params.session_id);
        self.cancel_lost_dynamic_tool_owners(lost_dynamic_tools);
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
            request,
        } = decode(params)?;
        self.session_view(&session_id)?;
        let mutation = SessionMutation {
            command_id: command_id.clone(),
            session_id: session_id.clone(),
        };

        match request {
            SessionRequest::Archive => result(&SessionRequestResult::Session(
                self.archive_session_request(mutation)?,
            )),
            SessionRequest::Stop => result(&SessionRequestResult::Session(
                self.stop_session_request(mutation)?,
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
            SessionRequest::RewriteThread {
                parent_thread_id,
                before_turn_id,
                title,
                tool_mode,
                input,
            } => result(&SessionRequestResult::Rewrite(
                self.rewrite_session_thread_request(
                    connection.connection_id,
                    mutation,
                    RewriteSessionMutation {
                        parent_thread_id,
                        before_turn_id,
                        title,
                        tool_mode,
                        input,
                    },
                )?,
            )),
            SessionRequest::StartTurn {
                thread_id,
                expected_sequence,
                approval_mode,
                tool_mode,
                input,
            } => result(&SessionRequestResult::Turn(self.start_turn_request(
                thread_mutation(mutation, expected_sequence),
                thread_id,
                approval_mode,
                tool_mode,
                input,
            )?)),
            SessionRequest::StartReview {
                thread_id,
                expected_sequence,
                target,
            } => result(&SessionRequestResult::Turn(self.start_review_request(
                thread_mutation(mutation, expected_sequence),
                thread_id,
                target,
            )?)),
            SessionRequest::StartShellTurn {
                thread_id,
                expected_sequence,
                approval_mode,
                command,
                working_directory,
            } => result(&SessionRequestResult::Turn(self.start_shell_turn_request(
                thread_mutation(mutation, expected_sequence),
                thread_id,
                approval_mode,
                command,
                working_directory,
            )?)),
            SessionRequest::CompactContext {
                thread_id,
                expected_sequence,
                retention_prompt,
            } => result(&SessionRequestResult::Turn(
                self.start_context_compaction_request(
                    thread_mutation(mutation, expected_sequence),
                    thread_id,
                    retention_prompt,
                )?,
            )),
            SessionRequest::SteerTurn {
                thread_id,
                expected_sequence,
                turn_id,
                input,
            } => result(&SessionRequestResult::TurnSteer(self.steer_turn_request(
                thread_mutation(mutation, expected_sequence),
                thread_id,
                turn_id,
                input,
            )?)),
            SessionRequest::InterruptTurn {
                thread_id,
                expected_sequence,
                turn_id,
            } => result(&SessionRequestResult::TurnInterrupt(
                self.interrupt_turn_request(
                    thread_mutation(mutation, expected_sequence),
                    thread_id,
                    turn_id,
                )?,
            )),
            SessionRequest::ResolveInteraction {
                thread_id,
                expected_sequence,
                turn_id,
                request_id,
                response,
            } => result(&SessionRequestResult::Interaction(
                self.resolve_turn_interaction_request(
                    connection.connection_id,
                    thread_mutation(mutation, expected_sequence),
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
            .threads
            .create_branch(CreateBranchRequest {
                command_id: mutation.command_id,
                session_id: mutation.session_id.clone(),
                title,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            created.thread_id.clone(),
            0,
        );
        self.notify_thread_updates(&created.thread_id, 0)?;
        self.updates.publish_session_changed(&mutation.session_id);
        Ok(SessionThreadResult {
            session: self.session_view(&mutation.session_id)?,
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
            .threads
            .fork_thread(ForkThreadRequest {
                command_id: mutation.command_id,
                source_thread_id: parent_thread_id,
                title,
            })
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            forked.thread_id.clone(),
            0,
        );
        self.notify_thread_updates(&forked.thread_id, 0)?;
        self.updates.publish_session_changed(&mutation.session_id);
        Ok(SessionThreadResult {
            session: self.session_view(&mutation.session_id)?,
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
            .threads
            .rewind_thread(RewindThreadRequest {
                command_id: mutation.command_id,
                source_thread_id: parent_thread_id,
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
        self.notify_thread_updates(&rewound.thread_id, 0)?;
        self.updates.publish_session_changed(&mutation.session_id);
        Ok(SessionThreadResult {
            session: self.session_view(&mutation.session_id)?,
            thread_id: rewound.thread_id,
        })
    }

    fn rewrite_session_thread_request(
        &self,
        connection_id: u64,
        mutation: SessionMutation,
        rewrite: RewriteSessionMutation,
    ) -> Result<SessionRewriteResult, RpcError> {
        let normalized_input = normalize_input(rewrite.input.clone());
        let rewound = self
            .threads
            .rewind_thread(RewindThreadRequest {
                command_id: rewrite_phase_command_id(&mutation.command_id, RewritePhase::Rewind)?,
                source_thread_id: rewrite.parent_thread_id,
                before_turn_id: rewrite.before_turn_id,
                title: rewrite.title,
            })
            .map_err(core_error)?;
        let thread_id = rewound.thread_id;
        let thread_before = self.threads.read_thread(&thread_id).map_err(core_error)?;
        let start_command_id = rewrite_phase_command_id(&mutation.command_id, RewritePhase::Start)?;
        let turn = match super::start_turn::replayed_rewrite_result(
            &thread_before,
            &start_command_id,
            &normalized_input,
        )? {
            Some(replayed) => replayed,
            None => self.start_turn_request(
                ThreadMutation {
                    command_id: start_command_id,
                    session_id: mutation.session_id.clone(),
                    expected_sequence: thread_before.sequence,
                },
                thread_id.clone(),
                zeta_protocol::ApprovalMode::default(),
                rewrite.tool_mode,
                rewrite.input,
            )?,
        };
        self.updates.subscribe_session_thread(
            connection_id,
            mutation.session_id.clone(),
            thread_id.clone(),
            0,
        );
        self.notify_thread_updates(&thread_id, 0)?;
        self.updates.publish_session_changed(&mutation.session_id);
        Ok(SessionRewriteResult {
            session: self.session_view(&mutation.session_id)?,
            thread_id,
            turn,
        })
    }

    fn archive_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        let session_id = mutation.session_id.clone();
        let result = self.lifecycle_request(mutation)?;
        self.clear_session_dirs(&session_id);
        Ok(result)
    }

    fn stop_session_request(&self, mutation: SessionMutation) -> Result<SessionResult, RpcError> {
        let thread_sequences = self
            .threads
            .list_session_threads(&mutation.session_id)
            .map_err(core_error)?
            .into_iter()
            .map(|thread| (thread.thread_id, thread.sequence))
            .collect::<Vec<_>>();
        self.threads
            .archive_session_threads(
                &mutation.session_id,
                &mutation.command_id,
                zeta_protocol::ThreadArchiveReason::Stopped,
            )
            .map_err(core_error)?;
        self.clear_session_dirs(&mutation.session_id);
        for (thread_id, _) in &thread_sequences {
            self.multi_agent
                .cancel_descendants(thread_id)
                .map_err(core_error)?;
        }
        for (thread_id, sequence) in thread_sequences {
            self.notify_thread_updates(&thread_id, sequence)?;
        }
        self.updates.publish_session_changed(&mutation.session_id);
        if let Some(runtime) = &self.turn_changes
            && let Err(error) = runtime.enforce_cleanup_policy()
        {
            log::warn!("Thread worktree cleanup policy failed: {error}");
        }
        Ok(SessionResult {
            session: self.session_view(&mutation.session_id)?,
        })
    }

    fn lifecycle_request(&self, mutation: SessionMutation) -> Result<SessionResult, RpcError> {
        self.threads
            .archive_session_threads(
                &mutation.session_id,
                &mutation.command_id,
                zeta_protocol::ThreadArchiveReason::Completed,
            )
            .map_err(core_error)?;
        self.updates.publish_session_changed(&mutation.session_id);
        if let Some(runtime) = &self.turn_changes
            && let Err(error) = runtime.enforce_cleanup_policy()
        {
            log::warn!("Thread worktree cleanup policy failed: {error}");
        }
        Ok(SessionResult {
            session: self.session_view(&mutation.session_id)?,
        })
    }

    pub(super) fn session_thread_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SessionThreadReadParams = decode(params)?;
        let include_transient = !matches!(
            params.history.as_ref(),
            Some(ThreadSnapshotHistory::Before { .. })
        );
        let snapshot = self.read_session_thread_snapshot(&params.session_id, &params.thread_id)?;
        let (thread, history) = bounded_thread_snapshot(snapshot.public_thread(), params.history)?;
        let transcript = self
            .updates
            .thread_transcript_snapshot(&thread, include_transient);
        result(&SessionThreadReadResult {
            thread,
            transcript,
            history,
        })
    }

    pub(super) fn thread_goal_get(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ThreadGoalGetParams = decode(params)?;
        let thread = self.read_goal_thread(&params.thread_id)?;
        result(&ThreadGoalGetResponse { goal: thread.goal })
    }

    pub(super) fn thread_goal_set(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ThreadGoalSetParams = decode(params)?;
        self.read_goal_thread(&params.thread_id)?;
        let result_goal = self
            .threads
            .set_goal(
                &params.thread_id,
                zeta_core::SetGoalRequest {
                    objective: params.objective,
                    status: params.status,
                    token_budget: params.token_budget,
                },
            )
            .map_err(core_error)?;
        if result_goal.changed {
            self.updates.publish_thread_goal_updated(
                zeta_app_server_protocol::protocol::goal::ThreadGoalUpdatedNotification {
                    thread_id: params.thread_id.clone(),
                    turn_id: None,
                    goal: result_goal.goal.clone(),
                },
            );
            if result_goal.goal.status == zeta_protocol::ThreadGoalStatus::Active {
                self.turn_executor_snapshot()
                    .resume_goal_continuation(&params.thread_id)
                    .map_err(core_error)?;
            }
        }
        result(&ThreadGoalSetResponse {
            goal: result_goal.goal,
        })
    }

    pub(super) fn thread_goal_clear(&self, params: &Value) -> Result<Value, RpcError> {
        let params: ThreadGoalClearParams = decode(params)?;
        self.read_goal_thread(&params.thread_id)?;
        let cleared = self
            .threads
            .clear_goal(&params.thread_id)
            .map_err(core_error)?;
        if cleared {
            self.updates.publish_thread_goal_cleared(
                zeta_app_server_protocol::protocol::goal::ThreadGoalClearedNotification {
                    thread_id: params.thread_id,
                },
            );
        }
        result(&ThreadGoalClearResponse { cleared })
    }

    pub(super) fn session_thread_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadSubscribeParams = decode(params)?;
        let inserted_subscription = self.updates.subscribe_session_thread(
            connection.connection_id,
            params.session_id.clone(),
            params.thread_id.clone(),
            params.after_sequence,
        );
        let result = self.session_thread_subscribe_after_registration(connection, params.clone());
        if result.is_err() && inserted_subscription {
            let lost_dynamic_tools = self.updates.unsubscribe_session_thread(
                connection.connection_id,
                &params.session_id,
                &params.thread_id,
            );
            self.cancel_lost_dynamic_tool_owners(lost_dynamic_tools);
        }
        result
    }

    fn session_thread_subscribe_after_registration(
        &self,
        connection: &mut ConnectionState,
        params: SessionThreadSubscribeParams,
    ) -> Result<Value, RpcError> {
        let bounded_history = params.history.is_some();
        let include_transient = !matches!(
            params.history.as_ref(),
            Some(ThreadSnapshotHistory::Before { .. })
        );
        let snapshot = self.read_session_thread_snapshot(&params.session_id, &params.thread_id)?;
        let (thread, history) = bounded_thread_snapshot(snapshot.public_thread(), params.history)?;
        let transcript = self
            .updates
            .thread_transcript_snapshot(&thread, include_transient);
        let replay_after = if bounded_history {
            params.after_sequence.max(thread.sequence)
        } else {
            params.after_sequence
        };
        let updates = self
            .threads
            .thread_updates_after(&params.thread_id, replay_after)
            .map_err(core_error)?;
        self.updates.subscribe_session_thread(
            connection.connection_id,
            params.session_id.clone(),
            params.thread_id.clone(),
            thread.sequence,
        );
        self.offer_pending_interactions(&snapshot);
        result(&SessionThreadSubscribeResult {
            thread,
            transcript,
            updates,
            history,
        })
    }

    pub(super) fn session_thread_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionThreadUnsubscribeParams = decode(params)?;
        let lost_dynamic_tools = self.updates.unsubscribe_session_thread(
            connection.connection_id,
            &params.session_id,
            &params.thread_id,
        );
        self.cancel_lost_dynamic_tool_owners(lost_dynamic_tools);
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
        let thread = self.threads.read_thread(thread_id).map_err(core_error)?;
        if thread.session_id != *session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        Ok(thread)
    }

    fn read_goal_thread(
        &self,
        thread_id: &zeta_protocol::ThreadId,
    ) -> Result<ThreadSnapshot, RpcError> {
        let thread = self.threads.read_thread(thread_id).map_err(core_error)?;
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
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        approval_mode: zeta_protocol::ApprovalMode,
        requested_tool_mode: Option<zeta_protocol::ToolMode>,
        input: Vec<InputItem>,
    ) -> Result<TurnStartResult, RpcError> {
        let tool_mode = match requested_tool_mode {
            Some(tool_mode) => TurnToolModeSelection::Explicit(tool_mode),
            None => TurnToolModeSelection::ConfiguredDefault,
        };
        self.start_agent_turn_request(
            mutation,
            thread_id,
            approval_mode,
            tool_mode,
            normalize_input(input),
            zeta_protocol::TurnKind::Coding,
            zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
        )
    }

    fn start_review_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        target: zeta_protocol::ReviewTarget,
    ) -> Result<TurnStartResult, RpcError> {
        let prompt = zeta_prompts::review_target_prompt(&target)
            .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
        self.start_agent_turn_request(
            mutation,
            thread_id,
            zeta_protocol::ApprovalMode::default(),
            TurnToolModeSelection::Explicit(zeta_protocol::ToolMode::Direct),
            vec![UserInput::Text { text: prompt }],
            zeta_protocol::TurnKind::Review,
            zeta_prompts::REVIEW_PROMPT.freeze(),
        )
    }

    fn start_agent_turn_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        approval_mode: zeta_protocol::ApprovalMode,
        tool_mode_selection: TurnToolModeSelection,
        input: Vec<UserInput>,
        kind: zeta_protocol::TurnKind,
        instructions: zeta_protocol::TurnInstructions,
    ) -> Result<TurnStartResult, RpcError> {
        let thread_before = self.threads.read_thread(&thread_id).map_err(core_error)?;
        if thread_before.session_id != mutation.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        if thread_before.status != ThreadStatus::Active {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let tool_mode = match tool_mode_selection {
            TurnToolModeSelection::Explicit(tool_mode) => tool_mode,
            TurnToolModeSelection::ConfiguredDefault => match self.config.as_ref() {
                Some(config) => {
                    config
                        .read_snapshot()
                        .map_err(|_| RpcError::new(-32030, AppServerErrorName::ConfigUnavailable))?
                        .values
                        .tool_mode
                }
                None => zeta_protocol::ToolMode::Direct,
            },
        };
        if let Some(replayed) = super::start_turn::replayed_result(
            &thread_before,
            &mutation.command_id,
            tool_mode,
            &input,
        )? {
            return Ok(replayed);
        }
        let model = self
            .model_catalog
            .configured_default()
            .map_err(core_error)?;
        let _env_runtime = self
            .env_runtime_gate
            .lock()
            .map_err(|_| RpcError::new(-32000, AppServerErrorName::ServerOverloaded))?;
        let turn_executor = self.turn_executor_snapshot();
        let policy_revision = turn_executor.policy_revision();
        let tool_profile = turn_executor.tool_profile_snapshot().map_err(core_error)?;
        let command_id = mutation.command_id.clone();
        let replay_input = input.clone();
        let start = self
            .threads
            .start_turn(
                &thread_id,
                StartTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    model,
                    kind,
                    instructions,
                    policy_revision,
                    approval_mode,
                    tool_mode,
                    tool_profile: Some(tool_profile),
                    activated_skills: Vec::new(),
                    input,
                },
            )
            .map_err(core_error)?;
        let turn_id = start.turn_id;
        if start.disposition == StartTurnDisposition::Replayed {
            let snapshot = self.threads.read_thread(&thread_id).map_err(core_error)?;
            return super::start_turn::replayed_result(
                &snapshot,
                &command_id,
                tool_mode,
                &replay_input,
            )?
            .ok_or_else(|| RpcError::new(-32000, AppServerErrorName::InternalError));
        }
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        self.turn_backend
            .start(&thread_id, &turn_id)
            .map_err(core_error)?;
        Ok(TurnStartResult {
            turn_id,
            sequence: start.sequence,
        })
    }

    fn start_shell_turn_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        approval_mode: zeta_protocol::ApprovalMode,
        command: String,
        working_directory: String,
    ) -> Result<TurnStartResult, RpcError> {
        let thread_before = self.read_session_thread(&mutation.session_id, &thread_id)?;
        if thread_before.status != ThreadStatus::Active {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let turn_executor = self.turn_executor_snapshot();
        let policy_revision = turn_executor.policy_revision();
        let tool_call_id =
            zeta_protocol::ToolCallId::new(format!("shell-turn-{}", mutation.command_id))
                .expect("validated command identity produces a valid Tool Call ID");
        let shell_call = zeta_protocol::ToolCall {
            id: tool_call_id.clone(),
            name: zeta_protocol::ToolName::new("shell-command")
                .expect("static shell-command name is valid"),
            arguments: serde_json::json!({
                "program": "/bin/sh",
                "arguments": ["-lc", command],
                "working_directory": working_directory,
            }),
        };
        let binding = turn_executor
            .bind_tool_call(&shell_call, zeta_protocol::ToolCallCaller::Direct)
            .map_err(core_error)?;
        let start = self
            .threads
            .start_shell_turn(
                &thread_id,
                StartShellTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    policy_revision,
                    approval_mode,
                    tool_call_id,
                    binding,
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

    fn start_context_compaction_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        retention_prompt: Option<String>,
    ) -> Result<TurnStartResult, RpcError> {
        let thread = self.read_session_thread(&mutation.session_id, &thread_id)?;
        if thread.status != ThreadStatus::Active {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let model = self
            .model_catalog
            .configured_default()
            .map_err(core_error)?;
        let turn_executor = self.turn_executor_snapshot();
        let start = self
            .threads
            .start_context_compaction(
                &thread_id,
                StartContextCompactionRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    model,
                    policy_revision: turn_executor.policy_revision(),
                    retention_prompt,
                },
            )
            .map_err(core_error)?;
        if start.disposition == StartTurnDisposition::Replayed {
            return Ok(TurnStartResult {
                turn_id: start.turn_id,
                sequence: start.sequence,
            });
        }
        self.notify_thread_updates(&thread_id, thread.sequence)?;
        self.turn_backend
            .start(&thread_id, &start.turn_id)
            .map_err(core_error)?;
        Ok(TurnStartResult {
            turn_id: start.turn_id,
            sequence: start.sequence,
        })
    }

    fn interrupt_turn_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        turn_id: zeta_protocol::TurnId,
    ) -> Result<TurnInterruptResult, RpcError> {
        self.read_session_thread(&mutation.session_id, &thread_id)?;
        let descendant_sequences = self
            .threads
            .list_session_threads(&mutation.session_id)
            .map_err(core_error)?
            .into_iter()
            .filter(|descendant| descendant.thread_id != thread_id)
            .map(|descendant| (descendant.thread_id, descendant.sequence))
            .collect::<Vec<_>>();
        let interrupted = self
            .threads
            .interrupt_turn(
                &thread_id,
                InterruptTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    turn_id,
                },
            )
            .map_err(core_error)?;
        self.multi_agent
            .cancel_descendants(&thread_id)
            .map_err(core_error)?;
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        for (descendant_id, sequence) in descendant_sequences {
            self.notify_thread_updates(&descendant_id, sequence)?;
        }
        Ok(TurnInterruptResult {
            sequence: interrupted.sequence,
        })
    }

    fn steer_turn_request(
        &self,
        mutation: ThreadMutation,
        thread_id: zeta_protocol::ThreadId,
        turn_id: zeta_protocol::TurnId,
        input: Vec<InputItem>,
    ) -> Result<TurnSteerResult, RpcError> {
        let thread = self.read_session_thread(&mutation.session_id, &thread_id)?;
        let turn = thread
            .turns
            .iter()
            .find(|turn| turn.turn_id == turn_id)
            .ok_or_else(|| RpcError::new(-32011, AppServerErrorName::CoreOperationFailed))?;
        let subscription = turn.model.as_ref().is_some_and(|model| {
            zeta_model_provider_config::find_static_model(model)
                .is_some_and(|entry| entry.access == ModelAccess::Subscription)
        });
        if subscription
            && input
                .iter()
                .any(|item| !matches!(item, InputItem::Text { .. } | InputItem::Context { .. }))
        {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let input = input
            .into_iter()
            .map(|item| match item {
                InputItem::Text { text } => UserInput::Text { text },
                InputItem::Context { name, content } => UserInput::Context { name, content },
                InputItem::ImageAttachment { attachment } => {
                    UserInput::ImageAttachment { attachment }
                }
                InputItem::Image { url } => UserInput::Image { url },
                InputItem::Skill { skill } => UserInput::Skill { skill },
            })
            .collect::<Vec<_>>();
        let command_id = mutation.command_id.clone();
        let steered = self
            .threads
            .steer_turn(
                &thread_id,
                SteerTurnRequest {
                    command_id: mutation.command_id,
                    expected_sequence: SequenceExpectation::Exact(mutation.expected_sequence),
                    turn_id: turn_id.clone(),
                    input: input.clone(),
                },
            )
            .map_err(core_error)?;
        let sequence = match steered.disposition {
            SteerTurnDisposition::Steered => {
                if let Err(error) =
                    self.turn_backend
                        .steer(&thread_id, &turn_id, &command_id, &input)
                {
                    let _ = self.threads.fail_turn(
                        &thread_id,
                        &turn_id,
                        StableTurnError::model_invocation_failed(),
                    );
                    let _ = self.notify_thread_updates(&thread_id, mutation.expected_sequence);
                    return Err(core_error(error));
                }
                self.threads
                    .mark_turn_steer_delivered(&thread_id, &turn_id, &command_id)
                    .map_err(core_error)?
            }
            SteerTurnDisposition::Replayed => self
                .threads
                .read_thread(&thread_id)
                .map_err(core_error)?
                .steer_deliveries
                .get(&command_id)
                .copied()
                .ok_or_else(|| RpcError::new(-32010, AppServerErrorName::CoreOperationFailed))?,
        };
        self.notify_thread_updates(&thread_id, mutation.expected_sequence)?;
        Ok(TurnSteerResult { turn_id, sequence })
    }

    fn resolve_turn_interaction_request(
        &self,
        connection_id: u64,
        mutation: ThreadMutation,
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
        let _env_runtime = self
            .env_runtime_gate
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
        let before = self.threads.read_thread(&thread_id).map_err(core_error)?;
        if before.session_id != mutation.session_id {
            return Err(RpcError::new(
                -32010,
                AppServerErrorName::CoreOperationFailed,
            ));
        }
        let turn_id_for_resume = turn_id.clone();
        let resolved = self
            .threads
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
        if resolved.disposition == zeta_core::ResolveTurnInteractionDisposition::Resolved
            && !resolved.live_execution_woken
        {
            self.turn_backend
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

    fn session_view(&self, session_id: &zeta_protocol::SessionId) -> Result<Session, RpcError> {
        let mut snapshots = self
            .threads
            .list_session_threads(session_id)
            .map_err(core_error)?;
        if snapshots.is_empty() {
            return Err(core_error(zeta_core::CoreError::NotFound(
                session_id.to_string(),
            )));
        }
        snapshots.sort_by(|left, right| left.thread_id.cmp(&right.thread_id));
        let root = snapshots
            .iter()
            .find(|thread| thread.thread_id.as_str() == session_id.as_str())
            .unwrap_or(&snapshots[0]);
        let status = if snapshots
            .iter()
            .all(|thread| thread.status == ThreadStatus::Archived)
        {
            SessionStatus::Archived
        } else {
            SessionStatus::Active
        };
        let manager = session_manager_info(&snapshots, root.created_at_unix_ms, status);
        Ok(Session {
            session_id: session_id.clone(),
            title: root.title.clone(),
            status,
            manager,
            threads: snapshots
                .into_iter()
                .map(|thread| SessionThread {
                    completed_turn_duration_ms: thread.completed_turn_duration_ms(),
                    active_turn_started_at_unix_ms: thread.active_turn_started_at_unix_ms(),
                    thread_id: thread.thread_id,
                    title: thread.title,
                    created_at_unix_ms: thread.created_at_unix_ms,
                    parent_thread_id: thread.parent_thread_id,
                    forked_from_id: thread.forked_from_id,
                    status: thread.status,
                })
                .collect(),
        })
    }

    fn session_views(&self) -> Result<Vec<Session>, RpcError> {
        let session_ids = self
            .threads
            .list_threads()
            .map_err(core_error)?
            .into_iter()
            .map(|thread| thread.session_id)
            .collect::<std::collections::BTreeSet<_>>();
        session_ids
            .into_iter()
            .map(|session_id| self.session_view(&session_id))
            .collect()
    }

    fn notify_thread_updates(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        after_sequence: u64,
    ) -> Result<(), RpcError> {
        let updates = self
            .threads
            .thread_updates_after(thread_id, after_sequence)
            .map_err(core_error)?;
        self.updates.publish_thread(thread_id, &updates);
        Ok(())
    }
}

fn session_manager_info(
    threads: &[ThreadSnapshot],
    created_at_unix_ms: u64,
    lifecycle: SessionStatus,
) -> SessionManagerInfo {
    if lifecycle == SessionStatus::Archived {
        let stopped = threads
            .iter()
            .any(|thread| thread.archive_reason == Some(ThreadArchiveReason::Stopped));
        let archived_at = threads
            .iter()
            .filter_map(|thread| thread.archived_at_unix_ms)
            .max()
            .unwrap_or(created_at_unix_ms);
        let completed_at = latest_turn(threads, |_| true)
            .filter(|(_, turn)| turn.status == TurnStatus::Completed)
            .map(|(_, turn)| turn.status_changed_at_unix_ms)
            .unwrap_or(archived_at);
        return SessionManagerInfo {
            status: if stopped {
                SessionManagerStatus::Stopped
            } else {
                SessionManagerStatus::Completed
            },
            status_changed_at_unix_ms: if stopped { archived_at } else { completed_at },
            activity: None,
            summary: None,
        };
    }

    if let Some((_, turn)) = latest_turn(threads, |status| {
        matches!(
            status,
            TurnStatus::WaitingForApproval
                | TurnStatus::WaitingForUserInput
                | TurnStatus::WaitingForCapability
        )
    }) {
        return SessionManagerInfo {
            status: SessionManagerStatus::NeedsInput,
            status_changed_at_unix_ms: turn.status_changed_at_unix_ms,
            activity: turn.pending_interaction.as_ref().map(|interaction| {
                SessionManagerActivity::Question {
                    text: interaction_question(&interaction.request),
                }
            }),
            summary: None,
        };
    }

    if let Some((thread, turn)) = latest_turn(threads, |status| {
        matches!(
            status,
            TurnStatus::Created | TurnStatus::Running | TurnStatus::Cancelling
        )
    }) {
        return SessionManagerInfo {
            status: SessionManagerStatus::Working,
            status_changed_at_unix_ms: turn.status_changed_at_unix_ms,
            activity: working_operation(thread, turn),
            summary: None,
        };
    }

    let Some((_, turn)) = latest_turn(threads, |_| true) else {
        return SessionManagerInfo {
            status: SessionManagerStatus::Idle,
            status_changed_at_unix_ms: created_at_unix_ms,
            activity: None,
            summary: None,
        };
    };
    match turn.status {
        TurnStatus::Failed => SessionManagerInfo {
            status: SessionManagerStatus::Failed,
            status_changed_at_unix_ms: turn.status_changed_at_unix_ms,
            activity: turn
                .failure
                .as_ref()
                .map(|failure| SessionManagerActivity::Failure {
                    text: failure.message.clone(),
                }),
            summary: None,
        },
        TurnStatus::Interrupted => SessionManagerInfo {
            status: SessionManagerStatus::Stopped,
            status_changed_at_unix_ms: turn.status_changed_at_unix_ms,
            activity: None,
            summary: None,
        },
        TurnStatus::Completed => SessionManagerInfo {
            status: SessionManagerStatus::ReadyForReview,
            status_changed_at_unix_ms: turn.status_changed_at_unix_ms,
            activity: None,
            summary: None,
        },
        _ => SessionManagerInfo {
            status: SessionManagerStatus::Idle,
            status_changed_at_unix_ms: created_at_unix_ms,
            activity: None,
            summary: None,
        },
    }
}

fn latest_turn(
    threads: &[ThreadSnapshot],
    accepts: impl Fn(TurnStatus) -> bool,
) -> Option<(&ThreadSnapshot, &zeta_core::TurnSnapshot)> {
    threads
        .iter()
        .flat_map(|thread| thread.turns.iter().map(move |turn| (thread, turn)))
        .filter(|(_, turn)| accepts(turn.status))
        .max_by_key(|(_, turn)| turn.status_changed_at_unix_ms)
}

fn interaction_question(request: &AgentRequest) -> String {
    match request {
        AgentRequest::Approval { request } => request.reason.clone(),
        AgentRequest::UserInput { request } => request
            .questions
            .first()
            .map(|question| question.question.clone())
            .unwrap_or_else(|| "Waiting for user input".into()),
        AgentRequest::DynamicTool { call } => format!("Run {}?", call.name),
    }
}

fn working_operation(
    thread: &ThreadSnapshot,
    turn: &zeta_core::TurnSnapshot,
) -> Option<SessionManagerActivity> {
    let unresolved_tool = thread.items.iter().rev().find_map(|item| {
        let ThreadItem::ToolCall {
            turn_id,
            tool_call_id,
            name,
            ..
        } = item
        else {
            return None;
        };
        if turn_id != &turn.turn_id
            || thread.items.iter().any(|candidate| {
                matches!(
                    candidate,
                    ThreadItem::ToolResult {
                        tool_call_id: result_id,
                        ..
                    } if result_id == tool_call_id
                )
            })
        {
            return None;
        }
        Some(format!("Running {name}"))
    });
    let text = unresolved_tool.or_else(|| {
        turn.plan.as_ref().and_then(|plan| {
            plan.steps
                .iter()
                .find(|step| step.status == zeta_protocol::PlanStepStatus::InProgress)
                .map(|step| step.step.clone())
        })
    })?;
    Some(SessionManagerActivity::Operation { text })
}

fn thread_mutation(mutation: SessionMutation, expected_sequence: u64) -> ThreadMutation {
    ThreadMutation {
        command_id: mutation.command_id,
        session_id: mutation.session_id,
        expected_sequence,
    }
}

fn normalize_input(input: Vec<InputItem>) -> Vec<UserInput> {
    input
        .into_iter()
        .map(|item| match item {
            InputItem::Text { text } => UserInput::Text { text },
            InputItem::Context { name, content } => UserInput::Context { name, content },
            InputItem::ImageAttachment { attachment } => UserInput::ImageAttachment { attachment },
            InputItem::Image { url } => UserInput::Image { url },
            InputItem::Skill { skill } => UserInput::Skill { skill },
        })
        .collect()
}

fn rewrite_phase_command_id(
    operation_id: &zeta_protocol::CommandId,
    phase: RewritePhase,
) -> Result<zeta_protocol::CommandId, RpcError> {
    zeta_protocol::CommandId::new(format!(
        "session-rewrite/{}/{}",
        phase.as_str(),
        operation_id.as_str()
    ))
    .map_err(|_| RpcError::new(-32602, AppServerErrorName::InvalidParams))
}

fn bounded_thread_snapshot(
    mut thread: zeta_protocol::Thread,
    history: Option<ThreadSnapshotHistory>,
) -> Result<(zeta_protocol::Thread, Option<ThreadHistoryBoundary>), RpcError> {
    let Some(history) = history else {
        return Ok((thread, None));
    };
    let (start, end, turn_limit) = match history {
        ThreadSnapshotHistory::Latest { turn_limit } => {
            let retained = usize::try_from(turn_limit).unwrap_or(usize::MAX);
            (
                thread.turns.len().saturating_sub(retained),
                thread.turns.len(),
                turn_limit,
            )
        }
        ThreadSnapshotHistory::Before {
            turn_id,
            turn_limit,
        } => {
            let end = thread
                .turns
                .iter()
                .position(|turn| turn.turn_id == turn_id)
                .ok_or_else(|| RpcError::new(-32602, AppServerErrorName::InvalidParams))?;
            let retained = usize::try_from(turn_limit).unwrap_or(usize::MAX);
            (end.saturating_sub(retained), end, turn_limit)
        }
    };
    if turn_limit == 0 || turn_limit > MAX_THREAD_SNAPSHOT_TURNS {
        return Err(RpcError::new(-32602, AppServerErrorName::InvalidParams));
    }
    let has_older_turns = start > 0;
    thread.turns = thread.turns[start..end].to_vec();
    let oldest_turn_id = thread.turns.first().map(|turn| turn.turn_id.clone());
    Ok((
        thread,
        Some(ThreadHistoryBoundary {
            has_older_turns,
            oldest_turn_id,
        }),
    ))
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
