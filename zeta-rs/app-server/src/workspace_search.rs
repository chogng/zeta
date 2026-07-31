use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use zeta_app_server_protocol::protocol::search::{
    WorkspaceSearchCaseSensitivity, WorkspaceSearchMatch, WorkspaceSearchMatchRange,
    WorkspaceSearchPatternKind, WorkspaceSearchReadParams, WorkspaceSearchReadResult,
    WorkspaceSearchStartParams,
};
use zeta_async_utils::CancellationSource;
use zeta_sandboxing::WorkspaceRoot;
use zeta_shell_command::RipgrepExecutable;

const MAX_ACTIVE_SEARCHES: usize = 32;
const SEARCH_RETENTION: Duration = Duration::from_secs(300);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_ERROR_BYTES: u64 = 64 * 1024;
const MAX_GLOB_BYTES: usize = 1024;

/// Owns bounded background content searches rooted at one trusted workspace.
///
/// Callers identify their connection when starting, reading, or cancelling a
/// job. Implementations must never expose results to a different connection.
pub(crate) struct WorkspaceSearchService {
    workspace: RwLock<WorkspaceRoot>,
    ripgrep: RipgrepExecutable,
    next_search_id: AtomicU64,
    jobs: Mutex<HashMap<String, SearchJob>>,
}

