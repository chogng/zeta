use std::env;
use std::path::PathBuf;

use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::LocalProductServicesConfig;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::discovered_product_services_path;
use zeta_app_server_client::local_profile_root;

const WORKSPACE_TRUST_SOURCE: &str = "ZETA_WORKSPACE_TRUST_SOURCE";

pub(super) fn run(arguments: Vec<String>) -> Result<(), String> {
    let product_services = match arguments.as_slice() {
        [listen, address] if listen == "--listen" && address == "stdio://" => None,
        [listen, address, product, path]
            if listen == "--listen" && address == "stdio://" && product == "--product-services" =>
        {
            Some(PathBuf::from(path))
        }
        _ => {
            return Err(
                "usage: zeta-server app-server --listen stdio:// [--product-services PATH]".into(),
            );
        }
    };
    let profile_root = local_profile_root();
    let mut options = LocalAppServerOptions::new(&profile_root);
    if let Some(workspace_root) = env::var_os("ZETA_WORKSPACE_ROOT") {
        options = match env::var(WORKSPACE_TRUST_SOURCE).as_deref() {
            Ok("userConfig") => {
                options.with_user_config_workspace_root(PathBuf::from(workspace_root))
            }
            Ok("hostConfiguration") | Err(env::VarError::NotPresent) => {
                options.with_workspace_root(PathBuf::from(workspace_root))
            }
            Ok(_) | Err(env::VarError::NotUnicode(_)) => {
                return Err(format!(
                    "{WORKSPACE_TRUST_SOURCE} must be userConfig or hostConfiguration"
                ));
            }
        };
    }
    if let Some(path) = product_services.or_else(product_services_path) {
        options = options.with_product_services(
            LocalProductServicesConfig::load(path, &profile_root)
                .map_err(|error| error.to_string())?,
        );
    }
    open_local_app_server(options)
        .map_err(|error| error.to_string())?
        .serve_stdio()
        .map_err(|error| error.to_string())
}

pub(super) fn product_services_path() -> Option<PathBuf> {
    discovered_product_services_path()
}
