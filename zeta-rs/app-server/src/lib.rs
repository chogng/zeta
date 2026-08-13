//! JSON-RPC application boundary between product clients and Zeta's domain components.

mod code_retrieval_context;
mod code_retrieval_tool;
mod debug_service;
mod dynamic_tools;
mod extension_tools;
mod git_service;
mod local;
mod local_tools;
mod model_catalog;
mod resource_store;
mod review;
mod server;
mod terminal_command_status;
mod terminal_environment;
mod terminal_profiles;
mod terminal_service;
mod tool_composition;
mod tool_executor_adapter;
mod tool_search_embedding;
mod tool_search_models;

pub use dynamic_tools::DynamicToolCompositionError;
pub use local::OpenAppServerError;
pub use local::open_local_app_server;
pub use local::open_local_app_server_with_cloud_providers;
pub use local::open_local_app_server_with_code_index_providers;
pub use local::{
    BuiltInSkillRoot, LocalAppServerOptions, LocalCodeIndexProviders, LocalConnectorRuntime,
    LocalWorkspaceConfigOptions, SessionStateMode,
};
pub use review::{ProviderReviewModel, ReviewModelResolutionError, ReviewModelResolver};
pub use server::AppServer;
pub use server::CodeIndexSemanticModels;
pub use server::ConnectionNotifications;
pub use server::ConnectionState;
pub use zeta_extensions::{ExtensionRoot, ExtensionRootKind};
pub use zeta_slash_commands::{SlashCommandCatalog, SlashCommandCatalogError};

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
