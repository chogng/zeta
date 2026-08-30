use serde_json::json;
use std::sync::Condvar;
use std::sync::Mutex;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ModelUsage;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;

pub(crate) struct InducementModel {
    state: Mutex<InducementState>,
    allowed_patch: String,
    forbidden_patch: String,
}

#[derive(Default)]
struct InducementState {
    invocation: usize,
}

impl InducementModel {
    pub(crate) fn new(
        allowed_path: &str,
        allowed_content: &str,
        forbidden_path: &std::path::Path,
        forbidden_content: &str,
    ) -> Self {
        Self {
            state: Mutex::new(InducementState::default()),
            allowed_patch: add_file_patch(allowed_path, allowed_content),
            forbidden_patch: add_file_patch(&forbidden_path.to_string_lossy(), forbidden_content),
        }
    }
}

impl ModelService for InducementModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut state = self.state.lock().unwrap();
        let invocation = state.invocation;
        state.invocation += 1;
        let response = match invocation {
            0 => tool_response("write-allowed", &self.allowed_patch),
            1 => tool_response("write-forbidden", &self.forbidden_patch),
            _ => ModelResponse {
                output: vec![ResponseItem::Text(
                    "The assigned file was written; the out-of-scope request was rejected.".into(),
                )],
                usage: deterministic_usage(),
                stop_reason: StopReason::Completed,
            },
        };
        Ok(response)
    }
}

#[derive(Default)]
pub(crate) struct StaleResponseModel {
    state: Mutex<StaleResponseState>,
    changed: Condvar,
}

#[derive(Default)]
struct StaleResponseState {
    invoked: bool,
    release: bool,
    returned: bool,
    observed_cancellation: bool,
}

impl StaleResponseModel {
    pub(crate) fn wait_until_invoked(&self) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                !state.invoked
            })
            .map_err(lock_error)?;
        if !state.invoked || timeout.timed_out() {
            return Err("scripted model was not invoked before the deadline".into());
        }
        Ok(())
    }

    pub(crate) fn release(&self) -> Result<(), String> {
        self.state.lock().map_err(lock_error)?.release = true;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn wait_until_returned(&self) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                !state.returned
            })
            .map_err(lock_error)?;
        if !state.returned || timeout.timed_out() {
            return Err("scripted stale response did not return before the deadline".into());
        }
        Ok(())
    }

    pub(crate) fn observed_cancellation(&self) -> Result<bool, String> {
        Ok(self.state.lock().map_err(lock_error)?.observed_cancellation)
    }
}

impl ModelService for StaleResponseModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoreError::Model("scripted model lock poisoned".into()))?;
        state.invoked = true;
        self.changed.notify_all();
        while !state.release {
            state = self
                .changed
                .wait(state)
                .map_err(|_| CoreError::Model("scripted model lock poisoned".into()))?;
        }
        state.observed_cancellation = cancellation.is_cancelled();
        state.returned = true;
        self.changed.notify_all();
        Ok(tool_response(
            "stale-write",
            &add_file_patch("stale-write.txt", "stale\n"),
        ))
    }
}

pub(crate) struct ConcurrentStaleResponseModel {
    state: Mutex<ConcurrentStaleResponseState>,
    changed: Condvar,
    patch: String,
}

#[derive(Default)]
struct ConcurrentStaleResponseState {
    invoked: usize,
    release: bool,
    returned: usize,
    observed_cancellations: usize,
}

impl ConcurrentStaleResponseModel {
    pub(crate) fn new(path: &str, content: &str) -> Self {
        Self {
            state: Mutex::new(ConcurrentStaleResponseState::default()),
            changed: Condvar::new(),
            patch: add_file_patch(path, content),
        }
    }

    pub(crate) fn wait_until_invoked(&self, expected: usize) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                state.invoked < expected
            })
            .map_err(lock_error)?;
        if state.invoked < expected || timeout.timed_out() {
            return Err(format!(
                "only {} of {expected} scripted models were invoked before the deadline",
                state.invoked
            ));
        }
        Ok(())
    }

    pub(crate) fn release(&self) -> Result<(), String> {
        self.state.lock().map_err(lock_error)?.release = true;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn wait_until_returned(&self, expected: usize) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                state.returned < expected
            })
            .map_err(lock_error)?;
        if state.returned < expected || timeout.timed_out() {
            return Err(format!(
                "only {} of {expected} stale model responses returned before the deadline",
                state.returned
            ));
        }
        Ok(())
    }

    pub(crate) fn observed_cancellations(&self) -> Result<usize, String> {
        Ok(self
            .state
            .lock()
            .map_err(lock_error)?
            .observed_cancellations)
    }
}

impl ModelService for ConcurrentStaleResponseModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        _: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoreError::Model("scripted model lock poisoned".into()))?;
        state.invoked += 1;
        self.changed.notify_all();
        while !state.release {
            state = self
                .changed
                .wait(state)
                .map_err(|_| CoreError::Model("scripted model lock poisoned".into()))?;
        }
        if cancellation.is_cancelled() {
            state.observed_cancellations += 1;
        }
        state.returned += 1;
        self.changed.notify_all();
        Ok(tool_response("conflict-stale-write", &self.patch))
    }
}

fn tool_response(id: &str, patch: &str) -> ModelResponse {
    ModelResponse {
        output: vec![ResponseItem::ToolCall(ToolCall {
            id: ToolCallId::new(id).unwrap(),
            name: ToolName::new("apply_patch").unwrap(),
            arguments: json!({"patch": patch}),
        })],
        usage: deterministic_usage(),
        stop_reason: StopReason::ToolUse,
    }
}

fn deterministic_usage() -> Option<ModelUsage> {
    Some(ModelUsage {
        input_tokens: Some(100),
        output_tokens: Some(10),
        cached_input_tokens: Some(0),
        reasoning_tokens: Some(0),
    })
}

fn add_file_patch(path: &str, content: &str) -> String {
    let lines = content
        .split_inclusive('\n')
        .map(|line| format!("+{}", line.strip_suffix('\n').unwrap_or(line)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("*** Begin Patch\n*** Add File: {path}\n{lines}\n*** End Patch")
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "scripted model lock poisoned".into()
}
