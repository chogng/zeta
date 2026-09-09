use super::AppServer;
use super::RpcError;
use super::issue_assignment::now;
use super::issue_operations::issue_error;
use super::turn_changes_runtime::TurnChangesRuntime;
use std::collections::BTreeMap;
use zeta_protocol::ContentDigest;
use zeta_state::IssueAssignmentCommand;
use zeta_turn_changes::TurnChangeStore;
use zeta_work_coordination::IssueAssignment;
use zeta_work_coordination::IssueDelivery;
use zeta_work_coordination::IssueDeliveryReceipt;

impl TurnChangesRuntime {
    pub(super) fn validate_issue_delivery_tree(
        &self,
        assignment: &IssueAssignment,
        tree: &zeta_git::GitTreeId,
        target: &str,
    ) -> Result<ContentDigest, String> {
        if assignment.workflow.validation_commands.is_empty() {
            return Err("Issue workflow has no validation commands".into());
        }
        let thread = assignment
            .thread_id
            .as_ref()
            .ok_or("Issue has no execution Thread")?;
        let permission = self
            .file_access
            .thread_scope(thread, zeta_file_access::Permission::ExecuteCommands)
            .map_err(|error| error.to_string())?
            .ok_or("Issue verification command permission is unavailable")?;
        let key = ContentDigest::sha256(
            &serde_json::to_vec(&(
                assignment.id.as_str(),
                assignment.epoch,
                target,
                tree.as_str(),
                &assignment.workflow.validation_commands,
            ))
            .map_err(|error| error.to_string())?,
        );
        let request = zeta_worktree::ManagedDirProvisionRequest {
            source: zeta_worktree::ManagedDirSource::ImmutableTree {
                source_directory: self.dir_root.clone(),
                tree_id: tree.as_str().into(),
                repository_trees: BTreeMap::new(),
            },
            target: zeta_worktree::ManagedDirTarget::Detached {
                object_id: target.into(),
            },
            repository_targets: BTreeMap::new(),
            source_dir_id: self.dir_id.to_string(),
            owner: zeta_worktree::ManagedDirOwner::VerificationRoot {
                work_run_id: assignment
                    .work_run_id
                    .as_ref()
                    .ok_or("Issue has no WorkRun")?
                    .to_string(),
                verification_key: key.to_string(),
                source_dir_id: self.dir_id.to_string(),
            },
        };
        let binding = self
            .worktree_runtime
            .block_on(self.worktrees.provision(&request))
            .map_err(|error| error.to_string())?;
        let directory =
            zeta_file_access::Dir::open_local(binding.dir()).map_err(|error| error.to_string())?;
        let hidden = [
            &self.profile_root,
            &self.dir_root,
            &self.worktrees.settings().root,
        ]
        .into_iter()
        .map(zeta_file_access::Dir::open_local)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
        let hidden = hidden
            .into_iter()
            .map(|dir| (dir.id(), dir))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        let watch = self.watch_issue_validation(std::slice::from_ref(assignment))?;
        let cancellation = watch.token();
        let mut outputs = Vec::new();
        for command in &assignment.workflow.validation_commands {
            let policy = self
                .verification_tool_config
                .read()
                .map_err(|_| "Verification policy lock poisoned")?
                .clone();
            let output = crate::local_tools::execute_verification_command(
                permission.primary(),
                &policy,
                directory.clone(),
                hidden.clone(),
                command,
                &cancellation,
            )?;
            if output.exit_code != Some(0) {
                return Err(format!(
                    "Delivery validation failed: {command}\n{}\n{}",
                    output.stdout, output.stderr
                ));
            }
            outputs.push((command, output.stdout, output.stderr));
        }
        let actual = self
            .worktree_runtime
            .block_on(async {
                let git = zeta_git::GitClient::system();
                let repository = git.open_repository(binding.dir()).await?;
                git.capture_worktree_tree(&repository).await
            })
            .map_err(|error| error.to_string())?;
        if &actual != tree {
            return Err("Validation modified the delivery candidate".into());
        }
        Ok(ContentDigest::sha256(
            &serde_json::to_vec(&(key, outputs)).map_err(|error| error.to_string())?,
        ))
    }
}

