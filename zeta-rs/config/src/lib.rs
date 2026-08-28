//! Durable user configuration authority and its resolved runtime snapshot.
//!
//! This crate owns ordinary, non-secret user configuration. It does not own credential values,
//! Plugin packages, MCP connections, or any live runtime state.

mod code_index;
mod command;
mod document;
mod exec_policy;
mod hooks;
mod language_servers;
mod mcp;
mod mutation;
mod plugins;
mod resolution;
mod skills;
mod store;
mod store_file;
mod store_monitor;
mod store_schema;
mod tool_search;
mod workspace;
mod workspace_trust;

pub use code_index::{
    SemanticCodeIndexAutomaticContext, SemanticCodeIndexConfig, SemanticCodeIndexEgressGrant,
    SemanticCodeIndexModelSelection, SemanticCodeIndexSelection,
};
pub use command::{
    ConfigCommandDisposition, ConfigCommandError, ConfigCommandRequest, ConfigCommandResult,
    PreferencesUpdate, UserConfigCommand,
};
pub use document::{
    AgentConfig, AgentGrepBackend, ApprovalReviewModelSelection, ConfigGeneration, ConfigRevision,
    ResolvedConfig, ResolvedConfigSnapshot, UserConfigDocument,
};
pub use exec_policy::{UserExecPolicyConfig, WorkspaceExecPolicyConfig, compose_exec_policy};
pub use hooks::{
    HookAction, HookConfig, HookEnablement, HookEvent, HookId, HookMatcher, HooksConfig,
};
pub use language_servers::{
    LanguageServerConfig, LanguageServerId, LanguageServerModeConfig, LanguageServersConfig,
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
pub use tool_search::ToolSearchConfig;
pub use tool_search::ToolSearchModeConfig;
pub use workspace::{
    WorkspaceAgentConfig, WorkspaceConfigDocument, WorkspaceConfigIntent, WorkspaceConfigRevision,
    WorkspaceConfigScope, WorkspaceConfigStore, WorkspaceId, WorkspaceMcpConfig,
    WorkspaceMcpServerConfig, WorkspaceSkillsConfig,
};
pub use workspace_trust::{WorkspaceTrustConfig, WorkspaceTrustSetting};
pub use zeta_protocol::{ModelRef, SkillId, SkillName, SkillSourceId, ToolMode};
pub use zeta_workspace::WorkspaceTrustId;

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
