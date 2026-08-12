use crate::AppServer;
use crate::CodeIndexSemanticModels;
use crate::SlashCommandCatalog;
use crate::model_catalog::ModelCatalog;
use crate::server::WorkspaceSwitchTrustPolicy;
use crate::server::WorkspaceToolPorts;
use crate::tool_composition::ToolPort;
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_async_utils::CancellationToken;
use zeta_client::OperationClient;
use zeta_code_index_cloud::CloudCodeIndexProviderRegistry;
use zeta_config::{
    ConfigStore, ResolvedConfig, ResolvedConfigSnapshot, WorkspaceConfigDocument,
    WorkspaceConfigInput, WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceConfigStore,
    WorkspaceId, resolve_scoped_config,
};
use zeta_core::ContextBudget;
use zeta_core::ContextCompactionLimit;
use zeta_core::ContextTokenCount;
use zeta_core::ContextTokenMeasurementCapability;
use zeta_core::ContextTokenMeasurementOutcome;
use zeta_core::CoreError;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_extensions::ExtensionRoot;
use zeta_install_context::InstallContext;
use zeta_mcp_extension::ConnectorMcpRuntimeProvider;
use zeta_mcp_extension::McpCatalogUpdateSubscription;
use zeta_mcp_extension::McpCatalogUpdates;
use zeta_mcp_extension::PluginConnectorMcpRuntimeProvider;
use zeta_mcp_extension::compose_mcp_tools_at_generation_with_updates;
use zeta_mcp_extension::compose_mcp_tools_with_connectors_and_updates;
use zeta_model_provider::HttpTokenizerAssetDownloader;
use zeta_model_provider::HuggingFaceTokenizerAssetDiscoverer;
use zeta_model_provider::ManagedLocalTokenizerService;
use zeta_model_provider::MemoryTokenizerCapacity;
use zeta_model_provider::ModelInvoker;
use zeta_model_provider::ModelProvider;
use zeta_model_provider::ModelProviderRuntime;
use zeta_model_provider::ModelRuntimeRequest;
use zeta_model_provider::TokenizerAssetCatalog;
use zeta_model_provider::UnavailableModel;
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_models_manager::CatalogQuery;
use zeta_models_manager::ModelRequirements;
use zeta_models_manager::ModelsManager;
use zeta_plugins::PluginActivationSnapshot;
use zeta_protocol::ContextWindow;
use zeta_rollout::LocalStateRepository;
use zeta_secrets::FileSecretStore;
use zeta_secrets::SecretStore;
use zeta_skills_extension::BuiltInSkillSource;
use zeta_skills_extension::SkillConfigSnapshotProvider;

const MAX_GIT_STATUS_LINES: usize = 40;
const DEFAULT_MODEL_OUTPUT_RESERVATION_TOKENS: u32 = 4_096;
const MODEL_CONTEXT_SAFETY_MARGIN_TOKENS: u32 = 1_024;

pub(crate) fn render_environment(workspace_root: &Path) -> String {
    let is_git_repo = command_output(
        workspace_root,
        "git",
        &["rev-parse", "--is-inside-work-tree"],
    )
    .is_some_and(|output| output.trim() == "true");
    let branch = if is_git_repo {
        command_output(workspace_root, "git", &["branch", "--show-current"])
            .filter(|value| !value.trim().is_empty())
    } else {
        None
    };
    let main_branch = if is_git_repo {
        command_output(
            workspace_root,
            "git",
            &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        )
        .map(|value| value.trim_start_matches("origin/").trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| branch.clone())
    } else {
        None
    };
    let status = if is_git_repo {
        command_output(workspace_root, "git", &["status", "--porcelain"])
            .map(|value| truncate_lines(&value, MAX_GIT_STATUS_LINES))
    } else {
        None
    };
    let commits = if is_git_repo {
        command_output(workspace_root, "git", &["log", "--oneline", "-5"])
    } else {
        None
    };
    format!(
        "<environment>\nworking_directory: {}\nis_git_repo: {}\nplatform: {}\nos_version: {}\nshell: {}\ntoday: {}\ngit_branch: {}\ngit_main_branch: {}\ngit_status: {}\ngit_recent_commits: {}\n</environment>\nThis snapshot was taken at session start and does not update. Run commands\n(e.g. `git status`) when you need current state.",
        workspace_root.display(),
        is_git_repo,
        platform_name(),
        command_output(workspace_root, "uname", &["-sr"]).unwrap_or_else(|| "unknown".into()),
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
        command_output(workspace_root, "date", &["+%Y-%m-%d"]).unwrap_or_else(|| "unknown".into()),
        branch.unwrap_or_else(|| "(none)".into()),
        main_branch.unwrap_or_else(|| "(none)".into()),
        status.unwrap_or_else(|| "(none)".into()),
        commits.unwrap_or_else(|| "(none)".into()),
    )
}

