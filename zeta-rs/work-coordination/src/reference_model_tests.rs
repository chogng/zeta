use super::AuthorizationSnapshotRef;
use super::ExternalEffectsStatus;
use super::GitRepositoryCheckpoint;
use super::GitRootTarget;
use super::ManagedRootBinding;
use super::ResolveWaitOutcome;
use super::RootCheckpoint;
use super::RootState;
use super::ValidationProfileRef;
use super::WorkAttemptCoordinationStatus;
use super::WorkAttemptExecutionStatus;
use super::WorkAttemptVerificationStatus;
use super::WorkContractDraft;
use super::WorkContractRef;
use super::WorkCoordinationError;
use super::WorkParticipant;
use super::WorkParticipantRelation;
use super::WorkRelationKind;
use super::WorkRelationStatus;
use super::WorkRun;
use super::WorkRunCommand;
use super::WorkRunCommandRequest;
use super::WorkScopeClaim;
use super::WorkStartMode;
use super::WorkWaitCondition;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::str::FromStr;
use zeta_environment::EnvId;
use zeta_file_access::DirId;
use zeta_protocol::CommandId;
use zeta_protocol::ContentDigest;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::WorkAttemptId;
use zeta_protocol::WorkContractId;
use zeta_protocol::WorkExecutionId;
use zeta_protocol::WorkRelationId;
use zeta_protocol::WorkRunId;
use zeta_turn_changes::ChangeSetId;

