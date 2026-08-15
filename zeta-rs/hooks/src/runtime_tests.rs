use super::*;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use zeta_action_policy::ExecutionDecision;
use zeta_async_utils::CancellationSource;

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
}

impl HookProcessExecutor for RecordingProcess {
    fn workspace(&self) -> &WorkspaceRoot {
        &self.workspace
    }

    fn execute(
        &self,
        hook: &HookConfig,
        _: CommandExecutionAuthority,
        _: &CancellationToken,
    ) -> Result<(), CoreError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.executions
            .lock()
            .expect("recording process lock")
            .push(hook.id.to_string());
        Ok(())
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
        .run(
            &HookEvent::BeforeTool {
                tool_name: "shell-command".into(),
            },
            &source.token(),
        )
        .expect("before Hook run");
    runtime
        .run(
            &HookEvent::AfterTool {
                tool_name: "file-system".into(),
                outcome: zeta_core::HookOutcome::Succeeded,
            },
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
        .run(
            &HookEvent::BeforeTool {
                tool_name: "shell-command".into(),
            },
            &source.token(),
        )
        .expect_err("cancelled Hook run");
    assert!(matches!(error, CoreError::Cancelled(_)));
    assert_eq!(process.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn review_authority_is_bound_to_the_exact_hook_identity() {
    let workspace = test_workspace();
    let first = hook(
        "user:hook:first",
        ConfigHookEvent::BeforeTool,
        &[],
        HookEnablement::Enabled,
    );
    let second = hook(
        "user:hook:second",
        ConfigHookEvent::BeforeTool,
        &[],
        HookEnablement::Enabled,
    );

    let first_review = review_request(&first, &workspace, "hook-test-policy".into()).unwrap();
    let second_review = review_request(&second, &workspace, "hook-test-policy".into()).unwrap();
    assert_eq!(first_review.provenance().source_id(), "user:hook:first");
    assert_ne!(
        first_review.action().digest(),
        second_review.action().digest()
    );
}
