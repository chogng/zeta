use crate::ManagedRootBinding;
use crate::RootCheckpoint;
use crate::RootState;
use crate::WorkContractDraft;
use crate::WorkCoordinationError;
use crate::WorkParticipant;
use crate::WorkParticipantRelation;
use crate::WorkRun;
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;
use zeta_protocol::ContentDigest;
use zeta_protocol::ThreadId;

const MAX_TEXT_BYTES: usize = 256 * 1024;
const MAX_LIST_ITEMS: usize = 1_024;

pub(crate) fn goal(
    objective: &str,
    acceptance_conditions: &[String],
    exclusions: &[String],
) -> Result<(), WorkCoordinationError> {
    text("objective", objective)?;
    non_empty_texts("acceptance conditions", acceptance_conditions)?;
    texts("exclusions", exclusions)
}

pub(crate) fn text(label: &str, value: &str) -> Result<(), WorkCoordinationError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES {
        return Err(WorkCoordinationError::InvalidInput(format!(
            "{label} must be non-empty and bounded"
        )));
    }
    Ok(())
}

pub(crate) fn texts(label: &str, values: &[String]) -> Result<(), WorkCoordinationError> {
    if values.len() > MAX_LIST_ITEMS {
        return Err(WorkCoordinationError::InvalidInput(format!(
            "{label} contains too many entries"
        )));
    }
    for value in values {
        text(label, value)?;
    }
    Ok(())
}

pub(crate) fn non_empty_texts(label: &str, values: &[String]) -> Result<(), WorkCoordinationError> {
    if values.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(format!(
            "{label} must not be empty"
        )));
    }
    texts(label, values)
}

