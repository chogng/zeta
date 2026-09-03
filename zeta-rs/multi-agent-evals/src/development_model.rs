use serde_json::json;
use std::sync::Condvar;
use std::sync::Mutex;
use zeta_async_utils::CancellationToken;
use zeta_core::CoreError;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_protocol::ContentPart;
use zeta_protocol::InputItem;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ModelUsage;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolCallId;
use zeta_protocol::ToolName;

pub(crate) const SINGLE_LOOP_MARKER: &str = "EVAL_SINGLE_DEVELOPMENT_LOOP";
pub(crate) const TEAM_LOOP_ROOT_MARKER: &str = "EVAL_TEAM_DEVELOPMENT_LOOP_ROOT";
pub(crate) const MULTI_SESSION_ALPHA_MARKER: &str = "EVAL_MULTI_SESSION_ALPHA";
pub(crate) const MULTI_SESSION_BETA_MARKER: &str = "EVAL_MULTI_SESSION_BETA";

const TEAM_ALPHA_MARKER: &str = "EVAL_TEAM_CHILD_ALPHA";
const TEAM_BETA_MARKER: &str = "EVAL_TEAM_CHILD_BETA";
const TEAM_PLAN: &str = "team-loop-plan";
const TEAM_SPAWN_ALPHA: &str = "team-loop-spawn-alpha";
const TEAM_SPAWN_BETA: &str = "team-loop-spawn-beta";
const TEAM_SEND_ALPHA: &str = "team-loop-send-alpha";
const TEAM_SEND_BETA: &str = "team-loop-send-beta";
const TEAM_WAIT_ALL: &str = "team-loop-wait-all";

#[derive(Default)]
pub(crate) struct DevelopmentLoopModel {
    state: Mutex<DevelopmentLoopState>,
    changed: Condvar,
}

#[derive(Default)]
struct DevelopmentLoopState {
    team_plan_observed: bool,
    release_team_plan: bool,
    invoked_team_children: usize,
    release_team_children: bool,
}

impl DevelopmentLoopModel {
    pub(crate) fn wait_for_team_plan(&self) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                !state.team_plan_observed
            })
            .map_err(lock_error)?;
        if !state.team_plan_observed || timeout.timed_out() {
            return Err("Team root did not reach the plan admission boundary".into());
        }
        Ok(())
    }

    pub(crate) fn release_team_plan(&self) -> Result<(), String> {
        self.state.lock().map_err(lock_error)?.release_team_plan = true;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn wait_for_team_children(&self, expected: usize) -> Result<(), String> {
        let state = self.state.lock().map_err(lock_error)?;
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, std::time::Duration::from_secs(10), |state| {
                state.invoked_team_children < expected
            })
            .map_err(lock_error)?;
        if state.invoked_team_children < expected || timeout.timed_out() {
            return Err(format!(
                "only {} of {expected} Team children reached their model loop",
                state.invoked_team_children
            ));
        }
        Ok(())
    }

    pub(crate) fn release_team_children(&self) -> Result<(), String> {
        self.state.lock().map_err(lock_error)?.release_team_children = true;
        self.changed.notify_all();
        Ok(())
    }

    fn wait_for_team_admission(&self, cancellation: &CancellationToken) -> Result<(), CoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoreError::Model("development model lock poisoned".into()))?;
        state.team_plan_observed = true;
        self.changed.notify_all();
        while !state.release_team_plan {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            state = self
                .changed
                .wait_timeout(state, std::time::Duration::from_millis(100))
                .map_err(|_| CoreError::Model("development model lock poisoned".into()))?
                .0;
        }
        Ok(())
    }

    fn wait_for_team_binding(&self, cancellation: &CancellationToken) -> Result<(), CoreError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoreError::Model("development model lock poisoned".into()))?;
        state.invoked_team_children += 1;
        self.changed.notify_all();
        while !state.release_team_children {
            cancellation
                .check()
                .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
            state = self
                .changed
                .wait_timeout(state, std::time::Duration::from_millis(100))
                .map_err(|_| CoreError::Model("development model lock poisoned".into()))?
                .0;
        }
        Ok(())
    }

    fn team_root_response(
        &self,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        if !request_has_tool_call(request, TEAM_PLAN) {
            return Ok(tool_response(ToolCall {
                id: ToolCallId::new(TEAM_PLAN).unwrap(),
                name: ToolName::new("update_plan").unwrap(),
                arguments: json!({
                    "explanation": "Freeze the two child boundaries before either writer starts.",
                    "plan": [
                        {
                            "step": "Admit exact alpha.txt and beta.txt child scopes",
                            "status": "in_progress"
                        },
                        {
                            "step": "Dispatch and steer both child Agents",
                            "status": "pending"
                        },
                        {
                            "step": "Join, verify, and integrate exact results",
                            "status": "pending"
                        }
                    ]
                }),
            }));
        }
        if !request_has_tool_call(request, TEAM_SPAWN_ALPHA) {
            self.wait_for_team_admission(cancellation)?;
        }
        Ok(team_root_response(request))
    }
}

