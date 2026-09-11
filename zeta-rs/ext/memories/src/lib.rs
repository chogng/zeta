//! Agent integration for user-controlled memories, independent of storage and product hosts.

mod tool;

use async_utils::CancellationToken;
use extension_api::ContextContributor;
use extension_api::ContextEvidence;
use extension_api::ContextSourceRequest;
use extension_api::ExtensionError;
use extension_api::ExtensionRegistryBuilder;
use extension_api::ReadOnlyToolContributor;
use memories::Memories;
use memories::MemoryError;
use memories::MemoryScope;
use protocol::SessionId;
use protocol::ThreadId;
use std::sync::Arc;
use tools::ToolExecutor;

/// Resolves current task authority for every automatic lookup and model-requested read.
///
/// Hosts return only the Profile, active Projects linked to the Session, and directories currently
/// authorized for the Thread or Session. Memory consent is checked separately by the domain store.
/// Implementations must not derive identity or authority from model arguments.
pub trait MemoryScopeProvider: Send + Sync {
    fn scopes(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<Vec<MemoryScope>, MemoryError>;
}

struct MemoriesExtension {
    memories: Arc<Memories>,
    scopes: Arc<dyn MemoryScopeProvider>,
}

/// Installs or replaces Memory context and read-only tools as one extension.
pub fn install(
    builder: &mut ExtensionRegistryBuilder,
    memories: Arc<Memories>,
    scopes: Arc<dyn MemoryScopeProvider>,
) {
    let extension = Arc::new(MemoriesExtension { memories, scopes });
    builder.context_contributor("memories", extension.clone());
    builder.read_only_tool_contributor("memories", extension);
}

impl ContextContributor for MemoriesExtension {
    fn collect(
        &self,
        request: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, ExtensionError> {
        let collect = || {
            cancellation
                .check()
                .map_err(|signal| MemoryError::Cancelled(signal.reason().to_string()))?;
            let scopes = self.scopes.scopes(request.session_id, request.thread_id)?;
            self.memories
                .collect_context(scopes, request.query, cancellation)?
                .into_iter()
                .map(|entry| {
                    Ok(ContextEvidence {
                        source: "memories".into(),
                        reference: entry.citation.reference()?,
                        revision: entry.citation.revision.to_string(),
                        body: entry.body,
                    })
                })
                .collect::<Result<Vec<_>, MemoryError>>()
        };
        collect().map_err(|error| ExtensionError::new(error.to_string()))
    }
}

impl ReadOnlyToolContributor for MemoriesExtension {
    fn contribute(&self) -> Result<Vec<Arc<dyn ToolExecutor>>, ExtensionError> {
        Ok(tool::executors(Arc::new(Self {
            memories: Arc::clone(&self.memories),
            scopes: Arc::clone(&self.scopes),
        })))
    }
}

#[cfg(test)]
#[path = "memories_tests.rs"]
mod tests;
