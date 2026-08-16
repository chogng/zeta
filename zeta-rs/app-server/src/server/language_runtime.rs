use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use zeta_config::LanguageServerConfig;
use zeta_config::LanguageServerModeConfig;
use zeta_config::LanguageServersConfig;
use zeta_install_context::InstallContext;
use zeta_language_server_catalog::BASH_LANGUAGE_SERVER_ID;
use zeta_language_server_catalog::JSON_LANGUAGE_SERVER_ID;
use zeta_language_server_catalog::LanguageServerCatalog;
use zeta_language_server_catalog::LanguageServerExecutionPolicy;
use zeta_language_server_catalog::LanguageServerPreference;
use zeta_language_server_catalog::LanguageServerProviderLaunch;
use zeta_language_server_catalog::LanguageServerProviderRegistry;
use zeta_language_server_catalog::RUST_ANALYZER_SERVER_ID;
use zeta_language_server_catalog::TYPESCRIPT_LANGUAGE_SERVER_ID;
use zeta_language_service::LanguageRequestId;
use zeta_language_service::LanguageRequestMetric;
use zeta_language_service::LanguageServerMessageSeverity;
use zeta_language_service::LanguageServerMessageSource;
use zeta_language_service::LanguageServerState;
use zeta_language_service::LanguageService;
use zeta_language_service::LanguageServiceConfiguration;
use zeta_language_service::LanguageServiceDocument;
use zeta_language_service::LanguageServiceEvent;
use zeta_language_service::LanguageServiceEventSink;
use zeta_language_service::LanguageServiceMetricsSink;

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

