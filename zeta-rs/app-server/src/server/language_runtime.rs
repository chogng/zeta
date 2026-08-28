use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use zeta_async_utils::CancellationToken;
use zeta_config::LanguageServerConfig;
use zeta_config::LanguageServerModeConfig;
use zeta_config::LanguageServersConfig;
use zeta_install_context::InstallContext;
use zeta_lsp_manager::LanguageRequestId;
use zeta_lsp_manager::LanguageRequestMetric;
use zeta_lsp_manager::LanguageServerMessageSeverity;
use zeta_lsp_manager::LanguageServerMessageSource;
use zeta_lsp_manager::LanguageServerState;
use zeta_lsp_manager::LspDocumentSnapshot;
use zeta_lsp_manager::LspManager;
use zeta_lsp_manager::LspManagerConfiguration;
use zeta_lsp_manager::LspManagerEvent;
use zeta_lsp_manager::LspManagerEventSink;
use zeta_lsp_manager::LspManagerNotification;
use zeta_lsp_manager::LspManagerRequestResult;
use zeta_lsp_manager::LspRequestMetricsSink;
use zeta_lsp_server_provider::BASH_LANGUAGE_SERVER_ID;
use zeta_lsp_server_provider::JSON_LANGUAGE_SERVER_ID;
use zeta_lsp_server_provider::LanguageServerExecutionPolicy;
use zeta_lsp_server_provider::LanguageServerPreference;
use zeta_lsp_server_provider::LspServerLaunch;
use zeta_lsp_server_provider::LspServerProviders;
use zeta_lsp_server_provider::LspServerResolver;
use zeta_lsp_server_provider::RUST_ANALYZER_SERVER_ID;
use zeta_lsp_server_provider::TYPESCRIPT_LANGUAGE_SERVER_ID;

use zeta_app_server_protocol::protocol::language::LanguageDiagnosticsNotification;
use zeta_app_server_protocol::protocol::language::LanguageServerMessageNotification;
use zeta_app_server_protocol::protocol::language::LanguageServerMessageSeverityDto;
use zeta_app_server_protocol::protocol::language::LanguageServerMessageSourceDto;
use zeta_app_server_protocol::protocol::language::LanguageServerProgressNotification;
use zeta_app_server_protocol::protocol::language::LanguageServerStateDto;
use zeta_app_server_protocol::protocol::language::LanguageServerStateNotification;

use super::language_operations::diagnostic_to_dto;
use super::update_broker::UpdateBroker;

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

struct AppServerLanguageMetrics;

impl LspRequestMetricsSink for AppServerLanguageMetrics {
    fn record(&self, metric: LanguageRequestMetric) {
        log::debug!(
            target: "zeta_lsp_manager",
            "request completed: kind={:?} server={} incarnation={} config_generation={} service_generation={} cold={} elapsed_ms={} results={} outcome={:?}",
            metric.kind,
            metric.server.as_deref().unwrap_or("none"),
            metric.server_incarnation.map_or_else(|| "none".into(), |value| value.to_string()),
            metric.configuration_generation,
            metric.service_generation,
            metric.cold_for_incarnation,
            metric.elapsed_millis,
            metric.result_count,
            metric.outcome,
        );
    }
}

struct AppServerLanguageEventSink {
    sender: mpsc::Sender<LspManagerEvent>,
    diagnostics: Arc<LanguageDiagnosticPublisher>,
}

