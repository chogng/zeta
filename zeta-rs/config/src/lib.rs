//! Durable user configuration authority and its resolved runtime snapshot.
//!
//! This crate owns ordinary, non-secret user configuration. It does not own credential values,
//! Plugin packages, MCP connections, or any live runtime state.

mod command;
mod document;
mod hooks;
mod mcp;
mod mutation;
mod plugins;
mod resolution;
mod skills;
mod store;
mod store_file;
mod store_monitor;
mod store_schema;
mod workspace;

pub use command::{
    ConfigCommandDisposition, ConfigCommandError, ConfigCommandRequest, ConfigCommandResult,
    PreferencesUpdate, UserConfigCommand,
};
pub use document::{
    AgentConfig, ApprovalReviewModelSelection, ConfigGeneration, ConfigRevision, ResolvedConfig,
    ResolvedConfigSnapshot, UserConfigDocument,
};
pub use hooks::{
    HookAction, HookConfig, HookEnablement, HookEvent, HookId, HookMatcher, HooksConfig,
};
pub use mcp::{
    McpConfig, McpCredentialBinding, McpServerConfig, McpServerEnablement, McpServerId,
    McpTransportConfig,
};
pub use plugins::{
    PluginId, PluginRequest, PluginRequestEnablement, PluginVersion, PluginsConfig,
    WorkspacePluginRequest, WorkspacePluginRequestScope, WorkspacePluginRequests,
};
pub use resolution::{
    ConfigDiagnostic, ConfigDiagnosticCode, ConfigProvenance, ConfigValueSource,
    ScopedConfigSnapshot, WorkspaceConfigInput, resolve_scoped_config,
};
pub use skills::{SkillEnablement, SkillSourceConfig, SkillSourceEnablement, SkillsConfig};
pub use store::{ConfigChange, ConfigError, ConfigStore};
pub use workspace::{
    WorkspaceAgentConfig, WorkspaceConfigDocument, WorkspaceConfigIntent, WorkspaceConfigRevision,
    WorkspaceConfigScope, WorkspaceConfigStore, WorkspaceId, WorkspaceMcpConfig,
    WorkspaceMcpServerConfig, WorkspaceSkillsConfig,
};
pub use zeta_protocol::{ModelRef, SkillId, SkillName, SkillSourceId};

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
