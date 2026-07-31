use crate::{
    ConfigError, ConfigGeneration, ConfigRevision, HookId, McpServerId, PluginId, ResolvedConfig,
    ResolvedConfigSnapshot, SkillSourceId, UserConfigDocument, WorkspaceConfigDocument,
    WorkspaceConfigIntent, WorkspaceConfigRevision, WorkspaceConfigScope, WorkspaceId,
};
use std::collections::BTreeMap;
use zeta_protocol::{ModelRef, ProviderId};

/// Source that contributed a resolved value or pending capability request.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ConfigValueSource {
    #[default]
    User,
    Workspace(WorkspaceId),
}

/// Origin information for consumer-visible configuration values.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConfigProvenance {
    pub preferred_model: Option<ConfigValueSource>,
    pub approval_review_model: ConfigValueSource,
    pub providers: BTreeMap<ProviderId, ConfigValueSource>,
    pub mcp_servers: BTreeMap<McpServerId, ConfigValueSource>,
    pub skill_sources: BTreeMap<SkillSourceId, ConfigValueSource>,
    pub plugin_requests: BTreeMap<PluginId, ConfigValueSource>,
    pub hooks: BTreeMap<HookId, ConfigValueSource>,
}

impl ConfigProvenance {
    pub(crate) fn from_user(document: &UserConfigDocument) -> Self {
        Self {
            preferred_model: document
                .agent
                .preferred_model
                .as_ref()
                .map(|_| ConfigValueSource::User),
            approval_review_model: ConfigValueSource::User,
            providers: document
                .providers
                .keys()
                .cloned()
                .map(|id| (id, ConfigValueSource::User))
                .collect(),
            mcp_servers: document
                .mcp
                .servers
                .keys()
                .cloned()
                .map(|id| (id, ConfigValueSource::User))
                .collect(),
            skill_sources: document
                .skills
                .sources
                .keys()
                .cloned()
                .map(|id| (id, ConfigValueSource::User))
                .collect(),
            plugin_requests: document
                .plugins
                .requests
                .keys()
                .cloned()
                .map(|id| (id, ConfigValueSource::User))
                .collect(),
            hooks: document
                .hooks
                .hooks
                .keys()
                .cloned()
                .map(|id| (id, ConfigValueSource::User))
                .collect(),
        }
    }
}

/// Stable category for a configuration diagnostic that does not rewrite desired configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigDiagnosticCode {
    WorkspacePreferredModelProviderUnconfigured,
    WorkspaceMcpPendingTrust,
    WorkspaceSkillPendingTrust,
    WorkspacePluginPendingTrust,
    WorkspaceHookPendingTrust,
}

/// Redacted explanation of why a requested configuration value is not immediately usable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigDiagnostic {
    pub code: ConfigDiagnosticCode,
    pub subject: String,
}

/// Immutable result of resolving User configuration with an optional Workspace document.
///
/// Its generation remains the User authority generation because Workspace observation has its own
/// revision. App Server composition will assign a new environment generation after it combines
/// this result with Plugin, MCP, Skill, credential, and policy snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedConfigSnapshot {
    pub user_revision: ConfigRevision,
    pub user_generation: ConfigGeneration,
    pub workspace_revision: Option<WorkspaceConfigRevision>,
    pub values: ResolvedConfig,
    pub provenance: ConfigProvenance,
    pub diagnostics: Vec<ConfigDiagnostic>,
}

/// Workspace input passed to one configuration resolution.
pub struct WorkspaceConfigInput<'a> {
    pub scope: &'a WorkspaceConfigScope,
    pub revision: WorkspaceConfigRevision,
    pub document: &'a WorkspaceConfigDocument,
}

impl<'a> WorkspaceConfigInput<'a> {
    pub fn new(
        scope: &'a WorkspaceConfigScope,
        revision: WorkspaceConfigRevision,
        document: &'a WorkspaceConfigDocument,
    ) -> Self {
        Self {
            scope,
            revision,
            document,
        }
    }
}

