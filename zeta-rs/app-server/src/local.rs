use crate::AppServer;
use crate::CodeIndexSemanticModels;
use crate::SlashCommandCatalog;
use crate::model_catalog::CombinedModelCatalog;
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
use zeta_codex_app_server::CodexAppServerLoginDriver;
use zeta_codex_app_server::CodexAppServerOptions;
use zeta_codex_app_server::CodexAppServerRuntime;
use zeta_codex_app_server::CodexModelCatalog;
use zeta_codex_app_server::CodexThreadAccess;
use zeta_codex_app_server::CodexTurnDriver;
use zeta_codex_app_server::CodexTurnExecutionBackend;
use zeta_codex_app_server::CodexTurnExecutionBackendOptions;
use zeta_config::McpServerId;
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
use zeta_core::ModelStreamSink as CoreModelStreamSink;
use zeta_core::SessionCoordinator;
use zeta_core::ThreadController;
use zeta_extensions::ExtensionRoot;
use zeta_install_context::InstallContext;
use zeta_keyring_store::KeyringSecretStore;
use zeta_language_server_catalog::ManagedNodeRuntime;
use zeta_login::LoginService;
use zeta_mcp_extension::ConnectorMcpRuntimeProvider;
use zeta_mcp_extension::McpCatalogUpdateSubscription;
use zeta_mcp_extension::McpCatalogUpdates;
use zeta_mcp_extension::McpOAuthProvider;
use zeta_mcp_extension::McpOAuthService;
use zeta_mcp_extension::McpRuntimeStatusSnapshot;
use zeta_mcp_extension::PluginConnectorMcpRuntimeProvider;
use zeta_mcp_extension::compose_mcp_tools_at_generation_with_runtime_intents_and_updates;
use zeta_mcp_extension::compose_mcp_tools_with_connectors_and_runtime_intents_and_updates;
use zeta_model_provider::HttpTokenizerAssetDownloader;
use zeta_model_provider::HuggingFaceTokenizerAssetDiscoverer;
use zeta_model_provider::ManagedLocalTokenizerService;
use zeta_model_provider::MemoryTokenizerCapacity;
use zeta_model_provider::ModelEventSink;
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
use zeta_plugins::PluginActivationAuthority;
use zeta_plugins::PluginActivationSnapshot;
use zeta_protocol::ContextWindow;
use zeta_rollout::LocalStateRepository;
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
    mcp_oauth_providers: Vec<(McpServerId, Arc<dyn McpOAuthProvider>)>,
    marketplace_manager_client: Option<Arc<dyn zeta_marketplace_client::MarketplaceServiceClient>>,
    local_marketplace_manager: Option<Arc<zeta_marketplace_manager::MarketplaceManager>>,
    language_server_providers: zeta_language_server_catalog::LanguageServerProviderRegistry,
    product_services: Option<crate::LocalProductServicesConfig>,
    codex_app_server: CodexAppServerOptions,
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
            mcp_oauth_providers: Vec::new(),
            marketplace_manager_client: None,
            local_marketplace_manager: None,
            language_server_providers:
                zeta_language_server_catalog::LanguageServerProviderRegistry::new(),
            product_services: None,
            codex_app_server: CodexAppServerOptions::default(),
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

    /// Selects the upstream Codex binary used for managed ChatGPT login.
    pub fn with_codex_app_server(mut self, options: CodexAppServerOptions) -> Self {
        self.codex_app_server = options;
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

    /// Installs exact OAuth wire adapters for standalone MCP server declarations.
    pub fn with_mcp_oauth_providers(
        mut self,
        providers: impl IntoIterator<Item = (McpServerId, Arc<dyn McpOAuthProvider>)>,
    ) -> Self {
        self.mcp_oauth_providers = providers.into_iter().collect();
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

    /// Installs a live Plugin authority whose generations drive Connector and MCP replacement.
    pub fn with_plugin_authority(
        self,
        authority: PluginActivationAuthority,
        secrets: Arc<dyn SecretStore>,
    ) -> Result<Self, OpenAppServerError> {
        let runtime =
            LocalConnectorRuntime::from_plugin_authority(&self.profile_root, authority, secrets)?;
        Ok(self.with_connector_runtime(runtime))
    }

    /// Installs the product-facing client for the local Marketplace Manager.
    pub fn with_marketplace_manager_client(
        mut self,
        client: Arc<dyn zeta_marketplace_client::MarketplaceServiceClient>,
    ) -> Self {
        self.local_marketplace_manager = None;
        self.marketplace_manager_client = Some(client);
        self
    }

    /// Composes Zeta's local Marketplace Manager with one product-pinned remote registry.
    pub fn with_marketplace_registry(
        self,
        config: zeta_marketplace_client::RemoteMarketplaceConfig,
    ) -> Result<Self, OpenAppServerError> {
        let registry = zeta_marketplace_client::MarketplaceRemoteClient::open(config)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        let manager = Arc::new(
            zeta_marketplace_manager::MarketplaceManager::open(
                self.profile_root.join("marketplace-manager"),
                Arc::new(registry),
            )
            .map_err(|error| OpenAppServerError(error.to_string()))?,
        );
        let client: Arc<dyn zeta_marketplace_client::MarketplaceServiceClient> = manager.clone();
        Ok(Self {
            marketplace_manager_client: Some(client),
            local_marketplace_manager: Some(manager),
            ..self
        })
    }

    /// Installs exact, already materialized language-server providers for this App Server.
    pub fn with_language_server_providers(
        mut self,
        providers: zeta_language_server_catalog::LanguageServerProviderRegistry,
    ) -> Self {
        self.language_server_providers = providers;
        self
    }

    /// Installs distribution-pinned Marketplace roots and public OAuth product adapters.
    pub fn with_product_services(mut self, services: crate::LocalProductServicesConfig) -> Self {
        self.product_services = Some(services);
        self
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
            .field("mcp_oauth_provider_count", &self.mcp_oauth_providers.len())
            .field(
                "marketplace_manager_client_injected",
                &self.marketplace_manager_client.is_some(),
            )
            .field(
                "local_marketplace_manager_injected",
                &self.local_marketplace_manager.is_some(),
            )
            .field(
                "language_server_provider_count",
                &self.language_server_providers.len(),
            )
            .field(
                "product_services_injected",
                &self.product_services.is_some(),
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
            && self.mcp_oauth_providers.len() == other.mcp_oauth_providers.len()
            && self
                .mcp_oauth_providers
                .iter()
                .zip(&other.mcp_oauth_providers)
                .all(|((left_id, left), (right_id, right))| {
                    left_id == right_id && Arc::ptr_eq(left, right)
                })
            && match (
                &self.marketplace_manager_client,
                &other.marketplace_manager_client,
            ) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (
                &self.local_marketplace_manager,
                &other.local_marketplace_manager,
            ) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && self
                .language_server_providers
                .ptr_eq(&other.language_server_providers)
            && self.product_services == other.product_services
            && self.codex_app_server == other.codex_app_server
    }
}

impl Eq for LocalAppServerOptions {}

/// Host-provided Connector runtime ports used by the local App Server composition root.
#[derive(Clone)]
pub struct LocalConnectorRuntime {
    service: Arc<zeta_connectors_extension::ConnectorCredentialService>,
    secrets: Arc<dyn SecretStore>,
    mcp: Arc<dyn ConnectorMcpRuntimeProvider>,
    base_definitions: Vec<zeta_connectors::ConnectorDefinition>,
    base_mcp: Arc<dyn ConnectorMcpRuntimeProvider>,
    plugin_authority: Option<PluginActivationAuthority>,
    marketplace_manager: Option<Arc<zeta_marketplace_manager::MarketplaceManager>>,
    oauth: Option<Arc<zeta_connectors_extension::ConnectorOAuthService>>,
    device_oauth: Option<Arc<zeta_connectors_extension::ConnectorDeviceOAuthService>>,
}

impl LocalConnectorRuntime {
    pub fn new(
        service: Arc<zeta_connectors_extension::ConnectorCredentialService>,
        secrets: Arc<dyn SecretStore>,
        mcp: Arc<dyn ConnectorMcpRuntimeProvider>,
    ) -> Self {
        let base_definitions = service
            .authority()
            .snapshot()
            .entries()
            .iter()
            .map(|entry| entry.definition().clone())
            .collect();
        Self {
            service,
            secrets,
            mcp: Arc::clone(&mcp),
            base_definitions,
            base_mcp: mcp,
            plugin_authority: None,
            marketplace_manager: None,
            oauth: None,
            device_oauth: None,
        }
    }

    /// Installs concrete provider adapters while keeping PKCE and credentials in the shared runtime.
    pub fn with_oauth_providers(
        mut self,
        providers: impl IntoIterator<
            Item = (
                zeta_connectors::ConnectorId,
                Arc<dyn zeta_connectors_extension::ConnectorOAuthProvider>,
            ),
        >,
    ) -> Self {
        self.oauth = Some(Arc::new(
            zeta_connectors_extension::ConnectorOAuthService::new(
                Arc::clone(&self.service),
                providers,
            ),
        ));
        self
    }

    /// Installs public-client device adapters over the canonical Connector authority.
    pub fn with_device_oauth_providers(
        mut self,
        providers: impl IntoIterator<
            Item = (
                zeta_connectors::ConnectorId,
                Arc<dyn zeta_connectors_extension::ConnectorDeviceOAuthProvider>,
            ),
        >,
    ) -> Self {
        self.device_oauth = Some(Arc::new(
            zeta_connectors_extension::ConnectorDeviceOAuthService::new(
                Arc::clone(&self.service),
                providers,
            ),
        ));
        self
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

    /// Builds a reloadable Connector/MCP projection from one live Plugin authority.
    pub fn from_plugin_authority(
        profile_root: impl AsRef<Path>,
        plugin_authority: PluginActivationAuthority,
        secrets: Arc<dyn SecretStore>,
    ) -> Result<Self, OpenAppServerError> {
        let snapshot = plugin_authority.snapshot();
        let catalog =
            zeta_connectors_extension::ConnectorCatalog::from_activation(snapshot.activation())
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
        let mcp: Arc<dyn ConnectorMcpRuntimeProvider> = Arc::new(
            PluginConnectorMcpRuntimeProvider::from_authority(&plugin_authority)
                .map_err(|error| OpenAppServerError(error.to_string()))?,
        );
        let service = Arc::new(zeta_connectors_extension::ConnectorCredentialService::new(
            authority,
            Arc::clone(&secrets),
        ));
        Ok(Self {
            service,
            secrets,
            mcp: Arc::clone(&mcp),
            base_definitions: catalog
                .snapshot()
                .entries()
                .iter()
                .map(|entry| entry.definition().clone())
                .collect(),
            base_mcp: mcp,
            plugin_authority: Some(plugin_authority),
            marketplace_manager: None,
            oauth: None,
            device_oauth: None,
        })
    }

    fn bind_marketplace_manager(
        &mut self,
        manager: Arc<zeta_marketplace_manager::MarketplaceManager>,
    ) -> Result<(), OpenAppServerError> {
        self.marketplace_manager = Some(manager);
        self.reconcile_sources()
    }

    fn reconcile_plugin_activation(&mut self) -> Result<(), OpenAppServerError> {
        let Some(authority) = &self.plugin_authority else {
            return Ok(());
        };
        let snapshot = authority.snapshot();
        let catalog =
            zeta_connectors_extension::ConnectorCatalog::from_activation(snapshot.activation())
                .map_err(|error| OpenAppServerError(error.to_string()))?;
        let mcp = PluginConnectorMcpRuntimeProvider::from_authority(authority)
            .map_err(|error| OpenAppServerError(error.to_string()))?;
        self.base_definitions = catalog
            .snapshot()
            .entries()
            .iter()
            .map(|entry| entry.definition().clone())
            .collect();
        self.base_mcp = Arc::new(mcp);
        self.reconcile_sources()
    }

    fn reconcile_marketplace(&mut self) -> Result<(), OpenAppServerError> {
        self.reconcile_sources()
    }

    fn reconcile_sources(&mut self) -> Result<(), OpenAppServerError> {
        let mut definitions = self.base_definitions.clone();
        self.mcp = Arc::clone(&self.base_mcp);
        if let Some(manager) = &self.marketplace_manager {
            let projection =
                crate::marketplace_connector_runtime::MarketplaceConnectorProjection::from_manager(
                    Arc::clone(manager),
                )
                .map_err(OpenAppServerError)?;
            definitions.extend(projection.definitions().iter().cloned());
            self.mcp = crate::marketplace_connector_runtime::combined_provider(
                Arc::clone(&self.base_mcp),
                projection.provider(),
            );
        }
        self.service
            .authority()
            .reconcile_definitions(definitions)
            .map(|_| ())
            .map_err(|error| OpenAppServerError(error.to_string()))
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
    let product_services = options.product_services.take();
    if options.marketplace_manager_client.is_none()
        && let Some(registry) = product_services
            .as_ref()
            .and_then(crate::LocalProductServicesConfig::marketplace_registry)
            .cloned()
    {
        options = options.with_marketplace_registry(registry)?;
    }
    let codex_app_server = options.codex_app_server.clone();
    let marketplace_manager_client = options.marketplace_manager_client.take();
    let local_marketplace_manager = options.local_marketplace_manager.take();
    let mcp_oauth_providers = std::mem::take(&mut options.mcp_oauth_providers);
    if options.workspace.is_none()
        && let Some(workspace_root) = &options.workspace_root
    {
        options.workspace = Some(default_workspace_config(workspace_root)?);
    }
    let image_store =
        zeta_attachments::FileImageAttachmentStore::open(options.profile_root.join("attachments"))
            .map_err(|error| OpenAppServerError(error.to_string()))?;
    let remote_images = zeta_attachments::SafeRemoteImageFetcher::production()
        .map_err(|error| OpenAppServerError(error.to_string()))?;
    let image_attachments = Arc::new(
        zeta_attachments::ImageAttachments::new(Arc::new(image_store))
            .with_remote_fetcher(Arc::new(remote_images)),
    );
    let (database_path, sessions) = match options.session_state_mode {
        SessionStateMode::Durable => {
            let repository =
                LocalStateRepository::open(&options.profile_root).map_err(open_error)?;
            let database_path = repository.database_path().to_path_buf();
            let sessions = repository
                .recover_coordinator_with_image_attachments(Arc::clone(&image_attachments))
                .map_err(open_error)?;
            (database_path, sessions)
        }
        SessionStateMode::Ephemeral => {
            let threads = Arc::new(ThreadController::with_store_and_image_attachments(
                Arc::new(InMemoryThreadStore::default()),
                Arc::clone(&image_attachments),
            ));
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
    let mut connector_runtime = match options.connector_runtime.take() {
        Some(runtime) => Some(runtime),
        None => {
            let secrets = Arc::new(
                KeyringSecretStore::for_profile(&options.profile_root)
                    .map_err(|error| OpenAppServerError(error.to_string()))?,
            );
            let plugin_authority =
                PluginActivationAuthority::open(options.profile_root.join("plugins"))
                    .map_err(|error| OpenAppServerError(error.to_string()))?;
            Some(LocalConnectorRuntime::from_plugin_authority(
                &options.profile_root,
                plugin_authority,
                secrets,
            )?)
        }
    };
    if let (Some(runtime), Some(manager)) = (&mut connector_runtime, &local_marketplace_manager) {
        runtime.bind_marketplace_manager(Arc::clone(manager))?;
    }
    let managed_node = ManagedNodeRuntime::from_install_context(&InstallContext::current()).ok();
    let marketplace_language_runtime = local_marketplace_manager.as_ref().map(|manager| {
        crate::server::marketplace_language_runtime::MarketplaceLanguageRuntime::new(
            Arc::clone(manager),
            managed_node,
            options.language_server_providers.clone(),
        )
    });
    if let Some(runtime) = &marketplace_language_runtime {
        options.language_server_providers = runtime.registry().map_err(OpenAppServerError)?;
    }
    if let (Some(runtime), Some(services)) = (&mut connector_runtime, product_services) {
        configure_product_connector_oauth(runtime, services.connector_oauth)?;
    }
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
    let codex_runtime = CodexAppServerRuntime::new(codex_app_server);
    let codex_login_driver = CodexAppServerLoginDriver::with_runtime(Arc::clone(&codex_runtime));
    let login_service = Arc::new(LoginService::deferred(codex_login_driver.clone()));
    codex_login_driver
        .install_login_service(&login_service)
        .map_err(|error| OpenAppServerError(error.to_string()))?;
    let direct_catalog: Arc<dyn ModelCatalog> = model.clone();
    let combined_catalog = Arc::new(
        CombinedModelCatalog::new(
            direct_catalog,
            CodexModelCatalog::new(Arc::clone(&codex_runtime)),
        )
        .map_err(|error| OpenAppServerError(error.to_string()))?,
    );
    let mut server = AppServer::new(sessions, model.clone())
        .with_model_catalog(combined_catalog)
        .with_approval_review_model(approval_review_model)
        .with_config_store(Arc::clone(&config))
        .with_login_service(login_service)
        .with_language_server_providers(options.language_server_providers)
        .with_slash_command_catalog(options.slash_commands)
        .with_code_index_storage_root(options.profile_root.join("code-index"))
        .with_symbol_index_storage_root(options.profile_root.join("symbol-index"))
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
    if let Some(runtime) = marketplace_language_runtime {
        server = server
            .with_marketplace_language_runtime(runtime)
            .map_err(OpenAppServerError)?;
    }
    if let Some(manager) = local_marketplace_manager {
        server = server.with_local_marketplace_manager(manager);
    } else if let Some(client) = marketplace_manager_client {
        server = server.with_marketplace_manager_client(client);
    }
    let mcp_updates = McpCatalogUpdates::default();
    let mcp_changes = mcp_updates.subscribe();
    let mcp_runtime_intents = server.mcp_runtime_intents.clone();
    let mcp_runtime_intent_changes = mcp_runtime_intents.subscribe();
    let mcp_runtime_intent_snapshot = mcp_runtime_intents.snapshot();
    let mcp = match &connector_runtime {
        Some(connectors) => compose_mcp_tools_with_connectors_and_runtime_intents_and_updates(
            &runtime_config,
            1,
            &mcp_runtime_intent_snapshot,
            connectors.service.authority().clone(),
            Arc::clone(&connectors.secrets),
            Arc::clone(&connectors.mcp),
            mcp_updates.clone(),
        ),
        None => compose_mcp_tools_at_generation_with_runtime_intents_and_updates(
            &runtime_config,
            1,
            &mcp_runtime_intent_snapshot,
            mcp_updates.clone(),
        ),
    }
    .map_err(|error| OpenAppServerError(error.to_string()))?;
    let mcp_status = mcp
        .as_ref()
        .map(|mcp| mcp.status.clone())
        .unwrap_or_else(|| McpRuntimeStatusSnapshot::empty(1));
    let mcp = mcp.map(|mcp| ToolPort::mcp(mcp.tools, mcp.policy));
    server = server.with_mcp_status_snapshot(mcp_status);
    if let Some(connectors) = &connector_runtime {
        server = server.with_connector_service(Arc::clone(&connectors.service));
        if !mcp_oauth_providers.is_empty() {
            server = server.with_mcp_oauth_service(Arc::new(McpOAuthService::new(
                Arc::clone(&connectors.secrets),
                mcp_oauth_providers,
            )));
        }
        if let Some(authority) = &connectors.plugin_authority {
            server = server.with_plugin_authority(authority.clone());
        }
        if let Some(oauth) = &connectors.oauth {
            server = server.with_connector_oauth_service(Arc::clone(oauth));
        }
        if let Some(oauth) = &connectors.device_oauth {
            server = server.with_connector_device_oauth_service(Arc::clone(oauth));
        }
    }
    server = server
        .with_local_exec_policy_config(crate::local_tools::LocalExecPolicyConfig::from_resolved(
            &runtime_config,
        ))
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
    let (codex_turn_driver, codex_turn_events) = CodexTurnDriver::new(codex_runtime);
    let codex_turn_backend = Arc::new(
        CodexTurnExecutionBackend::new(
            codex_turn_driver,
            codex_turn_events,
            server.sessions().threads().clone(),
            server.thread_update_sink(),
            CodexTurnExecutionBackendOptions::from_source(
                server.codex_workspace_source(),
                CodexThreadAccess::WorkspaceWrite,
            ),
        )
        .map_err(|error| OpenAppServerError(error.to_string()))?,
    );
    server = server
        .with_codex_turn_backend(codex_turn_backend)
        .map_err(|error| OpenAppServerError(error.to_string()))?;
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
    server = server.with_tool_config_watcher(ToolConfigWatcher::start(ToolConfigWatcherInputs {
        config,
        workspace,
        workspace_tools,
        workspace_runtime,
        connector_runtime,
        mcp_runtime_intents,
        mcp_updates,
        mcp_changes,
        mcp_runtime_intent_changes,
    }));
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

struct ToolConfigWatcherInputs {
    config: Arc<ConfigStore>,
    workspace: Option<Arc<WorkspaceConfigTracker>>,
    workspace_tools: Arc<WorkspaceToolPorts>,
    workspace_runtime: crate::server::WorkspaceRuntimeControl,
    connector_runtime: Option<LocalConnectorRuntime>,
    mcp_runtime_intents: crate::mcp_runtime::McpRuntimeIntents,
    mcp_updates: McpCatalogUpdates,
    mcp_changes: McpCatalogUpdateSubscription,
    mcp_runtime_intent_changes: std::sync::mpsc::Receiver<()>,
}

impl ToolConfigWatcher {
    fn start(inputs: ToolConfigWatcherInputs) -> Self {
        let ToolConfigWatcherInputs {
            config,
            workspace,
            workspace_tools,
            workspace_runtime,
            mut connector_runtime,
            mcp_runtime_intents,
            mcp_updates,
            mcp_changes,
            mcp_runtime_intent_changes,
        } = inputs;
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
        let plugin_changes = connector_runtime
            .as_ref()
            .and_then(|runtime| runtime.plugin_authority.as_ref())
            .map(PluginActivationAuthority::subscribe);
        let marketplace_changes = connector_runtime
            .as_ref()
            .and_then(|runtime| runtime.marketplace_manager.as_ref())
            .and_then(|manager| manager.subscribe().ok());
        let mut plugin_activation_generation = connector_runtime
            .as_ref()
            .and_then(|runtime| runtime.plugin_authority.as_ref())
            .map(|authority| authority.snapshot().activation().generation());
        let (shutdown, shutdown_receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-tool-config".into())
            .spawn(move || {
                let mut catalog_generation = 1_u64;
                let mut workspace_revision = workspace
                    .as_ref()
                    .and_then(|workspace| workspace.read().ok().map(|(_, revision)| revision));
                let mut config_dirty = false;
                let mut connector_dirty = false;
                let mut mcp_dirty = false;
                let mut mcp_runtime_intent_dirty = false;
                let mut plugin_dirty = false;
                let mut marketplace_dirty = false;
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        break;
                    }
                    config_dirty |= match changes.recv_timeout(Duration::from_millis(100)) {
                        Ok(_) => {
                            while changes.try_recv().is_ok() {}
                            true
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if let Some(workspace) = &workspace {
                        match workspace.read() {
                            Ok((_, revision)) if workspace_revision != Some(revision) => {
                                workspace_revision = Some(revision);
                                config_dirty = true;
                            }
                            Ok(_) => {}
                            Err(error) => {
                                workspace_tools.record_reconcile_failure(error.to_string());
                                continue;
                            }
                        }
                    }
                    if let Some(connector_changes) = &connector_changes {
                        while connector_changes.try_recv().is_ok() {
                            connector_dirty = true;
                        }
                    }
                    while mcp_changes.try_recv().is_ok() {
                        mcp_dirty = true;
                    }
                    while mcp_runtime_intent_changes.try_recv().is_ok() {
                        mcp_runtime_intent_dirty = true;
                    }
                    if let Some(plugin_changes) = &plugin_changes {
                        while let Ok(change) = plugin_changes.try_recv() {
                            if plugin_activation_generation != Some(change.activation_generation) {
                                plugin_activation_generation = Some(change.activation_generation);
                                plugin_dirty = true;
                            }
                        }
                    }
                    if let Some(marketplace_changes) = &marketplace_changes {
                        while marketplace_changes.try_recv().is_ok() {
                            marketplace_dirty = true;
                        }
                    }
                    if !config_dirty
                        && !connector_dirty
                        && !mcp_dirty
                        && !plugin_dirty
                        && !marketplace_dirty
                        && !mcp_runtime_intent_dirty
                    {
                        continue;
                    }
                    let snapshot = match config.read_snapshot() {
                        Ok(snapshot) => snapshot,
                        Err(error) => {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                    };
                    if config_dirty {
                        if let Err(error) = workspace_runtime.reconcile_user_trust(&snapshot.values)
                        {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                        if let Err(error) =
                            workspace_runtime.reconcile_hooks(&snapshot.values.hooks)
                        {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                        let next_semantic_binding = (
                            snapshot.values.semantic_code_index.clone(),
                            snapshot.values.providers.clone(),
                        );
                        if semantic_binding.as_ref() != Some(&next_semantic_binding) {
                            if let Err(error) =
                                workspace_runtime.reconcile_semantic_code_index_runtime()
                            {
                                workspace_tools.record_reconcile_failure(error.to_string());
                                continue;
                            }
                            semantic_binding = Some(next_semantic_binding);
                        }
                        let runtime_config =
                            match resolve_local_config(&snapshot, workspace.as_deref()) {
                                Ok(config) => config,
                                Err(error) => {
                                    workspace_tools.record_reconcile_failure(error.to_string());
                                    continue;
                                }
                            };
                        if let Err(error) =
                            workspace_runtime.reconcile_exec_policy_config(&runtime_config)
                        {
                            workspace_tools.record_reconcile_failure(error.to_string());
                            continue;
                        }
                    }
                    if plugin_dirty
                        && let Some(connectors) = connector_runtime.as_mut()
                        && let Err(error) = connectors.reconcile_plugin_activation()
                    {
                        workspace_tools.record_reconcile_failure(error.to_string());
                        continue;
                    }
                    if marketplace_dirty
                        && let Some(connectors) = connector_runtime.as_mut()
                        && let Err(error) = connectors.reconcile_marketplace()
                    {
                        workspace_tools.record_reconcile_failure(error.to_string());
                        continue;
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
                        Some(connectors) => {
                            let runtime_intents = mcp_runtime_intents.snapshot();
                            compose_mcp_tools_with_connectors_and_runtime_intents_and_updates(
                                &snapshot.values,
                                catalog_generation,
                                &runtime_intents,
                                connectors.service.authority().clone(),
                                Arc::clone(&connectors.secrets),
                                Arc::clone(&connectors.mcp),
                                mcp_updates.clone(),
                            )
                        }
                        None => {
                            let runtime_intents = mcp_runtime_intents.snapshot();
                            compose_mcp_tools_at_generation_with_runtime_intents_and_updates(
                                &snapshot.values,
                                catalog_generation,
                                &runtime_intents,
                                mcp_updates.clone(),
                            )
                        }
                    };
                    let (mcp, mcp_status) = match composition {
                        Ok(Some(mcp)) => {
                            let status = mcp.status.clone();
                            (Some(ToolPort::mcp(mcp.tools, mcp.policy)), status)
                        }
                        Ok(None) => (None, McpRuntimeStatusSnapshot::empty(catalog_generation)),
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
                        continue;
                    }
                    workspace_runtime.replace_mcp_status(mcp_status);
                    config_dirty = false;
                    connector_dirty = false;
                    mcp_dirty = false;
                    mcp_runtime_intent_dirty = false;
                    plugin_dirty = false;
                    marketplace_dirty = false;
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
    let mut roots = Vec::new();
    if let Some(root) = InstallContext::current()
        .bundled_resource_directory("extensions")
        .or_else(development_extension_root)
    {
        roots.push(ExtensionRoot::built_in(root));
    }
    roots.push(ExtensionRoot::user(profile_root.join("extensions")));
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

    fn stream(
        &self,
        selection: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn CoreModelStreamSink,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        let config = self.config_for_selection(selection)?;
        ProviderModelService::new(self.resolver.resolve(&config)).stream(
            ModelSelection::ConfiguredDefault,
            request,
            cancellation,
            sink,
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
        resolve_local_config(user, self.workspace.as_deref()).map_err(|error| {
            CoreError::Model(format!("failed to resolve Workspace config: {}", error.0))
        })
    }
}

fn resolve_local_config(
    user: &ResolvedConfigSnapshot,
    workspace: Option<&WorkspaceConfigTracker>,
) -> Result<ResolvedConfig, zeta_config::ConfigError> {
    let Some(workspace) = workspace else {
        return Ok(user.values.clone());
    };
    let (document, revision) = workspace.read()?;
    resolve_scoped_config(
        user,
        Some(WorkspaceConfigInput::new(
            workspace.scope(),
            revision,
            &document,
        )),
    )
    .map(|resolved| resolved.values)
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

    fn stream(
        &self,
        _: ModelSelection<'_>,
        request: &zeta_protocol::ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn CoreModelStreamSink,
    ) -> Result<zeta_protocol::ModelResponse, CoreError> {
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        let mut adapter = CoreProviderStreamSink {
            inner: sink,
            failure: None,
        };
        let response = self
            .invoker
            .stream_with_cancellation(request, cancellation, &mut adapter);
        if let Some(error) = adapter.failure {
            return Err(error);
        }
        let response = response.map_err(map_model_provider_error)?;
        cancellation
            .check()
            .map_err(|signal| CoreError::Cancelled(signal.reason().to_string()))?;
        Ok(response)
    }
}

struct CoreProviderStreamSink<'a> {
    inner: &'a mut dyn CoreModelStreamSink,
    failure: Option<CoreError>,
}

impl ModelEventSink for CoreProviderStreamSink<'_> {
    fn emit(
        &mut self,
        event: zeta_protocol::ModelStreamEvent,
    ) -> Result<(), zeta_model_provider::ModelProviderError> {
        if let Err(error) = self.inner.emit(event) {
            self.failure = Some(error);
            return Err(zeta_model_provider::ModelProviderError::Unavailable(
                "model stream consumer rejected an event".into(),
            ));
        }
        Ok(())
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

fn configure_product_connector_oauth(
    runtime: &mut LocalConnectorRuntime,
    configurations: Vec<crate::product_services::ProductConnectorOAuthConfig>,
) -> Result<(), OpenAppServerError> {
    let http: Arc<dyn zeta_http_client::HttpClient> = Arc::new(
        zeta_http_client::UreqHttpClient::new()
            .map_err(|error| OpenAppServerError(error.to_string()))?,
    );
    let mut browser = Vec::new();
    let mut device = Vec::new();
    for configuration in configurations {
        match configuration {
            crate::product_services::ProductConnectorOAuthConfig::GitHubBrokered {
                connector_id,
                config,
            } => {
                let provider = zeta_connectors_extension::GitHubBrokeredOAuthProvider::new(
                    config,
                    Arc::clone(&http),
                )
                .map_err(|error| OpenAppServerError(error.to_string()))?;
                browser.push((
                    connector_id,
                    Arc::new(provider)
                        as Arc<dyn zeta_connectors_extension::ConnectorOAuthProvider>,
                ));
            }
            crate::product_services::ProductConnectorOAuthConfig::GitHubDevice {
                connector_id,
                config,
            } => {
                let provider = zeta_connectors_extension::GitHubDeviceOAuthProvider::new(
                    config,
                    Arc::clone(&http),
                )
                .map_err(|error| OpenAppServerError(error.to_string()))?;
                device.push((
                    connector_id,
                    Arc::new(provider)
                        as Arc<dyn zeta_connectors_extension::ConnectorDeviceOAuthProvider>,
                ));
            }
        }
    }
    if !browser.is_empty() {
        runtime.oauth = Some(Arc::new(
            zeta_connectors_extension::ConnectorOAuthService::new(
                Arc::clone(&runtime.service),
                browser,
            ),
        ));
    }
    if !device.is_empty() {
        runtime.device_oauth = Some(Arc::new(
            zeta_connectors_extension::ConnectorDeviceOAuthService::new(
                Arc::clone(&runtime.service),
                device,
            ),
        ));
    }
    Ok(())
}

fn open_error(error: impl fmt::Display) -> OpenAppServerError {
    OpenAppServerError(error.to_string())
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
