use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;
use zeta_protocol::ImageAttachmentRef;
use zeta_protocol::ImageDetail;
use zeta_protocol::ImageMediaType;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadStartParams {
    pub media_type: ImageMediaType,
    #[ts(type = "number")]
    pub encoded_bytes: u64,
    pub detail: ImageDetail,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadStartResult {
    pub upload_id: String,
    pub max_chunk_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadWriteParams {
    #[schemars(length(min = 1))]
    pub upload_id: String,
    #[ts(type = "number")]
    pub offset: u64,
    pub data_base64: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadWriteResult {
    #[ts(type = "number")]
    pub next_offset: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadFinishParams {
    #[schemars(length(min = 1))]
    pub upload_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadCancelParams {
    #[schemars(length(min = 1))]
    pub upload_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentImportRemoteParams {
    #[schemars(length(min = 1, max = 8192))]
    pub url: String,
    pub detail: ImageDetail,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMaterializeResult {
    pub attachment: ImageAttachmentRef,
}

