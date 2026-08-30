use super::ResourcePath;
use super::analyze_work_serializability;
use super::normalized_relative_path;
use super::sets_overlap;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use zeta_file_access::DirId;
use zeta_file_access::EnvId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::ChangeSetId;
use zeta_turn_changes::MessageState;
use zeta_turn_changes::SnapshotBackend;
use zeta_turn_changes::TurnChangeSet;
use zeta_turn_changes::TurnChangeSetDraft;
use zeta_turn_changes::WorkAttemptChangeProvenance;
use zeta_work_coordination::AuthorizationSnapshotRef;
use zeta_work_coordination::ExternalEffectsStatus;
use zeta_work_coordination::ValidationProfileRef;
use zeta_work_coordination::WorkAttempt;
use zeta_work_coordination::WorkAttemptCoordinationStatus;
use zeta_work_coordination::WorkAttemptExecutionStatus;
use zeta_work_coordination::WorkAttemptIntegrationStatus;
use zeta_work_coordination::WorkAttemptResult;
use zeta_work_coordination::WorkAttemptVerificationStatus;
use zeta_work_coordination::WorkAttemptWorkspace;
use zeta_work_coordination::WorkContractRef;
use zeta_work_coordination::WorkContractVersion;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkRunStatus;
use zeta_work_coordination::WorkScopeClaim;
use zeta_work_coordination::WorkSerializabilityStatus;

