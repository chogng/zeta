use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const MAX_RESOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_READ_CHUNK_BYTES: usize = 262_144;

pub struct ResourceStore {
    next_id: AtomicU64,
    resources: BTreeMap<String, Resource>,
}

pub struct ResourceMetadata {
    pub resource_id: String,
    pub mime_type: String,
    pub size: usize,
    pub sha256: String,
}
pub struct ResourceChunk {
    pub offset: usize,
    pub data: Vec<u8>,
    pub eof: bool,
}

struct Resource {
    owner_connection_id: u64,
    mime_type: String,
    bytes: Vec<u8>,
    sha256: String,
    expires_at: Instant,
}

impl Default for ResourceStore {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            resources: BTreeMap::new(),
        }
    }
}

impl ResourceStore {
    pub fn create(
        &mut self,
        owner_connection_id: u64,
        mime_type: String,
        bytes: Vec<u8>,
        ttl: Duration,
    ) -> Result<ResourceMetadata, ResourceError> {
        if bytes.len() > MAX_RESOURCE_BYTES {
            return Err(ResourceError::TooLarge);
        }
        self.cleanup();
        let resource_id = format!(
            "resource_{:016x}",
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );
        let sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
        let metadata = ResourceMetadata {
            resource_id: resource_id.clone(),
            mime_type: mime_type.clone(),
            size: bytes.len(),
            sha256: sha256.clone(),
        };
        self.resources.insert(
            resource_id,
            Resource {
                owner_connection_id,
                mime_type,
                bytes,
                sha256,
                expires_at: Instant::now() + ttl,
            },
        );
        Ok(metadata)
    }

    pub fn metadata(
        &mut self,
        owner_connection_id: u64,
        resource_id: &str,
    ) -> Result<ResourceMetadata, ResourceError> {
        let resource = self.resource(owner_connection_id, resource_id)?;
        Ok(ResourceMetadata {
            resource_id: resource_id.into(),
            mime_type: resource.mime_type.clone(),
            size: resource.bytes.len(),
            sha256: resource.sha256.clone(),
        })
    }

    pub fn read(
        &mut self,
        owner_connection_id: u64,
        resource_id: &str,
        offset: usize,
        max_bytes: usize,
    ) -> Result<ResourceChunk, ResourceError> {
        if max_bytes == 0 || max_bytes > MAX_READ_CHUNK_BYTES {
            return Err(ResourceError::InvalidChunkSize);
        }
        let resource = self.resource(owner_connection_id, resource_id)?;
        if offset > resource.bytes.len() {
            return Err(ResourceError::InvalidOffset);
        }
        let end = (offset + max_bytes).min(resource.bytes.len());
        Ok(ResourceChunk {
            offset,
            data: resource.bytes[offset..end].to_vec(),
            eof: end == resource.bytes.len(),
        })
    }

    pub fn release(
        &mut self,
        owner_connection_id: u64,
        resource_id: &str,
    ) -> Result<(), ResourceError> {
        self.resource(owner_connection_id, resource_id)?;
        self.resources.remove(resource_id);
        Ok(())
    }

    pub fn release_owner(&mut self, owner_connection_id: u64) {
        self.resources
            .retain(|_, resource| resource.owner_connection_id != owner_connection_id);
    }

    fn resource(
        &mut self,
        owner_connection_id: u64,
        resource_id: &str,
    ) -> Result<&Resource, ResourceError> {
        self.cleanup();
        let resource = self
            .resources
            .get(resource_id)
            .ok_or(ResourceError::NotFound)?;
        if resource.owner_connection_id != owner_connection_id {
            return Err(ResourceError::NotOwner);
        }
        Ok(resource)
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        self.resources
            .retain(|_, resource| resource.expires_at > now);
    }
}

#[derive(Clone, Copy)]
pub enum ResourceError {
    NotFound,
    NotOwner,
    TooLarge,
    InvalidChunkSize,
    InvalidOffset,
}
