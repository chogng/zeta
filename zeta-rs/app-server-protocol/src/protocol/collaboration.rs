//! App Server exposure of the shared collaboration room contract.
//!
//! Room DTOs are owned by `zeta-collaboration` so the process-local App
//! Server and the durable remote host use one transport-neutral vocabulary.

pub use zeta_collaboration::DocumentCollaborationOpenParams;
pub use zeta_collaboration::DocumentCollaborationOpenResult;
pub use zeta_collaboration::DocumentCollaborationSnapshot;
pub use zeta_collaboration::DocumentCollaborationSubmitParams;
pub use zeta_collaboration::DocumentCollaborationSubmitResult;
pub use zeta_collaboration::DocumentCollaborationUpdate;
