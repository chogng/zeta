use crate::AppServer;
use crate::SlashCommandCatalog;
use crate::local_tools::compose_local_tools;
use crate::mcp_tools::compose_mcp_tools;
use crate::model_catalog::ModelCatalog;
use crate::server::skills_runtime::{BuiltInSkillSource, SkillConfigSnapshotProvider};
use crate::tool_composition::{ToolPort, combine_tool_ports};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigStore, ResolvedConfig, ResolvedConfigSnapshot, WorkspaceConfigDocument,
    WorkspaceConfigInput, WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceConfigStore,
    WorkspaceId, resolve_scoped_config,
};
use zeta_core::{CoreError, ModelSelection, ModelService};
use zeta_file_system::LocalFileSystem;
use zeta_install_context::InstallContext;
use zeta_model_provider::{
    ModelInvoker, ModelProvider, ModelProviderRuntime, ModelRuntimeRequest, UnavailableModel,
};
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_rollout::RolloutRepository;
use zeta_sandboxing::WorkspaceRoot;

/// Filesystem locations needed to open one persistent local App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAppServerOptions {
    pub state_root: PathBuf,
    pub workspace: Option<LocalWorkspaceConfigOptions>,
    pub slash_commands: SlashCommandCatalog,
    pub workspace_root: Option<PathBuf>,
    pub built_in_skills: BuiltInSkillRoot,
}

impl LocalAppServerOptions {
    pub fn new(state_root: impl Into<PathBuf>) -> Self {
        Self {
            state_root: state_root.into(),
            workspace: None,
            slash_commands: SlashCommandCatalog::default(),
            workspace_root: None,
            built_in_skills: BuiltInSkillRoot::AutoDetect,
        }
    }

    pub fn with_workspace(mut self, workspace: LocalWorkspaceConfigOptions) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn with_slash_command_catalog(mut self, slash_commands: SlashCommandCatalog) -> Self {
        self.slash_commands = slash_commands;
        self
    }

    /// Enables local filesystem and shell tools under one canonical Workspace root.
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace_root.into());
        self
    }

    pub fn with_built_in_skill_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.built_in_skills = BuiltInSkillRoot::Explicit(root.into());
        self
    }

    pub fn without_built_in_skills(mut self) -> Self {
        self.built_in_skills = BuiltInSkillRoot::Unavailable;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuiltInSkillRoot {
    AutoDetect,
    Explicit(PathBuf),
    Unavailable,
}

/// Read-only Workspace configuration source used by one local App Server composition root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalWorkspaceConfigOptions {
    pub config_path: PathBuf,
    pub workspace_id: WorkspaceId,
}

impl LocalWorkspaceConfigOptions {
    pub fn new(config_path: impl Into<PathBuf>, workspace_id: WorkspaceId) -> Self {
        Self {
            config_path: config_path.into(),
            workspace_id,
        }
    }
}

/// Failure to compose or recover a persistent local App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAppServerError(pub String);

impl fmt::Display for OpenAppServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OpenAppServerError {}

/// Opens the authoritative local composition root used by in-process and stdio clients.
pub fn open_local_app_server(
    options: LocalAppServerOptions,
) -> Result<AppServer, OpenAppServerError> {
    let sessions = RolloutRepository::open(&options.state_root)
        .map_err(open_error)?
        .recover_coordinator()
        .map_err(open_error)?;
    let config = Arc::new(
        ConfigStore::open(options.state_root.join("config.authority.json"))
            .map_err(|error| OpenAppServerError(error.0))?,
    );
    let user_config = config
        .read_snapshot()
        .map_err(|error| OpenAppServerError(error.0))?;
    let workspace = options.workspace.map(|workspace| {
        Arc::new(WorkspaceConfigTracker::new(WorkspaceConfigStore::open(
            workspace.config_path,
            WorkspaceConfigScope::new(workspace.workspace_id),
        )))
    });
    if let Some(workspace) = &workspace {
        workspace
            .read()
            .map_err(|error| OpenAppServerError(error.0))?;
    }
    let model = Arc::new(ConfigBackedModelService {
        config: config.clone(),
        workspace: workspace.clone(),
        resolver: Arc::new(ModelProviderSnapshotResolver {
            model_provider: Arc::new(ModelProviderRuntime::builtin()),
        }),
    });
    let runtime_config = model
        .resolve_config(&user_config)
        .map_err(|error| OpenAppServerError(error.to_string()))?;
    let skill_config = Arc::new(LocalSkillConfigProvider {
        config: Arc::clone(&config),
        config_path: options.state_root.join("config.authority.json"),
    });
    let built_in_skill_root = resolve_built_in_skill_root(options.built_in_skills);
    let mut server = AppServer::new(sessions, model.clone())
        .with_model_catalog(model)
        .with_config_store(config)
        .with_slash_command_catalog(options.slash_commands)
        .with_skill_runtime(built_in_skill_root, skill_config)
        .map_err(OpenAppServerError)?;
    let mut tool_ports = Vec::new();
    if let Some(workspace_root) = options.workspace_root {
        let workspace = WorkspaceRoot::open(workspace_root).map_err(open_error)?;
        let tools = compose_local_tools(workspace.clone())
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        server = server
            .with_file_system(Arc::new(LocalFileSystem::new(workspace.clone())))
            .with_file_system_watcher(workspace.path().to_path_buf())
            .with_workspace_search(workspace.clone(), tools.ripgrep.clone())
            .with_git_root(workspace.path().to_path_buf())
            .map_err(|_| OpenAppServerError("failed to initialize Git runtime".into()))?
            .with_terminal_root(workspace.path().to_path_buf())
            .map_err(|_| OpenAppServerError("failed to initialize terminal runtime".into()))?;
        tool_ports.push(ToolPort::local(tools.tools, tools.policy));
    }
    if let Some(mcp) = compose_mcp_tools(&runtime_config, user_config.generation)
        .map_err(|error| OpenAppServerError(error.to_string()))?
    {
        tool_ports.push(ToolPort::mcp(mcp.tools, mcp.policy));
    }
    if let Some(tools) =
        combine_tool_ports(tool_ports).map_err(|error| OpenAppServerError(error.to_string()))?
    {
        server = server.with_tool_service(tools.tools, tools.policy);
    }
    Ok(server)
}

