use super::*;
use memories::AddMemoryRequest;
use memories::Memories;
use memories::MemoryReadMode;
use memories::UpdateMemoryPolicyRequest;
use zeta_async_utils::CancellationSource;
use zeta_extension_api::ContextSourceRequest;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_file_access::Dir;
use zeta_file_access::Grant;
use zeta_file_access::GrantSource;
use zeta_file_access::Permissions;
use zeta_protocol::CommandId;
use zeta_protocol::ProjectId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;

#[test]
fn automatic_memories_follow_session_projects_and_current_directory_grants() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state.sqlite");
    let memories = Arc::new(Memories::new(Arc::new(
        zeta_state::SqliteMemoryStore::open(&path).unwrap(),
    )));
    let projects = Arc::new(ProjectCoordinator::new(Arc::new(
        zeta_state::SqliteProjectStore::open(&path).unwrap(),
    )));
    let dirs = Arc::new(DirGrants::default());
    let session = SessionId::new("session").unwrap();
    let other_session = SessionId::new("other-session").unwrap();
    let thread = ThreadId::new("thread").unwrap();
    let turn = TurnId::new("turn").unwrap();
    let project = ProjectId::new("project").unwrap();
    for (id, expected_revision, command) in [
        (
            "create",
            0,
            zeta_projects::ProjectCommand::Create {
                name: "project".into(),
                description: String::new(),
            },
        ),
        (
            "link",
            1,
            zeta_projects::ProjectCommand::LinkSession {
                session_id: session.clone(),
            },
        ),
    ] {
        projects
            .apply(zeta_projects::ProjectCommandRequest {
                command_id: CommandId::new(id).unwrap(),
                project_id: project.clone(),
                expected_revision,
                command,
            })
            .unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let dir = Dir::open_local(work.path()).unwrap();
    dirs.bind_thread_dir(thread.clone(), dir.clone());
    for (id, scope) in [
        ("profile", MemoryScope::Profile),
        (
            "project",
            MemoryScope::Project {
                project_id: project.clone(),
            },
        ),
        ("dir", MemoryScope::Dir { dir_id: dir.id() }),
    ] {
        memories
            .add_user_memory(
                AddMemoryRequest {
                    command_id: CommandId::new(format!("add-{id}")).unwrap(),
                    memory_id: memories::MemoryId::new(id).unwrap(),
                    scope: scope.clone(),
                    title: id.into(),
                    body: format!("Rust {id} decision"),
                },
                &CancellationSource::new().token(),
            )
            .unwrap();
        memories
            .update_policy(UpdateMemoryPolicyRequest {
                command_id: CommandId::new(format!("enable-{id}")).unwrap(),
                scope,
                expected_revision: 0,
                automatic_read: MemoryReadMode::FirstInvocation,
                model_write: memories::MemoryWriteMode::Disabled,
            })
            .unwrap();
    }
    let mut builder = ExtensionRegistryBuilder::new();
    memories_extension::install(
        &mut builder,
        memories,
        Arc::new(MemoryScopes {
            projects: Some(projects.clone()),
            dirs: dirs.clone(),
        }),
        Arc::new(super::super::update_broker::UpdateBroker::default()),
    );
    let source = builder.build();
    let request = ContextSourceRequest {
        session_id: &session,
        thread_id: &thread,
        turn_id: &turn,
        query: "Rust",
    };
    let cancellation = CancellationSource::new();
    let read_ids = |request: &ContextSourceRequest<'_>| {
        let mut ids = source
            .collect_context(request, &cancellation.token())
            .unwrap()
            .into_iter()
            .map(|entry| {
                memories::MemoryCitation::parse(&entry.reference)
                    .unwrap()
                    .memory_id
                    .to_string()
            })
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    assert_eq!(read_ids(&request), ["dir", "profile", "project"]);
    let other_thread = ThreadId::new("other-thread").unwrap();
    let other = ContextSourceRequest {
        session_id: &other_session,
        thread_id: &other_thread,
        turn_id: &turn,
        query: "Rust",
    };
    assert_eq!(read_ids(&other), ["profile"]);
    dirs.unbind_thread_dir(&thread);
    assert_eq!(read_ids(&request), ["profile", "project"]);
    dirs.add_dir(
        session.clone(),
        Grant::for_session_tree(
            session.clone(),
            dir.clone(),
            GrantSource::ExplicitUser,
            Permissions::new([Permission::ReadFiles]),
        ),
    )
    .unwrap();
    assert_eq!(read_ids(&request), ["dir", "profile", "project"]);
    dirs.remove_dir(&session, work.path());
    projects
        .apply(zeta_projects::ProjectCommandRequest {
            command_id: CommandId::new("unlink").unwrap(),
            project_id: project,
            expected_revision: 2,
            command: zeta_projects::ProjectCommand::UnlinkSession {
                session_id: session.clone(),
            },
        })
        .unwrap();
    assert_eq!(read_ids(&request), ["profile"]);
}

