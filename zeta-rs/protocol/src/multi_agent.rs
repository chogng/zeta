use crate::AgentJoinId;
use crate::AgentMessageId;
use crate::ContentDigest;
use crate::ContextCheckpointId;
use crate::DelegationId;
use crate::FrozenSkillActivation;
use crate::ImageAttachmentRef;
use crate::ItemId;
use crate::ModelRef;
use crate::ThreadId;
use crate::ToolName;
use crate::TurnId;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use ts_rs::TS;

/// Immutable task text captured when a parent delegates work to a child Agent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedTask {
    pub title: String,
    pub instructions: String,
}

/// Frozen Agent role instructions selected before a child Thread is created.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleSnapshot {
    pub name: String,
    pub instructions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub model: Option<ModelRef>,
}

/// One immutable source selected from a fixed sequence of another Thread.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentContextSource {
    Item {
        source_thread_id: ThreadId,
        #[ts(type = "number")]
        source_sequence: u64,
        item_id: ItemId,
    },
    Checkpoint {
        source_thread_id: ThreadId,
        #[ts(type = "number")]
        source_sequence: u64,
        checkpoint_id: ContextCheckpointId,
    },
}

/// Selection policy for inheriting a fixed prefix of parent Thread history.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ForkedAgentContext {
    Full,
    LastTurns { count: u32 },
    CheckpointAndTail,
}

/// Explicit context inheritance selected for one child Agent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentContextMode {
    Fresh,
    Selected { sources: Vec<AgentContextSource> },
    ForkedPrefix { selection: ForkedAgentContext },
}

/// Provider-independent content copied from one immutable source into a child seed.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentContextContent {
    UserText {
        text: String,
    },
    UserImage {
        url: String,
    },
    UserImageAttachment {
        attachment: ImageAttachmentRef,
    },
    AssistantText {
        text: String,
    },
    Reasoning {
        text: String,
    },
    Plan {
        text: String,
    },
    ToolCall {
        name: ToolName,
        arguments_json: String,
    },
    ToolResult {
        text: String,
        is_error: bool,
    },
    Checkpoint {
        summary: String,
    },
}

/// One verified source value materialized before the child Thread is allowed to run.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentMaterializedContext {
    pub source: AgentContextSource,
    pub content: AgentContextContent,
    pub content_digest: ContentDigest,
}

/// Policy revision ceiling delegated by a parent to a child Agent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedPolicyCeiling {
    pub policy_revision: String,
}

/// Frozen upper bound on the tools and Skill instructions visible to a child Agent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedCapabilityScope {
    pub tools: Vec<ToolName>,
    pub skills: Vec<FrozenSkillActivation>,
}

macro_rules! sha256_digest {
    ($name:ident, $error:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, JsonSchema, PartialEq, TS)]
        #[schemars(transparent)]
        #[ts(type = "string")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, $error> {
                let value = value.into();
                let valid = value.strip_prefix("sha256:").is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                });
                if !valid {
                    return Err($error);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $error;

        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!($label, " must be a lowercase sha256 digest"))
            }
        }

        impl std::error::Error for $error {}
    };
}

sha256_digest!(
    ContextSeedDigest,
    InvalidContextSeedDigest,
    "context seed digest"
);
sha256_digest!(
    DelegationResultDigest,
    InvalidDelegationResultDigest,
    "delegation result digest"
);

/// Durable, verifiable input captured before the first child Turn starts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentContextSeed {
    pub delegation_id: DelegationId,
    pub parent_thread_id: ThreadId,
    pub parent_turn_id: TurnId,
    #[ts(type = "number")]
    pub parent_sequence: u64,
    pub task: DelegatedTask,
    pub role: AgentRoleSnapshot,
    pub inheritance: AgentContextMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialized_context: Vec<AgentMaterializedContext>,
    pub policy_ceiling: DelegatedPolicyCeiling,
    pub capability_scope: DelegatedCapabilityScope,
    pub digest: ContextSeedDigest,
}

/// Inclusive durable child Thread sequence range supporting a delegation result.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSequenceRange {
    #[ts(type = "number")]
    pub start_sequence: u64,
    #[ts(type = "number")]
    pub end_sequence: u64,
}

/// Stable terminal outcome for a child Agent delegation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DelegationResultStatus {
    Completed,
    Failed,
    Cancelled,
    PolicyDenied,
    CapacityRejected,
    ContextSeedInvalid,
    DeliveryFailed,
    UnknownOutcome,
}

/// Artifact reference returned by a child without embedding artifact bytes in Thread history.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DelegationArtifactRef {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub content_digest: Option<ContentDigest>,
}

/// Bounded terminal result delivered from one child Thread to its parent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DelegationResult {
    pub delegation_id: DelegationId,
    pub child_thread_id: ThreadId,
    pub status: DelegationResultStatus,
    pub summary: String,
    pub artifacts: Vec<DelegationArtifactRef>,
    pub source_range: ThreadSequenceRange,
    pub digest: DelegationResultDigest,
}

/// Provenance class for one durable cross-Thread Agent message.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AgentMessageProvenance {
    Agent,
    User,
    System,
}

/// Bounded payload sent between independently ordered Agent Threads.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentMessageContent {
    Instruction { text: String },
    Result { result: DelegationResult },
}

/// Durable cross-Thread message with stable delivery and provenance identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub message_id: AgentMessageId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub delegation_id: Option<DelegationId>,
    pub sender_thread_id: ThreadId,
    pub receiver_thread_id: ThreadId,
    #[ts(type = "number")]
    pub sender_sequence: u64,
    pub content: AgentMessageContent,
    pub provenance: AgentMessageProvenance,
}

/// Durable join condition evaluated from delegation terminal facts.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentJoinPolicy {
    All,
    Any,
    Quorum { count: u32 },
    Explicit { delegations: Vec<DelegationId> },
}

/// Durable lifecycle of one parent-side Agent join.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AgentJoinStatus {
    Waiting,
    Satisfied,
}

/// Frozen target set and current durable outcome for one Agent join.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentJoin {
    pub join_id: AgentJoinId,
    pub parent_thread_id: ThreadId,
    pub policy: AgentJoinPolicy,
    pub delegations: Vec<DelegationId>,
    pub status: AgentJoinStatus,
    pub satisfied_by: Vec<DelegationId>,
}