impl ModelService for DevelopmentLoopModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        if request_contains(request, TEAM_LOOP_ROOT_MARKER) {
            return self.team_root_response(request, cancellation);
        }
        if request_contains(request, TEAM_ALPHA_MARKER) {
            if !request_has_tool_call(request, "team-alpha-write") {
                self.wait_for_team_binding(cancellation)?;
                return Ok(patch_response("team-alpha-write", "alpha.txt", "alpha\n"));
            }
            return Ok(completed("alpha Team child completed its exact file"));
        }
        if request_contains(request, TEAM_BETA_MARKER) {
            if !request_has_tool_call(request, "team-beta-write") {
                self.wait_for_team_binding(cancellation)?;
                return Ok(patch_response("team-beta-write", "beta.txt", "beta\n"));
            }
            return Ok(completed("beta Team child completed its exact file"));
        }
        if request_contains(request, SINGLE_LOOP_MARKER) {
            return Ok(single_response(request));
        }
        if request_contains(request, MULTI_SESSION_ALPHA_MARKER) {
            return Ok(one_file_response(
                request,
                "multi-alpha-write",
                "alpha.txt",
                "alpha\n",
            ));
        }
        if request_contains(request, MULTI_SESSION_BETA_MARKER) {
            return Ok(one_file_response(
                request,
                "multi-beta-write",
                "beta.txt",
                "beta\n",
            ));
        }
        Err(CoreError::Model(
            "development model received an unknown evaluation role".into(),
        ))
    }
}

