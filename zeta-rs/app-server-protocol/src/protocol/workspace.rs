use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;
use zeta_protocol::CommandId;
use zeta_protocol::WorkspaceTrustId;

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

/// One persisted user trust decision projected for the trust-management surface.
///
/// `workspace` remains the authoritative identity. `root` is display metadata and may be absent
/// for decisions written by older clients.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustEntryDto {
    pub workspace: WorkspaceTrustId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub root: Option<PathBuf>,
    pub setting: WorkspaceTrustSettingDto,
}

/// Lists all explicit user trust decisions in the active profile.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustListResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub entries: Vec<WorkspaceTrustEntryDto>,
}

/// Persists one user trust decision without switching the active Workspace.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustSetParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub root: PathBuf,
    pub setting: WorkspaceTrustSettingDto,
}

/// Removes one user trust decision by its opaque Workspace identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustForgetParams {
    pub command_id: CommandId,
    #[schemars(range(min = 0))]
    #[ts(type = "number")]
    pub expected_revision: u64,
    pub workspace: WorkspaceTrustId,
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