impl AppServer {
    pub(super) fn prepare_issue_delivery(
        &self,
        assignment: &IssueAssignment,
        verification: &ContentDigest,
    ) -> Result<IssueAssignment, RpcError> {
        if assignment.ownership != zeta_work_coordination::IssueOwnership::Held {
            return Err(issue_error("Issue assignment is no longer held".into()));
        }
        let changes = self.turn_changes_runtime()?;
        let attempt_id = assignment
            .attempt_id
            .as_ref()
            .ok_or_else(|| issue_error("Issue has no attempt".into()))?;
        let records = changes
            .store
            .list_for_thread(
                assignment
                    .thread_id
                    .as_ref()
                    .ok_or_else(|| issue_error("Issue has no Thread".into()))?,
            )
            .map_err(|error| issue_error(error.to_string()))?
            .into_iter()
            .filter(|record| {
                record
                    .work_attempt
                    .as_ref()
                    .is_some_and(|identity| &identity.attempt_id == attempt_id)
            })
            .collect::<Vec<_>>();
        let before = zeta_git::GitTreeId::new(
            records
                .first()
                .ok_or_else(|| issue_error("Issue has no captured changes".into()))?
                .before_tree
                .clone(),
        )
        .map_err(|error| issue_error(error.to_string()))?;
        let after = zeta_git::GitTreeId::new(
            records
                .last()
                .and_then(|record| record.after_tree.clone())
                .ok_or_else(|| issue_error("Issue result is not sealed".into()))?,
        )
        .map_err(|error| issue_error(error.to_string()))?;
        let (head, tree) = changes
            .worktree_runtime
            .block_on(async {
                let git = zeta_git::GitClient::system();
                let repository = git
                    .open_repository(&changes.dir_root)
                    .await
                    .map_err(|error| error.to_string())?;
                let head = git
                    .fetch_branch(&repository, &assignment.target_branch)
                    .await
                    .map_err(|error| error.to_string())?;
                let target = git
                    .resolve_tree(&repository, &head)
                    .await
                    .map_err(|error| error.to_string())?;
                let tree = match git
                    .replay_tree_delta(&repository, &before, &target, &after)
                    .await
                    .map_err(|error| error.to_string())?
                {
                    zeta_git::GitTreeReplayResult::Clean(tree) => tree,
                    zeta_git::GitTreeReplayResult::Conflict { paths } => {
                        return Err(format!(
                            "Issue conflicts with its current target: {paths:?}"
                        ));
                    }
                };
                Ok::<_, String>((head, tree))
            })
            .map_err(issue_error)?;
        let evidence = changes
            .validate_issue_delivery_tree(assignment, &tree, &head)
            .map_err(issue_error)?;
        let commit = changes.worktree_runtime.block_on(async {
            let git = zeta_git::GitClient::system(); let repository = git.open_repository(&changes.dir_root).await?;
            git.prepare_commit_object(&repository, &head, &tree, &format!("Implement {}\n\nIssue verification: {verification}\nDelivery validation: {evidence}", assignment.item.objective)).await
        }).map_err(|error| issue_error(error.to_string()))?;
        let receipt = IssueDeliveryReceipt {
            target_head: head,
            tree: tree.as_str().into(),
            commit,
            verification_key: verification.to_string(),
            pull_request_number: None,
            pull_request_url: None,
        };
        self.issue_assignment_store()?
            .apply(
                &format!("{}-delivery-{}", assignment.id, assignment.revision),
                &assignment.id,
                assignment.revision,
                IssueAssignmentCommand::RecordDelivery { receipt },
                now()?,
            )
            .map_err(issue_error)
    }