fn team_root_response(request: &ModelRequest) -> ModelResponse {
    if !request_has_tool_call(request, TEAM_SPAWN_ALPHA) {
        return tool_response(ToolCall {
            id: ToolCallId::new(TEAM_SPAWN_ALPHA).unwrap(),
            name: ToolName::new("spawn_agent").unwrap(),
            arguments: json!({
                "task": format!("[{TEAM_ALPHA_MARKER}] Create alpha.txt with exactly alpha followed by one newline."),
                "name": "alpha-worker",
                "agent": null,
                "context": null
            }),
        });
    }
    if !request_has_tool_call(request, TEAM_SPAWN_BETA) {
        return tool_response(ToolCall {
            id: ToolCallId::new(TEAM_SPAWN_BETA).unwrap(),
            name: ToolName::new("spawn_agent").unwrap(),
            arguments: json!({
                "task": format!("[{TEAM_BETA_MARKER}] Create beta.txt with exactly beta followed by one newline."),
                "name": "beta-worker",
                "agent": null,
                "context": null
            }),
        });
    }
    if !request_has_tool_call(request, TEAM_SEND_ALPHA) {
        return tool_response(ToolCall {
            id: ToolCallId::new(TEAM_SEND_ALPHA).unwrap(),
            name: ToolName::new("send_agent_message").unwrap(),
            arguments: json!({
                "delegation_id": format!("tool:{TEAM_SPAWN_ALPHA}"),
                "message": "Stay inside the frozen alpha.txt WorkAttempt and report only durable evidence."
            }),
        });
    }
    if !request_has_tool_call(request, TEAM_SEND_BETA) {
        return tool_response(ToolCall {
            id: ToolCallId::new(TEAM_SEND_BETA).unwrap(),
            name: ToolName::new("send_agent_message").unwrap(),
            arguments: json!({
                "delegation_id": format!("tool:{TEAM_SPAWN_BETA}"),
                "message": "Stay inside the frozen beta.txt WorkAttempt and report only durable evidence."
            }),
        });
    }
    if !request_has_tool_call(request, TEAM_WAIT_ALL) {
        return tool_response(ToolCall {
            id: ToolCallId::new(TEAM_WAIT_ALL).unwrap(),
            name: ToolName::new("wait_agent").unwrap(),
            arguments: json!({
                "delegation_id": null,
                "delegation_ids": [
                    format!("tool:{TEAM_SPAWN_ALPHA}"),
                    format!("tool:{TEAM_SPAWN_BETA}")
                ],
                "policy": "all",
                "quorum": null,
                "timeout_ms": 30_000
            }),
        });
    }
    completed("Team root joined both exact delegated results")
}

fn single_response(request: &ModelRequest) -> ModelResponse {
    if !request_has_tool_call(request, "single-alpha-write") {
        patch_response("single-alpha-write", "alpha.txt", "alpha\n")
    } else if !request_has_tool_call(request, "single-beta-write") {
        patch_response("single-beta-write", "beta.txt", "beta\n")
    } else {
        completed("single Agent completed both exact files")
    }
}

fn one_file_response(
    request: &ModelRequest,
    call_id: &str,
    path: &str,
    content: &str,
) -> ModelResponse {
    if request_has_tool_call(request, call_id) {
        completed("independent Session Agent completed its exact file")
    } else {
        patch_response(call_id, path, content)
    }
}

fn request_contains(request: &ModelRequest, needle: &str) -> bool {
    request.input.iter().any(|item| match item {
        InputItem::Message(message) => message
            .content
            .iter()
            .any(|part| matches!(part, ContentPart::Text(text) if text.contains(needle))),
        InputItem::ToolResult(result) => result
            .content
            .iter()
            .any(|part| matches!(part, ContentPart::Text(text) if text.contains(needle))),
    })
}

fn request_has_tool_call(request: &ModelRequest, call_id: &str) -> bool {
    request.input.iter().any(|item| match item {
        InputItem::Message(message) => message
            .tool_calls
            .iter()
            .any(|call| call.id.as_str() == call_id),
        InputItem::ToolResult(result) => result.call_id.as_str() == call_id,
    })
}

fn patch_response(call_id: &str, path: &str, content: &str) -> ModelResponse {
    tool_response(ToolCall {
        id: ToolCallId::new(call_id).unwrap(),
        name: ToolName::new("apply_patch").unwrap(),
        arguments: json!({"patch": add_file_patch(path, content)}),
    })
}

fn tool_response(call: ToolCall) -> ModelResponse {
    ModelResponse {
        output: vec![ResponseItem::ToolCall(call)],
        usage: deterministic_usage(),
        billing: None,
        stop_reason: StopReason::ToolUse,
    }
}

fn completed(message: &str) -> ModelResponse {
    ModelResponse {
        output: vec![ResponseItem::Text(message.into())],
        usage: deterministic_usage(),
        billing: None,
        stop_reason: StopReason::Completed,
    }
}

fn deterministic_usage() -> Option<ModelUsage> {
    Some(ModelUsage {
        input_tokens: Some(100),
        output_tokens: Some(10),
        cached_input_tokens: Some(0),
        cache_write_input_tokens: Some(0),
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
    "development model lock poisoned".into()
}
