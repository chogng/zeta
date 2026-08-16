use std::env;
use std::path::Path;
use std::path::PathBuf;

use std::sync::Arc;
use zeta_app_server::AppServer;
use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::LocalProductServicesConfig;
use zeta_app_server::LocalProfileRuntime;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::discovered_product_services_path;
use zeta_app_server_client::local_profile_root;

const WORKSPACE_TRUST_SOURCE: &str = "ZETA_WORKSPACE_TRUST_SOURCE";

pub(super) fn run(arguments: Vec<String>) -> Result<(), String> {
    let (command, product_services) = parse_arguments(&arguments)?;
    let options =
        AppServerHostOptions::from_environment(product_services.or_else(product_services_path))?;
    match command {
        AppServerHostCommand::Direct => open_server(&options)?
            .serve_stdio()
            .map_err(|error| error.to_string()),
        AppServerHostCommand::Connect => super::app_server_broker::connect(options),
        AppServerHostCommand::Daemon => super::app_server_broker::serve(options),
    }
}

fn parse_arguments(
    arguments: &[String],
) -> Result<(AppServerHostCommand, Option<PathBuf>), String> {
    let (command, remaining) = match arguments {
        [listen, address, remaining @ ..] if listen == "--listen" && address == "stdio://" => {
            (AppServerHostCommand::Direct, remaining)
        }
        [command, remaining @ ..] if command == "connect" => {
            (AppServerHostCommand::Connect, remaining)
        }
        [command, remaining @ ..] if command == "daemon" => {
            (AppServerHostCommand::Daemon, remaining)
        }
        _ => return Err(usage().into()),
    };
    let product_services = match remaining {
        [] => None,
        [product, path] if product == "--product-services" => Some(PathBuf::from(path)),
        _ => return Err(usage().into()),
    };
    Ok((command, product_services))
}

fn usage() -> &'static str {
    "usage: zeta-server app-server (--listen stdio:// | connect | daemon) [--product-services PATH]"
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppServerHostCommand {
    Direct,
    Connect,
    Daemon,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AppServerHostOptions {
    profile_root: PathBuf,
    workspace_root: Option<PathBuf>,
    workspace_trust_source: WorkspaceTrustSource,
    product_services: Option<PathBuf>,
}

impl AppServerHostOptions {
    pub(super) fn new(
        profile_root: impl Into<PathBuf>,
        workspace_root: Option<PathBuf>,
        workspace_trust_source: WorkspaceTrustSource,
        product_services: Option<PathBuf>,
    ) -> Self {
        Self {
            profile_root: profile_root.into(),
            workspace_root,
            workspace_trust_source,
            product_services,
        }
    }

    fn from_environment(product_services: Option<PathBuf>) -> Result<Self, String> {
        let workspace_trust_source = match env::var(WORKSPACE_TRUST_SOURCE).as_deref() {
            Ok("userConfig") => WorkspaceTrustSource::UserConfig,
            Ok("hostConfiguration") | Err(env::VarError::NotPresent) => {
                WorkspaceTrustSource::HostConfiguration
            }
            Ok(_) | Err(env::VarError::NotUnicode(_)) => {
                return Err(format!(
                    "{WORKSPACE_TRUST_SOURCE} must be userConfig or hostConfiguration"
                ));
            }
        };
        Ok(Self::new(
            local_profile_root(),
            env::var_os("ZETA_WORKSPACE_ROOT").map(PathBuf::from),
            workspace_trust_source,
            product_services,
        ))
    }

    pub(super) fn profile_root(&self) -> &Path {
        &self.profile_root
    }

    pub(super) fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub(super) fn workspace_trust_source(&self) -> WorkspaceTrustSource {
        self.workspace_trust_source
    }

    pub(super) fn product_services(&self) -> Option<&Path> {
        self.product_services.as_deref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkspaceTrustSource {
    HostConfiguration,
    UserConfig,
}

pub(super) fn open_server(host: &AppServerHostOptions) -> Result<AppServer, String> {
    open_server_with_profile_runtime(host, None)
}

pub(super) fn open_server_with_profile_runtime(
    host: &AppServerHostOptions,
    profile_runtime: Option<Arc<LocalProfileRuntime>>,
) -> Result<AppServer, String> {
    let mut options = LocalAppServerOptions::new(host.profile_root());
    if let Some(runtime) = profile_runtime {
        options = options.with_profile_runtime(runtime);
    }
    if let Some(workspace_root) = host.workspace_root() {
        options = match host.workspace_trust_source() {
            WorkspaceTrustSource::UserConfig => {
                options.with_user_config_workspace_root(workspace_root)
            }
            WorkspaceTrustSource::HostConfiguration => options.with_workspace_root(workspace_root),
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

pub(super) fn product_services_path() -> Option<PathBuf> {
    discovered_product_services_path()
}