impl WorkspaceSearchService {
    pub(crate) fn new(workspace: WorkspaceRoot, ripgrep: RipgrepExecutable) -> Self {
        Self {
            workspace: RwLock::new(workspace),
            ripgrep,
            next_search_id: AtomicU64::new(1),
            jobs: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn switch_workspace(&self, workspace: WorkspaceRoot) {
        *self
            .workspace
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = workspace;
    }

    pub(crate) fn start(
        &self,
        owner_connection_id: u64,
        params: WorkspaceSearchStartParams,
    ) -> Result<String, WorkspaceSearchError> {
        validate_start(&params)?;
        let mut jobs = self.jobs.lock().map_err(|_| WorkspaceSearchError::Busy)?;
        cleanup_jobs(&mut jobs);
        if jobs.len() >= MAX_ACTIVE_SEARCHES {
            return Err(WorkspaceSearchError::Busy);
        }

        let search_id = format!(
            "search-{:x}",
            self.next_search_id.fetch_add(1, Ordering::Relaxed)
        );
        let cancellation = CancellationSource::new();
        let state = Arc::new(Mutex::new(SearchJobState::default()));
        jobs.insert(
            search_id.clone(),
            SearchJob {
                owner_connection_id,
                cancellation: cancellation.clone(),
                state: state.clone(),
                created_at: Instant::now(),
            },
        );
        let workspace = self
            .workspace
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let ripgrep = self.ripgrep.clone();
        thread::spawn(move || run_search(workspace, ripgrep, params, cancellation, state));
        Ok(search_id)
    }

    pub(crate) fn read(
        &self,
        owner_connection_id: u64,
        params: WorkspaceSearchReadParams,
    ) -> Result<WorkspaceSearchReadResult, WorkspaceSearchError> {
        if params.max_matches == 0 || params.max_matches > 200 {
            return Err(WorkspaceSearchError::InvalidInput);
        }
        let mut jobs = self.jobs.lock().map_err(|_| WorkspaceSearchError::Busy)?;
        cleanup_jobs(&mut jobs);
        let job = jobs
            .get(&params.search_id)
            .ok_or(WorkspaceSearchError::NotFound)?;
        if job.owner_connection_id != owner_connection_id {
            return Err(WorkspaceSearchError::NotOwner);
        }
        let state = job.state.lock().map_err(|_| WorkspaceSearchError::Busy)?;
        if params.after_match > state.matches.len() {
            return Err(WorkspaceSearchError::InvalidInput);
        }
        let end = params
            .after_match
            .saturating_add(params.max_matches)
            .min(state.matches.len());
        Ok(WorkspaceSearchReadResult {
            search_id: params.search_id,
            matches: state.matches[params.after_match..end].to_vec(),
            next_match: end,
            completed: state.completed,
            limit_hit: state.limit_hit,
            error: state.error.clone(),
        })
    }

    pub(crate) fn cancel(
        &self,
        owner_connection_id: u64,
        search_id: &str,
    ) -> Result<(), WorkspaceSearchError> {
        let mut jobs = self.jobs.lock().map_err(|_| WorkspaceSearchError::Busy)?;
        cleanup_jobs(&mut jobs);
        let job = jobs.get(search_id).ok_or(WorkspaceSearchError::NotFound)?;
        if job.owner_connection_id != owner_connection_id {
            return Err(WorkspaceSearchError::NotOwner);
        }
        let job = jobs
            .remove(search_id)
            .expect("search job existed immediately before removal");
        job.cancellation.cancel();
        Ok(())
    }
}

struct SearchJob {
    owner_connection_id: u64,
    cancellation: CancellationSource,
    state: Arc<Mutex<SearchJobState>>,
    created_at: Instant,
}

impl SearchJob {
    fn is_expired(&self, now: Instant) -> bool {
        let completed = self.state.lock().map_or(true, |state| state.completed);
        completed && now.duration_since(self.created_at) >= SEARCH_RETENTION
    }
}

#[derive(Default)]
struct SearchJobState {
    matches: Vec<WorkspaceSearchMatch>,
    completed: bool,
    limit_hit: bool,
    error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceSearchError {
    InvalidInput,
    NotFound,
    NotOwner,
    Busy,
}

fn cleanup_jobs(jobs: &mut HashMap<String, SearchJob>) {
    let now = Instant::now();
    jobs.retain(|_, job| !job.is_expired(now));
}

fn validate_start(params: &WorkspaceSearchStartParams) -> Result<(), WorkspaceSearchError> {
    if params.query.is_empty()
        || params.query.len() > 16_384
        || params.query.contains('\0')
        || params.max_results == 0
        || params.max_results > 5_000
        || params.include_patterns.len() > 64
        || params.exclude_patterns.len() > 64
    {
        return Err(WorkspaceSearchError::InvalidInput);
    }
    params
        .include_patterns
        .iter()
        .chain(&params.exclude_patterns)
        .try_for_each(|pattern| validate_glob(pattern))
}

fn validate_glob(pattern: &str) -> Result<(), WorkspaceSearchError> {
    let normalized = pattern.replace('\\', "/");
    if pattern.is_empty()
        || pattern.len() > MAX_GLOB_BYTES
        || pattern.contains('\0')
        || pattern.starts_with('!')
        || normalized.starts_with('/')
        || normalized.split('/').any(|component| component == "..")
        || normalized
            .get(1..3)
            .is_some_and(|prefix| prefix.starts_with(":/"))
    {
        return Err(WorkspaceSearchError::InvalidInput);
    }
    Ok(())
}

fn run_search(
    workspace: WorkspaceRoot,
    ripgrep: RipgrepExecutable,
    params: WorkspaceSearchStartParams,
    cancellation: CancellationSource,
    state: Arc<Mutex<SearchJobState>>,
) {
    let mut command = Command::new(ripgrep.path());
    command
        .current_dir(workspace.path())
        .args(search_arguments(&params))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            complete_with_error(&state, "Unable to start workspace search.");
            return;
        }
    };
    let stdout = child
        .stdout
        .take()
        .expect("workspace search stdout was piped");
    let stderr = child
        .stderr
        .take()
        .expect("workspace search stderr was piped");
    let parser_state = state.clone();
    let parser_cancellation = cancellation.clone();
    let max_results = params.max_results;
    let stdout_reader = thread::spawn(move || {
        parse_stdout(stdout, max_results, &parser_cancellation, &parser_state)
    });
    let stderr_reader = thread::spawn(move || read_bounded_error(stderr));
    let token = cancellation.token();
    let mut cancelled = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(_) => break None,
        }
        if token.is_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break child.wait().ok();
        }
        thread::sleep(POLL_INTERVAL);
    };
    let parser_result = stdout_reader
        .join()
        .unwrap_or_else(|_| Err("Workspace search parser stopped unexpectedly.".into()));
    let _stderr = stderr_reader.join().unwrap_or_default();
    if let Err(message) = parser_result {
        set_error(&state, message);
    } else if !cancelled && !status.is_some_and(|status| matches!(status.code(), Some(0 | 1))) {
        set_error(&state, "Workspace search failed.".into());
    }
    if let Ok(mut state) = state.lock() {
        state.completed = true;
    }
}

