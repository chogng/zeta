use crate::AppServer;
use crate::SlashCommandCatalog;
use crate::local_tools::compose_local_tools;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigStore, ResolvedConfig, ResolvedConfigSnapshot, WorkspaceConfigDocument,
    WorkspaceConfigInput, WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceConfigStore,
    WorkspaceId, resolve_scoped_config,
};
use zeta_core::{CoreError, ModelService};
use zeta_file_system::LocalFileSystem;
use zeta_model_provider::{
    ModelInvoker, ModelProvider, ModelProviderRuntime, ModelRuntimeRequest, UnavailableModel,
};
use zeta_rollout::RolloutRepository;
use zeta_sandboxing::WorkspaceRoot;
use zeta_shell_command::{RipgrepDiscoveryError, RipgrepExecutable};

/// Filesystem locations needed to open one persistent local App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAppServerOptions {
    pub state_root: PathBuf,
    pub workspace: Option<LocalWorkspaceConfigOptions>,
    pub tool_workspace: Option<PathBuf>,
    pub optional_tool_workspace: Option<PathBuf>,
    pub slash_commands: SlashCommandCatalog,
    pub workspace_root: Option<PathBuf>,
}

impl LocalAppServerOptions {
    pub fn new(state_root: impl Into<PathBuf>) -> Self {
        Self {
            state_root: state_root.into(),
            workspace: None,
            tool_workspace: None,
            optional_tool_workspace: None,
            slash_commands: SlashCommandCatalog::default(),
            workspace_root: None,
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

    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace_root.into());
        self
    }

    /// Enables the local read-only tool registry rooted at `workspace`.
    pub fn with_tool_workspace(mut self, workspace: impl Into<PathBuf>) -> Self {
        self.tool_workspace = Some(workspace.into());
        self
    }

    /// Enables local read-only tools when ripgrep is available.
    ///
    /// A missing executable leaves tools disabled so capability-optional hosts
    /// can still start. An invalid explicit override or other composition
    /// failure remains fatal.
    pub fn with_optional_tool_workspace(mut self, workspace: impl Into<PathBuf>) -> Self {
        self.optional_tool_workspace = Some(workspace.into());
        self
    }
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
    config
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
        workspace,
        resolver: Arc::new(ModelProviderSnapshotResolver {
            model_provider: Arc::new(ModelProviderRuntime::builtin()),
        }),
    });
    let mut server = AppServer::new(sessions, model)
        .with_config_store(config)
        .with_slash_command_catalog(options.slash_commands);
    if let Some(workspace_root) = options.workspace_root {
        let workspace = WorkspaceRoot::open(workspace_root).map_err(open_error)?;
        server = server.with_file_system(Arc::new(LocalFileSystem::new(workspace.clone())));
        if let Ok(ripgrep) = RipgrepExecutable::discover() {
            server = server.with_workspace_search(workspace, ripgrep);
        }
    }
    if let Some(tool_workspace) = options.tool_workspace {
        let tools = compose_local_tools(tool_workspace)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        server = server.with_tool_service(tools.tools, tools.policy);
    } else if let Some(tool_workspace) = options.optional_tool_workspace
        && let Some(ripgrep) = optional_ripgrep(RipgrepExecutable::discover())?
    {
        let tools = crate::local_tools::compose_local_tools_with_ripgrep(tool_workspace, ripgrep)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        server = server.with_tool_service(tools.tools, tools.policy);
    }
    Ok(server)
}

fn optional_ripgrep(
    discovery: Result<RipgrepExecutable, RipgrepDiscoveryError>,
) -> Result<Option<RipgrepExecutable>, OpenAppServerError> {
    match discovery {
        Ok(ripgrep) => Ok(Some(ripgrep)),
        Err(RipgrepDiscoveryError::NotFound) => Ok(None),
        Err(error) => Err(OpenAppServerError(format!(
            "could not resolve ripgrep: {error}"
        ))),
    }
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
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        let user = self.config.read_snapshot().map_err(|error| {
            CoreError::Model(format!("failed to read model config: {}", error.0))
        })?;
        let config = self.resolve_config(&user)?;
        ProviderModelService::new(self.resolver.resolve(&config)).invoke(request, cancellation)
    }
}

impl ConfigBackedModelService {
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
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let response = self
            .invoker
            .invoke(request)
            .map_err(|error| CoreError::Model(error.to_string()))?;
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
