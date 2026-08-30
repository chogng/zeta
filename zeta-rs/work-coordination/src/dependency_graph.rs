use crate::WorkCoordinationError;
use crate::WorkRelation;
use crate::WorkRelationKind;
use crate::WorkRelationStatus;
use crate::WorkResultRef;
use crate::WorkRun;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use zeta_protocol::WorkAttemptId;

/// Returns the one stable topological order for an exact sealed-result selection.
///
/// Attempt identity breaks ties between otherwise independent results. Dependencies outside the
/// selection are accepted only when their exact result is already integrated.
pub fn ordered_result_refs(
    run: &WorkRun,
    selected: &BTreeSet<WorkAttemptId>,
) -> Result<Vec<WorkResultRef>, WorkCoordinationError> {
    ordered_result_refs_with_dependencies(run, selected, &BTreeSet::new())
}

/// Extends the declared WorkRun graph with host-derived dependency edges.
///
/// Each pair is `(dependent, prerequisite)`. Both identities must belong to the exact selection;
/// the combined graph still receives the same stable identity tie-break.
pub fn ordered_result_refs_with_dependencies(
    run: &WorkRun,
    selected: &BTreeSet<WorkAttemptId>,
    additional_dependencies: &BTreeSet<(WorkAttemptId, WorkAttemptId)>,
) -> Result<Vec<WorkResultRef>, WorkCoordinationError> {
    if selected.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "verification requires at least one selected WorkAttempt".into(),
        ));
    }
    for relation in run.relations.values() {
        if matches!(relation.kind, WorkRelationKind::Alternate)
            && selected.contains(&relation.source_attempt_id)
            && selected.contains(&relation.target_attempt_id)
        {
            return Err(WorkCoordinationError::InvalidInput(
                "alternate WorkAttempts cannot belong to one verification input".into(),
            ));
        }
    }

    let mut prerequisites = selected
        .iter()
        .map(|attempt_id| (attempt_id.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for attempt_id in selected {
        let attempt = run
            .attempts
            .get(attempt_id)
            .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))?;
        if attempt.result.is_none() {
            return Err(WorkCoordinationError::InvalidInput(
                "verification selection contains an unsealed WorkAttempt".into(),
            ));
        }
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| {
                WorkCoordinationError::NotFound(attempt.contract.contract_id.to_string())
            })?;
        for upstream in &contract.upstream_results {
            require_selected_or_integrated(run, selected, upstream)?;
            if selected.contains(&upstream.attempt_id) {
                prerequisites
                    .get_mut(attempt_id)
                    .ok_or_else(|| {
                        WorkCoordinationError::InvalidInput(
                            "verification selection lost an initialized WorkAttempt".into(),
                        )
                    })?
                    .insert(upstream.attempt_id.clone());
            }
        }
    }
    for relation in run.relations.values() {
        let Some((dependent, prerequisite)) = result_relation_edge(relation) else {
            continue;
        };
        if !selected.contains(dependent) {
            continue;
        }
        if matches!(relation.kind, WorkRelationKind::Wait { .. })
            && !matches!(relation.status, WorkRelationStatus::Satisfied { .. })
        {
            return Err(WorkCoordinationError::InvalidTransition(
                "verification dependency wait is not satisfied".into(),
            ));
        }
        if selected.contains(prerequisite) {
            prerequisites
                .get_mut(dependent)
                .ok_or_else(|| {
                    WorkCoordinationError::InvalidInput(
                        "verification dependency names an uninitialized WorkAttempt".into(),
                    )
                })?
                .insert(prerequisite.clone());
        } else {
            require_integrated_attempt(run, prerequisite, None)?;
        }
    }
    for (dependent, prerequisite) in additional_dependencies {
        if dependent == prerequisite
            || !selected.contains(dependent)
            || !selected.contains(prerequisite)
        {
            return Err(WorkCoordinationError::InvalidInput(
                "host-derived dependency must connect two distinct selected WorkAttempts".into(),
            ));
        }
        prerequisites
            .get_mut(dependent)
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "host-derived dependency names an uninitialized WorkAttempt".into(),
                )
            })?
            .insert(prerequisite.clone());
    }

    let mut remaining = prerequisites;
    let mut ordered = Vec::with_capacity(selected.len());
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .filter_map(|(attempt_id, prerequisites)| {
                prerequisites.is_empty().then_some(attempt_id.clone())
            })
            .next()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "verification dependency graph contains a cycle".into(),
                )
            })?;
        remaining.remove(&ready);
        for prerequisites in remaining.values_mut() {
            prerequisites.remove(&ready);
        }
        let result = run
            .attempts
            .get(&ready)
            .and_then(|attempt| attempt.result.as_ref())
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "ordered WorkAttempt lost its sealed result".into(),
                )
            })?;
        ordered.push(WorkResultRef {
            attempt_id: ready,
            result_digest: result.result_digest.clone(),
        });
    }
    Ok(ordered)
}

