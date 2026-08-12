use std::collections::BTreeSet;
use std::path::Component;
use std::sync::Arc;
use std::sync::Mutex;

use zeta_code_index::CodeIndex;
use zeta_code_index::CodeIndexManifest;
use zeta_code_index::IndexedChunkReference;

use crate::CloudCodeIndexCapabilities;
use crate::CloudCodeIndexDeletionSupport;
use crate::CloudCodeIndexError;
use crate::CloudCodeIndexGrant;
use crate::CloudCodeIndexLimitDisposition;
use crate::CloudCodeIndexPreview;
use crate::CloudCodeIndexProvider;
use crate::CloudCodeIndexProviderRegistry;
use crate::CloudCodeIndexPublicationRequest;
use crate::CloudCodeIndexQuery;
use crate::CloudCodeIndexQueryRequest;
use crate::CloudCodeIndexQueryResult;
use crate::CloudCodeIndexState;
use crate::CloudCodeIndexStatus;
use crate::CloudCodeIndexStorage;
use crate::CodeIndexDeploymentMode;
use crate::store::CloudStateStore;
use crate::store::DurableCloudState;

/// Coordinates explicit cloud grants with one local, revision-authoritative code index.
pub struct CloudCodeIndexController {
    index: Arc<CodeIndex>,
    providers: CloudCodeIndexProviderRegistry,
    operation: Mutex<()>,
    store: CloudStateStore,
}

impl CloudCodeIndexController {
    pub fn open(
        index: Arc<CodeIndex>,
        providers: CloudCodeIndexProviderRegistry,
        storage: CloudCodeIndexStorage,
    ) -> Result<Arc<Self>, CloudCodeIndexError> {
        let store = CloudStateStore::open(&storage, index.root_id().as_str())?;
        let controller = Arc::new(Self {
            index,
            providers,
            operation: Mutex::new(()),
            store,
        });
        controller.recover_interrupted_sync()?;
        Ok(controller)
    }

    pub fn preview(
        &self,
        selection: &crate::CloudCodeIndexSelection,
        max_egress_bytes: std::num::NonZeroU64,
    ) -> Result<CloudCodeIndexPreview, CloudCodeIndexError> {
        preview_manifest(&self.index.manifest()?, selection, max_egress_bytes)
    }

    pub fn root_id(&self) -> &zeta_code_index::IndexRootId {
        self.index.root_id()
    }

