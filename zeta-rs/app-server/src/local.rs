use crate::AppServer;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use zeta_config::{Config, ConfigStore};
use zeta_core::{AgentModel, ThreadManager};
use zeta_credentials::MacosKeychainCredentialStore;
use zeta_model_provider::{ProviderRegistry, UnavailableModel};
use zeta_storage::FileIdempotencyLedger;
use zeta_storage::ThreadLeaseDirectory;
use zeta_storage::ThreadRolloutStore;

/// Filesystem locations needed to open one persistent local App Server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAppServerOptions {
    pub state_root: PathBuf,
}

impl LocalAppServerOptions {
    pub fn new(state_root: impl Into<PathBuf>) -> Self {
        Self {
            state_root: state_root.into(),
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
    let journal = Arc::new(ThreadRolloutStore::open(&options.state_root).map_err(open_error)?);
    let writer_lease = Arc::new(
        ThreadLeaseDirectory::open(options.state_root.join("leases")).map_err(open_error)?,
    );
    let threads = Arc::new(ThreadManager::with_journal_and_lease(
        journal.clone(),
        writer_lease,
    ));
    for events in journal.all_thread_events().map_err(open_error)? {
        threads.recover_thread(events).map_err(open_error)?;
    }
    let idempotency = Arc::new(
        FileIdempotencyLedger::open(options.state_root.join("idempotency.rollout"))
            .map_err(open_error)?,
    );
    let config = Arc::new(
        ConfigStore::open(options.state_root.join("config.json"))
            .map_err(|error| OpenAppServerError(error.0))?,
    );
    let model = local_model(&config.read().map_err(|error| OpenAppServerError(error.0))?);
    Ok(AppServer::with_idempotency_ledger(threads, model, idempotency).with_config_store(config))
}

fn local_model(config: &Config) -> Arc<dyn AgentModel> {
    let Some(model_ref) = config.preferred_model.as_ref() else {
        return Arc::new(UnavailableModel::new(
            "model is not configured; set preferredModel and modelProvider in config.json",
        ));
    };
    let Some(provider) = config.model_provider.as_ref() else {
        return Arc::new(UnavailableModel::new(
            "model provider is not configured; set modelProvider in config.json",
        ));
    };
    ProviderRegistry::builtin()
        .build_model(
            provider,
            model_ref,
            Arc::new(MacosKeychainCredentialStore::new("zeta")),
        )
        .unwrap_or_else(|error| Arc::new(UnavailableModel::new(error.to_string())))
}

fn open_error(error: impl fmt::Display) -> OpenAppServerError {
    OpenAppServerError(error.to_string())
}