impl LspManagerEventSink for AppServerLanguageEventSink {
    fn on_event(&self, event: LspManagerEvent) {
        if let LspManagerEvent::Notification(LspManagerNotification::Diagnostics(diagnostics)) =
            &event
        {
            self.diagnostics.publish(diagnostics);
            return;
        }
        if let LspManagerEvent::Notification(LspManagerNotification::ServerMessage {
            server,
            severity,
            source,
            show,
            message,
        }) = &event
        {
            self.diagnostics.updates.publish_language_server_message(
                LanguageServerMessageNotification {
                    workspace_folder_id: self.diagnostics.workspace_folder_id.clone(),
                    server: server.clone(),
                    severity: match severity {
                        LanguageServerMessageSeverity::Error => {
                            LanguageServerMessageSeverityDto::Error
                        }
                        LanguageServerMessageSeverity::Warning => {
                            LanguageServerMessageSeverityDto::Warning
                        }
                        LanguageServerMessageSeverity::Information => {
                            LanguageServerMessageSeverityDto::Information
                        }
                        LanguageServerMessageSeverity::Log => LanguageServerMessageSeverityDto::Log,
                    },
                    source: match source {
                        LanguageServerMessageSource::Protocol => {
                            LanguageServerMessageSourceDto::Protocol
                        }
                        LanguageServerMessageSource::Stderr => {
                            LanguageServerMessageSourceDto::Stderr
                        }
                        LanguageServerMessageSource::Service => {
                            LanguageServerMessageSourceDto::Service
                        }
                    },
                    show: *show,
                    message: message.clone(),
                },
            );
            return;
        }
        if let LspManagerEvent::Notification(LspManagerNotification::ServerProgress(progress)) =
            &event
        {
            self.diagnostics.updates.publish_language_server_progress(
                LanguageServerProgressNotification {
                    workspace_folder_id: self.diagnostics.workspace_folder_id.clone(),
                    server: progress.server.clone(),
                    token: progress.token.clone(),
                    title: progress.title.clone(),
                    message: progress.message.clone(),
                    percentage: progress.percentage,
                    done: progress.done,
                },
            );
            return;
        }
        if let LspManagerEvent::Notification(LspManagerNotification::ServerStateChanged {
            server,
            state,
        }) = &event
        {
            self.diagnostics.updates.publish_language_server_state(
                LanguageServerStateNotification {
                    workspace_folder_id: self.diagnostics.workspace_folder_id.clone(),
                    server: server.clone(),
                    state: language_server_state_to_dto(state),
                },
            );
        }
        let _ = self.sender.send(event);
    }
}

fn language_server_state_to_dto(state: &LanguageServerState) -> LanguageServerStateDto {
    match state {
        LanguageServerState::Starting => LanguageServerStateDto::Starting,
        LanguageServerState::Ready => LanguageServerStateDto::Ready,
        LanguageServerState::BackingOff {
            attempt,
            retry_after,
        } => LanguageServerStateDto::BackingOff {
            attempt: *attempt,
            retry_after_millis: retry_after.as_millis().min(u128::from(u64::MAX)) as u64,
        },
        LanguageServerState::CrashLoop {
            restart_attempts,
            message,
        } => LanguageServerStateDto::CrashLoop {
            restart_attempts: *restart_attempts,
            message: message.clone(),
        },
        LanguageServerState::Failed(message) => LanguageServerStateDto::Failed {
            message: message.clone(),
        },
        LanguageServerState::Stopped => LanguageServerStateDto::Stopped,
    }
}

#[derive(Clone)]
struct AppServerLanguageDocumentSnapshot {
    relative_path: PathBuf,
    revision: zeta_lsp_manager::LanguageDocumentRevision,
    text: String,
}

struct LanguageDiagnosticPublisher {
    documents: Arc<Mutex<BTreeMap<PathBuf, AppServerLanguageDocumentSnapshot>>>,
    updates: Arc<UpdateBroker>,
    workspace_folder_id: Option<String>,
}

impl LanguageDiagnosticPublisher {
    fn publish(&self, diagnostics: &zeta_lsp_manager::LanguageDiagnostics) {
        let snapshot = self
            .documents
            .lock()
            .ok()
            .and_then(|documents| documents.get(diagnostics.path()).cloned());
        let Some(snapshot) =
            snapshot.filter(|snapshot| snapshot.revision == diagnostics.revision())
        else {
            return;
        };
        let diagnostics = diagnostics
            .diagnostics()
            .iter()
            .cloned()
            .filter_map(|diagnostic| diagnostic_to_dto(&snapshot.text, diagnostic))
            .collect();
        self.updates
            .publish_language_diagnostics(LanguageDiagnosticsNotification {
                workspace_folder_id: self.workspace_folder_id.clone(),
                path: snapshot.relative_path,
                revision: snapshot.revision.value(),
                diagnostics,
            });
    }
}

struct AppServerLanguageWorkspaceState {
    documents: Arc<Mutex<BTreeMap<PathBuf, AppServerLanguageDocumentSnapshot>>>,
    root: Option<PathBuf>,
    config_generation: Option<u64>,
    language_servers: BTreeMap<String, String>,
    server_states: BTreeMap<String, LanguageServerState>,
    workspace_folder_id: Option<String>,
}

