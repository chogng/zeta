use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether a server-advertised slash command accepts inline composer arguments.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SlashCommandArgumentModeDto {
    None,
    Optional,
}

/// One server-advertised command in the immutable initialization snapshot.
///
/// Clients may merge these commands with local presentation commands. A submitted dynamic command
/// remains ordinary ordered Turn input; the definition only makes its slash syntax discoverable
/// and declares whether inline arguments are accepted.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandDefinition {
    pub name: String,
    pub description: String,
    pub argument_mode: SlashCommandArgumentModeDto,
}
