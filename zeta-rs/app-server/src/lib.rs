//! JSON-RPC application boundary between product clients and Zeta's domain components.

mod attachment_upload_store;
mod browser_host;
mod browser_tool;
mod codebase_retrieval_context;
mod codebase_retrieval_tool;
mod debug_service;
mod dynamic_tools;
mod extension_tools;
mod git_service;
mod local;
mod local_tools;
mod marketplace_connector_runtime;
mod marketplace_editor_extensions;
mod mcp_runtime;
mod model_catalog;
mod model_provider_error;
mod product_services;
mod resource_store;
mod review;
mod server;
mod session_workspace_access;
mod terminal_command_status;
mod terminal_environment;
mod terminal_profiles;
mod terminal_service;
mod tool_composition;
mod tool_executor_adapter;
mod tool_search_embedding;
mod tool_search_models;

pub use dynamic_tools::DynamicToolCompositionError;
pub use local::BuiltInSkillRoot;
pub use local::LocalAppServerOptions;
pub use local::LocalCodebaseProviders;
pub use local::LocalConnectorRuntime;
pub use local::LocalProfileRuntime;
pub use local::LocalWorkspaceConfigOptions;
pub use local::OpenAppServerError;
pub use local::SessionStateMode;
pub use local::open_local_app_server;
pub use local::open_local_app_server_with_cloud_providers;
pub use local::open_local_app_server_with_codebase_providers;
pub use marketplace_editor_extensions::MarketplaceEditorExtensionAdmission;
pub use marketplace_editor_extensions::MarketplaceEditorExtensionAdmissionLease;
pub use marketplace_editor_extensions::MarketplaceEditorExtensionBinding;
pub use product_services::LocalProductServicesConfig;
pub use review::ProviderReviewModel;
pub use review::ReviewModelResolutionError;
pub use review::ReviewModelResolver;
pub use server::AppServer;
pub use server::CodebaseModels;
pub use server::ConnectionNotifications;
pub use server::ConnectionState;
pub use zeta_extensions::ExtensionRoot;
pub use zeta_extensions::ExtensionRootKind;
pub use zeta_slash_commands::SlashCommandCatalog;
pub use zeta_slash_commands::SlashCommandCatalogError;

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "protocol_schema_tests.rs"]
mod protocol_schema_tests;