fn search_arguments(params: &WorkspaceSearchStartParams) -> Vec<String> {
    let mut arguments = vec![
        "--no-config".into(),
        "--json".into(),
        "--line-number".into(),
        "--color=never".into(),
        "--no-messages".into(),
        "--max-columns=1000".into(),
        "--max-columns-preview".into(),
        "--max-filesize=16M".into(),
    ];
    match params.pattern_kind {
        WorkspaceSearchPatternKind::Literal => arguments.push("--fixed-strings".into()),
        WorkspaceSearchPatternKind::Regex => {}
    }
    arguments.push(
        match params.case_sensitivity {
            WorkspaceSearchCaseSensitivity::Smart => "--smart-case",
            WorkspaceSearchCaseSensitivity::Sensitive => "--case-sensitive",
            WorkspaceSearchCaseSensitivity::Insensitive => "--ignore-case",
        }
        .into(),
    );
    for pattern in &params.include_patterns {
        arguments.extend(["-g".into(), pattern.clone()]);
    }
    for pattern in &params.exclude_patterns {
        arguments.extend(["-g".into(), format!("!{pattern}")]);
    }
    arguments.extend(["--".into(), params.query.clone(), ".".into()]);
    arguments
}

fn parse_stdout(
    stdout: impl Read,
    max_results: usize,
    cancellation: &CancellationSource,
    state: &Arc<Mutex<SearchJobState>>,
) -> Result<(), String> {
    for line in BufReader::new(stdout).lines() {
        if cancellation.token().is_cancelled() {
            return Ok(());
        }
        let line = line.map_err(|_| "Unable to read workspace search output.".to_owned())?;
        let Some(search_match) = parse_match(&line)? else {
            continue;
        };
        let mut job = state
            .lock()
            .map_err(|_| "Workspace search result state is unavailable.".to_owned())?;
        if job.matches.len() == max_results {
            job.limit_hit = true;
            drop(job);
            cancellation.cancel();
            return Ok(());
        }
        job.matches.push(search_match);
    }
    Ok(())
}

fn parse_match(line: &str) -> Result<Option<WorkspaceSearchMatch>, String> {
    let value: Value = serde_json::from_str(line)
        .map_err(|_| "Workspace search returned invalid JSON.".to_owned())?;
    if value.get("type").and_then(Value::as_str) != Some("match") {
        return Ok(None);
    }
    let data = value
        .get("data")
        .ok_or_else(|| "Workspace search match omitted its data.".to_owned())?;
    let path = data
        .pointer("/path/text")
        .and_then(Value::as_str)
        .ok_or_else(|| "Workspace search returned a non-UTF-8 path.".to_owned())?
        .trim_start_matches("./")
        .replace('\\', "/");
    let line_number = data
        .get("line_number")
        .and_then(Value::as_u64)
        .and_then(|line| usize::try_from(line).ok())
        .ok_or_else(|| "Workspace search match omitted its line number.".to_owned())?;
    let preview = data
        .pointer("/lines/text")
        .and_then(Value::as_str)
        .ok_or_else(|| "Workspace search returned a non-UTF-8 preview.".to_owned())?
        .trim_end_matches(['\r', '\n'])
        .to_owned();
    let ranges = data
        .get("submatches")
        .and_then(Value::as_array)
        .ok_or_else(|| "Workspace search match omitted its ranges.".to_owned())?
        .iter()
        .filter_map(|range| {
            let start = usize::try_from(range.get("start")?.as_u64()?).ok()?;
            let end = usize::try_from(range.get("end")?.as_u64()?).ok()?;
            Some(WorkspaceSearchMatchRange {
                start: byte_offset_to_utf16(&preview, start)?,
                end: byte_offset_to_utf16(&preview, end)?,
            })
        })
        .collect::<Vec<_>>();
    if ranges.is_empty() {
        return Ok(None);
    }
    Ok(Some(WorkspaceSearchMatch {
        path: Path::new(&path).to_path_buf(),
        line_number,
        preview,
        ranges,
    }))
}

fn byte_offset_to_utf16(text: &str, offset: usize) -> Option<usize> {
    text.is_char_boundary(offset)
        .then(|| text[..offset].encode_utf16().count())
}

fn read_bounded_error(mut stderr: impl Read) -> String {
    let mut bytes = Vec::new();
    let _ = stderr
        .by_ref()
        .take(MAX_ERROR_BYTES)
        .read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn complete_with_error(state: &Arc<Mutex<SearchJobState>>, message: &str) {
    if let Ok(mut state) = state.lock() {
        state.error = Some(message.into());
        state.completed = true;
    }
}

fn set_error(state: &Arc<Mutex<SearchJobState>>, message: String) {
    if let Ok(mut state) = state.lock()
        && state.error.is_none()
    {
        state.error = Some(message);
    }
}

#[cfg(test)]
#[path = "workspace_search_tests.rs"]
mod tests;