impl Default for AppServerLanguageWorkspaceState {
    fn default() -> Self {
        Self {
            documents: Arc::new(Mutex::new(BTreeMap::new())),
            root: None,
            config_generation: None,
            language_servers: BTreeMap::new(),
            server_states: BTreeMap::new(),
            workspace_folder_id: None,
        }
    }
}

pub(super) struct AppServerLanguageRuntime {
    pub(super) manager: Option<LspManager>,
    receiver: Option<mpsc::Receiver<LspManagerEvent>>,
    updates: Arc<UpdateBroker>,
    workspace: AppServerLanguageWorkspaceState,
    providers: LspServerProviders,
    workspace_runtimes: BTreeMap<PathBuf, Box<AppServerLanguageRuntime>>,
    active_workspace_root: Option<PathBuf>,
}

impl AppServerLanguageRuntime {
    pub(super) fn new(updates: Arc<UpdateBroker>) -> Self {
        Self {
            manager: None,
            receiver: None,
            updates,
            workspace: AppServerLanguageWorkspaceState::default(),
            providers: LspServerProviders::new(),
            workspace_runtimes: BTreeMap::new(),
            active_workspace_root: None,
        }
    }

    pub(super) fn set_server_providers(&mut self, providers: LspServerProviders) {
        self.shutdown();
        self.providers = providers;
    }

    pub(super) fn reset_workspace(&mut self) {
        self.shutdown();
    }

    pub(super) fn retain_workspace_roots(&mut self, roots: &std::collections::BTreeSet<PathBuf>) {
        self.workspace_runtimes
            .retain(|root, _| roots.contains(root));
        if self
            .active_workspace_root
            .as_ref()
            .is_some_and(|root| !roots.contains(root))
        {
            self.active_workspace_root = None;
        }
    }

    pub(super) fn ensure(
        &mut self,
        workspace_root: &Path,
        workspace_folder_id: Option<&str>,
        config_generation: u64,
        configuration: &LanguageServersConfig,
        language_id: &str,
    ) -> Result<&LspManager, String> {
        if self.workspace.root.is_some() && self.workspace.root.as_deref() != Some(workspace_root) {
            self.active_workspace_root = Some(workspace_root.to_path_buf());
            let runtime = self
                .workspace_runtimes
                .entry(workspace_root.to_path_buf())
                .or_insert_with(|| {
                    let mut runtime = AppServerLanguageRuntime::new(Arc::clone(&self.updates));
                    runtime.providers = self.providers.clone();
                    Box::new(runtime)
                });
            return runtime.ensure(
                workspace_root,
                workspace_folder_id,
                config_generation,
                configuration,
                language_id,
            );
        }
        self.active_workspace_root = None;
        self.workspace.workspace_folder_id = workspace_folder_id.map(str::to_owned);
        if self.workspace.root.as_deref() != Some(workspace_root)
            || self.workspace.config_generation != Some(config_generation)
        {
            self.restart(workspace_root, config_generation, configuration)?;
        }
        self.wait_until_ready(language_id)?;
        self.manager
            .as_ref()
            .ok_or_else(|| "language service is unavailable".into())
    }

    pub(super) fn manager(&self) -> Option<&LspManager> {
        match &self.active_workspace_root {
            Some(root) => self
                .workspace_runtimes
                .get(root)
                .and_then(|runtime| runtime.manager()),
            None => self.manager.as_ref(),
        }
    }

