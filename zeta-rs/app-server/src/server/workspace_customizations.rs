use super::fs_watcher::SessionDirectoryFileChangeSink;
use super::fs_watcher::WorkspaceFileChangeSink;
use super::workspace_environment::WorkspaceEnvironment;
use crate::session_workspace_access::SessionWorkspaceAccess;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
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
use zeta_protocol::SessionId;
use zeta_workspace::TrustedWorkspace;

struct AdditionalCustomizationCatalog {
    workspace: TrustedWorkspace,
    instructions: InstructionCatalog,
    agents: AgentDefinitionCatalog,
}

pub(super) struct WorkspaceCustomizations {
    environment: WorkspaceEnvironment,
    instructions: Mutex<InstructionCatalog>,
    agents: Mutex<AgentDefinitionCatalog>,
    harness_instructions: RwLock<Arc<HarnessInstructions>>,
    session_workspace_access: Arc<SessionWorkspaceAccess>,
    additional: Mutex<BTreeMap<SessionId, BTreeMap<PathBuf, AdditionalCustomizationCatalog>>>,
    hooks: RwLock<Option<Arc<zeta_hooks::DeclarativeHookRuntime>>>,
}

impl WorkspaceCustomizations {
    pub(super) fn discover(
        workspace_root: impl AsRef<Path>,
        session_workspace_access: Arc<SessionWorkspaceAccess>,
    ) -> Result<Arc<Self>, zeta_agent_environment::AgentEnvironmentError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let instructions = InstructionCatalog::discover(&workspace_root);
        let agents = AgentDefinitionCatalog::discover(&workspace_root);
        let environment = WorkspaceEnvironment::capture(&workspace_root)?;
        let harness_instructions = Arc::new(render_harness_instructions(
            instructions.snapshot().as_ref(),
        ));
        Ok(Arc::new(Self {
            environment,
            instructions: Mutex::new(instructions),
            agents: Mutex::new(agents),
            harness_instructions: RwLock::new(harness_instructions),
            session_workspace_access,
            additional: Mutex::new(BTreeMap::new()),
            hooks: RwLock::new(None),
        }))
    }

    pub(super) fn bind_hooks(&self, hooks: Arc<zeta_hooks::DeclarativeHookRuntime>) {
        *self
            .hooks
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(hooks);
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

    pub(super) fn agent_snapshots_for(
        &self,
        session_id: &SessionId,
    ) -> Vec<Arc<AgentDefinitionCatalogSnapshot>> {
        let mut snapshots = vec![self.agent_snapshot()];
        let additional = self
            .additional
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(catalogs) = additional.get(session_id) {
            snapshots.extend(
                catalogs
                    .values()
                    .filter(|catalog| catalog.workspace.ensure_active().is_ok())
                    .map(|catalog| catalog.agents.snapshot()),
            );
        }
        snapshots
    }

    pub(super) fn instruction_snapshots_for(
        &self,
        session_id: &SessionId,
    ) -> Vec<Arc<InstructionCatalogSnapshot>> {
        let mut snapshots = vec![self.instruction_snapshot()];
        let additional = self
            .additional
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(catalogs) = additional.get(session_id) {
            snapshots.extend(
                catalogs
                    .values()
                    .filter(|catalog| catalog.workspace.ensure_active().is_ok())
                    .map(|catalog| catalog.instructions.snapshot()),
            );
        }
        snapshots
    }

    pub(super) fn reconcile_session(
        &self,
        session_id: &SessionId,
        workspaces: Vec<TrustedWorkspace>,
    ) {
        let mut additional = self
            .additional
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut previous = additional.remove(session_id).unwrap_or_default();
        let catalogs = workspaces
            .into_iter()
            .map(|workspace| {
                let root = workspace.root().canonical_path().to_path_buf();
                let catalog = if let Some(mut catalog) = previous.remove(&root) {
                    catalog.workspace = workspace;
                    catalog
                } else {
                    AdditionalCustomizationCatalog {
                        instructions: InstructionCatalog::discover(&root),
                        agents: AgentDefinitionCatalog::discover(&root),
                        workspace,
                    }
                };
                (root, catalog)
            })
            .collect::<BTreeMap<_, _>>();
        if !catalogs.is_empty() {
            additional.insert(session_id.clone(), catalogs);
        }
    }

    pub(super) fn remove_session(&self, session_id: &SessionId) {
        self.additional
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
    }

    pub(super) fn additional_files_changed(
        &self,
        session_id: &SessionId,
        root: &Path,
        changed: &FsChanged,
    ) {
        let refresh_hooks = matches!(changed, FsChanged::RescanRequired { .. })
            || matches!(changed, FsChanged::PathsChanged { paths, .. } if paths.iter().any(|path| affects(path, ".zeta/config.toml")));
        let mut additional = self
            .additional
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(catalog) = additional
            .get_mut(session_id)
            .and_then(|catalogs| catalogs.get_mut(root))
        else {
            drop(additional);
            if refresh_hooks {
                self.refresh_session_hooks(session_id);
            }
            return;
        };
        if catalog.workspace.ensure_active().is_err() {
            return;
        }
        match changed {
            FsChanged::RescanRequired { .. } => {
                catalog.instructions.refresh();
                catalog.agents.refresh();
            }
            FsChanged::PathsChanged { paths, .. } => {
                if paths.iter().any(|path| affects(path, ".zeta/instructions")) {
                    catalog.instructions.refresh();
                }
                if paths.iter().any(|path| affects(path, ".zeta/agents")) {
                    catalog.agents.refresh();
                }
            }
        }
        drop(additional);
        if refresh_hooks {
            self.refresh_session_hooks(session_id);
        }
    }

    fn refresh_session_hooks(&self, session_id: &SessionId) {
        let Some(hooks) = self
            .hooks
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        else {
            return;
        };
        let workspaces = self
            .session_workspace_access
            .snapshot_for(
                session_id,
                zeta_workspace::WorkspaceCapability::DiscoverHooks,
            )
            .ok()
            .flatten()
            .into_iter()
            .flat_map(|snapshot| snapshot.additional_roots().to_vec())
            .filter_map(|discovery| {
                self.session_workspace_access
                    .workspace_for(
                        session_id,
                        discovery.root().canonical_path(),
                        zeta_workspace::WorkspaceCapability::ExecuteProcess,
                    )
                    .ok()
                    .flatten()
                    .map(|execution| (discovery, execution))
            })
            .filter_map(|(discovery, execution)| {
                super::workspace_runtime::read_additional_workspace_config(discovery.root())
                    .ok()
                    .map(|document| (document.hooks, discovery, execution))
            })
            .collect();
        if let Err(error) = hooks.replace_session_workspaces(session_id.clone(), workspaces) {
            log::warn!("failed to refresh additional-directory Hooks: {error}");
        }
    }

    fn refresh_instructions(&self) {
        let snapshot = self
            .instructions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .refresh();
        let harness_instructions = Arc::new(render_harness_instructions(snapshot.as_ref()));
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
        let base_instructions = Arc::clone(
            &self
                .harness_instructions
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let roots = self
            .session_workspace_access
            .snapshot_for(
                request.session_id,
                zeta_workspace::WorkspaceCapability::InspectRepository,
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
        let additional_content = {
            let additional = self
                .additional
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            additional
                .get(request.session_id)
                .into_iter()
                .flat_map(BTreeMap::iter)
                .filter(|(_, catalog)| catalog.workspace.ensure_active().is_ok())
                .filter_map(|(root, catalog)| {
                    catalog
                        .instructions
                        .snapshot()
                        .global_content()
                        .map(|content| (root.clone(), content))
                })
                .collect::<Vec<_>>()
        };
        let instructions = if additional_content.is_empty() {
            base_instructions
        } else {
            let primary = self.instruction_snapshot();
            Arc::new(render_harness_instructions_with_additional(
                primary.as_ref(),
                &additional_content,
            ))
        };
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

impl SessionDirectoryFileChangeSink for WorkspaceCustomizations {
    fn session_files_changed(&self, session_id: &SessionId, root: &Path, changed: &FsChanged) {
        self.additional_files_changed(session_id, root, changed);
    }
}

fn affects(path: &Path, customization_root: &str) -> bool {
    path == Path::new(".zeta")
        || path.starts_with(customization_root)
        || Path::new(customization_root).starts_with(path)
}

fn render_harness_instructions(instructions: &InstructionCatalogSnapshot) -> HarnessInstructions {
    let workspace_content = instructions.global_content();
    let workspace_revision = content_revision(
        "workspace-instructions",
        workspace_content.as_deref().unwrap_or_default(),
    );
    HarnessInstructions::workspace(workspace_content).with_workspace_revision(workspace_revision)
}

fn render_harness_instructions_with_additional(
    instructions: &InstructionCatalogSnapshot,
    additional: &[(PathBuf, String)],
) -> HarnessInstructions {
    let mut sections = Vec::new();
    if let Some(primary) = instructions.global_content() {
        sections.push(primary);
    }
    sections.extend(additional.iter().map(|(root, content)| {
        format!(
            "<additional-directory root=\"{}\">\n{}\n</additional-directory>",
            escape_xml(&root.display().to_string()),
            content
        )
    }));
    let content = sections.join("\n\n");
    let workspace_revision = content_revision("workspace-instructions", &content);
    HarnessInstructions::workspace((!content.is_empty()).then_some(content))
        .with_workspace_revision(workspace_revision)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
