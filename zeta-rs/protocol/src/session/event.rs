use crate::AgentContextSeed;
use crate::ModelRef;
use crate::SessionId;
use crate::SessionThread;
use crate::ThreadId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A durable structural fact in one product Session's event stream.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionEvent {
    SessionCreated {
        session_id: SessionId,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        model: Option<ModelRef>,
    },
    SessionModelChanged {
        session_id: SessionId,
        model: ModelRef,
    },
    ThreadCreationPlanned {
        session_id: SessionId,
        thread: SessionThread,
        title: String,
    },
    AgentThreadCreationPlanned {
        session_id: SessionId,
        thread: SessionThread,
        title: String,
        context_seed: Box<AgentContextSeed>,
    },
    ThreadAttached {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    ThreadArchived {
        session_id: SessionId,
        thread_id: ThreadId,
    },
    SessionCompleted {
        session_id: SessionId,
    },
    SessionArchived {
        session_id: SessionId,
    },
}

impl SessionEvent {
    pub fn session_id(&self) -> &SessionId {
        match self {
            Self::SessionCreated { session_id, .. }
            | Self::SessionModelChanged { session_id, .. }
            | Self::ThreadCreationPlanned { session_id, .. }
            | Self::AgentThreadCreationPlanned { session_id, .. }
            | Self::ThreadAttached { session_id, .. }
            | Self::ThreadArchived { session_id, .. }
            | Self::SessionCompleted { session_id }
            | Self::SessionArchived { session_id } => session_id,
        }
    }
}
