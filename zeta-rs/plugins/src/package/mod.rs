mod digest;
mod local;
mod snapshot;
mod store;

#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;

pub use local::{
    LocalPluginCatalog, LocalPluginPackage, PackageFileStats, PluginPackageDigestAlgorithm,
    PluginPackageSource,
};
pub use store::InstalledPluginPackage;
pub use store::PluginPackageStore;