pub(crate) fn new_participant(
    run: &WorkRun,
    participant: &WorkParticipant,
) -> Result<(), WorkCoordinationError> {
    if run.participants.contains_key(&participant.thread_id) {
        return Err(WorkCoordinationError::AlreadyExists(
            participant.thread_id.to_string(),
        ));
    }
    match &participant.relation {
        WorkParticipantRelation::Root => {
            if run
                .participants
                .values()
                .any(|existing| existing.session_id == participant.session_id)
            {
                return Err(WorkCoordinationError::InvalidInput(
                    "a WorkRun may bind only one root participant for each Session".into(),
                ));
            }
        }
        WorkParticipantRelation::Delegated {
            parent_thread_id,
            delegation_id,
        } => {
            if parent_thread_id == &participant.thread_id {
                return Err(WorkCoordinationError::InvalidInput(
                    "a delegated participant cannot be its own parent".into(),
                ));
            }
            let parent = run.participants.get(parent_thread_id).ok_or_else(|| {
                WorkCoordinationError::InvalidInput(
                    "a delegated participant requires its existing parent".into(),
                )
            })?;
            if parent.session_id != participant.session_id {
                return Err(WorkCoordinationError::InvalidInput(
                    "delegated participants and their parent must share one Session".into(),
                ));
            }
            if run.participants.values().any(|existing| {
                matches!(
                    &existing.relation,
                    WorkParticipantRelation::Delegated {
                        delegation_id: existing_id,
                        ..
                    } if existing_id == delegation_id
                )
            }) {
                return Err(WorkCoordinationError::InvalidInput(
                    "a delegation may bind only one participant".into(),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn contract(
    run: &WorkRun,
    draft: &WorkContractDraft,
) -> Result<(), WorkCoordinationError> {
    if draft.goal_revision
        != run
            .current_goal()
            .ok_or_else(|| {
                WorkCoordinationError::InvalidInput("WorkRun has no goal revision".into())
            })?
            .revision
    {
        return Err(WorkCoordinationError::InvalidInput(
            "a new contract must bind the current goal revision".into(),
        ));
    }
    if draft.topology_revision != run.topology_revision {
        return Err(WorkCoordinationError::InvalidInput(
            "a new contract must bind the current collaboration topology revision".into(),
        ));
    }
    if !run.participants.contains_key(&draft.owner_thread_id) {
        return Err(WorkCoordinationError::InvalidInput(
            "contract owner is not a WorkRun participant".into(),
        ));
    }
    goal(
        &draft.objective,
        &draft.acceptance_conditions,
        &draft.exclusions,
    )?;
    root_checkpoints(&draft.roots, &draft.environment_id)?;
    if !draft
        .roots
        .iter()
        .any(|root| root.dir_id == draft.primary_root_dir_id)
    {
        return Err(WorkCoordinationError::InvalidInput(
            "the primary root must be one of the contract root checkpoints".into(),
        ));
    }
    text("authorization authority", &draft.authorization.authority)?;
    text(
        "authorization policy revision",
        &draft.authorization.policy_revision,
    )?;
    text("validation profile name", &draft.validation_profile.name)?;
    scope_claim(&draft.expected_scope)?;
    for decision_id in &draft.decision_ids {
        if !run.decisions.contains_key(decision_id) {
            return Err(WorkCoordinationError::InvalidInput(format!(
                "contract references unknown decision {decision_id}"
            )));
        }
    }
    for result in &draft.upstream_results {
        let attempt = run.attempts.get(&result.attempt_id).ok_or_else(|| {
            WorkCoordinationError::InvalidInput(format!(
                "contract references unknown attempt {}",
                result.attempt_id
            ))
        })?;
        if attempt
            .result
            .as_ref()
            .is_none_or(|existing| existing.result_digest != result.result_digest)
        {
            return Err(WorkCoordinationError::InvalidInput(format!(
                "contract references an unsealed or mismatched result from {}",
                result.attempt_id
            )));
        }
    }
    Ok(())
}

pub(crate) fn root_checkpoints(
    roots: &[RootCheckpoint],
    environment_id: &zeta_environment::EnvId,
) -> Result<(), WorkCoordinationError> {
    if roots.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "a work contract requires at least one root checkpoint".into(),
        ));
    }
    let mut dirs = BTreeSet::new();
    let mut repositories = BTreeSet::new();
    for root in roots {
        root_checkpoint(root, environment_id)?;
        if !dirs.insert(&root.dir_id) {
            return Err(WorkCoordinationError::InvalidInput(
                "a contract cannot bind the same directory more than once".into(),
            ));
        }
        if let RootState::Git {
            repositories: root_repositories,
        } = &root.state
        {
            for repository in root_repositories {
                if !repositories.insert(&repository.repository_id) {
                    return Err(WorkCoordinationError::InvalidInput(
                        "a contract cannot select the same repository through multiple roots"
                            .into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn root_checkpoint(
    root: &RootCheckpoint,
    environment_id: &zeta_environment::EnvId,
) -> Result<(), WorkCoordinationError> {
    if &root.environment_id != environment_id {
        return Err(WorkCoordinationError::InvalidInput(
            "every contract root must belong to the contract Environment".into(),
        ));
    }
    match &root.state {
        RootState::Git { repositories } => {
            if repositories.is_empty() {
                return Err(WorkCoordinationError::InvalidInput(
                    "a Git root checkpoint requires at least one repository".into(),
                ));
            }
            let mut identities = BTreeSet::new();
            let mut paths = BTreeSet::new();
            for repository in repositories {
                text("repository identity", &repository.repository_id)?;
                text("baseline tree", &repository.baseline_tree)?;
                if !identities.insert(&repository.repository_id) {
                    return Err(WorkCoordinationError::InvalidInput(
                        "a root checkpoint repeats a repository identity".into(),
                    ));
                }
                let path = Path::new(&repository.relative_path);
                if path.as_os_str().is_empty()
                    || path.is_absolute()
                    || path.components().any(|component| {
                        matches!(
                            component,
                            Component::ParentDir | Component::RootDir | Component::Prefix(_)
                        )
                    })
                    || !paths.insert(path)
                {
                    return Err(WorkCoordinationError::InvalidInput(
                        "repository paths must be unique contained relative paths".into(),
                    ));
                }
                match &repository.target {
                    crate::GitRootTarget::Branch {
                        name,
                        expected_head,
                    } => {
                        text("target branch", name)?;
                        text("expected target head", expected_head)?;
                    }
                    crate::GitRootTarget::UnbornBranch {
                        name,
                        anchor_object_id,
                    } => {
                        text("target branch", name)?;
                        text("target anchor", anchor_object_id)?;
                    }
                    crate::GitRootTarget::Detached { object_id } => {
                        text("detached target", object_id)?;
                    }
                }
            }
        }
        RootState::Directory { snapshot_id } => text("directory snapshot", snapshot_id)?,
    }
    let mut precedence = BTreeSet::new();
    for resource in &root.control_resources {
        if resource.source_dir_id != root.dir_id {
            return Err(WorkCoordinationError::InvalidInput(
                "a root control resource must name its containing directory".into(),
            ));
        }
        text("control resource scope", &resource.scope)?;
        let path = Path::new(&resource.relative_path);
        if path.as_os_str().is_empty()
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(WorkCoordinationError::InvalidInput(
                "control resource path must be a contained relative path".into(),
            ));
        }
        if !precedence.insert((resource.kind, resource.scope.clone(), resource.precedence)) {
            return Err(WorkCoordinationError::InvalidInput(
                "control resources in one scope require a unique deterministic precedence".into(),
            ));
        }
    }
    Ok(())
}

/// Returns the canonical content identity used to bind a managed root to one checkpoint.
pub fn root_checkpoint_digest(
    root: &RootCheckpoint,
) -> Result<ContentDigest, WorkCoordinationError> {
    let encoded = serde_json::to_vec(&(1_u32, root))
        .map_err(|error| WorkCoordinationError::InvalidInput(error.to_string()))?;
    Ok(ContentDigest::sha256(&encoded))
}

pub(crate) fn workspace_bindings(
    checkpoints: &[RootCheckpoint],
    bindings: &[ManagedRootBinding],
    private_output_dir_id: &zeta_file_access::DirId,
) -> Result<(), WorkCoordinationError> {
    if checkpoints.len() != bindings.len() || bindings.is_empty() {
        return Err(WorkCoordinationError::InvalidInput(
            "managed roots must match every contract root exactly".into(),
        ));
    }
    let mut managed_dirs = BTreeSet::new();
    for (checkpoint, binding) in checkpoints.iter().zip(bindings) {
        if checkpoint.dir_id != binding.source_dir_id
            || checkpoint.dir_id == binding.managed_dir_id
            || binding.root_checkpoint_digest != root_checkpoint_digest(checkpoint)?
            || !managed_dirs.insert(&binding.managed_dir_id)
        {
            return Err(WorkCoordinationError::InvalidInput(
                "managed root identity does not match its immutable checkpoint".into(),
            ));
        }
    }
    if checkpoints
        .iter()
        .any(|checkpoint| &checkpoint.dir_id == private_output_dir_id)
        || managed_dirs.contains(private_output_dir_id)
    {
        return Err(WorkCoordinationError::InvalidInput(
            "private output must be separate from source and managed roots".into(),
        ));
    }
    Ok(())
}

pub(crate) fn scope_claim(claim: &crate::WorkScopeClaim) -> Result<(), WorkCoordinationError> {
    for component in &claim.components {
        text("expected component", component)?;
    }
    for path in &claim.paths {
        text("expected path", path)?;
    }
    for contract in &claim.contracts {
        text("expected contract", contract)?;
    }
    for resource in &claim.resources {
        text("expected resource", resource)?;
    }
    Ok(())
}

pub(crate) fn participant_for_attempt<'a>(
    run: &'a WorkRun,
    thread_id: &ThreadId,
) -> Result<&'a WorkParticipant, WorkCoordinationError> {
    run.participants.get(thread_id).ok_or_else(|| {
        WorkCoordinationError::InvalidInput("attempt owner is not a WorkRun participant".into())
    })
}
