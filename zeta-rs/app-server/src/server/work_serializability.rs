use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_protocol::WorkAttemptId;
use zeta_turn_changes::TurnChangeSet;
use zeta_work_coordination::WorkResultRef;
use zeta_work_coordination::WorkRun;
use zeta_work_coordination::WorkSerializabilityEvidence;
use zeta_work_coordination::WorkSerializabilityStatus;
use zeta_work_coordination::ordered_result_refs;
use zeta_work_coordination::ordered_result_refs_with_dependencies;

pub(super) struct WorkSerializabilityAnalysis {
    pub(super) ordered_results: Vec<WorkResultRef>,
    pub(super) evidence: WorkSerializabilityEvidence,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourcePath {
    source_dir_id: DirId,
    repository_id: String,
    path: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AttemptAccess {
    reads: BTreeSet<ResourcePath>,
    writes: BTreeSet<ResourcePath>,
}

/// Combines declared dependencies with actual file reads and writes.
///
/// A reader must remain before an independent writer because it observed the original baseline.
/// Overlapping writes and opaque effects remain indeterminate even if Git could textually merge
/// them; they require a new ordered attempt that consumes an exact upstream result.
pub(super) fn analyze_work_serializability(
    run: &WorkRun,
    selected: &BTreeSet<WorkAttemptId>,
    records: &BTreeMap<WorkAttemptId, Vec<TurnChangeSet>>,
) -> Result<WorkSerializabilityAnalysis, String> {
    let declared = ordered_result_refs(run, selected).map_err(|error| error.to_string())?;
    let mut accesses = selected
        .iter()
        .map(|attempt_id| (attempt_id.clone(), AttemptAccess::default()))
        .collect::<BTreeMap<_, _>>();
    let mut issues = BTreeSet::new();
    for attempt_id in selected {
        let attempt_records = records
            .get(attempt_id)
            .ok_or_else(|| format!("WorkAttempt {attempt_id} has no ChangeSet evidence"))?;
        let access = accesses
            .get_mut(attempt_id)
            .ok_or_else(|| "serializability access map lost a WorkAttempt".to_string())?;
        for record in attempt_records {
            let provenance = record.work_attempt.as_ref().ok_or_else(|| {
                "serializability ChangeSet omitted WorkAttempt provenance".to_string()
            })?;
            if record.opaque_dependencies {
                issues.insert(format!(
                    "WorkAttempt {attempt_id} executed a tool with opaque file dependencies"
                ));
            }
            if !record.external_dependency_paths.is_empty() {
                issues.insert(format!(
                    "WorkAttempt {attempt_id} consumed source state outside its publishable result"
                ));
            }
            for path in &record.read_paths {
                match resource_path(provenance, record, path) {
                    Some(path) => {
                        access.reads.insert(path);
                    }
                    None => {
                        issues.insert(format!(
                            "WorkAttempt {attempt_id} recorded a non-relative read path"
                        ));
                    }
                }
            }
            for path in &record.write_paths {
                match resource_path(provenance, record, path) {
                    Some(path) => {
                        access.writes.insert(path);
                    }
                    None => {
                        issues.insert(format!(
                            "WorkAttempt {attempt_id} recorded a non-relative write path"
                        ));
                    }
                }
            }
        }
    }

    let attempt_ids = selected.iter().collect::<Vec<_>>();
    let mut dependencies = BTreeSet::new();
    for (index, left_id) in attempt_ids.iter().enumerate() {
        for right_id in attempt_ids.iter().skip(index + 1) {
            let left = accesses
                .get(*left_id)
                .ok_or_else(|| "serializability access map lost its left attempt".to_string())?;
            let right = accesses
                .get(*right_id)
                .ok_or_else(|| "serializability access map lost its right attempt".to_string())?;
            if sets_overlap(&left.writes, &right.writes) {
                issues.insert(format!(
                    "WorkAttempts {left_id} and {right_id} have overlapping writes"
                ));
            }
            if sets_overlap(&left.reads, &right.writes) {
                dependencies.insert(((*right_id).clone(), (*left_id).clone()));
            }
            if sets_overlap(&right.reads, &left.writes) {
                dependencies.insert(((*left_id).clone(), (*right_id).clone()));
            }
        }
    }

    let ordered = match ordered_result_refs_with_dependencies(run, selected, &dependencies) {
        Ok(ordered) => ordered,
        Err(error) => {
            issues.insert(format!(
                "actual access order conflicts with declared dependencies: {error}"
            ));
            declared
        }
    };
    let encoded = serde_json::to_vec(&(1_u32, &accesses, &dependencies, &issues, &ordered))
        .map_err(|error| error.to_string())?;
    let status = if issues.is_empty() {
        WorkSerializabilityStatus::Proven
    } else {
        WorkSerializabilityStatus::Indeterminate
    };
    let reason = if issues.is_empty() {
        "declared dependencies and actual read/write effects form one stable order".into()
    } else {
        issues.into_iter().collect::<Vec<_>>().join("; ")
    };
    Ok(WorkSerializabilityAnalysis {
        ordered_results: ordered,
        evidence: WorkSerializabilityEvidence {
            status,
            evidence_digest: ContentDigest::sha256(&encoded),
            reason,
        },
    })
}

fn resource_path(
    provenance: &zeta_turn_changes::WorkAttemptChangeProvenance,
    record: &TurnChangeSet,
    path: &Path,
) -> Option<ResourcePath> {
    let path = normalized_relative_path(path)?;
    Some(ResourcePath {
        source_dir_id: provenance.source_root_dir_id.clone(),
        repository_id: record.repository_id.clone(),
        path,
    })
}

fn normalized_relative_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => normalized.push(component),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    Some(normalized)
}

fn sets_overlap(left: &BTreeSet<ResourcePath>, right: &BTreeSet<ResourcePath>) -> bool {
    left.iter().any(|left| {
        right.iter().any(|right| {
            left.source_dir_id == right.source_dir_id
                && left.repository_id == right.repository_id
                && paths_overlap(&left.path, &right.path)
        })
    })
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

#[cfg(test)]
#[path = "work_serializability_tests.rs"]
mod tests;
