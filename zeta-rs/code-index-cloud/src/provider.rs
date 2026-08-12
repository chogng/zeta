use std::collections::BTreeMap;
use std::sync::Arc;

use crate::CloudCodeIndexCapabilities;
use crate::CloudCodeIndexError;
use crate::CloudCodeIndexGrant;
use crate::CloudCodeIndexProviderError;
use crate::CloudCodeIndexProviderId;
use crate::CloudCodeIndexPublication;
use crate::CloudCodeIndexPublicationRequest;
use crate::CloudCodeIndexQueryRequest;
use crate::CloudCodeIndexQueryResult;

/// Provider boundary for cloud code-index publication and deletion.
///
/// Implementations are expected to bind credentials, network policy, tenant isolation, request
/// size limits, and logging redaction before sending any request. Publication must be retry-safe
/// for the same grant and local generation. `delete_grant` must be idempotent and cover every
/// remote object created under the supplied grant.
pub trait CloudCodeIndexProvider: Send + Sync {
    fn id(&self) -> &CloudCodeIndexProviderId;

    fn capabilities(&self) -> CloudCodeIndexCapabilities;

    /// Publishes chunks produced and revision-verified inside the Workspace authority.
    ///
    /// Implementations are thin transport adapters to a remote CodeIndex service. They must not
    /// read Workspace files, change chunk boundaries, or substitute complete source files.
    fn publish(
        &self,
        request: CloudCodeIndexPublicationRequest,
    ) -> Result<CloudCodeIndexPublication, CloudCodeIndexProviderError>;

    /// Queries one exact, ready remote generation without changing its lifecycle state.
    ///
    /// Implementations forward this typed request to the remote CodeIndex service. The remote
    /// service owns query preparation, embedding/vector recall, optional reranking, and result
    /// filtering/truncation. It returns candidates in final relevance order; local callers
    /// preserve that order during cross-source fusion and do not receive model scores.
    ///
    /// Implementations must return revision-bound references only from the supplied grant,
    /// destination, selection, and generation. The controller validates those boundaries before
    /// candidates can reach the retrieval layer.
    fn query(
        &self,
        request: CloudCodeIndexQueryRequest,
    ) -> Result<CloudCodeIndexQueryResult, CloudCodeIndexProviderError>;

    fn delete_grant(&self, grant: &CloudCodeIndexGrant) -> Result<(), CloudCodeIndexProviderError>;
}

/// Immutable set of cloud providers available to one host composition.
#[derive(Clone, Default)]
pub struct CloudCodeIndexProviderRegistry {
    providers: Arc<BTreeMap<CloudCodeIndexProviderId, Arc<dyn CloudCodeIndexProvider>>>,
}

impl CloudCodeIndexProviderRegistry {
    pub fn new(
        providers: impl IntoIterator<Item = Arc<dyn CloudCodeIndexProvider>>,
    ) -> Result<Self, CloudCodeIndexError> {
        let mut indexed = BTreeMap::new();
        for provider in providers {
            if indexed.insert(provider.id().clone(), provider).is_some() {
                return Err(CloudCodeIndexError::InvalidInput(
                    "cloud provider identities must be unique",
                ));
            }
        }
        Ok(Self {
            providers: Arc::new(indexed),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    pub(crate) fn get(
        &self,
        id: &CloudCodeIndexProviderId,
    ) -> Option<Arc<dyn CloudCodeIndexProvider>> {
        self.providers.get(id).cloned()
    }
}
