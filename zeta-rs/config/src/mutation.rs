use crate::{ConfigError, PreferencesUpdate, UserConfigCommand, UserConfigDocument};
use zeta_protocol::Patch;

pub(crate) fn apply_command(
    document: &mut UserConfigDocument,
    command: &UserConfigCommand,
) -> Result<(), ConfigError> {
    match command {
        UserConfigCommand::UpdatePreferences(update) => apply_preferences(document, update),
        UserConfigCommand::ConfigureProvider { provider, config } => {
            if &config.provider != provider {
                return Err(ConfigError(format!(
                    "provider command key '{}' does not match configuration provider '{}'",
                    provider, config.provider
                )));
            }
            document.providers.insert(provider.clone(), config.clone());
        }
        UserConfigCommand::RemoveProvider { provider } => {
            if document
                .agent
                .preferred_model
                .as_ref()
                .is_some_and(|model| model.provider == *provider)
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while it is the preferred model provider",
                    provider
                )));
            }
            if document
                .agent
                .approval_review_model
                .explicit_model()
                .is_some_and(|model| model.provider == *provider)
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while it is the approval review model provider",
                    provider
                )));
            }
            if document
                .semantic_code_index
                .selection
                .remote_models()
                .is_some_and(|models| {
                    models.embedding_model.provider == *provider
                        || models
                            .rerank_model
                            .as_ref()
                            .is_some_and(|model| model.provider == *provider)
                })
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while semantic code indexing uses it",
                    provider
                )));
            }
            if document
                .tool_search
                .embedding_model
                .as_ref()
                .is_some_and(|model| model.provider == *provider)
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while Tool Search uses it",
                    provider
                )));
            }
            document.providers.remove(provider);
        }
        UserConfigCommand::UpsertMcpServer { server } => {
            document
                .mcp
                .servers
                .insert(server.id.clone(), server.clone());
        }
        UserConfigCommand::RemoveMcpServer { server_id } => {
            document.mcp.servers.remove(server_id);
        }
        UserConfigCommand::SetMcpServerEnablement {
            server_id,
            enablement,
        } => {
            let server = document.mcp.servers.get_mut(server_id).ok_or_else(|| {
                ConfigError(format!("MCP server '{}' is not configured", server_id))
            })?;
            server.enablement = *enablement;
        }
        UserConfigCommand::AddSkillSource { source } => {
            document
                .skills
                .sources
                .insert(source.id.clone(), source.clone());
        }
        UserConfigCommand::RemoveSkillSource { source_id } => {
            document.skills.sources.remove(source_id);
        }
        UserConfigCommand::SetSkillSourceEnablement {
            source_id,
            enablement,
        } => {
            let source = document.skills.sources.get_mut(source_id).ok_or_else(|| {
                ConfigError(format!("Skill source '{}' is not configured", source_id))
            })?;
            source.enablement = *enablement;
        }
        UserConfigCommand::SetSkillEnablement {
            skill_id,
            enablement,
        } => {
            document.skills.set_skill_enablement(skill_id, *enablement);
        }
        UserConfigCommand::UpsertPluginRequest { request } => {
            document
                .plugins
                .requests
                .insert(request.plugin_id.clone(), request.clone());
        }
        UserConfigCommand::RemovePluginRequest { plugin_id } => {
            document.plugins.requests.remove(plugin_id);
        }
        UserConfigCommand::SetPluginRequestEnablement {
            plugin_id,
            enablement,
        } => {
            let request = document
                .plugins
                .requests
                .get_mut(plugin_id)
                .ok_or_else(|| ConfigError(format!("Plugin '{}' is not requested", plugin_id)))?;
            request.enablement = *enablement;
        }
        UserConfigCommand::UpsertHook { hook } => {
            document.hooks.hooks.insert(hook.id.clone(), hook.clone());
        }
        UserConfigCommand::RemoveHook { hook_id } => {
            document.hooks.hooks.remove(hook_id);
        }
        UserConfigCommand::SetHookEnablement {
            hook_id,
            enablement,
        } => {
            let hook = document
                .hooks
                .hooks
                .get_mut(hook_id)
                .ok_or_else(|| ConfigError(format!("Hook '{}' is not configured", hook_id)))?;
            hook.enablement = *enablement;
        }
        UserConfigCommand::ConfigureLanguageServer { server_id, config } => {
            config.validate()?;
            document
                .language_servers
                .servers
                .insert(server_id.clone(), config.clone());
        }
        UserConfigCommand::RemoveLanguageServerConfiguration { server_id } => {
            document.language_servers.servers.remove(server_id);
        }
        UserConfigCommand::ConfigureSemanticCodeIndex {
            selection,
            automatic_context,
        } => {
            document
                .semantic_code_index
                .replace_selection(selection.clone());
            document
                .semantic_code_index
                .replace_automatic_context(*automatic_context);
        }
        UserConfigCommand::ConfigureToolSearch { config } => {
            document.tool_search = config.clone();
        }
        UserConfigCommand::AuthorizeSemanticCodeIndexEgress { workspace } => {
            let providers = document.providers.clone();
            document
                .semantic_code_index
                .authorize(workspace.clone(), &providers)
                .map_err(|message| ConfigError(message.into()))?;
        }
        UserConfigCommand::RevokeSemanticCodeIndexEgress { workspace } => {
            document.semantic_code_index.revoke(workspace);
        }
        UserConfigCommand::SetWorkspaceTrust { workspace, setting } => {
            document
                .workspace_trust
                .roots
                .insert(workspace.clone(), *setting);
        }
        UserConfigCommand::ForgetWorkspaceTrust { workspace } => {
            document.workspace_trust.roots.remove(workspace);
        }
    }
    Ok(())
}

fn apply_preferences(document: &mut UserConfigDocument, update: &PreferencesUpdate) {
    match &update.preferred_model {
        Patch::Missing => {}
        Patch::Null => document.agent.preferred_model = None,
        Patch::Value(model) => document.agent.preferred_model = Some(model.clone()),
    }
    match &update.approval_review_model {
        Patch::Missing => {}
        Patch::Null => {
            document.agent.approval_review_model = crate::ApprovalReviewModelSelection::Automatic;
        }
        Patch::Value(selection) => {
            document.agent.approval_review_model = selection.clone();
        }
    }
}
