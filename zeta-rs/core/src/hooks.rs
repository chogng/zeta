use crate::CoreError;
use zeta_async_utils::CancellationToken;

/// Canonical Core safe-point at which a configured Hook may run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookEvent {
    BeforeTool {
        tool_name: String,
    },
    AfterTool {
        tool_name: String,
        outcome: HookOutcome,
    },
    TurnCompleted,
}

/// Terminal outcome supplied to an `afterTool` Hook without exposing raw tool output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookOutcome {
    Succeeded,
    Failed,
}

/// Executes Hook work at Core-owned safe-points.
///
/// Implementations must resolve configuration from an immutable runtime snapshot, preserve Hook
/// identity and event ordering, enforce policy before process start, bound output, and observe the
/// supplied cancellation token. `TurnCompleted` runs after the durable completion commit and is
/// therefore best-effort; a failure there must not reopen or rewrite the completed Turn.
pub trait HookService: Send + Sync {
    fn run(&self, event: &HookEvent, cancellation: &CancellationToken) -> Result<(), CoreError>;
}

/// Default Hook port for hosts that have no configured runtime.
pub struct NoHooks;

impl HookService for NoHooks {
    fn run(&self, _: &HookEvent, cancellation: &CancellationToken) -> Result<(), CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))
    }
}
