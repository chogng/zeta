use super::*;
use crate::HookRunEvent;
use crate::HookRunStatus;
use crate::process::HookProcessExecutor;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ExecutionDecision;
use zeta_async_utils::CancellationSource;
use zeta_config::HookAction;
use zeta_config::HookConfig;
use zeta_config::HookEnablement;
use zeta_config::HookEvent as ConfigHookEvent;
use zeta_core::AfterToolHookRequest;
use zeta_core::BeforeToolHookRequest;
use zeta_core::HookOutcome;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolCallId;
use zeta_protocol::TurnId;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxPolicy;
use zeta_tool_executor::CommandExecutionAuthority;

struct TestPolicy;

impl zeta_core::ActionPolicyService for TestPolicy {
    fn revision(&self) -> String {
        "hook-test-policy".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Ok(ExecutionDecision::RunSandboxed(SandboxPolicy::new(
            FileSystemAccess::WorkspaceWrite,
            NetworkAccess::Denied,
        )))
    }
}

struct RecordingProcess {
    workspace: WorkspaceRoot,
    executions: Mutex<Vec<String>>,
    calls: AtomicUsize,
    decision: Mutex<crate::outcome::HookDecision>,
}

impl HookProcessExecutor for RecordingProcess {
    fn workspace(&self) -> &WorkspaceRoot {
        &self.workspace
    }

    fn execute(
        &self,
        hook: &HookConfig,
        _: Vec<u8>,
        _: CommandExecutionAuthority,
        _: &CancellationToken,
    ) -> Result<crate::outcome::HookDecision, CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.executions
            .lock()
            .expect("recording process lock")
            .push(hook.id.to_string());
        Ok(self
            .decision
            .lock()
            .expect("recording decision lock")
            .clone())
    }
}

fn hook(
    id: &str,
    event: ConfigHookEvent,
    tool_names: &[&str],
    enablement: HookEnablement,
) -> HookConfig {
    HookConfig {
        id: zeta_config::HookId::new(id).expect("test Hook id"),
        event,
        matcher: zeta_config::HookMatcher {
            tool_names: tool_names.iter().map(|name| (*name).into()).collect(),
        },
        action: HookAction::Process {
            program: "hook-program".into(),
            args: Vec::new(),
        },
        enablement,
    }
}

fn runtime(
    hooks: impl IntoIterator<Item = HookConfig>,
) -> (DeclarativeHookRuntime, Arc<RecordingProcess>) {
    let workspace = test_workspace();
    let process = Arc::new(RecordingProcess {
        workspace,
        executions: Mutex::new(Vec::new()),
        calls: AtomicUsize::new(0),
        decision: Mutex::new(crate::outcome::HookDecision::Continue),
    });
    let config = HooksConfig {
        hooks: hooks
            .into_iter()
            .map(|hook| (hook.id.clone(), hook))
            .collect(),
    };
    (
        DeclarativeHookRuntime::with_process(config, Arc::new(TestPolicy), process.clone()),
        process,
    )
}

fn test_workspace() -> WorkspaceRoot {
    WorkspaceRoot::open(std::env::current_dir().expect("test working directory"))
        .expect("workspace root")
}

fn before_request(tool_name: &str) -> BeforeToolHookRequest {
    BeforeToolHookRequest {
        thread_id: ThreadId::new("thread-test").unwrap(),
        turn_id: TurnId::new("turn-test").unwrap(),
        tool_call_id: ToolCallId::new("tool-test").unwrap(),
        tool_name: tool_name.into(),
    }
}

fn after_request(tool_name: &str, outcome: HookOutcome) -> AfterToolHookRequest {
    AfterToolHookRequest {
        thread_id: ThreadId::new("thread-test").unwrap(),
        turn_id: TurnId::new("turn-test").unwrap(),
        tool_call_id: ToolCallId::new("tool-test").unwrap(),
        tool_name: tool_name.into(),
        outcome,
    }
}

#[test]
fn runtime_matches_events_and_tool_filters_in_stable_order() {
    let (runtime, process) = runtime([
        hook(
            "user:hook:after",
            ConfigHookEvent::AfterTool,
            &["shell-command"],
            HookEnablement::Enabled,
        ),
        hook(
            "user:hook:before-shell",
            ConfigHookEvent::BeforeTool,
            &["shell-command"],
            HookEnablement::Enabled,
        ),
        hook(
            "user:hook:before-all",
            ConfigHookEvent::BeforeTool,
            &[],
            HookEnablement::Enabled,
        ),
        hook(
            "user:hook:disabled",
            ConfigHookEvent::BeforeTool,
            &[],
            HookEnablement::Disabled,
        ),
    ]);
    let source = CancellationSource::new();

    runtime
        .before_tool(&before_request("shell-command"), &source.token())
        .expect("before Hook run");
    runtime
        .after_tool(
            &after_request("file-system", HookOutcome::Succeeded),
            &source.token(),
        )
        .expect("non-matching after Hook run");

    assert_eq!(
        process
            .executions
            .lock()
            .expect("recording process lock")
            .as_slice(),
        ["user:hook:before-all", "user:hook:before-shell"]
    );
}

#[test]
fn runtime_checks_cancellation_between_hooks() {
    let (runtime, process) = runtime([
        hook(
            "user:hook:first",
            ConfigHookEvent::BeforeTool,
            &[],
            HookEnablement::Enabled,
        ),
        hook(
            "user:hook:second",
            ConfigHookEvent::BeforeTool,
            &[],
            HookEnablement::Enabled,
        ),
    ]);
    let source = CancellationSource::new();
    source.cancel();

    let error = runtime
        .before_tool(&before_request("shell-command"), &source.token())
        .expect_err("cancelled Hook run");
    assert!(matches!(error, CoreError::Cancelled(_)));
    assert_eq!(process.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn before_tool_denial_is_typed_and_projected_as_a_terminal_run() {
    let (runtime, process) = runtime([hook(
        "user:hook:guard",
        ConfigHookEvent::BeforeTool,
        &[],
        HookEnablement::Enabled,
    )]);
    *process.decision.lock().expect("recording decision lock") =
        crate::outcome::HookDecision::Deny {
            reason: "blocked by repository policy".into(),
        };

    let decision = runtime
        .before_tool(
            &before_request("shell-command"),
            &CancellationSource::new().token(),
        )
        .expect("typed Hook decision");

    assert_eq!(
        decision,
        BeforeToolHookDecision::Deny {
            reason: "blocked by repository policy".into(),
        }
    );
    let runs = runtime.recent_runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, "hook-run-1");
    assert_eq!(runs[0].hook_id, "user:hook:guard");
    assert_eq!(runs[0].event, HookRunEvent::BeforeTool);
    assert_eq!(
        runs[0].status,
        HookRunStatus::Denied {
            reason: "blocked by repository policy".into(),
        }
    );
}
