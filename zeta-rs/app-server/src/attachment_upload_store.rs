use std::collections::BTreeMap;

use zeta_protocol::ImageDetail;
use zeta_protocol::ImageMediaType;

pub const MAX_ATTACHMENT_UPLOAD_CHUNK_BYTES: usize = 192 * 1024;

#[derive(Default)]
pub struct AttachmentUploadStore {
    next_id: u64,
    uploads: BTreeMap<String, AttachmentUpload>,
}

pub struct CompletedAttachmentUpload {
    pub media_type: ImageMediaType,
    pub detail: ImageDetail,
    pub bytes: Vec<u8>,
}

struct AttachmentUpload {
    owner_connection_id: u64,
    media_type: ImageMediaType,
    detail: ImageDetail,
    expected_bytes: usize,
    bytes: Vec<u8>,
}

impl AttachmentUploadStore {
    pub fn start(
        &mut self,
        owner_connection_id: u64,
        media_type: ImageMediaType,
        detail: ImageDetail,
        expected_bytes: usize,
    ) -> Result<String, AttachmentUploadError> {
        if expected_bytes == 0 || expected_bytes > zeta_attachments::MAX_IMAGE_ATTACHMENT_BYTES {
            return Err(AttachmentUploadError::InvalidSize);
        }
        self.next_id = self.next_id.saturating_add(1);
        let upload_id = format!("attachment-upload_{:016x}", self.next_id);
        self.uploads.insert(
            upload_id.clone(),
            AttachmentUpload {
                owner_connection_id,
                media_type,
                detail,
                expected_bytes,
                bytes: Vec::with_capacity(expected_bytes),
            },
        );
        Ok(upload_id)
    }

    pub fn write(
        &mut self,
        owner_connection_id: u64,
        upload_id: &str,
        offset: usize,
        chunk: &[u8],
    ) -> Result<usize, AttachmentUploadError> {
        let upload = self.upload(owner_connection_id, upload_id)?;
        if chunk.is_empty()
            || chunk.len() > MAX_ATTACHMENT_UPLOAD_CHUNK_BYTES
            || offset != upload.bytes.len()
            || upload.bytes.len().saturating_add(chunk.len()) > upload.expected_bytes
        {
            return Err(AttachmentUploadError::InvalidChunk);
        }
        upload.bytes.extend_from_slice(chunk);
        Ok(upload.bytes.len())
    }

    pub fn finish(
        &mut self,
        owner_connection_id: u64,
        upload_id: &str,
    ) -> Result<CompletedAttachmentUpload, AttachmentUploadError> {
        self.upload(owner_connection_id, upload_id)?;
        let upload = self
            .uploads
            .remove(upload_id)
            .expect("an upload validated immediately before removal exists");
        if upload.bytes.len() != upload.expected_bytes {
            return Err(AttachmentUploadError::Incomplete);
        }
        Ok(CompletedAttachmentUpload {
            media_type: upload.media_type,
            detail: upload.detail,
            bytes: upload.bytes,
        })
    }

    pub fn cancel(
        &mut self,
        owner_connection_id: u64,
        upload_id: &str,
    ) -> Result<(), AttachmentUploadError> {
        self.upload(owner_connection_id, upload_id)?;
        self.uploads.remove(upload_id);
        Ok(())
    }

    pub fn release_owner(&mut self, owner_connection_id: u64) {
        self.uploads
            .retain(|_, upload| upload.owner_connection_id != owner_connection_id);
    }

    fn upload(
        &mut self,
        owner_connection_id: u64,
        upload_id: &str,
    ) -> Result<&mut AttachmentUpload, AttachmentUploadError> {
        let upload = self
            .uploads
            .get_mut(upload_id)
            .ok_or(AttachmentUploadError::NotFound)?;
        if upload.owner_connection_id != owner_connection_id {
            return Err(AttachmentUploadError::NotOwner);
        }
        Ok(upload)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachmentUploadError {
    NotFound,
    NotOwner,
    InvalidSize,
    InvalidChunk,
    Incomplete,
}

#[cfg(test)]
#[path = "attachment_upload_store_tests.rs"]
mod tests;
