use std::env;
use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use zeta_marketplace_client::CapabilityKind;
use zeta_marketplace_client::InstallPackageRequest;
use zeta_marketplace_client::MarketplaceRemoteClient;
use zeta_marketplace_client::MarketplaceServiceClient;
use zeta_marketplace_client::RemoteMarketplaceConfig;
use zeta_marketplace_client::SearchPackagesRequest;
use zeta_marketplace_manager::MarketplaceManager;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let [distribution, trusted_root, state_root] = arguments.as_slice() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: local_distribution_smoke <distribution> <trusted-root> <state-root>",
        )
        .into());
    };
    let config = RemoteMarketplaceConfig::from_directory(
        distribution,
        std::fs::read(trusted_root)?,
        state_root.join("remote-cache"),
    )?;
    let registry = Arc::new(MarketplaceRemoteClient::open(config)?);
    let manager = MarketplaceManager::open(state_root.join("manager"), registry)?;
    for (package_type, package_id) in [
        ("plugin", "marketplace/github"),
        ("skill", "marketplace/commit"),
        ("language", "marketplace/css"),
    ] {
        let search = manager.search(SearchPackagesRequest {
            query: package_id.rsplit('/').next().unwrap_or_default().to_owned(),
            package_type: Some(package_type.to_owned()),
            limit: Some(10),
        })?;
        let package = search
            .packages
            .iter()
            .find(|package| package.id == package_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{package_id} package was not found"),
                )
            })?;
        let installed = manager.install(InstallPackageRequest {
            package_id: package.id.clone(),
            version: Some(package.version.clone()),
        })?;
        println!(
            "installed {}@{} with {} capabilities as {}",
            installed.package.id,
            installed.package.version,
            installed.capabilities.len(),
            installed.installation_id
        );
    }
    let skill_sources = manager.local_capability_sources(CapabilityKind::Skill)?;
    let language_sources = manager.local_capability_sources(CapabilityKind::Language)?;
    let executable_sources = manager.local_capability_sources(CapabilityKind::Executable)?;
    if skill_sources.is_empty() || language_sources.is_empty() {
        return Err(io::Error::other(
            "installed Skill and Language packages were not projected as local capabilities",
        )
        .into());
    }
    let css_server = executable_sources
        .iter()
        .find(|source| source.package().id == "marketplace/css")
        .ok_or_else(|| io::Error::other("CSS executable capability was not projected"))?;
    if css_server.language_ids() != ["css", "less", "scss"] {
        return Err(io::Error::other("CSS executable lost its signed language route").into());
    }
    Ok(())
}
