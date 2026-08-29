use crate::CoreError;
use zeta_protocol::{SessionId, ThreadId, ThreadOrigin};

/// Durable Thread identity supplied before Core creates or attaches its Thread event stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreadWorkspaceProvisionRequest {
    pub session_id: SessionId,
    pub thread_id: ThreadId,
    pub origin: ThreadOrigin,
}

/// Host boundary that must persist an isolated workspace binding before Thread execution exists.
pub trait ThreadWorkspaceProvisioner: Send + Sync {
    fn provision(&self, request: &ThreadWorkspaceProvisionRequest) -> Result<(), CoreError>;
}

pub struct NoThreadWorkspaceProvisioner;

impl ThreadWorkspaceProvisioner for NoThreadWorkspaceProvisioner {
    fn provision(&self, _: &ThreadWorkspaceProvisionRequest) -> Result<(), CoreError> {
        Ok(())
    }
}
