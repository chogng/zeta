use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;
use zeta_protocol::CommandId;

/// User-owned trust setting collected by a product host.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceTrustSettingDto {
    Restricted,
    Trusted,
}

/// Authority used to resolve one client-requested Workspace switch.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum WorkspaceSwitchTrust {
    /// Resolves the canonical root against the durable UserConfig authority.
    UserConfig,
    /// Grants only this App Server runtime a host-configured trust lease.
    HostSession,
    /// Persists the explicit choice before activating the canonical root.
    UserDecision {
        command_id: CommandId,
        #[schemars(range(min = 0))]
        #[ts(type = "number")]
        expected_revision: u64,
        setting: WorkspaceTrustSettingDto,
    },
}

/// Effective trust state committed by the App Server.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum WorkspaceTrustStateDto {
    Restricted,
    Trusted,
}

/// Reads whether one exact canonical Workspace already has a durable user decision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustReadParams {
    pub root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustReadResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub setting: Option<WorkspaceTrustSettingDto>,
}

/// Replaces the active local Workspace hosted by one App Server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitchParams {
    pub root: PathBuf,
    pub trust: WorkspaceSwitchTrust,
}

/// Confirms the canonical Workspace root accepted by the App Server.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitchResult {
    pub root: PathBuf,
    pub trust: WorkspaceTrustStateDto,
}
