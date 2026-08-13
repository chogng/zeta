#[path = "../../../zeta-rs/plugins/src/package/digest.rs"]
mod digest;
#[path = "../../../zeta-rs/plugins/src/package/local.rs"]
mod local;

pub use local::LocalPluginCatalog;
pub use local::LocalPluginPackage;
pub use local::PackageFileStats;
pub use local::PluginPackageSource;
