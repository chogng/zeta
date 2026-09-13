//! App Server exposure of the shared collaboration room contract.
//!
//! Room DTOs are owned by `ash-collaboration` so the process-local App
//! Server and the durable remote host use one transport-neutral vocabulary.

pub use ash_collaboration::DocumentCollaborationOpenParams;
pub use ash_collaboration::DocumentCollaborationOpenResult;
pub use ash_collaboration::DocumentCollaborationPresence;
pub use ash_collaboration::DocumentCollaborationPresenceParams;
pub use ash_collaboration::DocumentCollaborationPresenceReadParams;
pub use ash_collaboration::DocumentCollaborationPresenceSnapshot;
pub use ash_collaboration::DocumentCollaborationSnapshot;
pub use ash_collaboration::DocumentCollaborationSubmitParams;
pub use ash_collaboration::DocumentCollaborationSubmitResult;
pub use ash_collaboration::DocumentCollaborationUpdate;
