use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One server-owned shell profile available to interactive terminal clients.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalProfile {
    pub profile_id: String,
    pub title: String,
    pub is_default: bool,
}

/// Trusted terminal profiles discovered by the local App Server composition.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalProfileListResult {
    pub profiles: Vec<TerminalProfile>,
}

/// Selects either the server default or one previously listed trusted profile.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "type", deny_unknown_fields)]
pub enum TerminalProfileSelection {
    Default,
    Profile {
        #[serde(rename = "profileId")]
        #[ts(rename = "profileId")]
        profile_id: String,
    },
}

/// Starts one connection-owned interactive terminal at the server's trusted workspace root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalCreateParams {
    #[schemars(range(min = 1, max = 512))]
    pub rows: u16,
    #[schemars(range(min = 1, max = 512))]
    pub cols: u16,
    pub profile: TerminalProfileSelection,
}

/// Identity allocated for one interactive terminal.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreateResult {
    pub terminal_id: String,
    pub profile: TerminalProfile,
}

/// Writes one bounded UTF-8 input batch to an interactive terminal.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalWriteParams {
    #[schemars(length(min = 1))]
    pub terminal_id: String,
    #[schemars(length(min = 1, max = 65536))]
    pub data: String,
}

/// Changes the PTY character-cell dimensions.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalResizeParams {
    #[schemars(length(min = 1))]
    pub terminal_id: String,
    #[schemars(range(min = 1, max = 512))]
    pub rows: u16,
    #[schemars(range(min = 1, max = 512))]
    pub cols: u16,
}

/// Reads output after the last sequence observed by this client.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalReadParams {
    #[schemars(length(min = 1))]
    pub terminal_id: String,
    #[ts(type = "number")]
    pub after_sequence: u64,
    #[ts(type = "number")]
    pub after_command_sequence: u64,
    #[schemars(range(min = 1, max = 128))]
    pub max_chunks: usize,
}

/// One ordered raw PTY output chunk encoded for JSON transport.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputChunk {
    #[ts(type = "number")]
    pub sequence: u64,
    pub data_base64: String,
}

/// Renderer-independent lifecycle state for one shell command.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalCommandStatus {
    Running,
    Completed,
    Succeeded,
    Failed,
    Canceled,
}

/// One ordered command lifecycle transition associated with PTY output.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCommandStatusEvent {
    #[ts(type = "number")]
    pub sequence: u64,
    pub command_id: String,
    pub status: TerminalCommandStatus,
    pub exit_code: Option<i32>,
    #[ts(type = "number")]
    pub after_output_sequence: u64,
}

/// Bounded output and process state for one interactive terminal.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalReadResult {
    pub terminal_id: String,
    pub chunks: Vec<TerminalOutputChunk>,
    #[ts(type = "number")]
    pub next_sequence: u64,
    pub output_gap: bool,
    pub command_events: Vec<TerminalCommandStatusEvent>,
    #[ts(type = "number")]
    pub next_command_sequence: u64,
    pub command_event_gap: bool,
    pub exited: bool,
    pub exit_code: Option<i32>,
}

/// Terminates and releases one connection-owned terminal.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerminalCloseParams {
    #[schemars(length(min = 1))]
    pub terminal_id: String,
}