pub(crate) fn ensure_relation_acyclic(
    run: &WorkRun,
    source_attempt_id: &WorkAttemptId,
    target_attempt_id: &WorkAttemptId,
    kind: &WorkRelationKind,
) -> Result<(), WorkCoordinationError> {
    let Some((dependent, prerequisite)) =
        coordination_edge(source_attempt_id, target_attempt_id, kind)
    else {
        return Ok(());
    };
    let mut visited = BTreeSet::new();
    if reaches(run, prerequisite, dependent, &mut visited) {
        return Err(WorkCoordinationError::InvalidInput(
            "work dependency graph would contain a cycle".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_acyclic(run: &WorkRun) -> Result<(), WorkCoordinationError> {
    for attempt_id in run.attempts.keys() {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if contains_cycle(run, attempt_id, &mut visiting, &mut visited) {
            return Err(WorkCoordinationError::InvalidInput(
                "work dependency graph contains a cycle".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_order(
    run: &WorkRun,
    ordered_results: &[WorkResultRef],
) -> Result<(), WorkCoordinationError> {
    let positions = ordered_results
        .iter()
        .enumerate()
        .map(|(index, result)| (&result.attempt_id, index))
        .collect::<std::collections::BTreeMap<_, _>>();
    for relation in run.relations.values() {
        let Some((dependent, prerequisite)) = result_relation_edge(relation) else {
            continue;
        };
        let Some(dependent_position) = positions.get(dependent) else {
            continue;
        };
        if let Some(prerequisite_position) = positions.get(prerequisite) {
            if prerequisite_position >= dependent_position {
                return Err(WorkCoordinationError::InvalidInput(
                    "verification result order violates the WorkRun dependency graph".into(),
                ));
            }
        } else if run.attempts.get(prerequisite).is_none_or(|attempt| {
            attempt.integration_status != crate::WorkAttemptIntegrationStatus::Integrated
        }) {
            return Err(WorkCoordinationError::InvalidInput(
                "verification omits a dependency that is not already integrated".into(),
            ));
        }
    }
    for result in ordered_results {
        let attempt = &run.attempts[&result.attempt_id];
        let contract = run
            .contract(&attempt.contract.contract_id, attempt.contract.revision)
            .ok_or_else(|| {
                WorkCoordinationError::NotFound(attempt.contract.contract_id.to_string())
            })?;
        for upstream in &contract.upstream_results {
            let Some(dependent_position) = positions.get(&result.attempt_id) else {
                continue;
            };
            if let Some(prerequisite_position) = positions.get(&upstream.attempt_id) {
                if prerequisite_position >= dependent_position {
                    return Err(WorkCoordinationError::InvalidInput(
                        "verification result order violates a contract result dependency".into(),
                    ));
                }
            } else {
                require_integrated_attempt(
                    run,
                    &upstream.attempt_id,
                    Some(&upstream.result_digest),
                )?;
            }
        }
    }
    Ok(())
}

fn require_selected_or_integrated(
    run: &WorkRun,
    selected: &BTreeSet<WorkAttemptId>,
    result: &WorkResultRef,
) -> Result<(), WorkCoordinationError> {
    let attempt = run
        .attempts
        .get(&result.attempt_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(result.attempt_id.to_string()))?;
    if attempt
        .result
        .as_ref()
        .is_none_or(|sealed| sealed.result_digest != result.result_digest)
    {
        return Err(WorkCoordinationError::InvalidInput(
            "contract dependency does not match its exact sealed result".into(),
        ));
    }
    if selected.contains(&result.attempt_id) {
        Ok(())
    } else {
        require_integrated_attempt(run, &result.attempt_id, Some(&result.result_digest))
    }
}

fn require_integrated_attempt(
    run: &WorkRun,
    attempt_id: &WorkAttemptId,
    result_digest: Option<&zeta_protocol::ContentDigest>,
) -> Result<(), WorkCoordinationError> {
    let attempt = run
        .attempts
        .get(attempt_id)
        .ok_or_else(|| WorkCoordinationError::NotFound(attempt_id.to_string()))?;
    if attempt.integration_status != crate::WorkAttemptIntegrationStatus::Integrated
        || result_digest.is_some_and(|expected| {
            attempt
                .result
                .as_ref()
                .is_none_or(|result| &result.result_digest != expected)
        })
    {
        return Err(WorkCoordinationError::InvalidInput(
            "verification omits a dependency that is not already integrated".into(),
        ));
    }
    Ok(())
}

fn contains_cycle<'a>(
    run: &'a WorkRun,
    attempt_id: &'a WorkAttemptId,
    visiting: &mut BTreeSet<&'a WorkAttemptId>,
    visited: &mut BTreeSet<&'a WorkAttemptId>,
) -> bool {
    if visited.contains(attempt_id) {
        return false;
    }
    if !visiting.insert(attempt_id) {
        return true;
    }
    for prerequisite in prerequisites(run, attempt_id) {
        if contains_cycle(run, prerequisite, visiting, visited) {
            return true;
        }
    }
    visiting.remove(attempt_id);
    visited.insert(attempt_id);
    false
}

fn reaches<'a>(
    run: &'a WorkRun,
    current: &'a WorkAttemptId,
    target: &WorkAttemptId,
    visited: &mut BTreeSet<&'a WorkAttemptId>,
) -> bool {
    if current == target {
        return true;
    }
    if !visited.insert(current) {
        return false;
    }
    prerequisites(run, current)
        .into_iter()
        .any(|prerequisite| reaches(run, prerequisite, target, visited))
}

fn prerequisites<'a>(run: &'a WorkRun, attempt_id: &WorkAttemptId) -> Vec<&'a WorkAttemptId> {
    run.relations
        .values()
        .filter_map(coordination_relation_edge)
        .filter_map(|(dependent, prerequisite)| (dependent == attempt_id).then_some(prerequisite))
        .collect()
}

fn coordination_relation_edge(relation: &WorkRelation) -> Option<(&WorkAttemptId, &WorkAttemptId)> {
    coordination_edge(
        &relation.source_attempt_id,
        &relation.target_attempt_id,
        &relation.kind,
    )
}

fn coordination_edge<'a>(
    source_attempt_id: &'a WorkAttemptId,
    target_attempt_id: &'a WorkAttemptId,
    kind: &WorkRelationKind,
) -> Option<(&'a WorkAttemptId, &'a WorkAttemptId)> {
    match kind {
        WorkRelationKind::Wait { .. } | WorkRelationKind::ResultDependency { .. } => {
            Some((source_attempt_id, target_attempt_id))
        }
        WorkRelationKind::Handoff { .. } => Some((target_attempt_id, source_attempt_id)),
        WorkRelationKind::Observation | WorkRelationKind::Alternate => None,
    }
}

fn result_relation_edge(relation: &WorkRelation) -> Option<(&WorkAttemptId, &WorkAttemptId)> {
    result_edge(
        &relation.source_attempt_id,
        &relation.target_attempt_id,
        &relation.kind,
    )
}

fn result_edge<'a>(
    source_attempt_id: &'a WorkAttemptId,
    target_attempt_id: &'a WorkAttemptId,
    kind: &WorkRelationKind,
) -> Option<(&'a WorkAttemptId, &'a WorkAttemptId)> {
    match kind {
        WorkRelationKind::Wait {
            condition:
                crate::WorkWaitCondition::AttemptSealed | crate::WorkWaitCondition::ExactResult { .. },
            ..
        }
        | WorkRelationKind::ResultDependency { .. } => Some((source_attempt_id, target_attempt_id)),
        WorkRelationKind::Handoff { .. } => Some((target_attempt_id, source_attempt_id)),
        WorkRelationKind::Observation
        | WorkRelationKind::Alternate
        | WorkRelationKind::Wait {
            condition: crate::WorkWaitCondition::ExecutionFinished,
            ..
        } => None,
    }
}