const RANDOM_SEEDS: u64 = 128;
const RANDOM_STEPS: u64 = 24;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceAttempt {
    execution_id: Option<WorkExecutionId>,
    execution_status: WorkAttemptExecutionStatus,
    coordination_status: WorkAttemptCoordinationStatus,
    verification_status: WorkAttemptVerificationStatus,
    waiting_relation_id: Option<WorkRelationId>,
    scope_expansion_evidence: Vec<String>,
    result_digest: Option<ContentDigest>,
    failure: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceRelation {
    source_attempt_id: WorkAttemptId,
    target_attempt_id: WorkAttemptId,
    kind: WorkRelationKind,
    status: WorkRelationStatus,
    resume_execution_status: Option<WorkAttemptExecutionStatus>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReferenceState {
    revision: u64,
    attempts: BTreeMap<WorkAttemptId, ReferenceAttempt>,
    relations: BTreeMap<WorkRelationId, ReferenceRelation>,
}

#[derive(Default)]
struct Coverage {
    accepted: usize,
    rejected: usize,
    derived_commands: usize,
    idle_reconciliations: usize,
}

#[derive(Clone, Copy)]
enum FinishKind {
    Failed,
    Interrupted,
    Cancelled,
}

struct Generator(u64);

impl Generator {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn index(&mut self, length: u64) -> u64 {
        self.next() % length
    }
}

#[test]
fn executable_reference_model_matches_randomized_attempt_and_wait_histories() {
    let mut coverage = Coverage::default();
    for seed in 1..=RANDOM_SEEDS {
        let mut run = bootstrap(seed);
        let mut reference = ReferenceState::ready(run.revision);
        assert_matches_reference(&run, &reference, seed, 0);

        apply_step(
            &mut run,
            &mut reference,
            begin_command(attempt_a(), execution(seed, "a"), WorkStartMode::Write),
            seed,
            1,
            &mut coverage,
        );
        apply_step(
            &mut run,
            &mut reference,
            begin_command(attempt_b(), execution(seed, "b"), WorkStartMode::Explore),
            seed,
            2,
            &mut coverage,
        );

        if seed % 4 == 0 {
            run_structured_wait_history(&mut run, &mut reference, seed, &mut coverage);
        } else {
            reconcile_step(&mut run, &mut reference, seed, 3, &mut coverage);
        }

        let mut generator = Generator::new(seed);
        for random_step in 0..RANDOM_STEPS {
            let step = 100 + random_step;
            if generator.index(12) == 11 {
                reconcile_step(&mut run, &mut reference, seed, step, &mut coverage);
                continue;
            }
            let command = random_command(&reference, &mut generator, seed, step);
            apply_step(&mut run, &mut reference, command, seed, step, &mut coverage);
        }
    }

    assert!(coverage.accepted > 500, "insufficient accepted transitions");
    assert!(
        coverage.rejected > 1_000,
        "insufficient rejected transitions"
    );
    assert!(
        coverage.derived_commands >= (RANDOM_SEEDS / 4) as usize,
        "terminal waits did not produce enough host commands"
    );
    assert!(
        coverage.idle_reconciliations > 100,
        "active targets did not exercise idle reconciliation"
    );
}

fn run_structured_wait_history(
    run: &mut WorkRun,
    reference: &mut ReferenceState,
    seed: u64,
    coverage: &mut Coverage,
) {
    let (source, target) = if seed % 8 == 0 {
        (attempt_a(), attempt_b())
    } else {
        (attempt_b(), attempt_a())
    };
    let target_execution_id = reference.attempts[&target]
        .execution_id
        .clone()
        .expect("structured target is active");
    let target_result = digest(&format!("phase-result-{seed}"));
    let condition = match (seed / 4) % 4 {
        0 => WorkWaitCondition::ExecutionFinished,
        1 => WorkWaitCondition::AttemptSealed,
        2 => WorkWaitCondition::ExactResult {
            result_digest: target_result.clone(),
        },
        _ => WorkWaitCondition::ExactResult {
            result_digest: digest(&format!("different-phase-result-{seed}")),
        },
    };
    let relation_id = relation(seed, 10);
    apply_step(
        run,
        reference,
        wait_command(
            relation_id.clone(),
            source.clone(),
            target.clone(),
            target_execution_id.clone(),
            condition,
        ),
        seed,
        10,
        coverage,
    );

    let source_execution_id = reference.attempts[&source]
        .execution_id
        .clone()
        .expect("structured source has an execution");
    apply_step(
        run,
        reference,
        wait_command(
            relation(seed, 11),
            target.clone(),
            source.clone(),
            source_execution_id,
            WorkWaitCondition::ExecutionFinished,
        ),
        seed,
        11,
        coverage,
    );

    apply_step(
        run,
        reference,
        WorkRunCommand::ResolveWait {
            relation_id: relation_id.clone(),
            target_attempt_id: target.clone(),
            target_execution_id: target_execution_id.clone(),
            outcome: ResolveWaitOutcome::Satisfied {
                evidence_digest: digest("forged-active-target-evidence"),
            },
        },
        seed,
        12,
        coverage,
    );

    let terminal = match (seed / 4) % 4 {
        0 => seal_command(target.clone(), target_result, seed, 13),
        1 => finish_command(target.clone(), FinishKind::Failed, seed, 13),
        2 => finish_command(target.clone(), FinishKind::Interrupted, seed, 13),
        _ => finish_command(target.clone(), FinishKind::Cancelled, seed, 13),
    };
    apply_step(run, reference, terminal, seed, 13, coverage);
    apply_step(
        run,
        reference,
        WorkRunCommand::ResolveWait {
            relation_id,
            target_attempt_id: target,
            target_execution_id,
            outcome: ResolveWaitOutcome::Satisfied {
                evidence_digest: digest("forged-terminal-target-evidence"),
            },
        },
        seed,
        14,
        coverage,
    );
    reconcile_step(run, reference, seed, 15, coverage);
}

fn random_command(
    reference: &ReferenceState,
    generator: &mut Generator,
    seed: u64,
    step: u64,
) -> WorkRunCommand {
    let attempt_id = if generator.index(2) == 0 {
        attempt_a()
    } else {
        attempt_b()
    };
    match generator.index(11) {
        0 => {
            let execution_id = if generator.index(4) == 0 {
                execution(seed, "a")
            } else {
                WorkExecutionId::new(format!("random-execution-{seed}-{step}")).unwrap()
            };
            begin_command(
                attempt_id,
                execution_id,
                if generator.index(2) == 0 {
                    WorkStartMode::Explore
                } else {
                    WorkStartMode::Write
                },
            )
        }
        1 => seal_command(
            attempt_id,
            digest(&format!("random-result-{seed}-{step}")),
            seed,
            step,
        ),
        2 => finish_command(attempt_id, FinishKind::Failed, seed, step),
        3 => finish_command(attempt_id, FinishKind::Interrupted, seed, step),
        4 => finish_command(attempt_id, FinishKind::Cancelled, seed, step),
        5 => WorkRunCommand::RequestScopeExpansion {
            attempt_id,
            evidence: vec![format!("scope-evidence-{seed}-{step}")],
        },
        6 | 7 => random_wait_command(reference, generator, seed, step),
        8 => random_resolution_command(reference, generator, seed, step),
        9 => WorkRunCommand::ResolveWait {
            relation_id: relation(seed, step),
            target_attempt_id: attempt_id,
            target_execution_id: WorkExecutionId::new(format!("unknown-execution-{seed}-{step}"))
                .unwrap(),
            outcome: ResolveWaitOutcome::Cancelled,
        },
        _ => begin_command(
            attempt_id,
            WorkExecutionId::new(format!("late-execution-{seed}-{step}")).unwrap(),
            WorkStartMode::Write,
        ),
    }
}

fn random_wait_command(
    reference: &ReferenceState,
    generator: &mut Generator,
    seed: u64,
    step: u64,
) -> WorkRunCommand {
    let (source, target) = if generator.index(2) == 0 {
        (attempt_a(), attempt_b())
    } else {
        (attempt_b(), attempt_a())
    };
    let target_execution_id = if generator.index(4) == 0 {
        WorkExecutionId::new(format!("wrong-target-execution-{seed}-{step}")).unwrap()
    } else {
        reference.attempts[&target]
            .execution_id
            .clone()
            .unwrap_or_else(|| {
                WorkExecutionId::new(format!("missing-target-execution-{seed}-{step}")).unwrap()
            })
    };
    let condition = match generator.index(3) {
        0 => WorkWaitCondition::ExecutionFinished,
        1 => WorkWaitCondition::AttemptSealed,
        _ => WorkWaitCondition::ExactResult {
            result_digest: digest(&format!("expected-random-result-{seed}-{step}")),
        },
    };
    wait_command(
        relation(seed, step),
        source,
        target,
        target_execution_id,
        condition,
    )
}

fn random_resolution_command(
    reference: &ReferenceState,
    generator: &mut Generator,
    seed: u64,
    step: u64,
) -> WorkRunCommand {
    let Some((relation_id, relation)) = reference.relations.iter().next() else {
        return WorkRunCommand::ResolveWait {
            relation_id: relation(seed, step),
            target_attempt_id: attempt_b(),
            target_execution_id: WorkExecutionId::new(format!("no-relation-{seed}-{step}"))
                .unwrap(),
            outcome: ResolveWaitOutcome::Cancelled,
        };
    };
    let WorkRelationKind::Wait {
        target_execution_id,
        ..
    } = &relation.kind
    else {
        unreachable!("the reference model only creates wait relations")
    };
    let outcome = match generator.index(3) {
        0 => ResolveWaitOutcome::Cancelled,
        1 => ResolveWaitOutcome::Satisfied {
            evidence_digest: digest(&format!("claimed-result-{seed}-{step}")),
        },
        _ => ResolveWaitOutcome::Failed {
            reason: format!("claimed-failure-{seed}-{step}"),
        },
    };
    WorkRunCommand::ResolveWait {
        relation_id: relation_id.clone(),
        target_attempt_id: relation.target_attempt_id.clone(),
        target_execution_id: target_execution_id.clone(),
        outcome,
    }
}

fn reconcile_step(
    run: &mut WorkRun,
    reference: &mut ReferenceState,
    seed: u64,
    step: u64,
    coverage: &mut Coverage,
) {
    let expected = reference.next_wait_resolution();
    let actual = super::next_wait_resolution(run).expect("production wait derivation");
    assert_eq!(
        actual, expected,
        "seed {seed}, step {step}: derived wait command diverged"
    );
    let Some(command) = expected else {
        coverage.idle_reconciliations += 1;
        assert_matches_reference(run, reference, seed, step);
        return;
    };
    coverage.derived_commands += 1;
    apply_step(run, reference, command, seed, step, coverage);
}

fn apply_step(
    run: &mut WorkRun,
    reference: &mut ReferenceState,
    command: WorkRunCommand,
    seed: u64,
    step: u64,
    coverage: &mut Coverage,
) {
    let expected = reference.transition(&command);
    let request = WorkRunCommandRequest {
        command_id: CommandId::new(format!("reference-{seed}-{step}")).unwrap(),
        work_run_id: run.work_run_id.clone(),
        expected_revision: run.revision,
        command: command.clone(),
    };
    let actual = crate::reducer::apply(Some(run.clone()), &request);
    match (expected, actual) {
        (Ok(next_reference), Ok(next_run)) => {
            coverage.accepted += 1;
            *reference = next_reference;
            *run = next_run;
        }
        (Err(expected), Err(actual)) => {
            coverage.rejected += 1;
            assert_eq!(
                actual, expected,
                "seed {seed}, step {step}, command {command:?}: rejection diverged"
            );
        }
        (expected, actual) => panic!(
            "seed {seed}, step {step}, command {command:?}: reference returned {expected:?}, production returned {actual:?}"
        ),
    }
    assert_matches_reference(run, reference, seed, step);
}

impl ReferenceState {
    fn ready(revision: u64) -> Self {
        Self {
            revision,
            attempts: BTreeMap::from([
                (attempt_a(), ReferenceAttempt::ready()),
                (attempt_b(), ReferenceAttempt::ready()),
            ]),
            relations: BTreeMap::new(),
        }
    }

    fn transition(&self, command: &WorkRunCommand) -> Result<Self, WorkCoordinationError> {
        let mut next = self.clone();
        next.apply(command)?;
        next.revision = next.revision.checked_add(1).ok_or_else(|| {
            WorkCoordinationError::InvalidInput("work-run revision overflow".into())
        })?;
        next.validate();
        Ok(next)
    }

    fn apply(&mut self, command: &WorkRunCommand) -> Result<(), WorkCoordinationError> {
        match command {
            WorkRunCommand::BeginAttempt {
                attempt_id,
                execution_id,
                mode,
            } => self.begin(attempt_id, execution_id, *mode),
            WorkRunCommand::SealAttempt {
                attempt_id,
                result_digest,
                ..
            } => self.seal(attempt_id, result_digest),
            WorkRunCommand::FailAttempt {
                attempt_id,
                message,
            } => self.finish(attempt_id, WorkAttemptExecutionStatus::Failed, message),
            WorkRunCommand::InterruptAttempt {
                attempt_id,
                message,
            } => self.finish(attempt_id, WorkAttemptExecutionStatus::Interrupted, message),
            WorkRunCommand::CancelAttempt { attempt_id, reason } => {
                self.finish(attempt_id, WorkAttemptExecutionStatus::Cancelled, reason)
            }
            WorkRunCommand::RequestScopeExpansion {
                attempt_id,
                evidence,
            } => self.request_scope_expansion(attempt_id, evidence),
            WorkRunCommand::CreateRelation {
                relation_id,
                source_attempt_id,
                target_attempt_id,
                kind,
            } => self.create_relation(relation_id, source_attempt_id, target_attempt_id, kind),
            WorkRunCommand::ResolveWait {
                relation_id,
                target_attempt_id,
                target_execution_id,
                outcome,
            } => self.resolve_wait(relation_id, target_attempt_id, target_execution_id, outcome),
            _ => panic!("reference model received an out-of-scope command: {command:?}"),
        }
    }

    fn begin(
        &mut self,
        attempt_id: &WorkAttemptId,
        execution_id: &WorkExecutionId,
        mode: WorkStartMode,
    ) -> Result<(), WorkCoordinationError> {
        if self
            .attempts
            .values()
            .any(|attempt| attempt.execution_id.as_ref() == Some(execution_id))
        {
            return Err(WorkCoordinationError::AlreadyExists(
                execution_id.to_string(),
            ));
        }
        let attempt = self.attempt_mut(attempt_id)?;
        if attempt.execution_status != WorkAttemptExecutionStatus::Planned
            || attempt.coordination_status != WorkAttemptCoordinationStatus::Clear
            || attempt.execution_id.is_some()
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "only one clear planned attempt can begin".into(),
            ));
        }
        attempt.execution_id = Some(execution_id.clone());
        attempt.execution_status = match mode {
            WorkStartMode::Explore => WorkAttemptExecutionStatus::Exploring,
            WorkStartMode::Write => WorkAttemptExecutionStatus::Writing,
        };
        Ok(())
    }

    fn seal(
        &mut self,
        attempt_id: &WorkAttemptId,
        result_digest: &ContentDigest,
    ) -> Result<(), WorkCoordinationError> {
        let attempt = self.active_attempt_mut(attempt_id)?;
        if attempt.coordination_status != WorkAttemptCoordinationStatus::Clear {
            return Err(WorkCoordinationError::InvalidTransition(
                "an attempt with unresolved coordination state cannot be sealed".into(),
            ));
        }
        attempt.execution_status = WorkAttemptExecutionStatus::Sealed;
        attempt.result_digest = Some(result_digest.clone());
        Ok(())
    }

    fn finish(
        &mut self,
        attempt_id: &WorkAttemptId,
        status: WorkAttemptExecutionStatus,
        message: &str,
    ) -> Result<(), WorkCoordinationError> {
        let waiting_relation_id = {
            let attempt = self.attempt_mut(attempt_id)?;
            if is_terminal(attempt.execution_status) {
                return Err(WorkCoordinationError::InvalidTransition(
                    "a terminal attempt cannot transition again".into(),
                ));
            }
            attempt.execution_status = status;
            attempt.failure = Some(message.into());
            attempt.waiting_relation_id.take()
        };
        if let Some(relation_id) = waiting_relation_id
            && let Some(relation) = self.relations.get_mut(&relation_id)
            && relation.status == WorkRelationStatus::Waiting
        {
            relation.status = WorkRelationStatus::Cancelled;
        }
        Ok(())
    }

    fn request_scope_expansion(
        &mut self,
        attempt_id: &WorkAttemptId,
        evidence: &[String],
    ) -> Result<(), WorkCoordinationError> {
        let attempt = self.active_attempt_mut(attempt_id)?;
        if attempt.coordination_status != WorkAttemptCoordinationStatus::Clear {
            return Err(WorkCoordinationError::InvalidTransition(
                "scope expansion requires a clear attempt".into(),
            ));
        }
        attempt.scope_expansion_evidence = evidence.into();
        attempt.coordination_status = WorkAttemptCoordinationStatus::ExpansionRequested;
        attempt.execution_status = WorkAttemptExecutionStatus::Interrupted;
        attempt.failure = Some("scope expansion requires a new contract and WorkAttempt".into());
        Ok(())
    }

    fn create_relation(
        &mut self,
        relation_id: &WorkRelationId,
        source_attempt_id: &WorkAttemptId,
        target_attempt_id: &WorkAttemptId,
        kind: &WorkRelationKind,
    ) -> Result<(), WorkCoordinationError> {
        if source_attempt_id == target_attempt_id {
            return Err(WorkCoordinationError::InvalidInput(
                "a work relation requires two distinct attempts".into(),
            ));
        }
        if self.relations.contains_key(relation_id) {
            return Err(WorkCoordinationError::AlreadyExists(
                relation_id.to_string(),
            ));
        }
        let source = self
            .attempts
            .get(source_attempt_id)
            .cloned()
            .ok_or_else(|| WorkCoordinationError::NotFound(source_attempt_id.to_string()))?;
        let target = self
            .attempts
            .get(target_attempt_id)
            .cloned()
            .ok_or_else(|| WorkCoordinationError::NotFound(target_attempt_id.to_string()))?;
        if self.reaches(target_attempt_id, source_attempt_id, &mut BTreeSet::new()) {
            return Err(WorkCoordinationError::InvalidInput(
                "work dependency graph would contain a cycle".into(),
            ));
        }
        let WorkRelationKind::Wait {
            target_execution_id,
            ..
        } = kind
        else {
            panic!("reference model only accepts wait relations")
        };
        if target.execution_id.as_ref() != Some(target_execution_id)
            || is_terminal(target.execution_status)
        {
            return Err(WorkCoordinationError::InvalidInput(
                "wait target must name the active target execution".into(),
            ));
        }
        if !is_active(source.execution_status)
            || source.coordination_status != WorkAttemptCoordinationStatus::Clear
            || source.waiting_relation_id.is_some()
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "only one clear active attempt can enter a wait".into(),
            ));
        }
        self.relations.insert(
            relation_id.clone(),
            ReferenceRelation {
                source_attempt_id: source_attempt_id.clone(),
                target_attempt_id: target_attempt_id.clone(),
                kind: kind.clone(),
                status: WorkRelationStatus::Waiting,
                resume_execution_status: Some(source.execution_status),
            },
        );
        let source = self.attempt_mut(source_attempt_id)?;
        source.execution_status = WorkAttemptExecutionStatus::Waiting;
        source.waiting_relation_id = Some(relation_id.clone());
        Ok(())
    }

    fn resolve_wait(
        &mut self,
        relation_id: &WorkRelationId,
        target_attempt_id: &WorkAttemptId,
        target_execution_id: &WorkExecutionId,
        outcome: &ResolveWaitOutcome,
    ) -> Result<(), WorkCoordinationError> {
        let relation = self
            .relations
            .get(relation_id)
            .cloned()
            .ok_or_else(|| WorkCoordinationError::NotFound(relation_id.to_string()))?;
        let WorkRelationKind::Wait {
            target_execution_id: expected_execution_id,
            condition,
        } = &relation.kind
        else {
            return Err(WorkCoordinationError::InvalidTransition(
                "only a wait relation can be resolved as a wait".into(),
            ));
        };
        if relation.status != WorkRelationStatus::Waiting
            || &relation.target_attempt_id != target_attempt_id
            || expected_execution_id != target_execution_id
        {
            return Err(WorkCoordinationError::InvalidInput(
                "wait resolution does not match the frozen target attempt and execution".into(),
            ));
        }
        let target = self
            .attempts
            .get(target_attempt_id)
            .cloned()
            .ok_or_else(|| WorkCoordinationError::NotFound(target_attempt_id.to_string()))?;
        if !matches!(outcome, ResolveWaitOutcome::Cancelled)
            && self.expected_wait_outcome(
                target_attempt_id,
                condition,
                &target,
                target_execution_id,
            )? != Some(outcome.clone())
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "wait resolution does not match the outcome derived from the frozen target".into(),
            ));
        }
        let status = match outcome {
            ResolveWaitOutcome::Satisfied { evidence_digest } => WorkRelationStatus::Satisfied {
                evidence_digest: evidence_digest.clone(),
            },
            ResolveWaitOutcome::Failed { reason } => WorkRelationStatus::Failed {
                reason: reason.clone(),
            },
            ResolveWaitOutcome::Cancelled => WorkRelationStatus::Cancelled,
            ResolveWaitOutcome::SourceStale => WorkRelationStatus::Stale,
        };
        self.relations
            .get_mut(relation_id)
            .expect("reference relation exists")
            .status = status.clone();
        let source = self.attempt_mut(&relation.source_attempt_id)?;
        source.waiting_relation_id = None;
        match status {
            WorkRelationStatus::Satisfied { .. } => {
                source.execution_status = relation
                    .resume_execution_status
                    .expect("reference wait records its resume status");
            }
            WorkRelationStatus::Failed { reason } => {
                source.execution_status = WorkAttemptExecutionStatus::Failed;
                source.failure = Some(reason);
            }
            WorkRelationStatus::Cancelled => {
                source.execution_status = WorkAttemptExecutionStatus::Interrupted;
                source.failure = Some("wait was cancelled".into());
            }
            WorkRelationStatus::Stale => {
                source.execution_status = WorkAttemptExecutionStatus::Interrupted;
                source.coordination_status = WorkAttemptCoordinationStatus::Stale;
                source.verification_status = WorkAttemptVerificationStatus::Stale;
                source.failure = Some("wait source became stale".into());
            }
            WorkRelationStatus::Active | WorkRelationStatus::Waiting => {
                unreachable!("reference resolution is terminal")
            }
        }
        Ok(())
    }

    fn next_wait_resolution(&self) -> Option<WorkRunCommand> {
        for (relation_id, relation) in &self.relations {
            if relation.status != WorkRelationStatus::Waiting {
                continue;
            }
            let WorkRelationKind::Wait {
                target_execution_id,
                condition,
            } = &relation.kind
            else {
                unreachable!("reference relation kind")
            };
            let target = &self.attempts[&relation.target_attempt_id];
            let Some(outcome) = self
                .expected_wait_outcome(
                    &relation.target_attempt_id,
                    condition,
                    target,
                    target_execution_id,
                )
                .expect("reference wait outcome")
            else {
                continue;
            };
            return Some(WorkRunCommand::ResolveWait {
                relation_id: relation_id.clone(),
                target_attempt_id: relation.target_attempt_id.clone(),
                target_execution_id: target_execution_id.clone(),
                outcome,
            });
        }
        None
    }

    fn expected_wait_outcome(
        &self,
        target_attempt_id: &WorkAttemptId,
        condition: &WorkWaitCondition,
        target: &ReferenceAttempt,
        target_execution_id: &WorkExecutionId,
    ) -> Result<Option<ResolveWaitOutcome>, WorkCoordinationError> {
        if target.execution_id.as_ref() != Some(target_execution_id) {
            return Ok(Some(ResolveWaitOutcome::SourceStale));
        }
        let outcome = match target.execution_status {
            WorkAttemptExecutionStatus::Sealed => {
                let result_digest = target.result_digest.as_ref().ok_or_else(|| {
                    WorkCoordinationError::InvalidInput(
                        "a sealed wait target omitted its result".into(),
                    )
                })?;
                match condition {
                    WorkWaitCondition::ExecutionFinished | WorkWaitCondition::AttemptSealed => {
                        ResolveWaitOutcome::Satisfied {
                            evidence_digest: result_digest.clone(),
                        }
                    }
                    WorkWaitCondition::ExactResult {
                        result_digest: expected,
                    } if expected == result_digest => ResolveWaitOutcome::Satisfied {
                        evidence_digest: result_digest.clone(),
                    },
                    WorkWaitCondition::ExactResult { .. } => ResolveWaitOutcome::Failed {
                        reason: "wait target sealed a different exact result".into(),
                    },
                }
            }
            WorkAttemptExecutionStatus::Failed
            | WorkAttemptExecutionStatus::Interrupted
            | WorkAttemptExecutionStatus::Cancelled => match condition {
                WorkWaitCondition::ExecutionFinished => ResolveWaitOutcome::Satisfied {
                    evidence_digest: terminal_evidence_digest(target_attempt_id, target),
                },
                WorkWaitCondition::AttemptSealed | WorkWaitCondition::ExactResult { .. } => {
                    ResolveWaitOutcome::Failed {
                        reason: format!(
                            "wait target finished as {:?} without the required sealed result",
                            target.execution_status
                        ),
                    }
                }
            },
            WorkAttemptExecutionStatus::Planned
            | WorkAttemptExecutionStatus::Exploring
            | WorkAttemptExecutionStatus::Writing
            | WorkAttemptExecutionStatus::Waiting => return Ok(None),
        };
        Ok(Some(outcome))
    }

    fn reaches(
        &self,
        current: &WorkAttemptId,
        target: &WorkAttemptId,
        visited: &mut BTreeSet<WorkAttemptId>,
    ) -> bool {
        if current == target {
            return true;
        }
        if !visited.insert(current.clone()) {
            return false;
        }
        self.relations.values().any(|relation| {
            &relation.source_attempt_id == current
                && self.reaches(&relation.target_attempt_id, target, visited)
        })
    }

    fn attempt_mut(
        &mut self,
        attempt_id: &WorkAttemptId,
    ) -> Result<&mut ReferenceAttempt, WorkCoordinationError> {
        self.attempts
            .get_mut(attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))
    }

    fn active_attempt_mut(
        &mut self,
        attempt_id: &WorkAttemptId,
    ) -> Result<&mut ReferenceAttempt, WorkCoordinationError> {
        let attempt = self.attempt_mut(attempt_id)?;
        if !is_active(attempt.execution_status) {
            return Err(WorkCoordinationError::InvalidTransition(
                "attempt must be actively exploring or writing".into(),
            ));
        }
        Ok(attempt)
    }

    fn validate(&self) {
        for (attempt_id, attempt) in &self.attempts {
            assert_eq!(
                attempt.execution_status == WorkAttemptExecutionStatus::Waiting,
                attempt.waiting_relation_id.is_some(),
                "reference attempt {attempt_id} has inconsistent waiting state"
            );
            assert_eq!(
                attempt.execution_status == WorkAttemptExecutionStatus::Sealed,
                attempt.result_digest.is_some(),
                "reference attempt {attempt_id} has inconsistent result state"
            );
        }
        for (relation_id, relation) in &self.relations {
            if relation.status == WorkRelationStatus::Waiting {
                let source = &self.attempts[&relation.source_attempt_id];
                assert_eq!(
                    source.waiting_relation_id.as_ref(),
                    Some(relation_id),
                    "reference relation {relation_id} lost its waiting source"
                );
            }
        }
    }
}

