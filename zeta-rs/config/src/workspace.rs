use crate::{
    ConfigError, HooksConfig, McpServerEnablement, McpServerId, McpTransportConfig,
    SkillSourceConfig, SkillSourceId, WorkspacePluginRequests,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use zeta_protocol::ModelRef;

/// Host-validated identity used to namespace one Workspace configuration document.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        if value.trim().is_empty()
            || value.contains(':')
            || value.contains(char::is_whitespace)
            || value.contains('\0')
        {
            return Err(ConfigError(
                "workspace id must be a non-empty identifier without whitespace or ':'".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn namespace(&self) -> String {
        format!("workspace:{}", self.0)
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WorkspaceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Scope supplied by the host when it reads a Workspace configuration document.
///
/// The identity is deliberately outside the document: a checked-out file must not choose the
/// namespace in which its MCP or Skill declarations will be interpreted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceConfigScope {
    pub workspace_id: WorkspaceId,
}

impl WorkspaceConfigScope {
    pub fn new(workspace_id: WorkspaceId) -> Self {
        Self { workspace_id }
    }

    fn namespace(&self) -> String {
        self.workspace_id.namespace()
    }
}

/// Revision assigned by the host to one observed Workspace configuration document.
///
/// The document itself cannot author this revision. Hosts typically advance it after observing a
/// validated content change, which prevents checked-out workspace files from controlling runtime
/// generation semantics.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct WorkspaceConfigRevision(u64);

impl WorkspaceConfigRevision {
    pub const INITIAL: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }

    /// Advances a host-observed Workspace content revision.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Agent defaults requested by a Workspace configuration document.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceAgentConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_model: Option<ModelRef>,
}

/// Runtime-free MCP declaration requested by a Workspace.
///
/// It intentionally omits credential binding. A Workspace can request an unauthenticated server,
/// but cannot select a credential or grant itself access to one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceMcpServerConfig {
    pub id: McpServerId,
    pub display_name: String,
    pub transport: McpTransportConfig,
    #[serde(default)]
    pub enablement: McpServerEnablement,
}

impl WorkspaceMcpServerConfig {
    fn validate(&self, namespace: &str) -> Result<(), ConfigError> {
        if !self.id.belongs_to_namespace(namespace) {
            return Err(ConfigError(format!(
                "Workspace MCP server '{}' is outside the '{namespace}' namespace",
                self.id
            )));
        }
        validate_text(&self.display_name, "Workspace MCP server display name")?;
        self.transport.validate()
    }
}

/// Workspace-owned MCP declarations. They are desired configuration, not live connections.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceMcpConfig {
    #[serde(default)]
    pub servers: BTreeMap<McpServerId, WorkspaceMcpServerConfig>,
}

impl WorkspaceMcpConfig {
    fn validate(&self, namespace: &str) -> Result<(), ConfigError> {
        for (server_id, server) in &self.servers {
            if &server.id != server_id {
                return Err(ConfigError(format!(
                    "Workspace MCP entry '{}' contains declaration for '{}'",
                    server_id, server.id
                )));
            }
            server.validate(namespace)?;
        }
        Ok(())
    }
}

/// Workspace Skill sources. Each source remains an opaque reference until a Skill manager checks
/// containment and trust.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceSkillsConfig {
    #[serde(default)]
    pub sources: BTreeMap<SkillSourceId, SkillSourceConfig>,
}

impl WorkspaceSkillsConfig {
    fn validate(&self, namespace: &str) -> Result<(), ConfigError> {
        for (source_id, source) in &self.sources {
            if &source.id != source_id {
                return Err(ConfigError(format!(
                    "Workspace Skill entry '{}' contains declaration for '{}'",
                    source_id, source.id
                )));
            }
            if !source_id.belongs_to_namespace(namespace) {
                return Err(ConfigError(format!(
                    "Workspace Skill source '{}' is outside the '{namespace}' namespace",
                    source_id
                )));
            }
            source.validate()?;
        }
        Ok(())
    }
}

/// Workspace capability requests preserved after ordinary configuration resolution.
///
/// These requests are not active capabilities. Plugin, MCP, and Skill managers must combine them
/// with their own install, trust, grant, and compatibility decisions before publishing runtime
/// snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceConfigIntent {
    pub workspace_id: WorkspaceId,
    pub mcp: WorkspaceMcpConfig,
    pub plugin_requests: WorkspacePluginRequests,
    pub skills: WorkspaceSkillsConfig,
    pub hooks: HooksConfig,
}

/// Typed, non-authoritative intent loaded from one Workspace configuration file.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceConfigDocument {
    #[serde(default)]
    pub agent: WorkspaceAgentConfig,
    #[serde(default)]
    pub mcp: WorkspaceMcpConfig,
    #[serde(default)]
    pub plugin_requests: WorkspacePluginRequests,
    #[serde(default)]
    pub skills: WorkspaceSkillsConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
}

impl WorkspaceConfigDocument {
    pub fn validate(&self, scope: &WorkspaceConfigScope) -> Result<(), ConfigError> {
        let namespace = scope.namespace();
        self.mcp.validate(&namespace)?;
        self.plugin_requests.validate()?;
        self.skills.validate(&namespace)?;
        self.hooks.validate_for_namespace(&namespace)
    }
}

/// Read-only source for one Workspace configuration document.
///
/// Workspace config is intentionally not a command authority: it is reviewed repository input.
/// A host supplies the workspace identity and later decides whether declarations receive trust or
/// grants from the relevant Plugin, MCP, Skill, credential, and policy authorities.
pub struct WorkspaceConfigStore {
    path: PathBuf,
    scope: WorkspaceConfigScope,
}

impl WorkspaceConfigStore {
    pub fn open(path: impl Into<PathBuf>, scope: WorkspaceConfigScope) -> Self {
        Self {
            path: path.into(),
            scope,
        }
    }

    pub fn read_document(&self) -> Result<WorkspaceConfigDocument, ConfigError> {
        if !self.path.exists() {
            return Ok(WorkspaceConfigDocument::default());
        }
        let source = fs::read_to_string(&self.path).map_err(io_error)?;
        let document: WorkspaceConfigDocument =
            toml::from_str(&source).map_err(|error| ConfigError(error.to_string()))?;
        document.validate(&self.scope)?;
        Ok(document)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn scope(&self) -> &WorkspaceConfigScope {
        &self.scope
    }
}

fn validate_text(value: &str, label: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() || value.contains('\0') || value.contains(['\n', '\r']) {
        return Err(ConfigError(format!("{label} must be non-empty plain text")));
    }
    Ok(())
}

fn io_error(error: impl std::fmt::Display) -> ConfigError {
    ConfigError(error.to_string())
}
