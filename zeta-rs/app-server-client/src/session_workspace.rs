use std::path::Path;

use zeta_protocol::Session;
use zeta_protocol::WorkspaceBinding;
use zeta_workspace::WorkspacePathError;
use zeta_workspace::WorkspaceRoot;

/// Host action required before a product may mutate or execute one durable Session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionWorkspaceRoute {
    /// The accepting App Server connection already owns the Session's Workspace authority.
    Current,
    /// The product host must reconnect to the recorded Workspace before resuming the Session.
    Reconnect(WorkspaceBinding),
    /// The Session predates durable Workspace binding and remains readable but not executable.
    LegacyUnbound,
}

/// Resolves a Session against the canonical Workspace authority selected by a product host.
///
/// Products must act on [`SessionWorkspaceRoute::Reconnect`] at their host boundary instead of
/// attempting the Session mutation on the current connection. Opening a Workspace here performs
/// identity comparison only and does not grant trust or execution authority.
pub fn route_session_workspace(
    session: &Session,
    current_workspace_root: impl AsRef<Path>,
) -> Result<SessionWorkspaceRoute, WorkspacePathError> {
    let Some(binding) = session.workspace.as_ref() else {
        return Ok(SessionWorkspaceRoute::LegacyUnbound);
    };
    let current = WorkspaceRoot::open(current_workspace_root)?;
    Ok(if binding.matches_root(&current) {
        SessionWorkspaceRoute::Current
    } else {
        SessionWorkspaceRoute::Reconnect(binding.clone())
    })
}

#[cfg(test)]
#[path = "session_workspace_tests.rs"]
mod tests;
