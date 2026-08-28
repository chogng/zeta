use super::fs_watcher::WorkspaceFileChangeSink;
use super::workspace_environment::WorkspaceEnvironment;
use crate::session_workspace_access::SessionWorkspaceAccess;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use zeta_agents::AgentDefinitionCatalog;
use zeta_agents::AgentDefinitionCatalogSnapshot;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_core::CoreError;
use zeta_core::HarnessContext;
use zeta_core::HarnessContextProvider;
use zeta_core::HarnessContextRequest;
use zeta_core::HarnessInstructions;
use zeta_instructions::InstructionCatalog;
use zeta_instructions::InstructionCatalogSnapshot;
use zeta_prompts::SYSTEM_PROMPT;

pub(super) struct WorkspaceCustomizations {
    system_body: String,
    system_revision: String,
    environment: WorkspaceEnvironment,
    instructions: Mutex<InstructionCatalog>,
    agents: Mutex<AgentDefinitionCatalog>,
    harness_instructions: RwLock<Arc<HarnessInstructions>>,
    session_workspace_access: Arc<SessionWorkspaceAccess>,
}

impl WorkspaceCustomizations {
    pub(super) fn discover(
        workspace_root: impl AsRef<Path>,
        session_workspace_access: Arc<SessionWorkspaceAccess>,
    ) -> Result<Arc<Self>, zeta_agent_environment::AgentEnvironmentError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let instructions = InstructionCatalog::discover(&workspace_root);
        let agents = AgentDefinitionCatalog::discover(&workspace_root);
        let system_body = SYSTEM_PROMPT.body().to_owned();
        let system_revision = SYSTEM_PROMPT.revision().to_owned();
        let environment = WorkspaceEnvironment::capture(&workspace_root)?;
        let harness_instructions = Arc::new(render_harness_instructions(
            &system_body,
            &system_revision,
            instructions.snapshot().as_ref(),
        ));
        Ok(Arc::new(Self {
            system_body,
            system_revision,
            environment,
            instructions: Mutex::new(instructions),
            agents: Mutex::new(agents),
            harness_instructions: RwLock::new(harness_instructions),
            session_workspace_access,
        }))
    }

    pub(super) fn instruction_snapshot(&self) -> Arc<InstructionCatalogSnapshot> {
        self.instructions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot()
    }

    pub(super) fn agent_snapshot(&self) -> Arc<AgentDefinitionCatalogSnapshot> {
        self.agents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot()
    }

    fn refresh_instructions(&self) {
        let snapshot = self
            .instructions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .refresh();
        let harness_instructions = Arc::new(render_harness_instructions(
            &self.system_body,
            &self.system_revision,
            snapshot.as_ref(),
        ));
        *self
            .harness_instructions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = harness_instructions;
    }

    fn refresh_agents(&self) {
        self.agents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .refresh();
    }
}

impl HarnessContextProvider for WorkspaceCustomizations {
    fn snapshot(
        &self,
        request: &HarnessContextRequest<'_>,
    ) -> Result<Arc<HarnessContext>, CoreError> {
        let instructions = Arc::clone(
            &self
                .harness_instructions
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let roots = self
            .session_workspace_access
            .snapshot_for(
                request.session_id,
                zeta_workspace::WorkspaceCapability::MutateRepository,
            )
            .map_err(|error| CoreError::Context(error.to_string()))?
            .into_iter()
            .flat_map(|snapshot| {
                snapshot
                    .additional_roots()
                    .iter()
                    .filter(|root| root.is_active())
                    .map(|root| root.root().canonical_path().to_path_buf())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let environment = self
            .environment
            .snapshot(roots)
            .map_err(|error| CoreError::Context(error.to_string()))?;
        Ok(Arc::new(
            HarnessContext::new(instructions.as_ref().clone()).with_environment(environment),
        ))
    }
}

impl WorkspaceFileChangeSink for WorkspaceCustomizations {
    fn files_changed(&self, changed: &FsChanged) {
        match changed {
            FsChanged::RescanRequired { .. } => {
                self.refresh_instructions();
                self.refresh_agents();
            }
            FsChanged::PathsChanged { paths, .. } => {
                if paths.iter().any(|path| affects(path, ".zeta/instructions")) {
                    self.refresh_instructions();
                }
                if paths.iter().any(|path| affects(path, ".zeta/agents")) {
                    self.refresh_agents();
                }
            }
        }
    }
}

fn affects(path: &Path, customization_root: &str) -> bool {
    path == Path::new(".zeta")
        || path.starts_with(customization_root)
        || Path::new(customization_root).starts_with(path)
}

fn render_harness_instructions(
    system_body: &str,
    system_revision: &str,
    instructions: &InstructionCatalogSnapshot,
) -> HarnessInstructions {
    let workspace_content = instructions.global_content();
    let workspace_revision = content_revision(
        "workspace-instructions",
        workspace_content.as_deref().unwrap_or_default(),
    );
    HarnessInstructions::new(system_body, workspace_content)
        .with_system_revision(system_revision)
        .with_workspace_revision(workspace_revision)
}

fn content_revision(kind: &str, content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    format!(
        "{kind}:sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

#[cfg(test)]
#[path = "workspace_customizations_tests.rs"]
mod tests;
