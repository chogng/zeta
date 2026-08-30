use super::turn_changes_message::spawn_message_job;
use super::turn_changes_runtime::TurnChangesRuntime;
use super::work_attempt_workspace::ExecutionRootBinding;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use zeta_core::{
    CoreError, HookExecutionEvent, HookExecutionObserver, TurnExecutionFinished, TurnExecutionKind,
    TurnExecutionObserver, TurnExecutionStarted, TurnExecutionTerminalState,
    TurnToolExecutionFinished, TurnToolExecutionStarted,
};
use zeta_turn_changes::{
    MessageState, RepositoryCaptureTarget, SnapshotBackend, TerminalTurnState, ToolChangeScope,
    TurnChangeBeginRequest, TurnChangeSealRequest,
};
use zeta_worktree::ManagedDirKind;

impl TurnExecutionObserver for TurnChangesRuntime {
    fn will_execute(&self, event: &TurnExecutionStarted) -> Result<(), CoreError> {
        if self
            .sealing_threads
            .read()
            .map_err(|_| CoreError::Journal("WorkAttempt sealing lock poisoned".into()))?
            .contains(&event.thread_id)
        {
            return Err(CoreError::Execution(
                "Thread cannot start a Turn while its WorkAttempt result is sealing".into(),
            ));
        }
        let roots = self
            .execution_roots(&event.thread_id)
            .map_err(CoreError::Journal)?;
        if roots.is_empty() {
            return Err(CoreError::Journal(format!(
                "Thread {} has no durable execution roots",
                event.thread_id
            )));
        }
        let mut repository_ids = BTreeSet::new();
        let repositories = roots
            .iter()
            .flat_map(|root| {
                root.binding
                    .repositories()
                    .iter()
                    .map(move |repository| (root, repository))
            })
            .map(|(root, repository)| {
                if !repository_ids.insert(repository.repository_id().to_string()) {
                    return Err(CoreError::Journal(format!(
                        "execution roots repeat repository {}",
                        repository.repository_id()
                    )));
                }
                Ok(RepositoryCaptureTarget {
                    repository_id: repository.repository_id().to_string(),
                    worktree_root: repository.worktree_root().to_path_buf(),
                    target_branch: repository.target_branch().map(ToOwned::to_owned),
                    base_object_id: (!repository.target_unborn())
                        .then(|| repository.target_head().to_string()),
                    snapshot_backend: match root.binding.kind() {
                        ManagedDirKind::Git => SnapshotBackend::Git,
                        ManagedDirKind::Directory => SnapshotBackend::Directory {
                            object_store: root
                                .binding
                                .snapshot_store()
                                .ok_or_else(|| {
                                    CoreError::Journal(
                                        "directory binding omitted its snapshot store".into(),
                                    )
                                })?
                                .to_path_buf(),
                        },
                    },
                    baseline_dependency_paths: self
                        .initial_baseline_paths(&root.binding, repository)?,
                    work_attempt: root.work_attempt.clone(),
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let records = match self.ledger.begin_turn(TurnChangeBeginRequest {
            session_id: event.session_id.clone(),
            thread_id: event.thread_id.clone(),
            turn_id: event.turn_id.clone(),
            repositories,
            commit_message_configured: self.commit_message_configured(),
            opaque_dependencies: matches!(event.kind, TurnExecutionKind::Shell),
        }) {
            Ok(records) => records,
            Err(error) if !matches!(event.kind, TurnExecutionKind::Shell) => {
                let warning = "dir baseline capture failed; write Tools were disabled";
                self.capture_failures
                    .write()
                    .map_err(|_| CoreError::Journal("capture failure lock poisoned".into()))?
                    .insert(event.turn_id.clone(), warning.into());
                if let Ok(records) = self.ledger.mark_turn_incomplete(
                    event.session_id.clone(),
                    event.thread_id.clone(),
                    event.turn_id.clone(),
                    warning.into(),
                ) {
                    self.publish(&records);
                }
                log::error!(
                    "Turn {} dir baseline capture failed; write Tools are disabled: {error}",
                    event.turn_id
                );
                return Ok(());
            }
            Err(error) => return Err(CoreError::Execution(error.to_string())),
        };
        self.publish(&records);
        Ok(())
    }

    fn did_finish(&self, event: &TurnExecutionFinished) {
        if self
            .capture_failures
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&event.turn_id)
            .is_some()
        {
            return;
        }
        let terminal_state = match event.terminal_state {
            TurnExecutionTerminalState::Completed => TerminalTurnState::Completed,
            TurnExecutionTerminalState::Failed => TerminalTurnState::Failed,
            TurnExecutionTerminalState::Interrupted => TerminalTurnState::Interrupted,
        };
        match self.ledger.seal_turn(TurnChangeSealRequest {
            session_id: event.session_id.clone(),
            thread_id: event.thread_id.clone(),
            turn_id: event.turn_id.clone(),
            terminal_state,
        }) {
            Ok(records) => {
                self.publish(&records);
                for record in records {
                    if record.message_state == MessageState::Queued && !record.files.is_empty() {
                        spawn_message_job(
                            Arc::clone(&self.store),
                            Arc::clone(&self.threads),
                            Arc::clone(&self.model),
                            Arc::clone(&self.config),
                            self.dir_id.clone(),
                            Arc::clone(&self.updates),
                            record.change_set_id,
                        );
                    }
                }
            }
            Err(error) => {
                let warning = "dir checkpoint sealing failed; this ChangeSet cannot be committed";
                match self.ledger.mark_turn_incomplete(
                    event.session_id.clone(),
                    event.thread_id.clone(),
                    event.turn_id.clone(),
                    warning.into(),
                ) {
                    Ok(records) => self.publish(&records),
                    Err(mark_error) => log::error!(
                        "failed to retain Turn {} sealing failure: {mark_error}",
                        event.turn_id
                    ),
                }
                log::error!(
                    "failed to seal Turn {} dir checkpoint: {error}",
                    event.turn_id
                );
            }
        }
        if let Err(error) = self.enforce_cleanup_policy() {
            log::warn!("Thread worktree cleanup policy failed: {error}");
        }
    }

    fn tool_will_execute(&self, event: &TurnToolExecutionStarted) -> Result<(), CoreError> {
        let failed = self
            .capture_failures
            .read()
            .map_err(|_| CoreError::Journal("capture failure lock poisoned".into()))?
            .get(&event.turn_id)
            .cloned();
        if let Some(error) = failed
            && event.write_capable
        {
            return Err(CoreError::Execution(format!(
                "dir baseline capture failed; this Turn cannot run a write-capable Tool: {error}"
            )));
        }
        self.tool_write_capabilities
            .write()
            .map_err(|_| CoreError::Journal("Tool capability lock poisoned".into()))?
            .insert(
                (event.turn_id.clone(), event.tool_call_id.clone()),
                event.write_capable,
            );
        if event.write_capable {
            self.begin_write_lifecycle(&event.thread_id, &event.turn_id)?;
        }
        Ok(())
    }

    fn tool_did_finish(&self, event: &TurnToolExecutionFinished) {
        let write_capable = self
            .tool_write_capabilities
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&(event.turn_id.clone(), event.tool_call_id.clone()))
            .unwrap_or(false);
        let roots = match self.execution_roots(&event.thread_id) {
            Ok(roots) if !roots.is_empty() => roots,
            Ok(_) => {
                if write_capable {
                    self.end_write_lifecycle(&event.thread_id, &event.turn_id);
                }
                log::error!(
                    "Tool {} ran without a Thread execution root",
                    event.tool_call_id
                );
                return;
            }
            Err(error) => {
                if write_capable {
                    self.end_write_lifecycle(&event.thread_id, &event.turn_id);
                }
                log::error!(
                    "Tool {} execution roots could not be recovered: {error}",
                    event.tool_call_id
                );
                return;
            }
        };
        let (read_paths, write_paths, mut opaque_dependencies) = tool_paths(self, &roots, event);
        opaque_dependencies |= write_capable && read_paths.is_empty() && write_paths.is_empty();
        let recorded = if opaque_dependencies {
            self.record_opaque_lifecycle(
                &roots,
                &event.session_id,
                &event.thread_id,
                &event.turn_id,
            )
        } else {
            self.ledger
                .record_tool_scope(ToolChangeScope {
                    session_id: event.session_id.clone(),
                    thread_id: event.thread_id.clone(),
                    turn_id: event.turn_id.clone(),
                    read_paths,
                    write_paths,
                    repository_paths: repository_paths(&roots),
                    opaque_dependencies: false,
                })
                .map_err(|error| error.to_string())
                .and_then(|_| {
                    self.ledger
                        .refresh_turn(
                            event.session_id.clone(),
                            event.thread_id.clone(),
                            event.turn_id.clone(),
                        )
                        .map_err(|error| error.to_string())
                })
        };
        match recorded {
            Ok(records) => self.publish(&records),
            Err(error) => log::error!(
                "failed to record Tool {} change scope: {error}",
                event.tool_call_id
            ),
        }
        if event.outcome_unknown {
            match self.ledger.record_ambiguous_write(
                event.session_id.clone(),
                event.thread_id.clone(),
                event.turn_id.clone(),
                format!(
                    "Tool {} ended with an unknown write outcome",
                    event.tool_call_id
                ),
            ) {
                Ok(records) => self.publish(&records),
                Err(error) => log::error!(
                    "failed to record Tool {} unknown outcome: {error}",
                    event.tool_call_id
                ),
            }
        }
        if write_capable {
            self.end_write_lifecycle(&event.thread_id, &event.turn_id);
        }
    }
}

impl HookExecutionObserver for TurnChangesRuntime {
    fn will_execute(&self, event: &HookExecutionEvent) -> Result<(), CoreError> {
        let roots = self
            .execution_roots(&event.thread_id)
            .map_err(CoreError::Journal)?;
        if roots.iter().all(|root| event.dir != root.binding.dir()) {
            return Ok(());
        }
        if self
            .capture_failures
            .read()
            .map_err(|_| CoreError::Journal("capture failure lock poisoned".into()))?
            .contains_key(&event.turn_id)
        {
            return Err(CoreError::Execution(
                "dir baseline capture failed; this Turn cannot run a write-capable Hook".into(),
            ));
        }
        self.begin_write_lifecycle(&event.thread_id, &event.turn_id)?;
        Ok(())
    }

    fn did_finish(&self, event: &HookExecutionEvent) {
        let roots = match self.execution_roots(&event.thread_id) {
            Ok(roots) => roots,
            Err(error) => {
                log::error!(
                    "Hook {} execution roots could not be recovered: {error}",
                    event.hook_id
                );
                self.end_write_lifecycle(&event.thread_id, &event.turn_id);
                return;
            }
        };
        if roots.iter().all(|root| event.dir != root.binding.dir()) {
            self.end_write_lifecycle(&event.thread_id, &event.turn_id);
            return;
        }
        match self.record_opaque_lifecycle(
            &roots,
            &event.session_id,
            &event.thread_id,
            &event.turn_id,
        ) {
            Ok(records) => self.publish(&records),
            Err(error) => log::error!(
                "failed to record Hook {} change scope: {error}",
                event.hook_id
            ),
        }
        self.end_write_lifecycle(&event.thread_id, &event.turn_id);
    }
}

fn tool_paths(
    runtime: &TurnChangesRuntime,
    roots: &[ExecutionRootBinding],
    event: &TurnToolExecutionFinished,
) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>, bool) {
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    let path = |key: &str| {
        event
            .arguments
            .get(key)
            .and_then(serde_json::Value::as_str)
            .and_then(|path| runtime.relative_tool_path(roots, &event.thread_id, path))
    };
    match event.name.as_str() {
        "read_file" => reads.extend(path("path")),
        "write_file" => writes.extend(path("path")),
        "edit" => {
            reads.extend(path("path"));
            writes.extend(path("path"));
        }
        "grep" | "glob" => {
            reads.insert(path("path").unwrap_or_else(|| PathBuf::from(".")));
        }
        "apply_patch" => {
            if let Some(patch) = event
                .arguments
                .get("patch")
                .and_then(serde_json::Value::as_str)
            {
                for (target, reads_existing) in patch_targets(patch) {
                    if let Some(target) =
                        runtime.relative_tool_path(roots, &event.thread_id, &target)
                    {
                        if reads_existing {
                            reads.insert(target.clone());
                        }
                        writes.insert(target);
                    }
                }
            }
        }
        "shell" | "shell-command" | "shell_command" | "exec" => return (reads, writes, true),
        _ => {}
    }
    (reads, writes, false)
}

impl TurnChangesRuntime {
    fn begin_write_lifecycle(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        let mut active = self
            .active_write_lifecycles
            .write()
            .map_err(|_| CoreError::Journal("write lifecycle lock poisoned".into()))?;
        let count = active
            .entry((thread_id.clone(), turn_id.clone()))
            .or_insert(0);
        *count = count
            .checked_add(1)
            .ok_or_else(|| CoreError::Journal("write lifecycle count exhausted".into()))?;
        Ok(())
    }