struct LocalSkillConfigProvider {
    config: Arc<ConfigStore>,
    config_path: PathBuf,
}

impl SkillConfigSnapshotProvider for LocalSkillConfigProvider {
    fn snapshot(&self) -> Result<zeta_config::SkillsConfig, String> {
        self.config
            .read_snapshot()
            .map(|snapshot| snapshot.values.skills)
            .map_err(|error| error.0)
    }

    fn watched_config_paths(&self) -> Vec<PathBuf> {
        vec![self.config_path.clone()]
    }
}

fn resolve_built_in_skill_root(selection: BuiltInSkillRoot) -> BuiltInSkillSource {
    match selection {
        BuiltInSkillRoot::Explicit(root) => BuiltInSkillSource::Root(root),
        BuiltInSkillRoot::Unavailable => BuiltInSkillSource::Omitted,
        BuiltInSkillRoot::AutoDetect => InstallContext::current()
            .bundled_resource_directory("skills")
            .or_else(development_built_in_skill_root)
            .map(BuiltInSkillSource::Root)
            .unwrap_or(BuiltInSkillSource::Missing),
    }
}

fn development_built_in_skill_root() -> Option<PathBuf> {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills/assets");
    candidate.is_dir().then_some(candidate)
}

/// Resolves an immutable model runtime from one persisted configuration snapshot.
///
/// Implementations must not retain a mutable view of `config`: one resolved model belongs to one
/// invocation, so configuration changes can affect later invocations without changing one already
/// in progress.
trait ModelSnapshotResolver: Send + Sync {
    fn resolve(&self, config: &ResolvedConfig) -> Arc<dyn ModelInvoker>;
}

struct ModelProviderSnapshotResolver {
    model_provider: Arc<dyn ModelProvider>,
}

impl ModelSnapshotResolver for ModelProviderSnapshotResolver {
    fn resolve(&self, config: &ResolvedConfig) -> Arc<dyn ModelInvoker> {
        let Some(model_ref) = config.preferred_model.as_ref() else {
            return Arc::new(UnavailableModel::new(
                "model is not configured; configure a provider and set preferredModel",
            ));
        };
        let Some(provider) = config.selected_provider() else {
            return Arc::new(UnavailableModel::new(
                "preferred model provider is not configured",
            ));
        };
        self.model_provider
            .runtime(ModelRuntimeRequest::new(
                model_ref.clone(),
                provider.clone(),
            ))
            .unwrap_or_else(|error| Arc::new(UnavailableModel::new(error.to_string())))
    }
}

struct ConfigBackedModelService {
    config: Arc<ConfigStore>,
    workspace: Option<Arc<WorkspaceConfigTracker>>,
    resolver: Arc<dyn ModelSnapshotResolver>,
}

impl ModelService for ConfigBackedModelService {
    fn invoke(
        &self,
        selection: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        let user = self.config.read_snapshot().map_err(|error| {
            CoreError::Model(format!("failed to read model config: {}", error.0))
        })?;
        let mut config = self.resolve_config(&user)?;
        if let ModelSelection::Session(model) = selection {
            config.preferred_model = Some(model.clone());
        }
        ProviderModelService::new(self.resolver.resolve(&config)).invoke(
            ModelSelection::ConfiguredDefault,
            request,
            cancellation,
        )
    }
}

