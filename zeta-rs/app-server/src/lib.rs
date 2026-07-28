//! JSON-RPC application boundary between product clients and Zeta's domain components.

mod local;
mod resource_store;
mod review;
mod server;

pub use local::OpenAppServerError;
pub use local::open_local_app_server;
pub use local::{LocalAppServerOptions, LocalWorkspaceConfigOptions};
pub use review::{ProviderReviewModel, ReviewModelResolutionError, ReviewModelResolver};
pub use server::AppServer;
pub use server::ConnectionState;

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
