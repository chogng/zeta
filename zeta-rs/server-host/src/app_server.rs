use std::env;
use std::path::PathBuf;

use zeta_app_server::LocalAppServerOptions;
use zeta_app_server::LocalProductServicesConfig;
use zeta_app_server::open_local_app_server;
use zeta_app_server_client::local_profile_root;
use zeta_install_context::InstallContext;

const PRODUCT_SERVICES_OVERRIDE: &str = "ZETA_PRODUCT_SERVICES_PATH";
const BUNDLED_PRODUCT_SERVICES: &str = "product-services/product-services.json";

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
        options = options.with_workspace_root(PathBuf::from(workspace_root));
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
    env::var_os(PRODUCT_SERVICES_OVERRIDE)
        .map(PathBuf::from)
        .or_else(|| InstallContext::current().bundled_resource(BUNDLED_PRODUCT_SERVICES))
}
