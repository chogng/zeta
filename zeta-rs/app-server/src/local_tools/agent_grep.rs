use super::suite::ResolvedFilePath;
use super::suite::limit_matches;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;
use zeta_async_utils::CancellationToken;
use zeta_config::AgentGrepBackend;
use zeta_core::CoreError;
use zeta_core::ToolExecutionOutput;
use zeta_fast_regex_search::FastRegexCaseSensitivity;
use zeta_fast_regex_search::FastRegexError;
use zeta_fast_regex_search::FastRegexPattern;
use zeta_fast_regex_search::FastRegexQuery;
use zeta_fast_regex_search::FastRegexSearch;
use zeta_fast_regex_search::FastRegexSearchLimits;
use zeta_fast_regex_search::FastRegexSearchResult;
use zeta_fast_regex_search::FastRegexSearchSnapshot;
use zeta_fast_regex_search::FastRegexSearchStorage;
use zeta_fast_regex_search::FastRegexUpdateOutcome;
use zeta_fast_regex_search::FastRegexWorkerClient;
use zeta_fast_regex_search::FastRegexWorkerCommand;
use zeta_file_watcher::FileWatcherEvent;
use zeta_shell_command::RipgrepExecutable;
use zeta_state::StateRuntime;
use zeta_state::WorkspaceIndexKind;
use zeta_state::WorkspaceIndexLease;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustId;

const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_AGENT_MATCHES: usize = 100;
const MAX_RESULT_LINE_CHARS: usize = 500;

/// Selects and owns the implementation used only by the Agent `grep` Tool.
///
/// Workspace Search is implemented by `zeta-workspace-search`; this service has its own Agent grep
/// execution path and may select `zeta-fast-regex-search`.
pub(crate) struct AgentGrepService {
    backend: AgentGrepBackend,
    ripgrep: RipgrepExecutable,
    indexes: Arc<FastRegexIndexes>,
}

struct FastRegexIndexes {
    enabled: AtomicBool,
    storage: Option<Arc<StateRuntime>>,
    worker_command: Option<FastRegexWorkerCommand>,
    indexes: Mutex<BTreeMap<WorkspaceTrustId, Arc<ManagedFastRegexSearch>>>,
}

struct ManagedFastRegexSearch {
    search: FastRegexSearchHandle,
    _lease: Option<WorkspaceIndexLease>,
}

enum FastRegexSearchHandle {
    InProcess(Box<FastRegexSearch>),
    Worker(FastRegexWorkerClient),
}

