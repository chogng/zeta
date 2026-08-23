use super::fs_watcher::WorkspaceFileChangeSink;
use crate::local::render_environment;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use zeta_agents::AgentDefinitionCatalog;
use zeta_agents::AgentDefinitionCatalogSnapshot;
use zeta_app_server_protocol::protocol::fs::FsChanged;
use zeta_core::HarnessInstructions;
use zeta_core::HarnessInstructionsProvider;
use zeta_instructions::InstructionCatalog;
use zeta_instructions::InstructionCatalogSnapshot;
use zeta_prompts::SYSTEM_PROMPT;

pub(super) struct WorkspaceCustomizations {
    system_body: String,
    system_revision: String,
    environment: String,
    instructions: Mutex<InstructionCatalog>,
    agents: Mutex<AgentDefinitionCatalog>,
    harness: RwLock<Arc<HarnessInstructions>>,
}

impl WorkspaceCustomizations {
    pub(super) fn discover(workspace_root: impl AsRef<Path>) -> Arc<Self> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let instructions = InstructionCatalog::discover(&workspace_root);
        let agents = AgentDefinitionCatalog::discover(&workspace_root);
        let system_body = SYSTEM_PROMPT.body().to_owned();
        let system_revision = SYSTEM_PROMPT.revision().to_owned();
        let environment = render_environment(&workspace_root);
        let harness = Arc::new(render_harness(
            &system_body,
            &system_revision,
            &environment,
            instructions.snapshot().as_ref(),
        ));
        Arc::new(Self {
            system_body,
            system_revision,
            environment,
            instructions: Mutex::new(instructions),
            agents: Mutex::new(agents),
            harness: RwLock::new(harness),
        })
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
        let harness = Arc::new(render_harness(
            &self.system_body,
            &self.system_revision,
            &self.environment,
            snapshot.as_ref(),
        ));
        *self
            .harness
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = harness;
    }

    fn refresh_agents(&self) {
        self.agents
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .refresh();
    }
}

impl HarnessInstructionsProvider for WorkspaceCustomizations {
    fn snapshot(&self) -> Arc<HarnessInstructions> {
        Arc::clone(
            &self
                .harness
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

impl WorkspaceFileChangeSink for WorkspaceCustomizations {
    fn files_changed(&self, changed: &FsChanged) {
        match changed {
            FsChanged::RescanRequired => {
                self.refresh_instructions();
                self.refresh_agents();
            }
            FsChanged::PathsChanged { paths } => {
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

fn render_harness(
    system_body: &str,
    system_revision: &str,
    environment: &str,
    instructions: &InstructionCatalogSnapshot,
) -> HarnessInstructions {
    let workspace_content = instructions.global_content();
    let workspace_revision = content_revision(
        "workspace-instructions",
        workspace_content.as_deref().unwrap_or_default(),
    );
    HarnessInstructions::new(system_body, environment, workspace_content)
        .with_system_revision(system_revision)
        .with_environment_revision(content_revision("workspace-environment", environment))
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
