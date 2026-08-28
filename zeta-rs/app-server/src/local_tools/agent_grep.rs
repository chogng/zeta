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
use zeta_fast_regex_search::FastRegexSearchStorage;
use zeta_file_watcher::FileWatcherEvent;
use zeta_shell_command::RipgrepExecutable;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustId;

const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_AGENT_MATCHES: usize = 100;
const MAX_RESULT_LINE_CHARS: usize = 500;

/// Selects and owns the implementation used only by the Agent `grep` Tool.
///
/// Workspace Search does not depend on this service and always executes its frozen ripgrep
/// command through `zeta-search`.
pub(crate) struct AgentGrepService {
    backend: AgentGrepBackend,
    ripgrep: RipgrepExecutable,
    indexes: Arc<FastRegexIndexes>,
}

struct FastRegexIndexes {
    enabled: AtomicBool,
    storage_root: Option<PathBuf>,
    indexes: Mutex<BTreeMap<WorkspaceTrustId, Arc<FastRegexSearch>>>,
}

impl AgentGrepService {
    pub(crate) fn new(
        backend: AgentGrepBackend,
        ripgrep: RipgrepExecutable,
        storage_root: Option<PathBuf>,
    ) -> Self {
        Self {
            backend,
            ripgrep,
            indexes: Arc::new(FastRegexIndexes {
                enabled: AtomicBool::new(backend == AgentGrepBackend::FastRegex),
                storage_root,
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
        if enabled && !was_enabled {
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
            FileWatcherEvent::RescanRequired { .. } => index
                .rebuild()
                .map(zeta_fast_regex_search::FastRegexUpdateOutcome::Rebuilt),
        };
        if let Err(error) = update {
            log::warn!("fast regex index refresh failed: {error}");
            self.invalidate_index(root);
        }
    }

    pub(crate) fn watches_fast_regex(&self) -> bool {
        self.indexes.enabled.load(Ordering::Acquire)
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

    fn index_for(&self, root: &WorkspaceRoot) -> Result<Arc<FastRegexSearch>, FastRegexError> {
        let trust_id = root.trust_id();
        let mut indexes = self
            .indexes
            .indexes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(index) = indexes.get(&trust_id) {
            return Ok(Arc::clone(index));
        }
        let storage = self.indexes.storage_root.as_ref().map_or(
            FastRegexSearchStorage::Memory,
            |storage_root| {
                FastRegexSearchStorage::Persistent(
                    storage_root.join(trust_id.to_string().replace(':', "-")),
                )
            },
        );
        let index = Arc::new(FastRegexSearch::open(
            root.clone(),
            storage,
            FastRegexSearchLimits::default(),
        )?);
        if index.snapshot().generation == 0 {
            index.rebuild()?;
        }
        indexes.insert(trust_id, Arc::clone(&index));
        Ok(index)
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
