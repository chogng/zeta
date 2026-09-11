use super::AppServer;
use crate::dir_grants::DirGrants;
use memories::MemoryError;
use memories::MemoryScope;
use memories_extension::MemoryScopeProvider;
use std::sync::Arc;
use zeta_file_access::Permission;
use zeta_projects::ProjectCoordinator;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

/// Adapts task identity and current directory authority to the Memory domain.
struct MemoryScopes {
    projects: Option<Arc<ProjectCoordinator>>,
    dirs: Arc<DirGrants>,
}

impl AppServer {
    pub(super) fn with_memory_extension(mut self) -> Result<Self, String> {
        let Some(memories) = self.memories.clone() else {
            return Ok(self);
        };
        let scopes = Arc::new(MemoryScopes {
            projects: self.projects.clone(),
            dirs: Arc::clone(&self.env_runtime_mut().dir_grants),
        });
        let mut builder =
            zeta_extension_api::ExtensionRegistryBuilder::from_registry(&self.agent_extensions);
        memories_extension::install(&mut builder, memories, scopes);
        let registry = Arc::new(builder.build());
        let tools = crate::extension_tools::compose_extension_tools(&registry)
            .map_err(|error| error.to_string())?;
        self.threads
            .install_extensions(Arc::clone(&registry))
            .map_err(|error| error.to_string())?;
        let executor = self
            .env_runtime_mut()
            .turn_executor
            .clone()
            .with_extensions(Arc::clone(&registry));
        self.turn_backend.install_executor(executor.clone());
        self.env_runtime_mut().turn_executor = executor;
        self.agent_extensions = registry;
        self.restart_extension_config_watcher();
        self.with_extension_tool_port(tools)
            .map_err(|error| error.to_string())
    }
}

impl MemoryScopeProvider for MemoryScopes {
    fn scopes(
        &self,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<Vec<MemoryScope>, MemoryError> {
        let mut scopes = vec![MemoryScope::Profile];
        if let Some(projects) = &self.projects {
            for project in projects
                .list()
                .map_err(|error| MemoryError::Storage(error.to_string()))?
            {
                if project.is_active() && project.session_ids.contains(session_id) {
                    scopes.push(MemoryScope::Project {
                        project_id: project.project_id,
                    });
                }
            }
        }
        if let Some(dir_id) = self.dirs.thread_dir_id(thread_id) {
            scopes.push(MemoryScope::Dir { dir_id });
        }
        for entry in self.dirs.list(session_id) {
            if entry.permissions().allows(Permission::ReadFiles) {
                scopes.push(MemoryScope::Dir {
                    dir_id: entry.dir().id(),
                });
            }
        }
        Ok(scopes)
    }
}

#[cfg(test)]
#[path = "memories_context_tests.rs"]
mod tests;
