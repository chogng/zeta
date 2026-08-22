use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;
use zeta_protocol::CommandId;
use zeta_protocol::WorkspaceTrustId;

/// Trust state collected by a product host.
///
/// `Restricted` is a transient/runtime choice and is represented durably by removing the trusted
/// root; it must not create a Restricted entry in the user's allowlist.
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

/// Reads the durable user decision and effective UserConfig state for one exact canonical Workspace.
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
    /// The durable decision, when one exists. Restricted is represented by omission.
    pub setting: Option<WorkspaceTrustSettingDto>,
    /// The effective state used by UserConfig-backed Workspace activation.
    pub state: WorkspaceTrustStateDto,
}

/// One persisted trusted folder projected for the trust-management surface.
///
/// `workspace` remains the authoritative identity. `root` is display metadata and may be absent
/// for decisions written by older clients. Restricted decisions are intentionally omitted from
/// this management projection; an absent entry is the normal Restricted-mode state.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustEntryDto {
    pub workspace: WorkspaceTrustId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub root: Option<PathBuf>,
}

/// Lists the trusted folders and workspace files in the active profile.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTrustListResult {
    #[ts(type = "number")]
    pub revision: u64,
    pub entries: Vec<WorkspaceTrustEntryDto>,
}

/// Updates one user's trusted-root decision without switching the active Workspace.
///
/// `trusted` adds an entry. `restricted` is retained for compatibility and removes the entry.
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