    pub fn authorize(
        &self,
        grant: CloudCodeIndexGrant,
    ) -> Result<CloudCodeIndexStatus, CloudCodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if grant.root_id != self.index.root_id().as_str() {
            return Err(CloudCodeIndexError::StorageRootMismatch);
        }
        let provider = self.provider_for(&grant)?;
        validate_capabilities(provider.capabilities())?;
        let preview = self.preview(&grant.selection, grant.max_egress_bytes)?;
        if preview.limit == CloudCodeIndexLimitDisposition::ExceedsLimit {
            return Err(CloudCodeIndexError::EgressLimitExceeded);
        }
        let current = self.store.load()?;
        if let Some(existing) = &current.grant {
            if existing == &grant && current.phase != CloudCodeIndexState::Revoking {
                return self.status();
            }
            return Err(CloudCodeIndexError::ConsentConflict);
        }
        self.store.save(&DurableCloudState {
            phase: CloudCodeIndexState::Granted,
            grant: Some(grant),
            synced_local_generation: None,
            remote_generation: None,
        })?;
        self.status()
    }

    pub fn sync(&self) -> Result<CloudCodeIndexStatus, CloudCodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut durable = self.store.load()?;
        if matches!(
            durable.phase,
            CloudCodeIndexState::LocalOnly
                | CloudCodeIndexState::Revoking
                | CloudCodeIndexState::Syncing
        ) {
            return Err(if durable.grant.is_none() {
                CloudCodeIndexError::NoActiveGrant
            } else {
                CloudCodeIndexError::InvalidState
            });
        }
        let grant = durable
            .grant
            .clone()
            .ok_or(CloudCodeIndexError::NoActiveGrant)?;
        let provider = self.provider_for(&grant)?;
        validate_capabilities(provider.capabilities())?;
        let manifest = self.index.manifest()?;
        let preview = preview_manifest(&manifest, &grant.selection, grant.max_egress_bytes)?;
        if preview.limit == CloudCodeIndexLimitDisposition::ExceedsLimit {
            return Err(CloudCodeIndexError::EgressLimitExceeded);
        }
        durable.phase = CloudCodeIndexState::Syncing;
        self.store.save(&durable)?;
        let publication = (|| -> Result<_, CloudCodeIndexError> {
            let chunks = selected_chunks(&manifest, &grant);
            let chunks = self.index.materialize_chunks(&chunks)?;
            Ok(provider.publish(CloudCodeIndexPublicationRequest {
                grant: grant.clone(),
                local_generation: manifest.snapshot.generation,
                chunks,
            })?)
        })();
        match publication {
            Ok(publication) => {
                if let Err(error) = validate_remote_generation(&publication.remote_generation) {
                    durable.phase = if durable.remote_generation.is_some() {
                        CloudCodeIndexState::Stale
                    } else {
                        CloudCodeIndexState::Failed
                    };
                    self.store.save(&durable)?;
                    return Err(error);
                }
                durable.phase = CloudCodeIndexState::Ready;
                durable.synced_local_generation = Some(manifest.snapshot.generation);
                durable.remote_generation = Some(publication.remote_generation);
                self.store.save(&durable)?;
                self.status()
            }
            Err(error) => {
                durable.phase = if durable.remote_generation.is_some() {
                    CloudCodeIndexState::Stale
                } else {
                    CloudCodeIndexState::Failed
                };
                self.store.save(&durable)?;
                Err(error)
            }
        }
    }

    pub fn revoke(&self) -> Result<CloudCodeIndexStatus, CloudCodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut durable = self.store.load()?;
        let Some(grant) = durable.grant.clone() else {
            return self.status();
        };
        durable.phase = CloudCodeIndexState::Revoking;
        self.store.save(&durable)?;
        let provider = self.provider_for(&grant)?;
        validate_deletion(provider.capabilities())?;
        provider.delete_grant(&grant)?;
        self.store.save(&DurableCloudState::default())?;
        self.status()
    }

    /// Queries the exact ready remote generation and rejects candidates outside its grant.
    pub fn query(
        &self,
        query: &CloudCodeIndexQuery,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexError> {
        let _operation = self
            .operation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let durable = self.store.load()?;
        let grant = durable
            .grant
            .clone()
            .ok_or(CloudCodeIndexError::NoActiveGrant)?;
        let manifest = self.index.manifest()?;
        let local_generation = manifest.snapshot.generation;
        if durable.phase != CloudCodeIndexState::Ready
            || durable.synced_local_generation != Some(local_generation)
        {
            return Err(CloudCodeIndexError::InvalidState);
        }
        let remote_generation = durable
            .remote_generation
            .clone()
            .ok_or(CloudCodeIndexError::InvalidState)?;
        let provider = self.provider_for(&grant)?;
        validate_capabilities(provider.capabilities())?;
        let result = provider.query(CloudCodeIndexQueryRequest {
            grant: grant.clone(),
            remote_generation: remote_generation.clone(),
            query: query.clone(),
        })?;
        validate_query_result(&grant, &remote_generation, query, &manifest, &result)?;
        Ok(result)
    }

    pub fn status(&self) -> Result<CloudCodeIndexStatus, CloudCodeIndexError> {
        let durable = self.store.load()?;
        let local_generation = self.index.snapshot()?.generation;
        let state = if durable.phase == CloudCodeIndexState::Ready
            && durable.synced_local_generation != Some(local_generation)
        {
            CloudCodeIndexState::Stale
        } else {
            durable.phase
        };
        let deployment_mode = durable
            .grant
            .as_ref()
            .map_or(CodeIndexDeploymentMode::LocalOnly, |_| {
                CodeIndexDeploymentMode::Cloud
            });
        Ok(CloudCodeIndexStatus {
            deployment_mode,
            state,
            grant: durable.grant,
            local_generation,
            synced_local_generation: durable.synced_local_generation,
            remote_generation: durable.remote_generation,
        })
    }

    fn provider_for(
        &self,
        grant: &CloudCodeIndexGrant,
    ) -> Result<Arc<dyn CloudCodeIndexProvider>, CloudCodeIndexError> {
        self.providers
            .get(&grant.destination.provider)
            .ok_or(CloudCodeIndexError::ProviderUnavailable)
    }

    fn recover_interrupted_sync(&self) -> Result<(), CloudCodeIndexError> {
        let mut durable = self.store.load()?;
        if durable.phase == CloudCodeIndexState::Syncing {
            durable.phase = if durable.remote_generation.is_some() {
                CloudCodeIndexState::Stale
            } else {
                CloudCodeIndexState::Failed
            };
            self.store.save(&durable)?;
        }
        Ok(())
    }
}