/// Resolves ordinary User configuration with an optional, host-scoped Workspace document.
///
/// The resolver never treats Workspace requests as installed Plugins, trusted MCP connections,
/// selected credential bindings, or active Skill content. Only a workspace model preference may
/// override the User preference, and only when its provider is already configured by the User.
pub fn resolve_scoped_config(
    user: &ResolvedConfigSnapshot,
    workspace: Option<WorkspaceConfigInput<'_>>,
) -> Result<ScopedConfigSnapshot, ConfigError> {
    let mut values = user.values.clone();
    let mut provenance = user.provenance.clone();
    let mut diagnostics = user.diagnostics.clone();
    let mut workspace_revision = None;

    if let Some(workspace) = workspace {
        workspace.document.validate(workspace.scope)?;
        let workspace_id = workspace.scope.workspace_id.clone();
        workspace_revision = Some(workspace.revision);

        if let Some(model) = &workspace.document.agent.preferred_model {
            apply_workspace_model_preference(
                &mut values,
                &mut provenance,
                &mut diagnostics,
                &workspace_id,
                model,
            );
        }

        let source = ConfigValueSource::Workspace(workspace_id.clone());
        provenance.mcp_servers.extend(
            workspace
                .document
                .mcp
                .servers
                .keys()
                .cloned()
                .map(|id| (id, source.clone())),
        );
        provenance.skill_sources.extend(
            workspace
                .document
                .skills
                .sources
                .keys()
                .cloned()
                .map(|id| (id, source.clone())),
        );
        provenance.plugin_requests.extend(
            workspace
                .document
                .plugin_requests
                .requests
                .keys()
                .cloned()
                .map(|id| (id, source.clone())),
        );
        provenance.hooks.extend(
            workspace
                .document
                .hooks
                .hooks
                .keys()
                .cloned()
                .map(|id| (id, source.clone())),
        );
        diagnostics.extend(
            workspace
                .document
                .mcp
                .servers
                .keys()
                .map(|id| ConfigDiagnostic {
                    code: ConfigDiagnosticCode::WorkspaceMcpPendingTrust,
                    subject: id.to_string(),
                }),
        );
        diagnostics.extend(
            workspace
                .document
                .plugin_requests
                .requests
                .keys()
                .map(|id| ConfigDiagnostic {
                    code: ConfigDiagnosticCode::WorkspacePluginPendingTrust,
                    subject: id.to_string(),
                }),
        );
        diagnostics.extend(
            workspace
                .document
                .hooks
                .hooks
                .keys()
                .map(|id| ConfigDiagnostic {
                    code: ConfigDiagnosticCode::WorkspaceHookPendingTrust,
                    subject: id.to_string(),
                }),
        );
        diagnostics.extend(
            workspace
                .document
                .skills
                .sources
                .keys()
                .map(|id| ConfigDiagnostic {
                    code: ConfigDiagnosticCode::WorkspaceSkillPendingTrust,
                    subject: id.to_string(),
                }),
        );
        values.workspace = Some(WorkspaceConfigIntent {
            workspace_id,
            mcp: workspace.document.mcp.clone(),
            plugin_requests: workspace.document.plugin_requests.clone(),
            skills: workspace.document.skills.clone(),
            hooks: workspace.document.hooks.clone(),
        });
    }

    Ok(ScopedConfigSnapshot {
        user_revision: user.revision,
        user_generation: user.generation,
        workspace_revision,
        values,
        provenance,
        diagnostics,
    })
}

fn apply_workspace_model_preference(
    values: &mut ResolvedConfig,
    provenance: &mut ConfigProvenance,
    diagnostics: &mut Vec<ConfigDiagnostic>,
    workspace_id: &WorkspaceId,
    model: &ModelRef,
) {
    if values.providers.contains_key(&model.provider) {
        values.preferred_model = Some(model.clone());
        provenance.preferred_model = Some(ConfigValueSource::Workspace(workspace_id.clone()));
        return;
    }
    diagnostics.push(ConfigDiagnostic {
        code: ConfigDiagnosticCode::WorkspacePreferredModelProviderUnconfigured,
        subject: model.provider.to_string(),
    });
}