impl ModelCatalog for ConfigBackedModelService {
    fn list(
        &self,
    ) -> Result<Vec<zeta_app_server_protocol::protocol::model::ModelCatalogEntry>, CoreError> {
        let config = self.resolved_config()?;
        let registry = ProviderConfigRegistry::builtin();
        let mut models = Vec::new();
        for provider_id in config.providers.keys() {
            let Some(provider) = registry.get(provider_id) else {
                continue;
            };
            models.extend(provider.models.iter().cloned().map(|model| {
                zeta_app_server_protocol::protocol::model::ModelCatalogEntry {
                    model: zeta_protocol::ModelRef::new(provider_id.clone(), model.id),
                    display_name: model.display_name,
                }
            }));
        }
        if let Some(preferred) = config.preferred_model
            && !models.iter().any(|entry| entry.model == preferred)
        {
            models.push(
                zeta_app_server_protocol::protocol::model::ModelCatalogEntry {
                    display_name: preferred.model.to_string(),
                    model: preferred,
                },
            );
        }
        models.sort_by(|left, right| {
            left.model
                .provider
                .cmp(&right.model.provider)
                .then_with(|| left.model.model.cmp(&right.model.model))
        });
        Ok(models)
    }

    fn configured_default(&self) -> Result<Option<zeta_protocol::ModelRef>, CoreError> {
        Ok(self.resolved_config()?.preferred_model)
    }

    fn validate(&self, model: &zeta_protocol::ModelRef) -> Result<(), CoreError> {
        let config = self.resolved_config()?;
        let provider = config.providers.get(&model.provider).ok_or_else(|| {
            CoreError::Model(format!(
                "model provider '{}' is not configured",
                model.provider
            ))
        })?;
        let registry = ProviderConfigRegistry::builtin();
        registry
            .normalize_for(provider, &model.provider)
            .and_then(|_| registry.validate_model_selection(model))
            .map_err(|error| CoreError::Model(error.to_string()))
    }
}

impl ConfigBackedModelService {
    fn resolved_config(&self) -> Result<ResolvedConfig, CoreError> {
        let user = self.config.read_snapshot().map_err(|error| {
            CoreError::Model(format!("failed to read model config: {}", error.0))
        })?;
        self.resolve_config(&user)
    }

    fn resolve_config(&self, user: &ResolvedConfigSnapshot) -> Result<ResolvedConfig, CoreError> {
        let Some(workspace) = &self.workspace else {
            return Ok(user.values.clone());
        };
        let (document, revision) = workspace.read().map_err(|error| {
            CoreError::Model(format!("failed to read Workspace config: {}", error.0))
        })?;
        resolve_scoped_config(
            user,
            Some(WorkspaceConfigInput::new(
                workspace.scope(),
                revision,
                &document,
            )),
        )
        .map(|resolved| resolved.values)
        .map_err(|error| {
            CoreError::Model(format!("failed to resolve Workspace config: {}", error.0))
        })
    }
}

struct WorkspaceConfigTracker {
    store: WorkspaceConfigStore,
    observed: Mutex<Option<WorkspaceConfigObservation>>,
}

struct WorkspaceConfigObservation {
    document: WorkspaceConfigDocument,
    revision: WorkspaceConfigRevision,
}

impl WorkspaceConfigTracker {
    fn new(store: WorkspaceConfigStore) -> Self {
        Self {
            store,
            observed: Mutex::new(None),
        }
    }

    fn read(
        &self,
    ) -> Result<(WorkspaceConfigDocument, WorkspaceConfigRevision), zeta_config::ConfigError> {
        let document = self.store.read_document()?;
        let mut observed = self.observed.lock().map_err(|_| {
            zeta_config::ConfigError("Workspace config tracker lock poisoned".into())
        })?;
        if let Some(previous) = observed.as_ref()
            && previous.document == document
        {
            return Ok((previous.document.clone(), previous.revision));
        }
        let revision = observed
            .as_ref()
            .map_or(WorkspaceConfigRevision::INITIAL, |previous| {
                previous.revision.next()
            });
        *observed = Some(WorkspaceConfigObservation {
            document: document.clone(),
            revision,
        });
        Ok((document, revision))
    }

    fn scope(&self) -> &WorkspaceConfigScope {
        self.store.scope()
    }
}

pub(crate) struct ProviderModelService {
    invoker: Arc<dyn ModelInvoker>,
}

impl ProviderModelService {
    pub(crate) fn new(invoker: Arc<dyn ModelInvoker>) -> Self {
        Self { invoker }
    }
}

impl ModelService for ProviderModelService {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let response = self
            .invoker
            .invoke_with_cancellation(request, cancellation)
            .map_err(|error| match error {
                zeta_model_provider::ModelProviderError::Cancelled(message) => {
                    CoreError::Cancelled(message)
                }
                error => CoreError::Model(error.to_string()),
            })?;
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(response)
    }
}

fn open_error(error: impl fmt::Display) -> OpenAppServerError {
    OpenAppServerError(error.to_string())
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