fn preview_manifest(
    manifest: &CodeIndexManifest,
    selection: &crate::CloudCodeIndexSelection,
    max_egress_bytes: std::num::NonZeroU64,
) -> Result<CloudCodeIndexPreview, CloudCodeIndexError> {
    if manifest.snapshot.generation == 0 {
        return Err(CloudCodeIndexError::LocalIndexNotReady);
    }
    let chunks = manifest
        .chunks
        .iter()
        .filter(|chunk| selection.includes(&chunk.reference.relative_path))
        .collect::<Vec<_>>();
    let egress_bytes = chunks.iter().fold(0u64, |total, chunk| {
        total.saturating_add(
            u64::try_from(
                chunk
                    .reference
                    .span
                    .end_byte
                    .saturating_sub(chunk.reference.span.start_byte),
            )
            .unwrap_or(u64::MAX),
        )
    });
    let file_count = chunks
        .iter()
        .map(|chunk| &chunk.reference.relative_path)
        .collect::<BTreeSet<_>>()
        .len();
    Ok(CloudCodeIndexPreview {
        local_generation: manifest.snapshot.generation,
        file_count,
        chunk_count: chunks.len(),
        upload_unit_count: chunks.len(),
        egress_bytes,
        limit: if egress_bytes <= max_egress_bytes.get() {
            CloudCodeIndexLimitDisposition::WithinLimit
        } else {
            CloudCodeIndexLimitDisposition::ExceedsLimit
        },
    })
}

fn selected_chunks(
    manifest: &CodeIndexManifest,
    grant: &CloudCodeIndexGrant,
) -> Vec<IndexedChunkReference> {
    manifest
        .chunks
        .iter()
        .filter(|chunk| grant.selection.includes(&chunk.reference.relative_path))
        .cloned()
        .collect()
}

fn validate_capabilities(
    capabilities: CloudCodeIndexCapabilities,
) -> Result<(), CloudCodeIndexError> {
    validate_deletion(capabilities)
}

fn validate_deletion(capabilities: CloudCodeIndexCapabilities) -> Result<(), CloudCodeIndexError> {
    if capabilities.deletion == CloudCodeIndexDeletionSupport::IdempotentGrantDeletion {
        Ok(())
    } else {
        Err(CloudCodeIndexError::DeletionUnsupported)
    }
}

fn validate_remote_generation(value: &str) -> Result<(), CloudCodeIndexError> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        Err(CloudCodeIndexError::InvalidInput(
            "remote generation must be bounded non-control text",
        ))
    } else {
        Ok(())
    }
}

fn validate_query_result(
    grant: &CloudCodeIndexGrant,
    expected_generation: &str,
    query: &CloudCodeIndexQuery,
    manifest: &CodeIndexManifest,
    result: &CloudCodeIndexQueryResult,
) -> Result<(), CloudCodeIndexError> {
    if result.remote_generation != expected_generation {
        return Err(CloudCodeIndexError::InvalidProviderResult(
            "remote generation does not match the ready publication",
        ));
    }
    if result.candidates.len() > query.result_limit().get() {
        return Err(CloudCodeIndexError::InvalidProviderResult(
            "candidate count exceeds the requested result limit",
        ));
    }
    for candidate in &result.candidates {
        let reference = &candidate.reference;
        if reference.span.start_byte >= reference.span.end_byte
            || reference.span.start_line >= reference.span.end_line_exclusive
        {
            return Err(CloudCodeIndexError::InvalidProviderResult(
                "candidate span must cover non-empty source content",
            ));
        }
        if reference.root_id.as_str() != grant.root_id {
            return Err(CloudCodeIndexError::InvalidProviderResult(
                "candidate belongs to another workspace root",
            ));
        }
        if reference.relative_path.is_absolute()
            || reference.relative_path.as_os_str().is_empty()
            || reference
                .relative_path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            || !grant.selection.includes(&reference.relative_path)
        {
            return Err(CloudCodeIndexError::InvalidProviderResult(
                "candidate path falls outside the granted source selection",
            ));
        }
        if !manifest
            .chunks
            .iter()
            .any(|chunk| chunk.reference == *reference)
        {
            return Err(CloudCodeIndexError::InvalidProviderResult(
                "candidate is not an exact chunk from the published Workspace generation",
            ));
        }
    }
    Ok(())
}