impl LanguageServiceMetricsSink for AppServerLanguageMetrics {
    fn record(&self, metric: LanguageRequestMetric) {
        log::debug!(
            target: "zeta_language_service",
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
    sender: mpsc::Sender<LanguageServiceEvent>,
    diagnostics: Arc<LanguageDiagnosticPublisher>,
}

impl LanguageServiceEventSink for AppServerLanguageEventSink {
    fn on_event(&self, event: LanguageServiceEvent) {
        if let LanguageServiceEvent::Diagnostics(diagnostics) = &event {
            self.diagnostics.publish(diagnostics);
            return;
        }
        if let LanguageServiceEvent::ServerMessage {
            server,
            severity,
            source,
            show,
            message,
        } = &event
        {
            self.diagnostics.updates.publish_language_server_message(
                LanguageServerMessageNotification {
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
        if let LanguageServiceEvent::ServerProgress(progress) = &event {
            self.diagnostics.updates.publish_language_server_progress(
                LanguageServerProgressNotification {
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
        if let LanguageServiceEvent::ServerStateChanged { server, state } = &event {
            self.diagnostics.updates.publish_language_server_state(
                LanguageServerStateNotification {
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
    revision: zeta_language_service::LanguageDocumentRevision,
    text: String,
}

struct LanguageDiagnosticPublisher {
    documents: Arc<Mutex<BTreeMap<PathBuf, AppServerLanguageDocumentSnapshot>>>,
    updates: Arc<UpdateBroker>,
}

impl LanguageDiagnosticPublisher {
    fn publish(&self, diagnostics: &zeta_language_service::LanguageDiagnostics) {
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
                path: snapshot.relative_path,
                revision: snapshot.revision.value(),
                diagnostics,
            });
    }
}

pub(super) struct AppServerLanguageRuntime {
    pub(super) service: Option<LanguageService>,
    receiver: Option<mpsc::Receiver<LanguageServiceEvent>>,
    documents: Arc<Mutex<BTreeMap<PathBuf, AppServerLanguageDocumentSnapshot>>>,
    updates: Arc<UpdateBroker>,
    workspace_root: Option<PathBuf>,
    config_generation: Option<u64>,
    language_servers: BTreeMap<String, String>,
    server_states: BTreeMap<String, LanguageServerState>,
    providers: LanguageServerProviderRegistry,
}

impl AppServerLanguageRuntime {
    pub(super) fn new(updates: Arc<UpdateBroker>) -> Self {
        Self {
            service: None,
            receiver: None,
            documents: Arc::new(Mutex::new(BTreeMap::new())),
            updates,
            workspace_root: None,
            config_generation: None,
            language_servers: BTreeMap::new(),
            server_states: BTreeMap::new(),
            providers: LanguageServerProviderRegistry::new(),
        }
    }

    pub(super) fn set_provider_registry(&mut self, providers: LanguageServerProviderRegistry) {
        self.shutdown();
        self.providers = providers;
    }

    pub(super) fn ensure(
        &mut self,
        workspace_root: &Path,
        config_generation: u64,
        configuration: &LanguageServersConfig,
        language_id: &str,
    ) -> Result<&LanguageService, String> {
        if self.workspace_root.as_deref() != Some(workspace_root)
            || self.config_generation != Some(config_generation)
        {
            self.restart(workspace_root, config_generation, configuration)?;
        }
        self.wait_until_ready(language_id)?;
        self.service
            .as_ref()
            .ok_or_else(|| "language service is unavailable".into())
    }

    pub(super) fn wait_for_request(
        &mut self,
        request_id: LanguageRequestId,
    ) -> Result<LanguageServiceEvent, String> {
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        loop {
            let event = self.recv_until(deadline)?;
            let matches = match &event {
                LanguageServiceEvent::Hover(result) => result.request_id == request_id,
                LanguageServiceEvent::Completions(result) => result.request_id == request_id,
                LanguageServiceEvent::CompletionDetails(result) => result.request_id == request_id,
                LanguageServiceEvent::CommandResult(result) => result.request_id == request_id,
                LanguageServiceEvent::Locations(result) => result.request_id == request_id,
                LanguageServiceEvent::Hierarchy(result) => result.request_id == request_id,
                LanguageServiceEvent::WorkspaceSymbols(result) => result.request_id == request_id,
                LanguageServiceEvent::WorkspaceDiagnostics(result) => {
                    result.request_id == request_id
                }
                LanguageServiceEvent::RenamePreparation(result) => result.request_id == request_id,
                LanguageServiceEvent::WorkspaceEdit(result) => result.request_id == request_id,
                LanguageServiceEvent::CodeActions(result) => result.request_id == request_id,
                LanguageServiceEvent::FormattingEdits(result) => result.request_id == request_id,
                LanguageServiceEvent::SignatureHelp(result) => result.request_id == request_id,
                LanguageServiceEvent::InlayHints(result) => result.request_id == request_id,
                LanguageServiceEvent::LinkedEditingRanges(result) => {
                    result.request_id == request_id
                }
                LanguageServiceEvent::SemanticTokens(result) => result.request_id == request_id,
                LanguageServiceEvent::DocumentSymbols(result) => result.request_id == request_id,
                LanguageServiceEvent::CodeLenses(result) => result.request_id == request_id,
                LanguageServiceEvent::DocumentLinks(result) => result.request_id == request_id,
                LanguageServiceEvent::DocumentColors(result) => result.request_id == request_id,
                LanguageServiceEvent::ColorPresentations(result) => result.request_id == request_id,
                LanguageServiceEvent::FoldingRanges(result) => result.request_id == request_id,
                LanguageServiceEvent::PulledDiagnostics(result) => result.request_id == request_id,
                LanguageServiceEvent::RequestFailed {
                    request_id: failed, ..
                } => *failed == request_id,
                _ => false,
            };
            self.accept_state(&event);
            if matches {
                return Ok(event);
            }
        }
    }

    pub(super) fn synchronize_document(
        &mut self,
        workspace_root: &Path,
        config_generation: u64,
        configuration: &LanguageServersConfig,
        relative_path: &Path,
        document: LanguageServiceDocument,
    ) -> Result<(), String> {
        let language_id = document.language_id().to_owned();
        self.ensure(
            workspace_root,
            config_generation,
            configuration,
            &language_id,
        )?;
        self.documents
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
        self.service
            .as_ref()
            .ok_or_else(|| String::from("language service is unavailable"))?
            .synchronize_document(document)
            .map_err(|error| error.to_string())
    }

    pub(super) fn close_document(&mut self, path: &Path) -> Result<(), String> {
        self.documents
            .lock()
            .map_err(|_| String::from("language document snapshots are unavailable"))?
            .remove(path);
        if let Some(service) = &self.service {
            service
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
        self.shutdown();
        let catalog =
            LanguageServerCatalog::new(preference(configuration, RUST_ANALYZER_SERVER_ID))
                .with_json_language_server(preference(configuration, JSON_LANGUAGE_SERVER_ID))
                .with_bash_language_server(preference(configuration, BASH_LANGUAGE_SERVER_ID))
                .with_typescript_language_server(preference(
                    configuration,
                    TYPESCRIPT_LANGUAGE_SERVER_ID,
                ));
        let resolution = catalog
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
        self.language_servers = definitions
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
        let service = LanguageService::start_with_metrics(
            LanguageServiceConfiguration::enabled(workspace_root, definitions)
                .with_generation(config_generation),
            Arc::new(AppServerLanguageEventSink {
                sender,
                diagnostics: Arc::new(LanguageDiagnosticPublisher {
                    documents: Arc::clone(&self.documents),
                    updates: Arc::clone(&self.updates),
                }),
            }),
            Arc::new(AppServerLanguageMetrics),
        )
        .map_err(|error| error.to_string())?;
        self.service = Some(service);
        self.receiver = Some(receiver);
        self.workspace_root = Some(workspace_root.to_path_buf());
        self.config_generation = Some(config_generation);
        Ok(())
    }

    fn wait_until_ready(&mut self, language_id: &str) -> Result<(), String> {
        let server = self
            .language_servers
            .get(language_id)
            .cloned()
            .ok_or_else(|| format!("no language server is available for '{language_id}'"))?;
        if matches!(
            self.server_states.get(&server),
            Some(LanguageServerState::Ready)
        ) {
            return Ok(());
        }
        let deadline = Instant::now() + SERVER_START_TIMEOUT;
        loop {
            let event = self.recv_until(deadline)?;
            self.accept_state(&event);
            match self.server_states.get(&server) {
                Some(LanguageServerState::Ready) => return Ok(()),
                Some(LanguageServerState::Failed(message)) => return Err(message.clone()),
                Some(LanguageServerState::CrashLoop { message, .. }) => return Err(message.clone()),
                _ => {}
            }
        }
    }

    fn recv_until(&self, deadline: Instant) -> Result<LanguageServiceEvent, String> {
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

    fn accept_state(&mut self, event: &LanguageServiceEvent) {
        if let LanguageServiceEvent::ServerStateChanged { server, state } = event {
            self.server_states.insert(server.clone(), state.clone());
        }
    }

    fn shutdown(&mut self) {
        if let Ok(mut documents) = self.documents.lock() {
            documents.clear();
        }
        if let Some(service) = self.service.take() {
            let _ = service.shutdown();
        }
        self.receiver = None;
        self.workspace_root = None;
        self.config_generation = None;
        self.language_servers.clear();
        self.server_states.clear();
    }
}

fn configured_provider_definitions(
    providers: &LanguageServerProviderRegistry,
    configuration: &LanguageServersConfig,
    workspace_root: &Path,
) -> Result<Vec<zeta_language_service::LanguageServerDefinition>, String> {
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
                LanguageServerProviderLaunch::Packaged,
                LanguageServerProviderLaunch::ExplicitExecutable,
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
