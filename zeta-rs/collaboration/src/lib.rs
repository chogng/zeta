//! Shared, backend-owned authority for ordered structured-document collaboration rooms.
//!
//! [`InMemoryDocumentCollaborationRooms`] is the App Server's local-process
//! implementation. [`SqliteDocumentCollaborationRooms`] is the durable room
//! authority used by a remote collaboration host. Both enforce the exact same
//! room, version, bounded-history, and replay semantics.

mod in_memory;
mod room;
mod sqlite;
mod types;

pub use in_memory::InMemoryDocumentCollaborationRooms;
pub use room::DocumentCollaborationReplay;
pub use sqlite::SqliteDocumentCollaborationRooms;
pub use types::DocumentCollaborationAuditEvent;
pub use types::DocumentCollaborationInvite;
pub use types::DocumentCollaborationMember;
pub use types::DocumentCollaborationOpenParams;
pub use types::DocumentCollaborationOpenResult;
pub use types::DocumentCollaborationPresence;
pub use types::DocumentCollaborationPresenceParams;
pub use types::DocumentCollaborationPresenceReadParams;
pub use types::DocumentCollaborationPresenceReplay;
pub use types::DocumentCollaborationPresenceSnapshot;
pub use types::DocumentCollaborationPrincipal;
pub use types::DocumentCollaborationRoomRole;
pub use types::DocumentCollaborationSnapshot;
pub use types::DocumentCollaborationSubmitParams;
pub use types::DocumentCollaborationSubmitResult;
pub use types::DocumentCollaborationUpdate;

#[cfg(test)]
#[path = "in_memory_tests.rs"]
mod in_memory_tests;

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod sqlite_tests;