    pub(super) fn wait_for_request(
        &mut self,
        request_id: LanguageRequestId,
        cancellation: &CancellationToken,
    ) -> Result<LspManagerRequestResult, String> {
        if let Some(root) = self.active_workspace_root.clone() {
            return self
                .workspace_runtimes
                .get_mut(&root)
                .ok_or_else(|| String::from("language workspace runtime is unavailable"))?
                .wait_for_request(request_id, cancellation);
        }
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        loop {
            if cancellation.is_cancelled() {
                if let Some(manager) = self.manager() {
                    let _ = manager.cancel_request(request_id);
                }
                return Err("language request cancelled".into());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("language service request timed out".into());
            }
            let event = self
                .receiver
                .as_ref()
                .ok_or_else(|| String::from("language service event channel is unavailable"))?
                .recv_timeout(remaining.min(Duration::from_millis(25)))
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => String::new(),
                    mpsc::RecvTimeoutError::Disconnected => {
                        String::from("language service event channel is unavailable")
                    }
                });
            let event = match event {
                Ok(event) => event,
                Err(message) if message.is_empty() => continue,
                Err(message) => return Err(message),
            };
            let matches = matches!(
                &event,
                LspManagerEvent::RequestResult(result) if result.request_id() == request_id
            );
            self.accept_state(&event);
            if matches {
                let LspManagerEvent::RequestResult(result) = event else {
                    unreachable!("request result match must contain a request result")
                };
                return Ok(result);
            }
        }
    }

    pub(super) fn synchronize_document(
        &mut self,
        workspace_root: &Path,
        workspace_folder_id: Option<&str>,
        config_generation: u64,
        configuration: &LanguageServersConfig,
        relative_path: &Path,
        document: LspDocumentSnapshot,
    ) -> Result<(), String> {
        let language_id = document.language_id().to_owned();
        self.ensure(
            workspace_root,
            workspace_folder_id,
            config_generation,
            configuration,
            &language_id,
        )?;
        if let Some(root) = self.active_workspace_root.clone() {
            return self
                .workspace_runtimes
                .get_mut(&root)
                .ok_or_else(|| String::from("language workspace runtime is unavailable"))?
                .synchronize_selected_document(relative_path, document);
        }
        self.synchronize_selected_document(relative_path, document)
    }

    fn synchronize_selected_document(
        &mut self,
        relative_path: &Path,
        document: LspDocumentSnapshot,
    ) -> Result<(), String> {
        self.workspace
            .documents
            .lock()
            .map_err(|_| String::from("language document snapshots are unavailable"))?
            .insert(
                document.path().to_path_buf(),
                AppServerLanguageDocumentSnapshot {
                    relative_path: relative_path.to_path_buf(),
                    revision: document.revision(),
                    text: document.text().to_owned(),
                },
            );
        self.manager
            .as_ref()
            .ok_or_else(|| String::from("language service is unavailable"))?
            .synchronize_document(document)
            .map_err(|error| error.to_string())
    }

    pub(super) fn close_document(
        &mut self,
        workspace_root: &Path,
        path: &Path,
    ) -> Result<(), String> {
        if self.workspace.root.as_deref() != Some(workspace_root) {
            return self
                .workspace_runtimes
                .get_mut(workspace_root)
                .map_or(Ok(()), |runtime| runtime.close_selected_document(path));
        }
        self.close_selected_document(path)
    }

    fn close_selected_document(&mut self, path: &Path) -> Result<(), String> {
        self.workspace
            .documents
            .lock()
            .map_err(|_| String::from("language document snapshots are unavailable"))?
            .remove(path);
        if let Some(manager) = &self.manager {
            manager
                .close_document(path)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn restart(
        &mut self,
        workspace_root: &Path,
        config_generation: u64,
        configuration: &LanguageServersConfig,
    ) -> Result<(), String> {
        self.shutdown_selected();
        let resolver = LspServerResolver::new(preference(configuration, RUST_ANALYZER_SERVER_ID))
            .with_json_language_server(preference(configuration, JSON_LANGUAGE_SERVER_ID))
            .with_bash_language_server(preference(configuration, BASH_LANGUAGE_SERVER_ID))
            .with_typescript_language_server(preference(
                configuration,
                TYPESCRIPT_LANGUAGE_SERVER_ID,
            ));
        let resolution = resolver
            .resolve(
                &InstallContext::current(),
                LanguageServerExecutionPolicy::Allowed,
                workspace_root,
            )
            .map_err(|error| error.to_string())?;
        let mut definitions = resolution.into_definitions();
        definitions.extend(configured_provider_definitions(
            &self.providers,
            configuration,
            workspace_root,
        )?);
        self.workspace.language_servers = definitions
            .iter()
            .flat_map(|definition| {
                let server = definition.name().to_string();
                definition
                    .language_ids()
                    .map(move |language| (language.to_owned(), server.clone()))
            })
            .collect();
        if definitions.is_empty() {
            return Err("no configured language-server executable is available".into());
        }
        let (sender, receiver) = mpsc::channel();
        let manager = LspManager::start_with_events_and_metrics(
            LspManagerConfiguration::enabled(workspace_root, definitions)
                .with_generation(config_generation),
            Arc::new(AppServerLanguageEventSink {
                sender,
                diagnostics: Arc::new(LanguageDiagnosticPublisher {
                    documents: Arc::clone(&self.workspace.documents),
                    updates: Arc::clone(&self.updates),
                    workspace_folder_id: self.workspace.workspace_folder_id.clone(),
                }),
            }),
            Arc::new(AppServerLanguageMetrics),
        )
        .map_err(|error| error.to_string())?;
        self.manager = Some(manager);
        self.receiver = Some(receiver);
        self.workspace.root = Some(workspace_root.to_path_buf());
        self.workspace.config_generation = Some(config_generation);
        Ok(())
    }

    fn wait_until_ready(&mut self, language_id: &str) -> Result<(), String> {
        let server = self
            .workspace
            .language_servers
            .get(language_id)
            .cloned()
            .ok_or_else(|| format!("no language server is available for '{language_id}'"))?;
        if matches!(
            self.workspace.server_states.get(&server),
            Some(LanguageServerState::Ready)
        ) {
            return Ok(());
        }
        let deadline = Instant::now() + SERVER_START_TIMEOUT;
        loop {
            let event = self.recv_until(deadline)?;
            self.accept_state(&event);
            match self.workspace.server_states.get(&server) {
                Some(LanguageServerState::Ready) => return Ok(()),
                Some(LanguageServerState::Failed(message)) => return Err(message.clone()),
                Some(LanguageServerState::CrashLoop { message, .. }) => return Err(message.clone()),
                _ => {}
            }
        }
    }

    fn recv_until(&self, deadline: Instant) -> Result<LspManagerEvent, String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("language service request timed out".into());
        }
        self.receiver
            .as_ref()
            .ok_or_else(|| String::from("language service event channel is unavailable"))?
            .recv_timeout(remaining)
            .map_err(|_| "language service request timed out".into())
    }

    fn accept_state(&mut self, event: &LspManagerEvent) {
        if let LspManagerEvent::Notification(LspManagerNotification::ServerStateChanged {
            server,
            state,
        }) = event
        {
            self.workspace
                .server_states
                .insert(server.clone(), state.clone());
        }
    }

    fn shutdown(&mut self) {
        self.workspace_runtimes.clear();
        self.active_workspace_root = None;
        self.shutdown_selected();
    }

    fn shutdown_selected(&mut self) {
        if let Ok(mut documents) = self.workspace.documents.lock() {
            documents.clear();
        }
        if let Some(manager) = self.manager.take() {
            let _ = manager.shutdown();
        }
        self.receiver = None;
        self.workspace.root = None;
        self.workspace.config_generation = None;
        self.workspace.language_servers.clear();
        self.workspace.server_states.clear();
    }
}

