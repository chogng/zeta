use crate::MemoryMutationDisposition;
use crate::MemoryScope;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::CommandId;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MemoryReadMode {
    #[default]
    Disabled,
    FirstInvocation,
}

/// Scope-specific consent for automatic reading. Explicit management does not grant this consent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryPolicy {
    pub scope: MemoryScope,
    #[ts(type = "number")]
    pub revision: u64,
    pub automatic_read: MemoryReadMode,
}

impl MemoryPolicy {
    pub fn disabled(scope: MemoryScope) -> Self {
        Self {
            scope,
            revision: 0,
            automatic_read: MemoryReadMode::Disabled,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPolicyMutationResult {
    pub disposition: MemoryMutationDisposition,
    #[ts(type = "number")]
    pub catalog_revision: u64,
    pub policy: MemoryPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UpdateMemoryPolicyRequest {
    pub command_id: CommandId,
    pub scope: MemoryScope,
    pub expected_revision: u64,
    pub automatic_read: MemoryReadMode,
}