fn truncate_lines(text: &str, maximum_lines: usize) -> String {
    let mut lines = text.lines().take(maximum_lines).collect::<Vec<_>>();
    if text.lines().count() > maximum_lines {
        lines.push("[... git status truncated ...]");
    }
    if lines.is_empty() {
        "(clean)".into()
    } else {
        lines.join("\n")
    }
}

fn command_output(workspace_root: &Path, program: &str, arguments: &[&str]) -> Option<String> {
    let output = if program == "git" {
        Command::new(program)
            .args(["-C", &workspace_root.to_string_lossy()])
            .args(arguments)
            .output()
    } else {
        Command::new(program)
            .args(arguments)
            .current_dir(workspace_root)
            .output()
    }
    .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

/// Filesystem and runtime inputs needed to open one local App Server.
#[derive(Clone)]
pub struct LocalAppServerOptions {
    pub profile_root: PathBuf,
    pub workspace: Option<LocalWorkspaceConfigOptions>,
    pub slash_commands: SlashCommandCatalog,
    pub workspace_root: Option<PathBuf>,
    pub built_in_skills: BuiltInSkillRoot,
    pub session_state_mode: SessionStateMode,
    model_operation_client: Option<Arc<dyn OperationClient>>,
    web_search_backend: Option<Arc<dyn zeta_web_search_extension::WebSearchBackend>>,
    connector_runtime: Option<LocalConnectorRuntime>,
}

impl LocalAppServerOptions {
    pub fn new(profile_root: impl Into<PathBuf>) -> Self {
        Self {
            profile_root: profile_root.into(),
            workspace: None,
            slash_commands: SlashCommandCatalog::default(),
            workspace_root: None,
            built_in_skills: BuiltInSkillRoot::AutoDetect,
            session_state_mode: SessionStateMode::Durable,
            model_operation_client: None,
            web_search_backend: None,
            connector_runtime: None,
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

    /// Selects whether Session and Thread event history is recovered from profile storage.
    pub fn with_session_state_mode(mut self, mode: SessionStateMode) -> Self {
        self.session_state_mode = mode;
        self
    }

    /// Replaces the production model operation client for this composition root.
    ///
    /// Embedded hosts and tests can use this to keep model transport offline while exercising the
    /// complete App Server stack. Product hosts normally leave the lazy production client in use.
    pub fn with_model_operation_client(mut self, client: Arc<dyn OperationClient>) -> Self {
        self.model_operation_client = Some(client);
        self
    }

    /// Installs the opt-in capability-bearing Web Search extension.
    ///
    /// Without an injected backend the `web_search` tool is absent. The backend's network and
    /// credential scopes are still reviewed by the ordinary extension policy before each call.
    pub fn with_web_search_backend(
        mut self,
        backend: Arc<dyn zeta_web_search_extension::WebSearchBackend>,
    ) -> Self {
        self.web_search_backend = Some(backend);
        self
    }

    /// Installs product/plugin Connector authority, secret storage, and MCP materialization.
    pub fn with_connector_runtime(mut self, runtime: LocalConnectorRuntime) -> Self {
        self.connector_runtime = Some(runtime);
        self
    }

    /// Projects one immutable Plugin activation into durable Connector authority and MCP runtime.
    pub fn with_plugin_activation(
        self,
        activation: &PluginActivationSnapshot,
        secrets: Arc<dyn SecretStore>,
    ) -> Result<Self, OpenAppServerError> {
        let runtime =
            LocalConnectorRuntime::from_plugin_activation(&self.profile_root, activation, secrets)?;
        Ok(self.with_connector_runtime(runtime))
    }
}

impl fmt::Debug for LocalAppServerOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalAppServerOptions")
            .field("profile_root", &self.profile_root)
            .field("workspace", &self.workspace)
            .field("slash_commands", &self.slash_commands)
            .field("workspace_root", &self.workspace_root)
            .field("built_in_skills", &self.built_in_skills)
            .field("session_state_mode", &self.session_state_mode)
            .field(
                "model_operation_client_injected",
                &self.model_operation_client.is_some(),
            )
            .field(
                "web_search_backend_injected",
                &self.web_search_backend.is_some(),
            )
            .field(
                "connector_runtime_injected",
                &self.connector_runtime.is_some(),
            )
            .finish()
    }
}

impl PartialEq for LocalAppServerOptions {
    fn eq(&self, other: &Self) -> bool {
        self.profile_root == other.profile_root
            && self.workspace == other.workspace
            && self.slash_commands == other.slash_commands
            && self.workspace_root == other.workspace_root
            && self.built_in_skills == other.built_in_skills
            && self.session_state_mode == other.session_state_mode
            && match (&self.model_operation_client, &other.model_operation_client) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (&self.web_search_backend, &other.web_search_backend) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (&self.connector_runtime, &other.connector_runtime) {
                (Some(left), Some(right)) => left.ptr_eq(right),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for LocalAppServerOptions {}

/// Host-provided Connector runtime ports used by the local App Server composition root.
#[derive(Clone)]
pub struct LocalConnectorRuntime {
    service: Arc<zeta_connectors_extension::ConnectorCredentialService>,
    secrets: Arc<dyn SecretStore>,
    mcp: Arc<dyn ConnectorMcpRuntimeProvider>,
}

impl LocalConnectorRuntime {
    pub fn new(
        service: Arc<zeta_connectors_extension::ConnectorCredentialService>,
        secrets: Arc<dyn SecretStore>,
        mcp: Arc<dyn ConnectorMcpRuntimeProvider>,
    ) -> Self {
        Self {
            service,
            secrets,
            mcp,
        }
    }

    /// Builds the canonical local Connector runtime from an exact Plugin activation snapshot.
    pub fn from_plugin_activation(
        profile_root: impl AsRef<Path>,
        activation: &PluginActivationSnapshot,
        secrets: Arc<dyn SecretStore>,
    ) -> Result<Self, OpenAppServerError> {
        let catalog = zeta_connectors_extension::ConnectorCatalog::from_activation(activation)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        let authority = zeta_connectors_extension::ConnectorAuthority::open_sqlite(
            profile_root.as_ref().join("connectors.sqlite3"),
            catalog
                .snapshot()
                .entries()
                .iter()
                .map(|entry| entry.definition().clone()),
        )
        .map_err(|error| OpenAppServerError(error.to_string()))?;
        let mcp = PluginConnectorMcpRuntimeProvider::from_activation(activation)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        let service = Arc::new(zeta_connectors_extension::ConnectorCredentialService::new(
            authority,
            Arc::clone(&secrets),
        ));
        Ok(Self::new(service, secrets, Arc::new(mcp)))
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.service, &other.service)
            && Arc::ptr_eq(&self.secrets, &other.secrets)
            && Arc::ptr_eq(&self.mcp, &other.mcp)
    }
}

/// Selects the lifecycle of Session and Thread state in a local App Server.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SessionStateMode {
    /// Recover and append Session and Thread event history in profile SQLite storage.
    #[default]
    Durable,
    /// Keep Session and Thread state in memory for this App Server process only.
    Ephemeral,
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

/// Optional code-index adapters installed before the local Workspace runtime is activated.
#[derive(Default)]
pub struct LocalCodeIndexProviders {
    semantic_models: Option<CodeIndexSemanticModels>,
    cloud: CloudCodeIndexProviderRegistry,
}

impl LocalCodeIndexProviders {
    pub fn new() -> Self {
        Self::default()
    }

    /// Installs models invoked by local semantic CodeIndex and opt-in Tool Search orchestration.
    pub fn with_semantic_models(mut self, models: CodeIndexSemanticModels) -> Self {
        self.semantic_models = Some(models);
        self
    }

    /// Installs optional remote code-index provider adapters.
    pub fn with_cloud(mut self, cloud: CloudCodeIndexProviderRegistry) -> Self {
        self.cloud = cloud;
        self
    }
}

/// Opens the authoritative local composition root used by in-process and stdio clients.
pub fn open_local_app_server(
    options: LocalAppServerOptions,
) -> Result<AppServer, OpenAppServerError> {
    open_local_app_server_with_code_index_providers(options, LocalCodeIndexProviders::default())
}

/// Opens a local composition with explicit cloud code-index provider adapters.
pub fn open_local_app_server_with_cloud_providers(
    options: LocalAppServerOptions,
    cloud_code_index_providers: CloudCodeIndexProviderRegistry,
) -> Result<AppServer, OpenAppServerError> {
    open_local_app_server_with_code_index_providers(
        options,
        LocalCodeIndexProviders::new().with_cloud(cloud_code_index_providers),
    )
}

/// Opens a local composition with semantic model and/or remote index provider adapters.
pub fn open_local_app_server_with_code_index_providers(
    mut options: LocalAppServerOptions,
    providers: LocalCodeIndexProviders,
) -> Result<AppServer, OpenAppServerError> {
    if options.workspace.is_none()
        && let Some(workspace_root) = &options.workspace_root
    {
        options.workspace = Some(default_workspace_config(workspace_root)?);
    }
    let (database_path, sessions) = match options.session_state_mode {
        SessionStateMode::Durable => {
            let repository =
                LocalStateRepository::open(&options.profile_root).map_err(open_error)?;
            let database_path = repository.database_path().to_path_buf();
            let sessions = repository.recover_coordinator().map_err(open_error)?;
            (database_path, sessions)
        }
        SessionStateMode::Ephemeral => {
            let threads = Arc::new(ThreadController::with_store(Arc::new(
                InMemoryThreadStore::default(),
            )));
            let sessions = Arc::new(SessionCoordinator::with_store(
                Arc::new(InMemorySessionStore::default()),
                threads,
            ));
            (options.profile_root.join("state.sqlite3"), sessions)
        }
    };
    let config = Arc::new(
        ConfigStore::open_with_paths(database_path, options.profile_root.join("config.toml"))
            .map_err(|error| OpenAppServerError(error.0))?,
    );
    let user_config = config
        .read_snapshot()
        .map_err(|error| OpenAppServerError(error.0))?;
    let connector_runtime = match options.connector_runtime.take() {
        Some(runtime) => Some(runtime),
        None => {
            let activation = PluginActivationSnapshot::empty(1)
                .map_err(|error| OpenAppServerError(error.to_string()))?;
            let secrets = Arc::new(
                FileSecretStore::open(options.profile_root.join("secrets"))
                    .map_err(|error| OpenAppServerError(error.to_string()))?,
            );
            Some(LocalConnectorRuntime::from_plugin_activation(
                &options.profile_root,
                &activation,
                secrets,
            )?)
        }
    };
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
    let tokenizer_downloader = Arc::new(HttpTokenizerAssetDownloader::production());
    let local_tokenizers = Arc::new(
        ManagedLocalTokenizerService::new(
            options.profile_root.join("cache/model-tokenizers"),
            TokenizerAssetCatalog::new(),
            tokenizer_downloader.clone(),
            MemoryTokenizerCapacity::default(),
        )
        .map(|service| {
            service.with_discoverer(Arc::new(HuggingFaceTokenizerAssetDiscoverer::new(
                tokenizer_downloader,
            )))
        })
        .map_err(|error| OpenAppServerError(error.to_string()))?,
    );
    let provider_configs = ProviderConfigRegistry::builtin();
    let model_provider = match options.model_operation_client.take() {
        Some(client) => ModelProviderRuntime::with_client(provider_configs.clone(), client),
        None => ModelProviderRuntime::new(provider_configs.clone()),
    }
    .with_local_tokenizers(local_tokenizers);
    let models_manager = model_provider.models_manager();
    let model_provider = Arc::new(model_provider);
    let model = Arc::new(ConfigBackedModelService {
        config: config.clone(),
        workspace: workspace.clone(),
        provider_configs: provider_configs.clone(),
        models_manager,
        resolver: Arc::new(ModelProviderSnapshotResolver {
            model_provider: model_provider.clone(),
        }),
    });
    let runtime_config = model
        .resolve_config(&user_config)
        .map_err(|error| OpenAppServerError(error.to_string()))?;
    let approval_model_provider: Arc<dyn ModelProvider> = model_provider.clone();
    let approval_review_model =
        crate::ReviewModelResolver::new(provider_configs, approval_model_provider)
            .resolve(&runtime_config)
            .ok();
    let skill_config = Arc::new(LocalSkillConfigProvider {
        config: Arc::clone(&config),
    });
    let built_in_skill_root = resolve_built_in_skill_root(options.built_in_skills);
    let extension_roots = resolve_extension_roots(&options.profile_root);
    let mut server = AppServer::new(sessions, model.clone())
        .with_model_catalog(model)
        .with_approval_review_model(approval_review_model)
        .with_config_store(Arc::clone(&config))
        .with_slash_command_catalog(options.slash_commands)
        .with_code_index_storage_root(options.profile_root.join("code-index"))
        .with_code_index_semantic_storage_root(options.profile_root.join("code-index-semantic"))
        .with_semantic_model_provider(model_provider)
        .with_cloud_code_index_storage_root(options.profile_root.join("code-index-cloud"))
        .with_cloud_code_index_providers(providers.cloud)
        .with_extension_roots(extension_roots)
        .with_skill_runtime(
            built_in_skill_root,
            skill_config,
            options.web_search_backend.take(),
        )
        .map_err(OpenAppServerError)?;
    if let Some(models) = providers.semantic_models {
        server = server.with_code_index_semantic_models(models);
    }
    let mcp_updates = McpCatalogUpdates::default();
    let mcp_changes = mcp_updates.subscribe();
    let mcp = match &connector_runtime {
        Some(connectors) => compose_mcp_tools_with_connectors_and_updates(
            &runtime_config,
            1,
            connectors.service.authority().clone(),
            Arc::clone(&connectors.secrets),
            Arc::clone(&connectors.mcp),
            mcp_updates.clone(),
        ),
        None => {
            compose_mcp_tools_at_generation_with_updates(&runtime_config, 1, mcp_updates.clone())
        }
    }
    .map_err(|error| OpenAppServerError(error.to_string()))?
    .map(|mcp| ToolPort::mcp(mcp.tools, mcp.policy));
    if let Some(connectors) = &connector_runtime {
        server = server.with_connector_service(Arc::clone(&connectors.service));
    }
    server = server
        .with_local_workspace_host(
            mcp,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .map_err(|error| OpenAppServerError(error.to_string()))?;
    if let Some(workspace_root) = options.workspace_root {
        server
            .activate_host_configured_workspace_root(workspace_root)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
    }
    server
        .resume_recovered_agent_coordinations()
        .map_err(open_error)?;
    server
        .resume_recovered_tool_continuations()
        .map_err(open_error)?;
    let workspace_tools = server
        .local_workspace_tool_ports()
        .ok_or_else(|| OpenAppServerError("local Workspace tools are unavailable".into()))?;
    let workspace_runtime = server
        .workspace_runtime_control()
        .ok_or_else(|| OpenAppServerError("local Workspace runtime is unavailable".into()))?;
    server = server.with_tool_config_watcher(ToolConfigWatcher::start(
        config,
        workspace_tools,
        workspace_runtime,
        connector_runtime,
        mcp_updates,
        mcp_changes,
    ));
    Ok(server)
}

fn default_workspace_config(
    workspace_root: &std::path::Path,
) -> Result<LocalWorkspaceConfigOptions, OpenAppServerError> {
    let canonical = std::fs::canonicalize(workspace_root).map_err(open_error)?;
    let digest = Sha256::digest(canonical.to_string_lossy().as_bytes());
    let workspace_id = WorkspaceId::new(format!(
        "local-{}",
        digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
    .map_err(|error| OpenAppServerError(error.0))?;
    Ok(LocalWorkspaceConfigOptions::new(
        canonical.join(".zeta/config.toml"),
        workspace_id,
    ))
}

pub(crate) struct ToolConfigWatcher {
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl ToolConfigWatcher {
    fn start(
        config: Arc<ConfigStore>,
        workspace_tools: Arc<WorkspaceToolPorts>,
        workspace_runtime: crate::server::WorkspaceRuntimeControl,
        connector_runtime: Option<LocalConnectorRuntime>,
        mcp_updates: McpCatalogUpdates,
        mcp_changes: McpCatalogUpdateSubscription,
    ) -> Self {
        let mut semantic_binding = config.read_snapshot().ok().map(|snapshot| {
            (
                snapshot.values.semantic_code_index,
                snapshot.values.providers,
            )
        });
        let changes = config.subscribe_changes();
        let connector_changes = connector_runtime
            .as_ref()
            .map(|runtime| runtime.service.authority().subscribe());
        let (shutdown, shutdown_receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-tool-config".into())
            .spawn(move || {
                let mut catalog_generation = 1_u64;
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        break;
                    }
                    let config_changed = match changes.recv_timeout(Duration::from_millis(100)) {
                        Ok(_) => {
                            while changes.try_recv().is_ok() {}
                            true
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let mut connector_changed = false;
                    if let Some(connector_changes) = &connector_changes {
                        while connector_changes.try_recv().is_ok() {
                            connector_changed = true;
                        }
                    }
                    let mut mcp_changed = false;
                    while mcp_changes.try_recv().is_ok() {
                        mcp_changed = true;
                    }
                    if !config_changed && !connector_changed && !mcp_changed {
                        continue;
                    }
                    let snapshot = match config.read_snapshot() {
                        Ok(snapshot) => snapshot,
                        Err(error) => {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                    };
                    if config_changed {
                        if let Err(error) = workspace_runtime.reconcile_user_trust(&snapshot.values)
                        {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                        let next_semantic_binding = (
                            snapshot.values.semantic_code_index.clone(),
                            snapshot.values.providers.clone(),
                        );
                        if semantic_binding.as_ref() != Some(&next_semantic_binding) {
                            semantic_binding = Some(next_semantic_binding);
                            if let Err(error) =
                                workspace_runtime.reconcile_semantic_code_index_runtime()
                            {
                                workspace_tools.record_reconcile_failure(error.to_string());
                                continue;
                            }
                        }
                    }
                    catalog_generation = match catalog_generation.checked_add(1) {
                        Some(generation) => generation,
                        None => {
                            workspace_tools
                                .record_reconcile_failure("MCP catalog generation overflow");
                            continue;
                        }
                    };
                    let composition = match &connector_runtime {
                        Some(connectors) => compose_mcp_tools_with_connectors_and_updates(
                            &snapshot.values,
                            catalog_generation,
                            connectors.service.authority().clone(),
                            Arc::clone(&connectors.secrets),
                            Arc::clone(&connectors.mcp),
                            mcp_updates.clone(),
                        ),
                        None => compose_mcp_tools_at_generation_with_updates(
                            &snapshot.values,
                            catalog_generation,
                            mcp_updates.clone(),
                        ),
                    };
                    let mcp = match composition {
                        Ok(mcp) => mcp.map(|mcp| ToolPort::mcp(mcp.tools, mcp.policy)),
                        Err(error) => {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                    };
                    if let Err(error) = workspace_tools.reconcile_user_config(
                        mcp,
                        &snapshot.values.tool_search,
                        &snapshot.values.providers,
                    ) {
                        log::error!("requested tool-search configuration is unavailable: {error}");
                        workspace_tools.record_reconcile_failure(error.to_string());
                    }
                }
            })
            .ok();
        Self {
            shutdown: Some(shutdown),
            thread,
        }
    }
}

impl Drop for ToolConfigWatcher {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct LocalSkillConfigProvider {
    config: Arc<ConfigStore>,
}

impl SkillConfigSnapshotProvider for LocalSkillConfigProvider {
    fn snapshot(&self) -> Result<zeta_config::SkillsConfig, String> {
        self.config
            .read_snapshot()
            .map(|snapshot| snapshot.values.skills)
            .map_err(|error| error.0)
    }

    fn config_changes(&self) -> Option<std::sync::mpsc::Receiver<zeta_config::ConfigChange>> {
        Some(self.config.subscribe_changes())
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

fn resolve_extension_roots(profile_root: &std::path::Path) -> Vec<ExtensionRoot> {
    let mut roots = vec![ExtensionRoot::user(profile_root.join("extensions"))];
    if let Some(root) = InstallContext::current()
        .bundled_resource_directory("extensions")
        .or_else(development_extension_root)
    {
        roots.push(ExtensionRoot::built_in(root));
    }
    roots
}

fn development_extension_root() -> Option<PathBuf> {
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../extensions");
    candidate.is_dir().then_some(candidate)
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
    provider_configs: ProviderConfigRegistry,
    models_manager: ModelsManager,
    resolver: Arc<dyn ModelSnapshotResolver>,
}

impl ModelService for ConfigBackedModelService {
    fn context_budget(&self, selection: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        context_budget_for_config(&self.config_for_selection(selection)?)
    }

    fn input_token_measurement_capability(
        &self,
        selection: ModelSelection<'_>,
    ) -> Result<ContextTokenMeasurementCapability, CoreError> {
        let config = self.config_for_selection(selection)?;
        ProviderModelService::new(self.resolver.resolve(&config))
            .input_token_measurement_capability(ModelSelection::ConfiguredDefault)
    }

    fn measure_input(
        &self,
        selection: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, CoreError> {
        let config = self.config_for_selection(selection)?;
        ProviderModelService::new(self.resolver.resolve(&config)).measure_input(
            ModelSelection::ConfiguredDefault,
            request,
            cancellation,
        )
    }

    fn invoke(
        &self,
        selection: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        let config = self.config_for_selection(selection)?;
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
        let providers = config.providers.keys().cloned().collect::<Vec<_>>();
        let mut models = self
            .models_manager
            .list_static(&providers, &CatalogQuery::selectable())
            .map_err(|error| CoreError::Model(error.to_string()))?
            .into_iter()
            .map(
                |entry| zeta_app_server_protocol::protocol::model::ModelCatalogEntry {
                    model: entry.model().clone(),
                    display_name: entry.info().display_name.clone(),
                },
            )
            .collect::<Vec<_>>();
        if let Some(preferred) = config.preferred_model
            && !models.iter().any(|entry| entry.model == preferred)
        {
            let resolved = self
                .models_manager
                .resolve_static(&preferred, &ModelRequirements::agent())
                .map_err(|error| CoreError::Model(error.to_string()))?;
            models.push(
                zeta_app_server_protocol::protocol::model::ModelCatalogEntry {
                    display_name: resolved.entry().info().display_name.clone(),
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
        self.provider_configs
            .normalize_for(provider, &model.provider)
            .map_err(|error| CoreError::Model(error.to_string()))?;
        self.models_manager
            .resolve_static(model, &ModelRequirements::agent())
            .map(|_| ())
            .map_err(|error| CoreError::Model(error.to_string()))
    }
}

impl ConfigBackedModelService {
    fn config_for_selection(
        &self,
        selection: ModelSelection<'_>,
    ) -> Result<ResolvedConfig, CoreError> {
        let user = self.config.read_snapshot().map_err(|error| {
            CoreError::Model(format!("failed to read model config: {}", error.0))
        })?;
        let mut config = self.resolve_config(&user)?;
        if let ModelSelection::Session(model) = selection {
            config.preferred_model = Some(model.clone());
        }
        Ok(config)
    }

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

fn context_budget_for_config(config: &ResolvedConfig) -> Result<ContextBudget, CoreError> {
    let Some(model_ref) = config.preferred_model.as_ref() else {
        return Ok(ContextBudget::provider_managed());
    };
    let Some(provider_config) = config.providers.get(&model_ref.provider) else {
        return Ok(ContextBudget::provider_managed());
    };
    let registry = ProviderConfigRegistry::builtin();
    let Some(definition) = registry.get(&model_ref.provider) else {
        return Ok(ContextBudget::provider_managed());
    };
    let catalog_model = definition
        .models
        .iter()
        .find(|model| model.id == model_ref.model);
    let configured_context = provider_config.model_context.get(&model_ref.model);
    let (context_window, auto_compact_token_limit) = match configured_context {
        Some(context) => {
            let default_limit = context.context_window.saturating_mul(9) / 10;
            (
                context.context_window,
                Some(
                    context
                        .auto_compact_token_limit
                        .map_or(default_limit, |configured| configured.min(default_limit)),
                ),
            )
        }
        None => {
            let Some(model) = catalog_model else {
                return Ok(ContextBudget::provider_managed());
            };
            let ContextWindow::Known(context_window) = model.context_window else {
                return Ok(ContextBudget::provider_managed());
            };
            (context_window, model.effective_auto_compact_token_limit())
        }
    };
    let normalized = registry
        .normalize_for(provider_config, &model_ref.provider)
        .map_err(|error| CoreError::Model(error.to_string()))?;
    let reserved_output = normalized
        .max_output_tokens
        .unwrap_or(DEFAULT_MODEL_OUTPUT_RESERVATION_TOKENS);
    let compaction_limit = auto_compact_token_limit
        .map_or(ContextCompactionLimit::ContextWindow, |tokens| {
            ContextCompactionLimit::Tokens(ContextTokenCount::new(tokens))
        });
    Ok(ContextBudget::core_managed(
        ContextTokenCount::new(context_window),
        ContextTokenCount::new(reserved_output),
        ContextTokenCount::new(MODEL_CONTEXT_SAFETY_MARGIN_TOKENS),
        compaction_limit,
    ))
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
    fn input_token_measurement_capability(
        &self,
        _: ModelSelection<'_>,
    ) -> Result<ContextTokenMeasurementCapability, CoreError> {
        Ok(self.invoker.input_token_measurement_capability())
    }

    fn measure_input(
        &self,
        _: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let outcome = self
            .invoker
            .measure_input_with_cancellation(request, cancellation)
            .map_err(map_model_provider_error)?;
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(outcome)
    }

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
            .map_err(map_model_provider_error)?;
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(response)
    }
}

fn map_model_provider_error(error: zeta_model_provider::ModelProviderError) -> CoreError {
    match error {
        zeta_model_provider::ModelProviderError::Cancelled(message) => {
            CoreError::Cancelled(message)
        }
        error if error.is_transient() => CoreError::ModelTransient(error.to_string()),
        error => CoreError::Model(error.to_string()),
    }
}

fn open_error(error: impl fmt::Display) -> OpenAppServerError {
    OpenAppServerError(error.to_string())
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
