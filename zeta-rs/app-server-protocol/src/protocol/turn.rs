use crate::protocol::common::TurnId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::SkillRef;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InputItem {
    Text {
        text: String,
    },
    Context {
        name: String,
        content: String,
    },
    ImageAttachment {
        attachment: ImageAttachmentRef,
    },
    /// Legacy transport form. New clients should use the attachment upload/import methods.
    Image {
        url: String,
    },
    Skill {
        skill: SkillRef,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnStartResult {
    pub turn_id: TurnId,
    #[ts(type = "number")]
    pub sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnSteerResult {
    pub turn_id: TurnId,
    #[ts(type = "number")]
    pub sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnInterruptResult {
    #[ts(type = "number")]
    pub sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnInteractionResolveResult {
    #[ts(type = "number")]
    pub sequence: u64,
}