    fn end_write_lifecycle(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
    ) {
        let mut active = self
            .active_write_lifecycles
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let key = (thread_id.clone(), turn_id.clone());
        let Some(count) = active.get_mut(&key) else {
            return;
        };
        if *count <= 1 {
            active.remove(&key);
        } else {
            *count -= 1;
        }
    }

    fn relative_tool_path(
        &self,
        roots: &[ExecutionRootBinding],
        thread_id: &zeta_protocol::ThreadId,
        raw: &str,
    ) -> Option<PathBuf> {
        let requested = Path::new(raw);
        if !requested.is_absolute() {
            let primary = roots.iter().find(|root| root.primary)?;
            return normalize_relative_path(requested)
                .map(|path| prefixed_path(&execution_root_prefix(primary), &path));
        }
        if let Some(output) = self.execution_output(thread_id)
            && let Ok(relative) = requested.strip_prefix(output)
        {
            return normalize_relative_path(relative)
                .map(|path| prefixed_path(Path::new(".zeta-output"), &path));
        }
        roots
            .iter()
            .filter_map(|root| {
                [
                    root.binding.dir(),
                    root.source.canonical_path(),
                    root.source.requested_path(),
                ]
                .into_iter()
                .filter_map(|base| {
                    requested
                        .strip_prefix(base)
                        .ok()
                        .map(|relative| (root, relative.to_path_buf(), base.components().count()))
                })
                .max_by_key(|(_, _, depth)| *depth)
            })
            .max_by_key(|(_, _, depth)| *depth)
            .and_then(|(root, relative, _)| {
                normalize_relative_path(&relative)
                    .map(|path| prefixed_path(&execution_root_prefix(root), &path))
            })
    }