fn resource(repository_id: &str, path: &str) -> ResourcePath {
    ResourcePath {
        source_dir_id: DirId::from_str(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        repository_id: repository_id.into(),
        path: PathBuf::from(path),
    }
}

#[test]
fn resource_overlap_is_scoped_by_root_repository_and_path_hierarchy() {
    assert!(sets_overlap(
        &BTreeSet::from([resource("repo-a", "src")]),
        &BTreeSet::from([resource("repo-a", "src/lib.rs")]),
    ));
    assert!(!sets_overlap(
        &BTreeSet::from([resource("repo-a", "src/lib.rs")]),
        &BTreeSet::from([resource("repo-b", "src/lib.rs")]),
    ));
}

#[test]
fn access_paths_reject_escape_and_have_one_canonical_root_shape() {
    assert_eq!(
        normalized_relative_path(Path::new("./src/lib.rs")),
        Some(PathBuf::from("src/lib.rs"))
    );
    assert_eq!(
        normalized_relative_path(Path::new(".")),
        Some(PathBuf::from("."))
    );
    assert_eq!(normalized_relative_path(Path::new("../secret")), None);
    assert_eq!(normalized_relative_path(Path::new("/tmp/secret")), None);
}

#[test]
fn actual_read_before_independent_write_changes_the_stable_result_order() {
    let writer_id = WorkAttemptId::new("a-writer").unwrap();
    let reader_id = WorkAttemptId::new("z-reader").unwrap();
    let run = run_with_sealed_attempts([writer_id.clone(), reader_id.clone()]);
    let selected = BTreeSet::from([writer_id.clone(), reader_id.clone()]);
    let records = BTreeMap::from([
        (
            writer_id.clone(),
            vec![change_set(&run, &writer_id, &[], &["src/shared.rs"])],
        ),
        (
            reader_id.clone(),
            vec![change_set(&run, &reader_id, &["src/shared.rs"], &[])],
        ),
    ]);

    let analysis = analyze_work_serializability(&run, &selected, &records).unwrap();

    assert_eq!(
        analysis
            .ordered_results
            .iter()
            .map(|result| result.attempt_id.clone())
            .collect::<Vec<_>>(),
        vec![reader_id, writer_id]
    );
    assert_eq!(analysis.evidence.status, WorkSerializabilityStatus::Proven);
}

#[test]
fn overlapping_writes_are_indeterminate_even_when_the_graph_has_an_order() {
    let first_id = WorkAttemptId::new("first-writer").unwrap();
    let second_id = WorkAttemptId::new("second-writer").unwrap();
    let run = run_with_sealed_attempts([first_id.clone(), second_id.clone()]);
    let selected = BTreeSet::from([first_id.clone(), second_id.clone()]);
    let records = BTreeMap::from([
        (
            first_id.clone(),
            vec![change_set(&run, &first_id, &[], &["src/shared.rs"])],
        ),
        (
            second_id.clone(),
            vec![change_set(&run, &second_id, &[], &["src/shared.rs"])],
        ),
    ]);

    let analysis = analyze_work_serializability(&run, &selected, &records).unwrap();

    assert_eq!(
        analysis.evidence.status,
        WorkSerializabilityStatus::Indeterminate
    );
    assert!(analysis.evidence.reason.contains("overlapping writes"));
}

fn run_with_sealed_attempts<const N: usize>(attempt_ids: [WorkAttemptId; N]) -> WorkRun {
    let work_run_id = WorkRunId::new("serializability-run").unwrap();
    let contract_id = WorkContractId::new("serializability-contract").unwrap();
    let root_dir_id = dir_id('a');
    let contract = WorkContractVersion {
        contract_id: contract_id.clone(),
        revision: 1,
        goal_revision: 1,
        topology_revision: 1,
        owner_thread_id: ThreadId::new("owner-thread").unwrap(),
        objective: "combine exact results".into(),
        acceptance_conditions: Vec::new(),
        exclusions: Vec::new(),
        environment_id: EnvId::local(),
        roots: Vec::new(),
        primary_root_dir_id: root_dir_id.clone(),
        authorization: AuthorizationSnapshotRef {
            authority: "test-authority".into(),
            policy_revision: "test-policy".into(),
            grant_set_digest: digest("grants"),
            granted_effects_digest: digest("effects"),
        },
        decision_ids: BTreeSet::new(),
        upstream_results: Vec::new(),
        expected_scope: WorkScopeClaim::default(),
        validation_profile: ValidationProfileRef {
            name: "test-profile".into(),
            content_digest: digest("profile"),
        },
    };
    let attempts = attempt_ids
        .into_iter()
        .map(|attempt_id| {
            let attempt = WorkAttempt {
                attempt_id: attempt_id.clone(),
                contract: WorkContractRef {
                    contract_id: contract_id.clone(),
                    revision: 1,
                },
                session_id: SessionId::new(format!("session-{attempt_id}")).unwrap(),
                thread_id: ThreadId::new(format!("thread-{attempt_id}")).unwrap(),
                environment_id: EnvId::local(),
                roots: Vec::new(),
                primary_root_dir_id: root_dir_id.clone(),
                workspace: WorkAttemptWorkspace::Provisioning,
                execution_id: Some(
                    WorkExecutionId::new(format!("execution-{attempt_id}")).unwrap(),
                ),
                execution_status: WorkAttemptExecutionStatus::Sealed,
                coordination_status: WorkAttemptCoordinationStatus::Clear,
                verification_status: WorkAttemptVerificationStatus::Pending,
                integration_status: WorkAttemptIntegrationStatus::Idle,
                waiting_relation_id: None,
                scope_expansion_evidence: Vec::new(),
                result: Some(WorkAttemptResult {
                    result_digest: digest(&format!("result-{attempt_id}")),
                    change_set_ids: vec![
                        ChangeSetId::new(format!("changes-{attempt_id}")).unwrap(),
                    ],
                    private_output_digest: digest(&format!("output-{attempt_id}")),
                    external_effects_digest: digest(&format!("effects-{attempt_id}")),
                    external_effects_status: ExternalEffectsStatus::None,
                }),
                failure: None,
            };
            (attempt_id, attempt)
        })
        .collect();
    WorkRun {
        schema_version: 4,
        work_run_id,
        revision: 1,
        topology_revision: 1,
        status: WorkRunStatus::Active,
        terminal_reason: None,
        goals: Vec::new(),
        participants: BTreeMap::new(),
        decisions: BTreeMap::new(),
        contracts: BTreeMap::from([(contract_id, vec![contract])]),
        attempts,
        relations: BTreeMap::new(),
        conflicts: BTreeMap::new(),
        verifications: BTreeMap::new(),
        integrations: BTreeMap::new(),
    }
}

fn change_set(
    run: &WorkRun,
    attempt_id: &WorkAttemptId,
    reads: &[&str],
    writes: &[&str],
) -> TurnChangeSet {
    let attempt = &run.attempts[attempt_id];
    let mut change_set = TurnChangeSet::open(TurnChangeSetDraft {
        change_set_id: attempt.result.as_ref().unwrap().change_set_ids[0].clone(),
        session_id: attempt.session_id.clone(),
        thread_id: attempt.thread_id.clone(),
        turn_id: TurnId::new(format!("turn-{attempt_id}")).unwrap(),
        repository_id: "repository-a".into(),
        worktree_root: PathBuf::from("/managed/repository-a"),
        target_branch: Some("main".into()),
        base_object_id: Some("base-object".into()),
        before_tree: "base-tree".into(),
        snapshot_backend: SnapshotBackend::Git,
        baseline_dependency_paths: BTreeSet::new(),
        message_state: MessageState::Unconfigured,
        work_attempt: Some(WorkAttemptChangeProvenance {
            work_run_id: run.work_run_id.clone(),
            attempt_id: attempt_id.clone(),
            execution_id: attempt.execution_id.clone().unwrap(),
            contract_id: attempt.contract.contract_id.clone(),
            contract_revision: attempt.contract.revision,
            source_root_dir_id: dir_id('a'),
            managed_root_dir_id: dir_id('b'),
            root_checkpoint_digest: digest("root-checkpoint"),
        }),
    })
    .unwrap();
    change_set.read_paths = reads.iter().map(PathBuf::from).collect();
    change_set.write_paths = writes.iter().map(PathBuf::from).collect();
    change_set
}

fn dir_id(seed: char) -> DirId {
    DirId::from_str(&format!("sha256:{}", seed.to_string().repeat(64))).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}
