use super::*;
use zeta_work_coordination::IssueIdentity;
use zeta_work_coordination::IssueRepositoryIdentity;
use zeta_work_coordination::IssueWorkItem;
use zeta_work_coordination::IssueWorkflow;

fn plan() -> IssueAssignmentPlan {
    let mut workflow = IssueWorkflow::default();
    workflow.assignee = "owner".into();
    IssueAssignmentPlan {
        repository: IssueRepositoryIdentity {
            host: "github.com".into(),
            node_id: "repo-node".into(),
            owner: "team".into(),
            name: "repo".into(),
        },
        workflow,
        config_revision: 1,
        model: None,
        planning_tokens: 0,
        base_commit: "a".repeat(40),
        target_branch: "main".into(),
        items: vec![IssueWorkItem {
            id: "one".into(),
            issues: vec![IssueIdentity {
                node_id: "issue18".into(),
                number: 18,
                title: "Search".into(),
                updated_at: "now".into(),
                material_digest: "digest".into(),
            }],
            objective: "Fix search".into(),
            acceptance_conditions: vec!["Search returns the exact issue".into()],
            scope: Default::default(),
            dependencies: Default::default(),
            agent: String::new(),
        }],
    }
}

#[test]
fn issue_claims_survive_restart_and_duplicate_requests_without_duplicate_ownership() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let plan = plan();
    let first = SqliteIssueAssignmentStore::open(&path)
        .unwrap()
        .claim("batch", &plan, 100)
        .unwrap();
    let store = SqliteIssueAssignmentStore::open(&path).unwrap();
    assert_eq!(store.claim("batch", &plan, 101).unwrap(), first);
    assert!(
        store
            .claim("other", &plan, 101)
            .unwrap_err()
            .contains("already claimed")
    );
    assert_eq!(store.list(&plan.repository.key()).unwrap(), first);
}

#[test]
fn concurrent_issue_claims_have_exactly_one_winner() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let first = SqliteIssueAssignmentStore::open(&path).unwrap();
    let second = SqliteIssueAssignmentStore::open(&path).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = barrier.clone();
    let thread = std::thread::spawn(move || {
        other.wait();
        first.claim("a", &plan(), 100)
    });
    barrier.wait();
    let result = second.claim("b", &plan(), 100);
    assert_eq!(
        usize::from(result.is_ok()) + usize::from(thread.join().unwrap().is_ok()),
        1
    );
}

#[test]
fn expired_execution_keeps_ownership_until_explicit_stop_and_rejects_old_epochs() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteIssueAssignmentStore::open(&dir.path().join("state.db")).unwrap();
    let initial = store.claim("batch", &plan(), 100).unwrap().remove(0);
    let running = store
        .apply(
            "renew",
            &initial.id,
            1,
            IssueAssignmentCommand::Renew {
                epoch: 1,
                lease_until: 160,
            },
            100,
        )
        .unwrap();
    assert!(running.check_writer(1, 160).is_err());
    assert!(store.claim("other", &plan(), 200).is_err());
    assert!(
        store
            .apply(
                "transfer",
                &initial.id,
                running.revision,
                IssueAssignmentCommand::Transfer {
                    owner: "next".into()
                },
                200
            )
            .is_err()
    );
    let stopped = store
        .apply(
            "stop",
            &initial.id,
            running.revision,
            IssueAssignmentCommand::Stop { epoch: 1 },
            200,
        )
        .unwrap();
    assert!(
        store
            .apply(
                "old",
                &initial.id,
                stopped.revision,
                IssueAssignmentCommand::Renew {
                    epoch: 1,
                    lease_until: 240
                },
                200
            )
            .is_err()
    );
    let transferred = store
        .apply(
            "transfer",
            &initial.id,
            stopped.revision,
            IssueAssignmentCommand::Transfer {
                owner: "next".into(),
            },
            200,
        )
        .unwrap();
    assert_eq!(transferred.owner, "next");
    let released = store
        .apply(
            "release",
            &initial.id,
            transferred.revision,
            IssueAssignmentCommand::Release,
            200,
        )
        .unwrap();
    assert_eq!(released.ownership, IssueOwnership::Released);
    assert!(store.claim("other", &plan(), 201).is_ok());
}

#[test]
fn a_conflicting_issue_rolls_back_the_entire_group_claim() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteIssueAssignmentStore::open(&dir.path().join("state.db")).unwrap();
    store.claim("first", &plan(), 100).unwrap();
    let mut combined = plan();
    let mut second = combined.items[0].issues[0].clone();
    second.number = 19;
    second.node_id = "issue19".into();
    combined.items[0].issues.insert(0, second.clone());
    assert!(store.claim("combined", &combined, 100).is_err());
    combined.items[0].issues = vec![second];
    assert!(store.claim("only19", &combined, 100).is_ok());
}

