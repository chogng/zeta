use super::AppServer;
use crate::dir_grants::DirGrants;
use memories::Memories;
use memories::MemoryError;
use memories::MemoryScope;
use std::sync::Arc;
use zeta_async_utils::CancellationToken;
use zeta_core::ContextEvidence;
use zeta_core::ContextSource;
use zeta_core::ContextSourceRequest;
use zeta_core::CoreError;
use zeta_file_access::Permission;
use zeta_projects::ProjectCoordinator;

/// Resolves task identity and current directory authority, then delegates reading to Memories.
struct MemoriesContextSource {
    memories: Arc<Memories>,
    projects: Option<Arc<ProjectCoordinator>>,
    dirs: Arc<DirGrants>,
}

impl AppServer {
    pub(super) fn install_memory_context(&mut self) {
        let Some(memories) = &self.memories else {
            return;
        };
        let source = Arc::new(MemoriesContextSource {
            memories: Arc::clone(memories),
            projects: self.projects.clone(),
            dirs: Arc::clone(&self.env_runtime_mut().dir_grants),
        });
        let executor = self
            .env_runtime_mut()
            .turn_executor
            .clone()
            .with_context_source("memories", source);
        self.turn_backend.install_executor(executor.clone());
        self.env_runtime_mut().turn_executor = executor;
    }
}

impl ContextSource for MemoriesContextSource {
    fn collect(
        &self,
        request: &ContextSourceRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContextEvidence>, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let mut scopes = vec![MemoryScope::Profile];
        if let Some(projects) = &self.projects {
            for project in projects
                .list()
                .map_err(|error| CoreError::Context(error.to_string()))?
            {
                if project.is_active() && project.session_ids.contains(request.session_id) {
                    scopes.push(MemoryScope::Project {
                        project_id: project.project_id,
                    });
                }
            }
        }
        if let Some(dir_id) = self.dirs.thread_dir_id(request.thread_id) {
            scopes.push(MemoryScope::Dir { dir_id });
        }
        for entry in self.dirs.list(request.session_id) {
            if entry.permissions().allows(Permission::ReadFiles) {
                scopes.push(MemoryScope::Dir {
                    dir_id: entry.dir().id(),
                });
            }
        }
        self.memories
            .collect_context(scopes, request.query, cancellation)
            .map_err(context_error)?
            .into_iter()
            .map(|entry| {
                Ok(ContextEvidence {
                    source: "memories".into(),
                    reference: entry.citation.reference().map_err(context_error)?,
                    revision: entry.citation.revision.to_string(),
                    body: entry.body,
                })
            })
            .collect()
    }
}

fn context_error(error: MemoryError) -> CoreError {
    match error {
        MemoryError::Cancelled(message) => CoreError::Cancelled(message),
        error => CoreError::Context(error.to_string()),
    }
}

#[cfg(test)]
#[path = "memories_context_tests.rs"]
mod tests;
