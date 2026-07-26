//! Durable user configuration authority and its resolved runtime snapshot.
//!
//! This crate owns ordinary, non-secret user configuration. It does not own credential values,
//! Plugin packages, MCP connections, or any live runtime state.

mod command;
mod document;
mod mcp;
mod resolution;
mod skills;
mod store;
mod workspace;

pub use command::{
    ConfigCommandDisposition, ConfigCommandError, ConfigCommandRequest, ConfigCommandResult,
    PreferencesUpdate, UserConfigCommand,
};
pub use document::{
    AgentConfig, ConfigGeneration, ConfigRevision, ResolvedConfig, ResolvedConfigSnapshot,
    UiConfig, UserConfigDocument,
};
pub use mcp::{
    McpConfig, McpCredentialBinding, McpServerConfig, McpServerEnablement, McpServerId,
    McpTransportConfig,
};
pub use resolution::{
    ConfigDiagnostic, ConfigDiagnosticCode, ConfigProvenance, ConfigValueSource,
    ScopedConfigSnapshot, WorkspaceConfigInput, resolve_scoped_config,
};
pub use skills::{SkillSourceConfig, SkillSourceEnablement, SkillSourceId, SkillsConfig};
pub use store::{ConfigError, ConfigStore};
pub use workspace::{
    PluginId, PluginVersion, WorkspaceAgentConfig, WorkspaceConfigDocument, WorkspaceConfigIntent,
    WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceConfigStore, WorkspaceId,
    WorkspaceMcpConfig, WorkspaceMcpServerConfig, WorkspacePluginRequest,
    WorkspacePluginRequestScope, WorkspacePluginRequests, WorkspaceSkillsConfig,
};
pub use zeta_protocol::{ModelRef, Theme};

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
