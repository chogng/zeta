use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
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
use zeta_language_server_catalog::RUST_ANALYZER_SERVER_ID;
use zeta_language_server_catalog::TYPESCRIPT_LANGUAGE_SERVER_ID;
use zeta_language_service::LanguageRequestId;
use zeta_language_service::LanguageServerState;
use zeta_language_service::LanguageService;
use zeta_language_service::LanguageServiceConfiguration;
use zeta_language_service::LanguageServiceEvent;
use zeta_language_service::LanguageServiceEventSink;

const SERVER_START_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

struct AppServerLanguageEventSink {
    sender: mpsc::Sender<LanguageServiceEvent>,
}

impl LanguageServiceEventSink for AppServerLanguageEventSink {
    fn on_event(&self, event: LanguageServiceEvent) {
        let _ = self.sender.send(event);
    }
}

#[derive(Default)]
pub(super) struct AppServerLanguageRuntime {
    pub(super) service: Option<LanguageService>,
    receiver: Option<mpsc::Receiver<LanguageServiceEvent>>,
    workspace_root: Option<PathBuf>,
    config_generation: Option<u64>,
    language_servers: BTreeMap<String, String>,
    server_states: BTreeMap<String, LanguageServerState>,
}

impl AppServerLanguageRuntime {
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
                LanguageServiceEvent::Locations(result) => result.request_id == request_id,
                LanguageServiceEvent::Hierarchy(result) => result.request_id == request_id,
                LanguageServiceEvent::WorkspaceSymbols(result) => result.request_id == request_id,
                LanguageServiceEvent::RenamePreparation(result) => result.request_id == request_id,
                LanguageServiceEvent::WorkspaceEdit(result) => result.request_id == request_id,
                LanguageServiceEvent::CodeActions(result) => result.request_id == request_id,
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
        self.language_servers = resolution
            .definitions()
            .iter()
            .flat_map(|definition| {
                let server = definition.name().to_string();
                definition
                    .language_ids()
                    .map(move |language| (language.to_owned(), server.clone()))
            })
            .collect();
        let definitions = resolution.into_definitions();
        if definitions.is_empty() {
            return Err("no configured language-server executable is available".into());
        }
        let (sender, receiver) = mpsc::channel();
        let service = LanguageService::start(
            LanguageServiceConfiguration::enabled(workspace_root, definitions),
            Arc::new(AppServerLanguageEventSink { sender }),
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
