use crate::AppServer;
use crate::SlashCommandCatalog;
use crate::mcp_tools::compose_mcp_tools;
use crate::model_catalog::ModelCatalog;
use crate::server::WorkspaceSwitchTrustPolicy;
use crate::server::WorkspaceToolPorts;
use crate::server::skills_runtime::{BuiltInSkillSource, SkillConfigSnapshotProvider};
use crate::tool_composition::ToolPort;
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigStore, ResolvedConfig, ResolvedConfigSnapshot, WorkspaceConfigDocument,
    WorkspaceConfigInput, WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceConfigStore,
    WorkspaceId, resolve_scoped_config,
};
use zeta_core::{CoreError, HarnessInstructions, ModelSelection, ModelService};
use zeta_extensions::ExtensionRoot;
use zeta_install_context::InstallContext;
use zeta_model_provider::{
    ModelInvoker, ModelProvider, ModelProviderRuntime, ModelRuntimeRequest, UnavailableModel,
};
use zeta_model_provider_config::ProviderConfigRegistry;
use zeta_prompts::SYSTEM_PROMPT;
use zeta_rollout::LocalStateRepository;

const MAX_WORKSPACE_INSTRUCTIONS_BYTES: usize = 32 * 1024;
const MAX_GIT_STATUS_LINES: usize = 40;

pub(crate) fn harness_instructions(workspace_root: &Path) -> HarnessInstructions {
    HarnessInstructions::new(
        SYSTEM_PROMPT.body(),
        render_environment(workspace_root),
        read_workspace_instructions(workspace_root),
    )
}

fn render_environment(workspace_root: &Path) -> String {
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

fn read_workspace_instructions(workspace_root: &Path) -> Option<String> {
    let path = workspace_root.join("AGENTS.md");
    let mut bytes = std::fs::read(path).ok()?;
    let truncated = bytes.len() > MAX_WORKSPACE_INSTRUCTIONS_BYTES;
    bytes.truncate(MAX_WORKSPACE_INSTRUCTIONS_BYTES);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        text.push_str("\n[... AGENTS.md truncated at 32768 bytes ...]");
    }
    Some(text)
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

/// Filesystem locations needed to open one persistent local App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAppServerOptions {
    pub profile_root: PathBuf,
    pub workspace: Option<LocalWorkspaceConfigOptions>,
    pub slash_commands: SlashCommandCatalog,
    pub workspace_root: Option<PathBuf>,
    pub built_in_skills: BuiltInSkillRoot,
}

impl LocalAppServerOptions {
    pub fn new(profile_root: impl Into<PathBuf>) -> Self {
        Self {
            profile_root: profile_root.into(),
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
    mut options: LocalAppServerOptions,
) -> Result<AppServer, OpenAppServerError> {
    if options.workspace.is_none()
        && let Some(workspace_root) = &options.workspace_root
    {
        options.workspace = Some(default_workspace_config(workspace_root)?);
    }
    let repository = LocalStateRepository::open(&options.profile_root).map_err(open_error)?;
    let config = Arc::new(
        ConfigStore::open_with_paths(
            repository.database_path(),
            options.profile_root.join("config.toml"),
        )
        .map_err(|error| OpenAppServerError(error.0))?,
    );
    let sessions = repository.recover_coordinator().map_err(open_error)?;
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
    });
    let built_in_skill_root = resolve_built_in_skill_root(options.built_in_skills);
    let extension_roots = resolve_extension_roots(&options.profile_root);
    let mut server = AppServer::new(sessions, model.clone())
        .with_model_catalog(model)
        .with_config_store(Arc::clone(&config))
        .with_slash_command_catalog(options.slash_commands)
        .with_extension_roots(extension_roots)
        .with_skill_runtime(built_in_skill_root, skill_config)
        .map_err(OpenAppServerError)?;
    let mcp = compose_mcp_tools(&runtime_config, user_config.generation)
        .map_err(|error| OpenAppServerError(error.to_string()))?
        .map(|mcp| ToolPort::mcp(mcp.tools, mcp.policy));
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
    ) -> Self {
        let changes = config.subscribe_changes();
        let (shutdown, shutdown_receiver) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("zeta-tool-config".into())
            .spawn(move || {
                loop {
                    if shutdown_receiver.try_recv().is_ok() {
                        break;
                    }
                    match changes.recv_timeout(Duration::from_millis(100)) {
                        Ok(mut change) => {
                            while let Ok(next) = changes.try_recv() {
                                change = next;
                            }
                            let snapshot = match config.read_snapshot() {
                                Ok(snapshot) => snapshot,
                                Err(error) => {
                                    workspace_tools.record_reconcile_failure(error.to_string());
                                    continue;
                                }
                            };
                            if let Err(error) =
                                workspace_runtime.reconcile_user_trust(&snapshot.values)
                            {
                                workspace_tools.record_reconcile_failure(error.to_string());
                                continue;
                            }
                            let mcp = match compose_mcp_tools(&snapshot.values, change.generation) {
                                Ok(mcp) => mcp.map(|mcp| ToolPort::mcp(mcp.tools, mcp.policy)),
                                Err(error) => {
                                    workspace_tools.record_reconcile_failure(error.to_string());
                                    continue;
                                }
                            };
                            if let Err(error) = workspace_tools.replace_mcp(mcp) {
                                workspace_tools.record_reconcile_failure(error.to_string());
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
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
                error if error.is_transient() => CoreError::ModelTransient(error.to_string()),
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
