use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use zeta_app_server::AppServer;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::LocalProductServicesConfig;
use zeta_app_server::LocalProfileRuntime;
use zeta_app_server::open_local_app_server;
use zeta_fast_regex_search::FastRegexWorkerCommand;

use crate::ConnectionOptions;
use crate::wire::ConnectionPrelude;
use crate::wire::ConnectionWorkspaceTrustSource;

const MAX_PRODUCT_SERVICES_IDENTITY_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct WorkspaceRuntimeKey {
    workspace_root: Option<PathBuf>,
    workspace_trust_source: ConnectionWorkspaceTrustSource,
    product_services_identity: Option<[u8; 32]>,
}

pub(crate) struct ProfileAppServerRegistry {
    host: ConnectionOptions,
    profile_runtime: Arc<LocalProfileRuntime>,
    servers: Mutex<BTreeMap<WorkspaceRuntimeKey, Arc<AppServer>>>,
}

impl ProfileAppServerRegistry {
    pub(crate) fn open(host: ConnectionOptions) -> Result<Self, String> {
        let profile_runtime = Arc::new(
            LocalProfileRuntime::open(host.profile_root()).map_err(|error| error.to_string())?,
        );
        Ok(Self {
            host,
            profile_runtime,
            servers: Mutex::new(BTreeMap::new()),
        })
    }

    pub(crate) fn server_for(&self, prelude: ConnectionPrelude) -> Result<Arc<AppServer>, String> {
        prelude.validate()?;
        let workspace_root = prelude
            .workspace_root
            .as_deref()
            .map(dunce::canonicalize)
            .transpose()
            .map_err(io_error)?;
        let product_services_identity = product_services_identity(
            prelude.product_services.as_deref(),
            self.host.profile_root(),
        )?;
        let key = WorkspaceRuntimeKey {
            workspace_root: workspace_root.clone(),
            workspace_trust_source: prelude.workspace_trust_source,
            product_services_identity,
        };
        let mut servers = self
            .servers
            .lock()
            .map_err(|_| "local App Server Workspace registry lock poisoned".to_string())?;
        if let Some(server) = servers.get(&key) {
            return Ok(Arc::clone(server));
        }
        let host = ConnectionOptions::new(
            self.host.profile_root(),
            workspace_root,
            prelude.trust_source(),
            prelude.product_services,
        );
        let server = Arc::new(open_server_with_profile_runtime(
            &host,
            Arc::clone(&self.profile_runtime),
        )?);
        servers.insert(key, Arc::clone(&server));
        Ok(server)
    }

    pub(crate) fn active_terminal_count(&self) -> usize {
        self.servers
            .lock()
            .map(|servers| {
                servers
                    .values()
                    .map(|server| server.active_terminal_count())
                    .sum()
            })
            .unwrap_or(1)
    }
}

fn open_server_with_profile_runtime(
    host: &ConnectionOptions,
    profile_runtime: Arc<LocalProfileRuntime>,
) -> Result<AppServer, String> {
    let mut options =
        LocalAppServerOptions::new(host.profile_root()).with_profile_runtime(profile_runtime);
    options = options.with_fast_regex_worker_command(FastRegexWorkerCommand::new(
        std::env::current_exe().map_err(|error| error.to_string())?,
        [crate::FAST_REGEX_WORKER_PROCESS_ARGUMENT],
    ));
    if let Some(workspace_root) = host.workspace_root() {
        options = match host.workspace_trust_source() {
            crate::WorkspaceTrustSource::UserConfig => {
                options.with_user_config_workspace_root(workspace_root)
            }
            crate::WorkspaceTrustSource::HostConfiguration => {
                options.with_workspace_root(workspace_root)
            }
        };
    }
    if let Some(path) = host.product_services() {
        options = options.with_product_services(
            LocalProductServicesConfig::load(path, host.profile_root())
                .map_err(|error| error.to_string())?,
        );
    }
    open_local_app_server(options).map_err(|error| error.to_string())
}

fn product_services_identity(
    path: Option<&Path>,
    profile_root: &Path,
) -> Result<Option<[u8; 32]>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_PRODUCT_SERVICES_IDENTITY_BYTES
    {
        return Err("Product services manifest is not a bounded regular file".into());
    }
    let services =
        LocalProductServicesConfig::load(path, profile_root).map_err(|error| error.to_string())?;
    Ok(Some(*services.authority_identity()))
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}