    fn record_opaque_lifecycle(
        &self,
        roots: &[ExecutionRootBinding],
        session_id: &zeta_protocol::SessionId,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
    ) -> Result<Vec<zeta_turn_changes::TurnChangeSet>, String> {
        let refreshed = self
            .ledger
            .refresh_turn(session_id.clone(), thread_id.clone(), turn_id.clone())
            .map_err(|error| error.to_string())?;
        let repository_prefixes = repository_paths(roots);
        let mut write_paths = BTreeSet::new();
        for record in refreshed {
            let prefix = repository_prefixes
                .get(&record.repository_id)
                .map(PathBuf::as_path)
                .ok_or_else(|| {
                    format!(
                        "execution roots omitted repository {} from the Turn scope",
                        record.repository_id
                    )
                })?;
            for path in record
                .files
                .iter()
                .flat_map(|file| [Some(&file.path), file.previous_path.as_ref()])
                .flatten()
            {
                write_paths.insert(prefixed_path(prefix, path));
            }
        }
        self.ledger
            .record_tool_scope(ToolChangeScope {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                turn_id: turn_id.clone(),
                read_paths: BTreeSet::new(),
                write_paths,
                repository_paths: repository_prefixes,
                opaque_dependencies: true,
            })
            .map_err(|error| error.to_string())
    }
}

fn repository_paths(roots: &[ExecutionRootBinding]) -> std::collections::BTreeMap<String, PathBuf> {
    roots
        .iter()
        .flat_map(|root| {
            root.binding.repositories().iter().map(|repository| {
                (
                    repository.repository_id().to_string(),
                    prefixed_path(&execution_root_prefix(root), repository.relative_path()),
                )
            })
        })
        .collect()
}

fn execution_root_prefix(root: &ExecutionRootBinding) -> PathBuf {
    let Some(provenance) = &root.work_attempt else {
        return PathBuf::from(".");
    };
    let digest = Sha256::digest(provenance.source_root_dir_id.as_str().as_bytes());
    PathBuf::from(".zeta-roots").join(format!("{digest:x}"))
}

fn prefixed_path(prefix: &Path, path: &Path) -> PathBuf {
    if prefix == Path::new(".") {
        path.to_path_buf()
    } else if path == Path::new(".") || path.as_os_str().is_empty() {
        prefix.to_path_buf()
    } else {
        prefix.join(path)
    }
}

fn normalize_relative_path(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => normalized.push(value),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    })
}

fn patch_targets(patch: &str) -> Vec<(String, bool)> {
    let mut targets = Vec::new();
    for line in patch.lines() {
        for (prefix, reads_existing) in [
            ("*** Update File: ", true),
            ("*** Delete File: ", true),
            ("*** Add File: ", false),
            ("*** Move to: ", false),
        ] {
            if let Some(path) = line.strip_prefix(prefix)
                && !path.trim().is_empty()
            {
                targets.push((path.to_owned(), reads_existing));
                break;
            }
        }
    }
    targets
}