#[test]
fn control_fences_the_writer_atomically_and_retains_claim_until_remote_release() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let store = SqliteIssueAssignmentStore::open(&path).unwrap();
    let initial = store.start("batch", &plan(), 100).unwrap().remove(0);
    let running = store
        .apply(
            "lease",
            &initial.id,
            initial.revision,
            IssueAssignmentCommand::Renew {
                epoch: initial.epoch,
                lease_until: 200,
            },
            100,
        )
        .unwrap();
    let command = IssueAssignmentCommand::Control {
        epoch: running.epoch,
        action: IssueControl::Release,
    };
    let releasing = store
        .apply(
            "release-intent",
            &initial.id,
            running.revision,
            command.clone(),
            101,
        )
        .unwrap();
    assert_eq!(releasing.ownership, IssueOwnership::Releasing);
    assert!(releasing.check_writer(running.epoch, 101).is_err());
    drop(store);
    let store = SqliteIssueAssignmentStore::open(&path).unwrap();
    assert_eq!(
        store
            .apply(
                "release-intent",
                &initial.id,
                running.revision,
                command,
                102
            )
            .unwrap(),
        releasing
    );
    assert!(store.claim("other", &plan(), 102).is_err());
    let synced = store
        .apply(
            "release-synced",
            &initial.id,
            releasing.revision,
            IssueAssignmentCommand::RecordSync {
                stage: zeta_work_coordination::IssueStage::Todo,
                labels: BTreeMap::new(),
            },
            103,
        )
        .unwrap();
    store
        .apply(
            "release-finished",
            &initial.id,
            synced.revision,
            IssueAssignmentCommand::Release,
            104,
        )
        .unwrap();
    assert!(store.claim("other", &plan(), 105).is_ok());
}

#[test]
fn transfer_retains_exclusive_claim_and_does_not_restart_implementation() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteIssueAssignmentStore::open(&dir.path().join("state.db")).unwrap();
    let initial = store.claim("batch", &plan(), 100).unwrap().remove(0);
    let transferring = store
        .apply(
            "transfer",
            &initial.id,
            initial.revision,
            IssueAssignmentCommand::Control {
                epoch: initial.epoch,
                action: IssueControl::Transfer("next".into()),
            },
            101,
        )
        .unwrap();
    assert_eq!(transferring.owner, "owner");
    assert_eq!(transferring.pending_owner.as_deref(), Some("next"));
    assert!(store.claim("other", &plan(), 102).is_err());
    let synced = store
        .apply(
            "synced",
            &initial.id,
            transferring.revision,
            IssueAssignmentCommand::RecordSync {
                stage: zeta_work_coordination::IssueStage::Queued,
                labels: BTreeMap::new(),
            },
            103,
        )
        .unwrap();
    let transferred = store
        .apply(
            "finish",
            &initial.id,
            synced.revision,
            IssueAssignmentCommand::Transfer {
                owner: "next".into(),
            },
            104,
        )
        .unwrap();
    assert_eq!(transferred.owner, "next");
    assert_eq!(transferred.sync_state, IssueSyncState::Synced);
    assert!(transferred.paused);
    assert!(!transferred.auto_start);
    assert!(store.claim("other", &plan(), 105).is_err());
}

#[test]
fn action_receipts_survive_restart_and_reject_reused_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let request = serde_json::json!({"assignment":"one","action":"pause"});
    let response = serde_json::json!({"epoch":2});
    SqliteIssueAssignmentStore::open(&path)
        .unwrap()
        .record_action("pause-one", &request, &response)
        .unwrap();
    let store = SqliteIssueAssignmentStore::open(&path).unwrap();
    assert_eq!(
        store.action_receipt("pause-one", &request).unwrap(),
        Some(response)
    );
    assert!(
        store
            .action_receipt("pause-one", &serde_json::json!({"action":"release"}))
            .is_err()
    );
}

#[test]
fn only_one_scheduler_can_reserve_a_queued_execution() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.db");
    let store = SqliteIssueAssignmentStore::open(&path).unwrap();
    let initial = store.start("batch", &plan(), 100).unwrap().remove(0);
    let queued = store
        .apply(
            "synced",
            &initial.id,
            initial.revision,
            IssueAssignmentCommand::RecordSync {
                stage: zeta_work_coordination::IssueStage::Queued,
                labels: BTreeMap::new(),
            },
            100,
        )
        .unwrap();
    let other = SqliteIssueAssignmentStore::open(&path).unwrap();
    let request = IssueAssignmentCommand::Acquire {
        epoch: queued.epoch,
        lease_until: 200,
    };
    let reserved = store
        .apply(
            "scheduler-a",
            &queued.id,
            queued.revision,
            request.clone(),
            101,
        )
        .unwrap();
    assert!(
        other
            .apply(
                "scheduler-b",
                &queued.id,
                queued.revision,
                request.clone(),
                101
            )
            .is_err()
    );
    assert!(
        other
            .apply(
                "scheduler-b-current",
                &queued.id,
                reserved.revision,
                request,
                101
            )
            .is_err()
    );
    assert_eq!(other.read(&queued.id).unwrap(), reserved);
}
