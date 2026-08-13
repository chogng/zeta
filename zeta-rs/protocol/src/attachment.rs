use crate::ContentDigest;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

/// Canonical encoded media type of one durable image attachment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ImageMediaType {
    Png,
    Jpeg,
    Gif,
    WebP,
}

impl ImageMediaType {
    /// Returns the MIME type used when materializing this attachment for a model provider.
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
        }
    }
}

/// Durable, provider-neutral reference to one immutable validated image representation.
///
/// The digest identifies the exact encoded bytes. Stores must verify every other field when the
/// bytes are admitted and again when an untrusted reference is resolved.
#[derive(Clone, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImageAttachmentRef {
    pub content_digest: ContentDigest,
    pub media_type: ImageMediaType,
    #[ts(type = "number")]
    pub encoded_bytes: u64,
    pub width: u32,
    pub height: u32,
}

