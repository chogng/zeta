use super::*;
use crate::InMemoryThreadStore;
use crate::StartTurnRequest;

fn coordinator() -> SessionCoordinator {
    SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        Arc::new(ThreadController::with_store(Arc::new(
            InMemoryThreadStore::default(),
        ))),
    )
}

fn create_session(coordinator: &SessionCoordinator) -> CreateSessionResult {
    coordinator
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").expect("test ID is non-empty"),
            title: "task".into(),
            model: None,
        })
        .unwrap()
}

#[test]
fn create_thread_commits_membership_and_child_stream() {
    let coordinator = coordinator();
    let session = create_session(&coordinator);
    let thread = coordinator
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-thread").expect("test ID is non-empty"),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(1),
            title: "root".into(),
        })
        .unwrap();

    let snapshot = coordinator.read_session(&session.session_id).unwrap();
    assert_eq!(snapshot.sequence, 3);
    assert_eq!(
        snapshot.threads[0].membership.status,
        SessionThreadStatus::Active
    );
    assert_eq!(
        coordinator
            .threads
            .read_thread(&thread.thread_id)
            .unwrap()
            .session_id,
        session.session_id
    );
}

#[test]
fn commands_replay_by_typed_identity_and_reject_payload_conflicts() {
    let coordinator = coordinator();
    let created = create_session(&coordinator);
    let replayed = create_session(&coordinator);
    assert_eq!(replayed.session_id, created.session_id);
    assert_eq!(replayed.disposition, CommandDisposition::Replayed);

    let conflict = coordinator.create_session(CreateSessionRequest {
        command_id: CommandId::new("create-session").expect("test ID is non-empty"),
        title: "different".into(),
        model: None,
    });
    assert!(matches!(conflict, Err(CoreError::CommandConflict)));
}

#[test]
fn model_selection_is_durable_and_isolated_per_session() {
    let coordinator = coordinator();
    let first = create_session(&coordinator);
    let second = coordinator
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-second").unwrap(),
            title: "second".into(),
            model: None,
        })
        .unwrap();
    let selected = zeta_protocol::ModelRef::new(
        zeta_protocol::ProviderId::new("openai").unwrap(),
        zeta_protocol::ModelId::new("gpt-5.6").unwrap(),
    );

    coordinator
        .set_model(SetSessionModelRequest {
            command_id: CommandId::new("set-model").unwrap(),
            session_id: first.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(first.sequence),
            model: selected.clone(),
        })
        .unwrap();

    assert_eq!(
        coordinator.read_session(&first.session_id).unwrap().model,
        Some(selected)
    );
    assert_eq!(
        coordinator.read_session(&second.session_id).unwrap().model,
        None
    );
}

#[test]
fn fork_captures_the_parent_sequence_in_session_lineage() {
    let coordinator = coordinator();
    let session = create_session(&coordinator);
    let root = coordinator
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("root").expect("test ID is non-empty"),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(1),
            title: "root".into(),
        })
        .unwrap();
    let parent_sequence = coordinator
        .threads
        .read_thread(&root.thread_id)
        .unwrap()
        .sequence;
    coordinator
        .fork_thread(ForkSessionThreadRequest {
            command_id: CommandId::new("fork").expect("test ID is non-empty"),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(3),
            parent_thread_id: root.thread_id.clone(),
            title: "branch".into(),
        })
        .unwrap();

    assert!(matches!(
        &coordinator
            .read_session(&session.session_id)
            .unwrap()
            .threads[1]
            .membership
            .origin,
        ThreadOrigin::Fork {
            parent_thread_id,
            parent_sequence: sequence,
        } if parent_thread_id == &root.thread_id && *sequence == parent_sequence
    ));
}

#[test]
fn rewind_creates_a_child_with_only_turns_before_the_checkpoint() {
    let coordinator = coordinator();
    let session = create_session(&coordinator);
    let root = coordinator
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("root").unwrap(),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(1),
            title: "root".into(),
        })
        .unwrap();
    let mut turn_ids = Vec::new();
    let mut sequence = 1;
    for (index, prompt) in ["first", "second", "third"].into_iter().enumerate() {
        let started = coordinator
            .threads
            .start_turn(
                &root.thread_id,
                StartTurnRequest {
                    command_id: CommandId::new(format!("turn-{index}")).unwrap(),
                    expected_sequence: SequenceExpectation::Exact(sequence),
                    model: None,
                    input: vec![zeta_protocol::UserInput::Text {
                        text: prompt.into(),
                    }],
                },
            )
            .unwrap();
        turn_ids.push(started.turn_id.clone());
        sequence = coordinator
            .threads
            .complete_turn(&root.thread_id, &started.turn_id, format!("answer {index}"))
            .unwrap()
            .sequence;
    }

    let rewound = coordinator
        .rewind_thread(RewindSessionThreadRequest {
            command_id: CommandId::new("rewind").unwrap(),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(3),
            parent_thread_id: root.thread_id.clone(),
            before_turn_id: turn_ids[1].clone(),
            title: "rewound".into(),
        })
        .unwrap();
    let replayed = coordinator
        .rewind_thread(RewindSessionThreadRequest {
            command_id: CommandId::new("rewind").unwrap(),
            session_id: session.session_id.clone(),
            expected_sequence: SequenceExpectation::Exact(3),
            parent_thread_id: root.thread_id.clone(),
            before_turn_id: turn_ids[1].clone(),
            title: "rewound".into(),
        })
        .unwrap();
    let child = coordinator
        .threads
        .read_thread(&rewound.thread_id)
        .unwrap()
        .public_thread();

    assert_eq!(replayed.thread_id, rewound.thread_id);
    assert_eq!(replayed.disposition, CommandDisposition::Replayed);
    assert_eq!(child.turns.len(), 1);
    assert_eq!(child.turns[0].turn_id, turn_ids[0]);
    assert!(child.turns[0].items.iter().any(|item| {
        matches!(item, zeta_protocol::ThreadItem::UserMessage { text, .. } if text == "first")
    }));
    assert!(matches!(
        &coordinator
            .read_session(&session.session_id)
            .unwrap()
            .threads[1]
            .membership
            .origin,
        ThreadOrigin::Rewind {
            parent_thread_id,
            before_turn_id,
            ..
        } if parent_thread_id == &root.thread_id && before_turn_id == &turn_ids[1]
    ));
}