impl ReferenceAttempt {
    fn ready() -> Self {
        Self {
            execution_id: None,
            execution_status: WorkAttemptExecutionStatus::Planned,
            coordination_status: WorkAttemptCoordinationStatus::Clear,
            verification_status: WorkAttemptVerificationStatus::Pending,
            waiting_relation_id: None,
            scope_expansion_evidence: Vec::new(),
            result_digest: None,
            failure: None,
        }
    }
}

fn assert_matches_reference(run: &WorkRun, reference: &ReferenceState, seed: u64, step: u64) {
    run.validate().unwrap_or_else(|error| {
        panic!("seed {seed}, step {step}: invalid production state: {error}")
    });
    let actual_attempts = run
        .attempts
        .iter()
        .map(|(attempt_id, attempt)| {
            (
                attempt_id.clone(),
                ReferenceAttempt {
                    execution_id: attempt.execution_id.clone(),
                    execution_status: attempt.execution_status,
                    coordination_status: attempt.coordination_status,
                    verification_status: attempt.verification_status,
                    waiting_relation_id: attempt.waiting_relation_id.clone(),
                    scope_expansion_evidence: attempt.scope_expansion_evidence.clone(),
                    result_digest: attempt
                        .result
                        .as_ref()
                        .map(|result| result.result_digest.clone()),
                    failure: attempt.failure.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let actual_relations = run
        .relations
        .iter()
        .map(|(relation_id, relation)| {
            (
                relation_id.clone(),
                ReferenceRelation {
                    source_attempt_id: relation.source_attempt_id.clone(),
                    target_attempt_id: relation.target_attempt_id.clone(),
                    kind: relation.kind.clone(),
                    status: relation.status.clone(),
                    resume_execution_status: relation.resume_execution_status,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        (run.revision, actual_attempts, actual_relations),
        (
            reference.revision,
            reference.attempts.clone(),
            reference.relations.clone()
        ),
        "seed {seed}, step {step}: durable state diverged"
    );
}

fn bootstrap(seed: u64) -> WorkRun {
    let run_id = WorkRunId::new(format!("reference-run-{seed}")).unwrap();
    let mut command_index = 0_u64;
    let mut run = reduce(
        None,
        &run_id,
        seed,
        &mut command_index,
        WorkRunCommand::Create {
            objective: "prove coordination transitions".into(),
            acceptance_conditions: vec!["reference and production states agree".into()],
            exclusions: vec!["no implicit authority".into()],
            root_participant: participant("reference-session-a", "reference-thread-a"),
        },
    );
    run = reduce(
        Some(run),
        &run_id,
        seed,
        &mut command_index,
        WorkRunCommand::AddParticipant {
            participant: participant("reference-session-b", "reference-thread-b"),
        },
    );
    for (contract_id, thread_id) in [
        ("reference-contract-a", "reference-thread-a"),
        ("reference-contract-b", "reference-thread-b"),
    ] {
        run = reduce(
            Some(run),
            &run_id,
            seed,
            &mut command_index,
            WorkRunCommand::CreateContract {
                contract: contract(contract_id, thread_id),
            },
        );
    }
    for (attempt_id, contract_id, thread_id) in [
        (attempt_a(), "reference-contract-a", "reference-thread-a"),
        (attempt_b(), "reference-contract-b", "reference-thread-b"),
    ] {
        run = reduce(
            Some(run),
            &run_id,
            seed,
            &mut command_index,
            WorkRunCommand::CreateAttempt {
                attempt_id: attempt_id.clone(),
                contract: WorkContractRef {
                    contract_id: WorkContractId::new(contract_id).unwrap(),
                    revision: 1,
                },
                participant_thread_id: ThreadId::new(thread_id).unwrap(),
            },
        );
        let checkpoint = &run.attempts[&attempt_id].roots[0];
        let roots = vec![ManagedRootBinding {
            source_dir_id: checkpoint.dir_id.clone(),
            managed_dir_id: test_dir(&format!("managed-{seed}-{attempt_id}")),
            root_checkpoint_digest: super::root_checkpoint_digest(checkpoint).unwrap(),
            binding_manifest_digest: digest(&format!("binding-{seed}-{attempt_id}")),
        }];
        run = reduce(
            Some(run),
            &run_id,
            seed,
            &mut command_index,
            WorkRunCommand::RecordAttemptWorkspaceReady {
                attempt_id: attempt_id.clone(),
                roots,
                private_output_dir_id: test_dir(&format!("output-{seed}-{attempt_id}")),
            },
        );
    }
    assert_eq!(run.revision, 8);
    run
}

fn reduce(
    current: Option<WorkRun>,
    run_id: &WorkRunId,
    seed: u64,
    command_index: &mut u64,
    command: WorkRunCommand,
) -> WorkRun {
    let expected_revision = current.as_ref().map_or(0, |run| run.revision);
    let request = WorkRunCommandRequest {
        command_id: CommandId::new(format!("bootstrap-{seed}-{command_index}")).unwrap(),
        work_run_id: run_id.clone(),
        expected_revision,
        command,
    };
    *command_index += 1;
    crate::reducer::apply(current, &request).expect("bootstrap reference WorkRun")
}

fn participant(session_id: &str, thread_id: &str) -> WorkParticipant {
    WorkParticipant {
        session_id: SessionId::new(session_id).unwrap(),
        thread_id: ThreadId::new(thread_id).unwrap(),
        relation: WorkParticipantRelation::Root,
    }
}

fn contract(contract_id: &str, thread_id: &str) -> WorkContractDraft {
    let root_dir_id = test_dir("reference-source-root");
    WorkContractDraft {
        contract_id: WorkContractId::new(contract_id).unwrap(),
        goal_revision: 1,
        topology_revision: 2,
        owner_thread_id: ThreadId::new(thread_id).unwrap(),
        objective: "exercise one bounded reference attempt".into(),
        acceptance_conditions: vec!["state agrees".into()],
        exclusions: Vec::new(),
        environment_id: EnvId::local(),
        roots: vec![RootCheckpoint {
            environment_id: EnvId::local(),
            dir_id: root_dir_id.clone(),
            state: RootState::Git {
                repositories: vec![GitRepositoryCheckpoint {
                    repository_id: "reference-repository".into(),
                    relative_path: ".".into(),
                    target: GitRootTarget::Branch {
                        name: "main".into(),
                        expected_head: "reference-head".into(),
                    },
                    baseline_tree: "reference-tree".into(),
                }],
            },
            control_resources: Vec::new(),
        }],
        primary_root_dir_id: root_dir_id,
        authorization: AuthorizationSnapshotRef {
            authority: "reference-authority".into(),
            policy_revision: "reference-policy-v1".into(),
            grant_set_digest: digest("reference-grants"),
            granted_effects_digest: digest("reference-effects"),
        },
        decision_ids: BTreeSet::new(),
        upstream_results: Vec::new(),
        expected_scope: WorkScopeClaim::default(),
        validation_profile: ValidationProfileRef {
            name: "reference-validation".into(),
            content_digest: digest("reference-validation-profile"),
        },
    }
}

fn begin_command(
    attempt_id: WorkAttemptId,
    execution_id: WorkExecutionId,
    mode: WorkStartMode,
) -> WorkRunCommand {
    WorkRunCommand::BeginAttempt {
        attempt_id,
        execution_id,
        mode,
    }
}

fn seal_command(
    attempt_id: WorkAttemptId,
    result_digest: ContentDigest,
    seed: u64,
    step: u64,
) -> WorkRunCommand {
    WorkRunCommand::SealAttempt {
        attempt_id,
        result_digest,
        change_set_ids: vec![ChangeSetId::new(format!("changes-{seed}-{step}")).unwrap()],
        private_output_digest: digest(&format!("private-output-{seed}-{step}")),
        external_effects_digest: digest(&format!("external-effects-{seed}-{step}")),
        external_effects_status: ExternalEffectsStatus::None,
    }
}

fn finish_command(
    attempt_id: WorkAttemptId,
    kind: FinishKind,
    seed: u64,
    step: u64,
) -> WorkRunCommand {
    let reason = format!("terminal-{seed}-{step}");
    match kind {
        FinishKind::Failed => WorkRunCommand::FailAttempt {
            attempt_id,
            message: reason,
        },
        FinishKind::Interrupted => WorkRunCommand::InterruptAttempt {
            attempt_id,
            message: reason,
        },
        FinishKind::Cancelled => WorkRunCommand::CancelAttempt { attempt_id, reason },
    }
}

fn wait_command(
    relation_id: WorkRelationId,
    source_attempt_id: WorkAttemptId,
    target_attempt_id: WorkAttemptId,
    target_execution_id: WorkExecutionId,
    condition: WorkWaitCondition,
) -> WorkRunCommand {
    WorkRunCommand::CreateRelation {
        relation_id,
        source_attempt_id,
        target_attempt_id,
        kind: WorkRelationKind::Wait {
            target_execution_id,
            condition,
        },
    }
}

fn terminal_evidence_digest(
    attempt_id: &WorkAttemptId,
    attempt: &ReferenceAttempt,
) -> ContentDigest {
    let encoded = serde_json::to_vec(&(
        1_u32,
        attempt_id,
        &attempt.execution_id,
        attempt.execution_status,
        &attempt.failure,
    ))
    .expect("encode reference terminal evidence");
    ContentDigest::sha256(&encoded)
}

fn is_active(status: WorkAttemptExecutionStatus) -> bool {
    matches!(
        status,
        WorkAttemptExecutionStatus::Exploring | WorkAttemptExecutionStatus::Writing
    )
}

fn is_terminal(status: WorkAttemptExecutionStatus) -> bool {
    matches!(
        status,
        WorkAttemptExecutionStatus::Sealed
            | WorkAttemptExecutionStatus::Failed
            | WorkAttemptExecutionStatus::Interrupted
            | WorkAttemptExecutionStatus::Cancelled
    )
}

fn attempt_a() -> WorkAttemptId {
    WorkAttemptId::new("reference-attempt-a").unwrap()
}

fn attempt_b() -> WorkAttemptId {
    WorkAttemptId::new("reference-attempt-b").unwrap()
}

fn execution(seed: u64, suffix: &str) -> WorkExecutionId {
    WorkExecutionId::new(format!("reference-execution-{seed}-{suffix}")).unwrap()
}

fn relation(seed: u64, step: u64) -> WorkRelationId {
    WorkRelationId::new(format!("reference-relation-{seed}-{step}")).unwrap()
}

fn test_dir(seed: &str) -> DirId {
    DirId::from_str(ContentDigest::sha256(seed.as_bytes()).as_str()).unwrap()
}

fn digest(value: &str) -> ContentDigest {
    ContentDigest::sha256(value.as_bytes())
}