fn configured_provider_definitions(
    providers: &LspServerProviders,
    configuration: &LanguageServersConfig,
    workspace_root: &Path,
) -> Result<Vec<zeta_lsp_manager::LanguageServerDefinition>, String> {
    let mut definitions = Vec::new();
    for server_id in providers.ids() {
        let config = configuration
            .servers
            .iter()
            .find_map(|(id, config)| (id.as_str() == server_id).then_some(config));
        if config.is_none() && !providers.activation_enables(server_id) {
            continue;
        }
        if config.is_some_and(|config| config.mode == LanguageServerModeConfig::Disabled) {
            continue;
        }
        let launch = config
            .and_then(|config| config.executable.as_deref())
            .map_or(
                LspServerLaunch::Packaged,
                LspServerLaunch::ExplicitExecutable,
            );
        if let Some(definition) = providers
            .definition(server_id, workspace_root, launch)
            .map_err(|error| error.to_string())?
        {
            definitions.push(definition);
        }
    }
    Ok(definitions)
}

impl Drop for AppServerLanguageRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn preference(configuration: &LanguageServersConfig, server_id: &str) -> LanguageServerPreference {
    let configured = configuration
        .servers
        .iter()
        .find_map(|(id, config)| (id.as_str() == server_id).then_some(config));
    let default = LanguageServerConfig::default();
    let configured = configured.unwrap_or(&default);
    let preference = match configured.mode {
        LanguageServerModeConfig::Disabled => LanguageServerPreference::disabled(),
        LanguageServerModeConfig::Automatic => LanguageServerPreference::automatic(),
        LanguageServerModeConfig::Enabled => LanguageServerPreference::enabled(),
    };
    configured
        .executable
        .as_ref()
        .map_or(preference.clone(), |path| {
            preference.with_explicit_executable(path)
        })
}

#[cfg(test)]
#[path = "language_runtime_tests.rs"]
mod tests;