impl AgentGrepService {
    pub(crate) fn new(
        backend: AgentGrepBackend,
        ripgrep: RipgrepExecutable,
        storage: Option<Arc<StateRuntime>>,
    ) -> Self {
        Self {
            backend,
            ripgrep,
            indexes: Arc::new(FastRegexIndexes {
                enabled: AtomicBool::new(backend == AgentGrepBackend::FastRegex),
                storage,
                worker_command: None,
                indexes: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    pub(crate) fn new_with_worker(
        backend: AgentGrepBackend,
        ripgrep: RipgrepExecutable,
        storage: Arc<StateRuntime>,
        worker_command: FastRegexWorkerCommand,
    ) -> Self {
        Self {
            backend,
            ripgrep,
            indexes: Arc::new(FastRegexIndexes {
                enabled: AtomicBool::new(backend == AgentGrepBackend::FastRegex),
                storage: Some(storage),
                worker_command: Some(worker_command),
                indexes: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    pub(crate) fn reconfigured(
        &self,
        backend: AgentGrepBackend,
        ripgrep: RipgrepExecutable,
    ) -> Self {
        let enabled = backend == AgentGrepBackend::FastRegex;
        let was_enabled = self.indexes.enabled.swap(enabled, Ordering::AcqRel);
        if enabled != was_enabled {
            self.indexes
                .indexes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clear();
        }
        Self {
            backend,
            ripgrep,
            indexes: Arc::clone(&self.indexes),
        }
    }

    pub(super) fn execute(
        &self,
        pattern: String,
        path: &ResolvedFilePath,
        glob: Option<String>,
        case_insensitive: bool,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        match self.backend {
            AgentGrepBackend::Ripgrep => {
                self.execute_ripgrep(pattern, path, glob, case_insensitive, cancellation)
            }
            AgentGrepBackend::FastRegex => {
                self.execute_fast_regex(pattern, path, glob, case_insensitive, cancellation)
            }
        }
    }

    pub(crate) fn apply_watcher_event(&self, root: &WorkspaceRoot, event: &FileWatcherEvent) {
        if !self.indexes.enabled.load(Ordering::Acquire) {
            return;
        }
        let index = self
            .indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&root.trust_id())
            .cloned();
        let Some(index) = index else {
            return;
        };
        let update = match event {
            FileWatcherEvent::PathsChanged { paths } => index.refresh_observed_paths(paths),
            FileWatcherEvent::RescanRequired { .. } => index.reconcile_workspace(),
        };
        if let Err(error) = update {
            log::warn!("fast regex index refresh failed: {error}");
            self.invalidate_index(root);
        }
    }

    pub(crate) fn watches_fast_regex(&self) -> bool {
        self.indexes.enabled.load(Ordering::Acquire)
    }

    pub(crate) fn fast_regex_snapshot(
        &self,
        root: &WorkspaceRoot,
    ) -> Result<Option<FastRegexSearchSnapshot>, FastRegexError> {
        let index = self
            .indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&root.trust_id())
            .cloned();
        index.map(|index| index.snapshot()).transpose()
    }

    pub(crate) fn rebuild_fast_regex(
        &self,
        root: &WorkspaceRoot,
    ) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        if !self.watches_fast_regex() {
            return Err(FastRegexError::NotReady);
        }
        self.index_for(root)?.rebuild()
    }

    #[cfg(test)]
    pub(crate) fn has_active_index(&self, root: &WorkspaceRoot) -> bool {
        self.indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(&root.trust_id())
    }

    pub(crate) fn invalidate_index(&self, root: &WorkspaceRoot) {
        self.indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&root.trust_id());
    }

    fn execute_ripgrep(
        &self,
        pattern: String,
        path: &ResolvedFilePath,
        glob: Option<String>,
        case_insensitive: bool,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        let mut command = Command::new(self.ripgrep.path());
        command.args(["--no-config", "-n", "--no-heading"]);
        if case_insensitive {
            command.arg("-i");
        }
        if let Some(glob) = glob {
            command.args(["--glob", &glob]);
        }
        command.arg("--").arg(pattern).arg(&path.absolute);
        let output = match run_search(command, cancellation) {
            Ok(output) => output,
            Err(SearchError::Cancelled(error)) => return Err(error),
            Err(SearchError::Failed(message)) => return Ok(ToolExecutionOutput::Failure(message)),
        };
        if output.status.code() == Some(1) {
            return Ok(ToolExecutionOutput::Success("no matches".into()));
        }
        if !output.status.success() {
            return Ok(ToolExecutionOutput::Failure(format!(
                "{}\nescape literal characters like . ( ) {{ }} with a backslash",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(ToolExecutionOutput::Success(limit_matches(
            &String::from_utf8_lossy(&output.stdout),
            MAX_RESULT_LINE_CHARS,
        )))
    }

    fn execute_fast_regex(
        &self,
        pattern: String,
        path: &ResolvedFilePath,
        glob: Option<String>,
        case_insensitive: bool,
        cancellation: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let index = match self.index_for(&path.root) {
            Ok(index) => index,
            Err(error) => return Ok(ToolExecutionOutput::Failure(error.to_string())),
        };
        let query = FastRegexQuery {
            query: pattern,
            pattern: FastRegexPattern::Regex,
            case_sensitivity: if case_insensitive {
                FastRegexCaseSensitivity::Insensitive
            } else {
                FastRegexCaseSensitivity::Sensitive
            },
            scope: path.relative.clone(),
            include_patterns: glob.into_iter().collect(),
            exclude_patterns: Vec::new(),
            max_results: MAX_AGENT_MATCHES,
        };
        let result = match index.search(&query) {
            Ok(result) => result,
            Err(FastRegexError::StaleSource(stale)) => {
                let absolute = path.root.canonical_path().join(stale);
                if let Err(error) = index.refresh_observed_paths(&[absolute]) {
                    return Ok(ToolExecutionOutput::Failure(error.to_string()));
                }
                match index.search(&query) {
                    Ok(result) => result,
                    Err(error) => return Ok(ToolExecutionOutput::Failure(error.to_string())),
                }
            }
            Err(error) => return Ok(ToolExecutionOutput::Failure(error.to_string())),
        };
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        if result.matches.is_empty() {
            return Ok(ToolExecutionOutput::Success("no matches".into()));
        }
        let limit_hit = result.limit_hit;
        let output = result
            .matches
            .into_iter()
            .map(|found| {
                format!(
                    "{}:{}:{}",
                    path.root.canonical_path().join(found.path).display(),
                    found.line_number,
                    found.preview
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut output = limit_matches(&output, MAX_RESULT_LINE_CHARS);
        if limit_hit {
            output.push_str("\n[more than 100 matches, showing first 100]");
        }
        Ok(ToolExecutionOutput::Success(output))
    }

    fn index_for(
        &self,
        root: &WorkspaceRoot,
    ) -> Result<Arc<ManagedFastRegexSearch>, FastRegexError> {
        let trust_id = root.trust_id();
        let mut indexes = self
            .indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(index) = indexes.get(&trust_id) {
            return Ok(Arc::clone(index));
        }
        let lease = self
            .indexes
            .storage
            .as_ref()
            .map(|storage| {
                storage
                    .acquire(&trust_id, WorkspaceIndexKind::AgentGrep)
                    .map_err(|source| FastRegexError::Io {
                        path: storage.index_directory(&trust_id, WorkspaceIndexKind::AgentGrep),
                        source,
                    })
            })
            .transpose()?;
        let search_storage = lease
            .as_ref()
            .map_or(FastRegexSearchStorage::Memory, |lease| {
                FastRegexSearchStorage::Persistent(lease.directory().to_path_buf())
            });
        let search =
            match (&self.indexes.worker_command, search_storage) {
                (Some(command), FastRegexSearchStorage::Persistent(storage)) => {
                    FastRegexSearchHandle::Worker(FastRegexWorkerClient::open(
                        command.clone(),
                        root,
                        storage,
                        FastRegexSearchLimits::default(),
                    )?)
                }
                (Some(_), FastRegexSearchStorage::Memory) => {
                    return Err(FastRegexError::Worker(
                        "worker-backed search requires persistent storage".to_owned(),
                    ));
                }
                (None, storage) => FastRegexSearchHandle::InProcess(Box::new(
                    FastRegexSearch::open(root.clone(), storage, FastRegexSearchLimits::default())?,
                )),
            };
        let index = Arc::new(ManagedFastRegexSearch {
            search,
            _lease: lease,
        });
        if index.snapshot()?.generation == 0 {
            index.rebuild()?;
        }
        indexes.insert(trust_id, Arc::clone(&index));
        Ok(index)
    }
}

impl ManagedFastRegexSearch {
    fn snapshot(&self) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        match &self.search {
            FastRegexSearchHandle::InProcess(search) => Ok(search.snapshot()),
            FastRegexSearchHandle::Worker(search) => search.snapshot(),
        }
    }

    fn rebuild(&self) -> Result<FastRegexSearchSnapshot, FastRegexError> {
        match &self.search {
            FastRegexSearchHandle::InProcess(search) => search.rebuild(),
            FastRegexSearchHandle::Worker(search) => search.rebuild(),
        }
    }

    fn refresh_observed_paths(
        &self,
        paths: &[PathBuf],
    ) -> Result<FastRegexUpdateOutcome, FastRegexError> {
        match &self.search {
            FastRegexSearchHandle::InProcess(search) => search.refresh_observed_paths(paths),
            FastRegexSearchHandle::Worker(search) => search.refresh_observed_paths(paths),
        }
    }

    fn reconcile_workspace(&self) -> Result<FastRegexUpdateOutcome, FastRegexError> {
        match &self.search {
            FastRegexSearchHandle::InProcess(search) => search.reconcile_workspace(),
            FastRegexSearchHandle::Worker(search) => search.reconcile_workspace(),
        }
    }

    fn search(&self, query: &FastRegexQuery) -> Result<FastRegexSearchResult, FastRegexError> {
        match &self.search {
            FastRegexSearchHandle::InProcess(search) => search.search(query),
            FastRegexSearchHandle::Worker(search) => search.search(query),
        }
    }
}

enum SearchError {
    Cancelled(CoreError),
    Failed(String),
}

fn run_search(
    mut command: Command,
    cancellation: &CancellationToken,
) -> Result<Output, SearchError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| SearchError::Failed(format!("could not execute search: {error}")))?;
    let started = Instant::now();
    loop {
        cancellation.check().map_err(|signal| {
            SearchError::Cancelled(CoreError::Cancelled(signal.reason().to_string()))
        })?;
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| SearchError::Failed(error.to_string()));
            }
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SearchError::Failed(format!(
                    "could not wait for search: {error}"
                )));
            }
        }
        if started.elapsed() >= SEARCH_TIMEOUT {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .map_err(|error| SearchError::Failed(error.to_string()))?;
            let partial = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(SearchError::Failed(format!(
                "search timed out after 30000 ms. Partial output:\n{partial}"
            )));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
#[path = "agent_grep_tests.rs"]
mod tests;
