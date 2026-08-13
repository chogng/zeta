//! Durable image attachment admission, storage, remote import, and model materialization.

mod error;
mod remote;
mod service;
mod store;

pub use error::AttachmentError;
pub use remote::RemoteImageFetcher;
pub use remote::SafeRemoteImageFetcher;
pub use service::ImageAttachments;
pub use service::image_media_type;
pub use store::FileImageAttachmentStore;
pub use store::ImageAttachmentStore;
pub use store::MemoryImageAttachmentStore;

/// Maximum encoded bytes accepted for one product image attachment.
pub const MAX_IMAGE_ATTACHMENT_BYTES: usize = 16 * 1024 * 1024;

#[cfg(test)]
#[path = "attachment_tests.rs"]
mod tests;
