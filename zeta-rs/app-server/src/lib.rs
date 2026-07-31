//! JSON-RPC application boundary between product clients and Zeta's domain components.

mod git_service;
mod local;
mod local_tools;
mod mcp_runtime;
mod mcp_tools;
mod model_catalog;
mod resource_store;
mod review;
mod server;
mod terminal_profiles;
mod terminal_service;
mod tool_composition;

pub use local::OpenAppServerError;
pub use local::open_local_app_server;
pub use local::{BuiltInSkillRoot, LocalAppServerOptions, LocalWorkspaceConfigOptions};
pub use review::{ProviderReviewModel, ReviewModelResolutionError, ReviewModelResolver};
pub use server::AppServer;
pub use server::ConnectionNotifications;
pub use server::ConnectionState;
pub use zeta_slash_commands::{SlashCommandCatalog, SlashCommandCatalogError};

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