    pub(super) fn publish_issue_delivery(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<IssueAssignment, RpcError> {
        if assignment.ownership != zeta_work_coordination::IssueOwnership::Held {
            return Err(issue_error("Issue assignment is no longer held".into()));
        }
        let changes = self.turn_changes_runtime()?;
        let cancellation = zeta_async_utils::CancellationSource::new();
        match audit_issue_remote(&changes, assignment, &cancellation.token())? {
            RemoteIssueState::Conflict(reason) => return Err(issue_error(reason)),
            RemoteIssueState::Merged => return self.complete_issue_delivery(assignment),
            RemoteIssueState::Unchanged => {}
        }
        let current = self
            .issue_assignment_store()?
            .read(&assignment.id)
            .map_err(issue_error)?;
        if current.epoch != assignment.epoch {
            return Err(issue_error("Issue publication was superseded".into()));
        }
        let assignment = &current;
        if assignment.paused
            || assignment.sync_state != zeta_work_coordination::IssueSyncState::Synced
        {
            return Err(issue_error(
                "Resolve the paused or unsynchronized assignment before publishing".into(),
            ));
        }
        let receipt = assignment
            .delivery
            .as_ref()
            .ok_or_else(|| issue_error("Verify the exact delivery candidate first".into()))?;
        let repository = zeta_github::Repository::new(
            assignment.repository.host.clone(),
            assignment.repository.owner.clone(),
            assignment.repository.name.clone(),
        )
        .map_err(issue_error)?;
        let github = zeta_github::GitHub::default();
        for issue in &assignment.item.issues {
            let metadata = changes
                .worktree_runtime
                .block_on(github.issue_metadata(&repository, issue.number))
                .map_err(issue_error)?;
            if metadata.node_id != issue.node_id
                || metadata
                    .issue
                    .assignees
                    .iter()
                    .any(|assignee| !assignee.login.eq_ignore_ascii_case(&assignment.owner))
            {
                return Err(issue_error(
                    "Issue ownership changed before publication".into(),
                ));
            }
        }
        changes
            .worktree_runtime
            .block_on(async {
                let git = zeta_git::GitClient::system();
                let source = git
                    .open_repository(&changes.dir_root)
                    .await
                    .map_err(|error| error.to_string())?;
                let target = git
                    .fetch_branch(&source, &assignment.target_branch)
                    .await
                    .map_err(|error| error.to_string())?;
                if target != receipt.target_head
                    && !(assignment.workflow.delivery == IssueDelivery::Branch
                        && target == receipt.commit)
                {
                    return Err("Target branch changed; verify the new delivery candidate".into());
                }
                let branch = if assignment.workflow.delivery == IssueDelivery::Branch {
                    &assignment.target_branch
                } else {
                    &assignment.branch
                };
                git.push_branch_commit(&source, branch, &receipt.commit)
                    .await
                    .map_err(|error| error.to_string())
            })
            .map_err(issue_error)?;
        if assignment.workflow.delivery == IssueDelivery::Branch {
            return self.complete_issue_delivery(assignment);
        }
        let pr = changes
            .worktree_runtime
            .block_on(async {
                if let Some(pr) = github
                    .find_pull_request(&repository, &assignment.branch, &assignment.target_branch)
                    .await?
                {
                    if pr.head.sha != receipt.commit {
                        return Err(
                            "Existing PR has a different head; review it before continuing".into(),
                        );
                    }
                    return Ok(pr);
                }
                let body = assignment
                    .item
                    .issues
                    .iter()
                    .map(|issue| {
                        format!(
                            "{} #{}",
                            if assignment.workflow.close_on_completion {
                                "Closes"
                            } else {
                                "Refs"
                            },
                            issue.number
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                github
                    .create_pull_request(
                        &repository,
                        zeta_github::CreatePullRequest {
                            title: &assignment.item.objective,
                            body: &body,
                            head: &assignment.branch,
                            base: &assignment.target_branch,
                            draft: assignment.workflow.delivery == IssueDelivery::DraftPullRequest,
                        },
                    )
                    .await
            })
            .map_err(issue_error)?;
        let mut receipt = receipt.clone();
        receipt.pull_request_number = Some(pr.number);
        receipt.pull_request_url = Some(pr.html_url.clone());
        let saved = self
            .issue_assignment_store()?
            .apply(
                &format!("{}-pr-{}", assignment.id, assignment.revision),
                &assignment.id,
                assignment.revision,
                IssueAssignmentCommand::RecordDelivery { receipt },
                now()?,
            )
            .map_err(issue_error)?;
        self.ensure_issue_scheduler(&saved)?;
        if assignment.workflow.auto_merge && pr.auto_merge.is_none() {
            changes
                .worktree_runtime
                .block_on(github.enable_auto_merge(
                    &repository,
                    &pr,
                    zeta_github::MergeMethod::Squash,
                ))
                .map_err(issue_error)?;
        }
        Ok(saved)
    }

    pub(super) fn complete_issue_delivery(
        &self,
        assignment: &IssueAssignment,
    ) -> Result<IssueAssignment, RpcError> {
        let changes = self.turn_changes_runtime()?;
        let cancellation = zeta_async_utils::CancellationSource::new();
        finish_issue_delivery(
            self.issue_assignment_store()?,
            &changes,
            assignment,
            &cancellation.token(),
        )
    }
}

/// Cancels an isolated validation process when its ownership epoch or execution policy changes.
pub(super) struct IssueValidationWatch {
    cancellation: zeta_async_utils::CancellationSource,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl IssueValidationWatch {
    pub(super) fn token(&self) -> zeta_async_utils::CancellationToken {
        self.cancellation.token()
    }
}
impl Drop for IssueValidationWatch {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        self.cancellation.cancel();
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}
impl TurnChangesRuntime {
    pub(super) fn watch_issue_validation(
        &self,
        assignments: &[IssueAssignment],
    ) -> Result<IssueValidationWatch, String> {
        let store = self.issue_assignments.clone();
        let expected = assignments
            .iter()
            .map(|assignment| {
                (
                    assignment.id.clone(),
                    assignment.epoch,
                    assignment.ownership.clone(),
                )
            })
            .collect::<Vec<_>>();
        let policy = self.verification_tool_config.clone();
        let expected_policy = policy
            .read()
            .map_err(|_| "Verification policy lock poisoned")?
            .clone();
        let mut authorizations = Vec::new();
        for assignment in assignments {
            let thread = assignment
                .thread_id
                .as_ref()
                .ok_or("Verification worker missing")?;
            let scope = self
                .file_access
                .thread_scope(thread, zeta_file_access::Permission::ExecuteCommands)
                .map_err(|error| error.to_string())?
                .ok_or("Verification permission unavailable")?;
            authorizations.push(scope.primary().clone());
        }
        let cancellation = zeta_async_utils::CancellationSource::new();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut watch = IssueValidationWatch {
            cancellation: cancellation.clone(),
            stop: stop.clone(),
            worker: None,
        };
        watch.worker = Some(
            std::thread::Builder::new()
                .name("zeta-issue-validation-scope".into())
                .spawn(move || {
                    while !stop.load(std::sync::atomic::Ordering::Acquire) {
                        let valid = policy
                            .read()
                            .is_ok_and(|current| *current == expected_policy)
                            && authorizations
                                .iter()
                                .all(|authorization| authorization.ensure_active().is_ok())
                            && expected.iter().all(|(id, epoch, owner)| {
                                store.read(id).is_ok_and(|current| {
                                    current.epoch == *epoch
                                        && &current.ownership == owner
                                        && current.sync_state
                                            != zeta_work_coordination::IssueSyncState::Conflict
                                })
                            });
                        if !valid {
                            cancellation.cancel();
                            break;
                        }
                        std::thread::park_timeout(std::time::Duration::from_millis(50));
                    }
                })
                .map_err(|error| error.to_string())?,
        );
        Ok(watch)
    }
}

pub(super) enum RemoteIssueState {
    Unchanged,
    Merged,
    Conflict(String),
}

pub(super) fn audit_issue_remote(
    changes: &TurnChangesRuntime,
    assignment: &IssueAssignment,
    cancellation: &zeta_async_utils::CancellationToken,
) -> Result<RemoteIssueState, RpcError> {
    let recovered = recover_candidate_pr(changes, assignment, cancellation)?;
    let assignment = &recovered;
    use super::issue_assignment_operations::sync_call;
    let github = zeta_github::GitHub::default();
    let repository = zeta_github::Repository::new(
        assignment.repository.host.clone(),
        assignment.repository.owner.clone(),
        assignment.repository.name.clone(),
    )
    .map_err(issue_error)?;
    let info = changes
        .worktree_runtime
        .block_on(sync_call(
            github.issue_repository(&repository),
            cancellation,
        ))
        .map_err(issue_error)?;
    if info.node_id != assignment.repository.node_id
        || !info
            .full_name
            .eq_ignore_ascii_case(&format!("{}/{}", repository.owner, repository.name))
    {
        return Ok(RemoteIssueState::Conflict(
            "Repository moved; reconcile its identity before continuing".into(),
        ));
    }
    let mut merged = false;
    if let Some(receipt) = &assignment.delivery {
        if let Some(number) = receipt.pull_request_number {
            let pr = changes
                .worktree_runtime
                .block_on(sync_call(
                    github.pull_request(&repository, number),
                    cancellation,
                ))
                .map_err(issue_error)?;
            if pr.head.sha != receipt.commit || pr.base.name != assignment.target_branch {
                return Ok(RemoteIssueState::Conflict(
                    "PR head or target changed after verification".into(),
                ));
            }
            merged = pr.merged_at.is_some();
            if pr.state == "closed" && !merged {
                return Ok(RemoteIssueState::Conflict(
                    "PR was closed without merging; the Issue is unfinished".into(),
                ));
            }
        }
    }
    for issue in &assignment.item.issues {
        let metadata = changes
            .worktree_runtime
            .block_on(sync_call(
                github.issue_metadata(&repository, issue.number),
                cancellation,
            ))
            .map_err(issue_error)?;
        if metadata.node_id != issue.node_id {
            return Ok(RemoteIssueState::Conflict(format!(
                "Issue #{} moved or changed identity",
                issue.number
            )));
        }
        if !merged && metadata.issue.state != "open" {
            return Ok(RemoteIssueState::Conflict(format!(
                "Issue #{} was closed outside this delivery",
                issue.number
            )));
        }
        if metadata.issue.assignees.len() != 1
            || !metadata.issue.assignees[0]
                .login
                .eq_ignore_ascii_case(&assignment.owner)
        {
            return Ok(RemoteIssueState::Conflict(format!(
                "Issue #{} responsible account changed",
                issue.number
            )));
        }
        let snapshot = changes
            .worktree_runtime
            .block_on(sync_call(
                github.issue(&repository, issue.number),
                cancellation,
            ))
            .map_err(issue_error)?;
        if super::issue_assignment::issue_material_digest(&snapshot)? != issue.material_digest {
            return Ok(RemoteIssueState::Conflict(format!(
                "Issue #{} requirements changed; review the new requirements",
                issue.number
            )));
        }
        if !merged {
            let actual = metadata
                .issue
                .labels
                .iter()
                .filter(|label| {
                    assignment
                        .workflow
                        .labels
                        .all()
                        .contains(&label.name.as_str())
                })
                .map(|label| label.name.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let expected = assignment
                .synced_labels
                .get(&issue.node_id)
                .into_iter()
                .flatten()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            if actual != expected {
                return Ok(RemoteIssueState::Conflict(format!(
                    "Issue #{} managed stage was changed externally",
                    issue.number
                )));
            }
        }
    }
    Ok(if merged {
        RemoteIssueState::Merged
    } else {
        RemoteIssueState::Unchanged
    })
}

pub(super) fn finish_issue_delivery(
    store: &zeta_state::SqliteIssueAssignmentStore,
    changes: &TurnChangesRuntime,
    assignment: &IssueAssignment,
    cancellation: &zeta_async_utils::CancellationToken,
) -> Result<IssueAssignment, RpcError> {
    use super::issue_assignment_operations::sync_call;
    let github = zeta_github::GitHub::default();
    let repository = zeta_github::Repository::new(
        assignment.repository.host.clone(),
        assignment.repository.owner.clone(),
        assignment.repository.name.clone(),
    )
    .map_err(issue_error)?;
    let managed = assignment
        .workflow
        .labels
        .all()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for issue in &assignment.item.issues {
        let current = store.read(&assignment.id).map_err(issue_error)?;
        if current.epoch != assignment.epoch
            || current.ownership != zeta_work_coordination::IssueOwnership::Held
        {
            return Err(issue_error("Issue delivery was superseded".into()));
        }
        let metadata = changes
            .worktree_runtime
            .block_on(sync_call(
                github.issue_metadata(&repository, issue.number),
                cancellation,
            ))
            .map_err(issue_error)?;
        if metadata.node_id != issue.node_id
            || metadata
                .issue
                .assignees
                .iter()
                .any(|owner| !owner.login.eq_ignore_ascii_case(&assignment.owner))
        {
            return Err(issue_error(
                "Issue ownership changed during completion".into(),
            ));
        }
        changes
            .worktree_runtime
            .block_on(sync_call(
                github.sync_issue_labels(
                    &repository,
                    issue.number,
                    &managed,
                    None,
                    assignment
                        .synced_labels
                        .get(&issue.node_id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ),
                cancellation,
            ))
            .map_err(issue_error)?;
        if metadata.issue.state == "open" && assignment.workflow.close_on_completion {
            changes
                .worktree_runtime
                .block_on(sync_call(
                    github.close_completed_issue(&repository, issue.number),
                    cancellation,
                ))
                .map_err(issue_error)?;
        }
    }
    let latest = store.read(&assignment.id).map_err(issue_error)?;
    if latest.epoch != assignment.epoch {
        return Err(issue_error("Issue delivery was superseded".into()));
    }
    store
        .apply(
            &format!("{}-complete-{}", latest.id, latest.epoch),
            &latest.id,
            latest.revision,
            IssueAssignmentCommand::Complete,
            now()?,
        )
        .map_err(issue_error)
}

fn recover_candidate_pr(
    changes: &TurnChangesRuntime,
    assignment: &IssueAssignment,
    cancellation: &zeta_async_utils::CancellationToken,
) -> Result<IssueAssignment, RpcError> {
    let Some(receipt) = &assignment.delivery else {
        return Ok(assignment.clone());
    };
    if receipt.pull_request_number.is_some()
        || assignment.workflow.delivery == IssueDelivery::Branch
    {
        return Ok(assignment.clone());
    }
    let repository = zeta_github::Repository::new(
        assignment.repository.host.clone(),
        assignment.repository.owner.clone(),
        assignment.repository.name.clone(),
    )
    .map_err(issue_error)?;
    let github = zeta_github::GitHub::default();
    let existing = changes
        .worktree_runtime
        .block_on(super::issue_assignment_operations::sync_call(
            github.find_pull_request(&repository, &assignment.branch, &assignment.target_branch),
            cancellation,
        ))
        .map_err(issue_error)?;
    let Some(pr) = existing else {
        return Ok(assignment.clone());
    };
    if pr.head.sha != receipt.commit || pr.base.name != assignment.target_branch {
        return Err(issue_error(
            "Existing PR differs from the verified delivery candidate".into(),
        ));
    }
    let current = changes
        .issue_assignments
        .read(&assignment.id)
        .map_err(issue_error)?;
    if current.epoch != assignment.epoch {
        return Err(issue_error("Issue publication was superseded".into()));
    }
    let mut receipt = receipt.clone();
    receipt.pull_request_number = Some(pr.number);
    receipt.pull_request_url = Some(pr.html_url);
    changes
        .issue_assignments
        .apply(
            &format!("{}-recover-pr-{}", current.id, current.revision),
            &current.id,
            current.revision,
            IssueAssignmentCommand::RecordDelivery { receipt },
            now()?,
        )
        .map_err(issue_error)
}
