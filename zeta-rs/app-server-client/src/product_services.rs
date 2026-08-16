use std::env;
use std::path::Path;
use std::path::PathBuf;

use zeta_app_server::LocalProductServicesConfig;
use zeta_app_server::OpenAppServerError;
use zeta_install_context::InstallContext;

const PRODUCT_SERVICES_OVERRIDE: &str = "ZETA_PRODUCT_SERVICES_PATH";
const BUNDLED_PRODUCT_SERVICES: &str = "product-services/product-services.json";

/// Locates the distribution-owned product services document for the current process.
///
/// An explicit `ZETA_PRODUCT_SERVICES_PATH` takes precedence over the packaged resource. The
/// caller remains responsible for deciding whether this configuration belongs in its App Server
/// composition.
pub fn discovered_product_services_path() -> Option<PathBuf> {
    select_product_services_path(
        env::var_os(PRODUCT_SERVICES_OVERRIDE).map(PathBuf::from),
        InstallContext::current().bundled_resource(BUNDLED_PRODUCT_SERVICES),
    )
}

/// Loads the discovered product services document against one profile cache root.
pub fn load_discovered_product_services(
    profile_root: impl AsRef<Path>,
) -> Result<Option<LocalProductServicesConfig>, OpenAppServerError> {
    discovered_product_services_path()
        .map(|path| LocalProductServicesConfig::load(path, profile_root))
        .transpose()
}

fn select_product_services_path(
    explicit: Option<PathBuf>,
    bundled: Option<PathBuf>,
) -> Option<PathBuf> {
    explicit.or(bundled)
}

#[cfg(test)]
#[path = "product_services_tests.rs"]
mod tests;