struct RevokingModel {
    memories: Arc<Memories>,
    measured: std::sync::Mutex<Vec<String>>,
    invoked: std::sync::Mutex<Vec<String>>,
}

impl zeta_core::ModelService for RevokingModel {
    fn context_budget(
        &self,
        _: zeta_core::ModelSelection<'_>,
    ) -> Result<zeta_core::ContextBudget, zeta_core::CoreError> {
        Ok(zeta_core::ContextBudget::core_managed(
            zeta_core::ContextTokenCount::new(15_000),
            zeta_core::ContextTokenCount::new(200),
            zeta_core::ContextTokenCount::new(100),
            zeta_core::ContextCompactionLimit::Tokens(zeta_core::ContextTokenCount::new(12_000)),
        ))
    }
    fn input_token_measurement_capability(
        &self,
        _: zeta_core::ModelSelection<'_>,
    ) -> Result<zeta_core::ContextTokenMeasurementCapability, zeta_core::CoreError> {
        Ok(zeta_core::ContextTokenMeasurementCapability::Remote)
    }
    fn measure_input(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<zeta_core::ContextTokenMeasurementOutcome, zeta_core::CoreError> {
        let mut measured = self.measured.lock().unwrap();
        measured.push(serde_json::to_string(request).unwrap());
        if measured.len() == 1 {
            self.memories
                .update_policy(UpdateMemoryPolicyRequest {
                    command_id: CommandId::new("revoke-during-prepare").unwrap(),
                    scope: MemoryScope::Profile,
                    expected_revision: 1,
                    automatic_read: MemoryReadMode::Disabled,
                    model_write: memories::MemoryWriteMode::Disabled,
                })
                .unwrap();
        }
        let count = if measured.len() == 1 { 12_000 } else { 1_000 };
        Ok(zeta_core::ContextTokenMeasurementOutcome::Measured(
            zeta_context_engine::ContextTokenMeasurement::exact(
                zeta_core::ContextTokenCount::new(count),
                zeta_context_engine::ContextTokenMeasurementSource::provider_preflight(
                    "memory-test",
                )
                .unwrap(),
            ),
        ))
    }
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, zeta_core::CoreError> {
        self.invoked
            .lock()
            .unwrap()
            .push(serde_json::to_string(request).unwrap());
        let text = if request
            .instructions
            .as_deref()
            .is_some_and(|text| text.contains("durable context checkpoint"))
        {
            "measured checkpoint"
        } else {
            "done"
        };
        Ok(zeta_protocol::ModelResponse {
            output: vec![zeta_protocol::ResponseItem::Text(text.into())],
            usage: None,
            billing: None,
            stop_reason: zeta_protocol::StopReason::Completed,
        })
    }
}

#[test]
fn memories_are_recollected_after_preflight_compaction_and_revocation() {
    let root = tempfile::tempdir().unwrap();
    let memories = Arc::new(Memories::new(Arc::new(
        zeta_state::SqliteMemoryStore::open(root.path().join("memories.sqlite")).unwrap(),
    )));
    memories
        .add_user_memory(
            AddMemoryRequest {
                command_id: CommandId::new("add").unwrap(),
                memory_id: memories::MemoryId::new("decision").unwrap(),
                scope: MemoryScope::Profile,
                title: "Rust decision".into(),
                body: "Rust MEMORY_REVOKED_DURING_PREPARATION".into(),
            },
            &CancellationSource::new().token(),
        )
        .unwrap();
    memories
        .update_policy(UpdateMemoryPolicyRequest {
            command_id: CommandId::new("enable").unwrap(),
            scope: MemoryScope::Profile,
            expected_revision: 0,
            automatic_read: MemoryReadMode::FirstInvocation,
            model_write: memories::MemoryWriteMode::Disabled,
        })
        .unwrap();
    let threads = Arc::new(zeta_core::ThreadController::with_store(Arc::new(
        zeta_state::SqliteThreadStore::open(root.path().join("threads.sqlite")).unwrap(),
    )));
    let thread = threads
        .start_thread(
            &zeta_core::NoThreadWorktreeBinder,
            zeta_core::StartThreadRequest {
                command_id: CommandId::new("thread").unwrap(),
                title: "Memory preparation test".into(),
                agent_id: None,
                agent: None,
            },
        )
        .unwrap();
    let start = |id: &str, text: &str| {
        threads
            .start_turn(
                &thread.thread_id,
                zeta_core::StartTurnRequest {
                    command_id: CommandId::new(id).unwrap(),
                    expected_sequence: zeta_core::SequenceExpectation::Any,
                    model: None,
                    kind: Default::default(),
                    instructions: zeta_protocol::TurnInstructions::new(
                        "memory-test",
                        "memory-test",
                        "1",
                        "Answer the user.",
                    )
                    .unwrap(),
                    policy_revision: "test".into(),
                    approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                    tool_mode: zeta_protocol::ToolMode::Direct,
                    tool_profile: None,
                    activated_skills: vec![],
                    input: vec![zeta_protocol::UserInput::Text { text: text.into() }],
                },
            )
            .unwrap()
            .turn_id
    };
    let seed = start("seed", "old request");
    threads
        .complete_turn(&thread.thread_id, &seed, "a".repeat(32_000))
        .unwrap();
    let current = start("current", "Rust current input");
    let mut builder = ExtensionRegistryBuilder::new();
    memories_extension::install(
        &mut builder,
        memories.clone(),
        Arc::new(MemoryScopes {
            projects: None,
            dirs: Arc::new(DirGrants::default()),
        }),
        Arc::new(super::super::update_broker::UpdateBroker::default()),
    );
    let model = Arc::new(RevokingModel {
        memories,
        measured: Default::default(),
        invoked: Default::default(),
    });
    zeta_core::TurnExecutor::without_tools(threads.clone(), model.clone())
        .with_extensions(Arc::new(builder.build()))
        .start(&thread.thread_id, &current)
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let snapshot = threads.read_thread(&thread.thread_id).unwrap();
        let turn = snapshot.turns.last().unwrap();
        if turn.status == zeta_protocol::TurnStatus::Completed {
            break;
        }
        assert!(
            !matches!(
                turn.status,
                zeta_protocol::TurnStatus::Failed | zeta_protocol::TurnStatus::Interrupted
            ),
            "{:?}",
            turn.failure
        );
        assert!(
            std::time::Instant::now() < deadline,
            "Memory compaction test timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let measured = model.measured.lock().unwrap();
    assert_eq!(measured.len(), 2);
    assert!(measured[0].contains("MEMORY_REVOKED_DURING_PREPARATION"));
    assert!(!measured[1].contains("MEMORY_REVOKED_DURING_PREPARATION"));
    assert_eq!(
        threads
            .read_thread(&thread.thread_id)
            .unwrap()
            .context_checkpoints
            .len(),
        1
    );
    assert!(
        model
            .invoked
            .lock()
            .unwrap()
            .iter()
            .all(|request| !request.contains("MEMORY_REVOKED_DURING_PREPARATION"))
    );
}
