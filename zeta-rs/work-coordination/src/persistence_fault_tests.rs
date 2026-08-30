use super::WorkCommandDisposition;
use super::WorkCoordinator;
use super::WorkParticipant;
use super::WorkParticipantRelation;
use super::WorkRun;
use super::WorkRunCommand;
use super::WorkRunCommandRequest;
use super::WorkRunCommit;
use super::WorkRunStore;
use super::WorkRunStoreError;
use super::WorkRunStoreOutcome;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_protocol::CommandId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkRunId;

#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    BeforeCommit,
    AfterCommit,
}

#[derive(Default)]
struct FaultStore {
    state: Mutex<FaultState>,
}

#[derive(Default)]
struct FaultState {
    runs: BTreeMap<WorkRunId, WorkRun>,
    commands: BTreeMap<CommandId, WorkRunCommit>,
    next_fault: Fault,
}

impl FaultStore {
    fn fail_next(&self, fault: Fault) {
        self.state.lock().expect("fault store lock").next_fault = fault;
    }
}

impl WorkRunStore for FaultStore {
    fn list(&self) -> Result<Vec<WorkRun>, WorkRunStoreError> {
        Ok(self
            .state
            .lock()
            .expect("fault store lock")
            .runs
            .values()
            .cloned()
            .collect())
    }

    fn load(&self, work_run_id: &WorkRunId) -> Result<WorkRun, WorkRunStoreError> {
        self.state
            .lock()
            .expect("fault store lock")
            .runs
            .get(work_run_id)
            .cloned()
            .ok_or_else(|| WorkRunStoreError::NotFound(work_run_id.to_string()))
    }

    fn load_command(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<WorkRunCommit>, WorkRunStoreError> {
        Ok(self
            .state
            .lock()
            .expect("fault store lock")
            .commands
            .get(command_id)
            .cloned())
    }

    fn commit(&self, commit: &WorkRunCommit) -> Result<WorkRunStoreOutcome, WorkRunStoreError> {
        let mut state = self.state.lock().expect("fault store lock");
        if let Some(existing) = state.commands.get(&commit.request.command_id) {
            return if existing.request == commit.request {
                Ok(WorkRunStoreOutcome::Replayed(existing.result.clone()))
            } else {
                Err(WorkRunStoreError::CommandConflict)
            };
        }
        let actual = state
            .runs
            .get(&commit.request.work_run_id)
            .map_or(0, |run| run.revision);
        if actual != commit.request.expected_revision {
            return Err(WorkRunStoreError::RevisionConflict {
                expected: commit.request.expected_revision,
                actual,
            });
        }
        let fault = std::mem::take(&mut state.next_fault);
        if matches!(fault, Fault::BeforeCommit) {
            return Err(WorkRunStoreError::Storage(
                "injected failure before atomic commit".into(),
            ));
        }
        state
            .runs
            .insert(commit.result.work_run_id.clone(), commit.result.clone());
        state
            .commands
            .insert(commit.request.command_id.clone(), commit.clone());
        if matches!(fault, Fault::AfterCommit) {
            return Err(WorkRunStoreError::Storage(
                "injected lost acknowledgement after atomic commit".into(),
            ));
        }
        Ok(WorkRunStoreOutcome::Applied)
    }
}

#[test]
fn a_failure_before_commit_leaves_no_partial_state_and_the_exact_retry_commits_once() {
    let store = Arc::new(FaultStore::default());
    let coordinator = WorkCoordinator::new(store.clone());
    let create = create_request("before-create", "before-run");
    store.fail_next(Fault::BeforeCommit);

    assert!(matches!(
        coordinator.apply(create.clone()),
        Err(super::WorkCoordinationError::Storage(message))
            if message.contains("before atomic commit")
    ));
    assert!(matches!(
        coordinator.read(&create.work_run_id),
        Err(super::WorkCoordinationError::NotFound(_))
    ));
    assert!(
        coordinator
            .command_receipt(&create.command_id)
            .unwrap()
            .is_none()
    );

    let committed = coordinator.apply(create.clone()).unwrap();
    assert_eq!(committed.disposition, WorkCommandDisposition::Committed);
    assert_eq!(committed.work_run.revision, 1);
    let replayed = coordinator.apply(create).unwrap();
    assert_eq!(replayed.disposition, WorkCommandDisposition::Replayed);
    assert_eq!(replayed.work_run, committed.work_run);
}

#[test]
fn a_lost_acknowledgement_replays_the_durable_result_without_a_second_transition() {
    let store = Arc::new(FaultStore::default());
    let coordinator = WorkCoordinator::new(store.clone());
    let created = coordinator
        .apply(create_request("after-create", "after-run"))
        .unwrap()
        .work_run;
    let revise = WorkRunCommandRequest {
        command_id: CommandId::new("after-revise").unwrap(),
        work_run_id: created.work_run_id.clone(),
        expected_revision: created.revision,
        command: WorkRunCommand::ReviseGoal {
            objective: "revised reliable goal".into(),
            acceptance_conditions: vec!["the exact receipt is replayed".into()],
            exclusions: Vec::new(),
        },
    };
    store.fail_next(Fault::AfterCommit);

    assert!(matches!(
        coordinator.apply(revise.clone()),
        Err(super::WorkCoordinationError::Storage(message))
            if message.contains("lost acknowledgement")
    ));
    let durable = coordinator.read(&created.work_run_id).unwrap();
    assert_eq!(durable.revision, 2);
    assert_eq!(durable.goals.len(), 2);

    let replayed = coordinator.apply(revise.clone()).unwrap();
    assert_eq!(replayed.disposition, WorkCommandDisposition::Replayed);
    assert_eq!(replayed.work_run, durable);
    assert_eq!(coordinator.read(&created.work_run_id).unwrap().revision, 2);

    let mut conflicting = revise;
    conflicting.command = WorkRunCommand::ReviseGoal {
        objective: "different retry payload".into(),
        acceptance_conditions: vec!["must not replace the receipt".into()],
        exclusions: Vec::new(),
    };
    assert_eq!(
        coordinator.apply(conflicting),
        Err(super::WorkCoordinationError::CommandConflict)
    );
}

fn create_request(command_id: &str, work_run_id: &str) -> WorkRunCommandRequest {
    WorkRunCommandRequest {
        command_id: CommandId::new(command_id).unwrap(),
        work_run_id: WorkRunId::new(work_run_id).unwrap(),
        expected_revision: 0,
        command: WorkRunCommand::Create {
            objective: "deliver a fault-safe result".into(),
            acceptance_conditions: vec!["no partial durable state".into()],
            exclusions: Vec::new(),
            root_participant: WorkParticipant {
                session_id: SessionId::new(format!("{work_run_id}-session")).unwrap(),
                thread_id: ThreadId::new(format!("{work_run_id}-thread")).unwrap(),
                relation: WorkParticipantRelation::Root,
            },
        },
    }
}
