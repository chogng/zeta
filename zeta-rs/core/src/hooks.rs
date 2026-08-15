use crate::CoreError;
use zeta_async_utils::CancellationToken;
use zeta_protocol::ThreadId;
use zeta_protocol::ToolCallId;
use zeta_protocol::TurnId;

/// Canonical request evaluated before a Tool crosses its execution boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeforeToolHookRequest {
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
}

/// Decision produced by configured Hooks before Tool execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeforeToolHookDecision {
    Continue,
    Deny { reason: String },
}

/// Canonical request observed after a Tool result has been committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AfterToolHookRequest {
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub outcome: HookOutcome,
}

/// Terminal Tool outcome exposed to an `afterTool` Hook without raw provider output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookOutcome {
    Succeeded,
    Failed,
}

/// Canonical request observed after durable Turn completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnCompletedHookRequest {
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
}

/// Executes Hook work at Core-owned safe-points.
///
/// Implementations must resolve configuration from an immutable runtime snapshot, preserve Hook
/// identity and event ordering, enforce policy before process start, bound input and output, and
/// observe the supplied cancellation token. A `before_tool` denial is returned as a typed decision
/// so Core can persist model-visible Tool feedback without turning it into a failed Turn.
pub trait HookService: Send + Sync {
    fn before_tool(
        &self,
        request: &BeforeToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<BeforeToolHookDecision, CoreError>;

    fn after_tool(
        &self,
        request: &AfterToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError>;

    /// Runs best-effort work after the durable completion commit.
    ///
    /// Callers must not reopen or rewrite the completed Turn when this method fails.
    fn turn_completed(
        &self,
        request: &TurnCompletedHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError>;
}

/// Default Hook port for hosts that have no configured runtime.
pub struct NoHooks;

impl HookService for NoHooks {
    fn before_tool(
        &self,
        _: &BeforeToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<BeforeToolHookDecision, CoreError> {
        check_cancellation(cancellation)?;
        Ok(BeforeToolHookDecision::Continue)
    }

    fn after_tool(
        &self,
        _: &AfterToolHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        check_cancellation(cancellation)
    }

    fn turn_completed(
        &self,
        _: &TurnCompletedHookRequest,
        cancellation: &CancellationToken,
    ) -> Result<(), CoreError> {
        check_cancellation(cancellation)
    }
}

fn check_cancellation(cancellation: &CancellationToken) -> Result<(), CoreError> {
    cancellation
        .check()
        .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))
}
