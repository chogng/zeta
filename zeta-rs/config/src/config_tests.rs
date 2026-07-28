use super::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_model_provider_config::{ModelProviderConfig, ProviderConfigRegistry};
use zeta_protocol::{CommandId, Patch, ProviderId};

fn config_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-config-{label}-{}-{}.authority.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn workspace_config_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-workspace-config-{label}-{}-{}.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn remove_config_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("lock"));
    let _ = std::fs::remove_file(path.with_extension("tmp"));
}

fn provider_id(value: &str) -> ProviderId {
    ProviderId::new(value).unwrap()
}

fn model_ref(provider: &str, model: &str) -> ModelRef {
    ModelRef::new(
        provider_id(provider),
        zeta_protocol::ModelId::new(model).unwrap(),
    )
}

fn configure_provider(store: &ConfigStore, revision: u64, provider: &str) -> ConfigCommandResult {
    store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new(format!("configure-{provider}-{revision}")).unwrap(),
            expected_revision: ConfigRevision::new(revision),
            command: UserConfigCommand::ConfigureProvider {
                provider: provider_id(provider),
                config: ModelProviderConfig::new(provider_id(provider)),
            },
        })
        .unwrap()
}

fn update_preferences(
    command_id: &str,
    revision: u64,
    preferred_model: Patch<ModelRef>,
    theme: Patch<Theme>,
) -> ConfigCommandRequest {
    ConfigCommandRequest {
        command_id: CommandId::new(command_id).unwrap(),
        expected_revision: ConfigRevision::new(revision),
        command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
            preferred_model,
            approval_review_model: Patch::Missing,
            theme,
        }),
    }
}

fn mcp_server() -> McpServerConfig {
    McpServerConfig {
        id: McpServerId::new("user:mcp:github").unwrap(),
        display_name: "GitHub".into(),
        transport: McpTransportConfig::StreamableHttp {
            url: "https://mcp.github.example".into(),
        },
        credential: McpCredentialBinding::Reference {
            credential_ref: "user:credential:github".into(),
        },
        enablement: McpServerEnablement::Disabled,
    }
}

fn skill_source() -> SkillSourceConfig {
    SkillSourceConfig {
        id: SkillSourceId::new("user:skill-source:personal").unwrap(),
        root_reference: "user:skill-root:personal".into(),
        enablement: SkillSourceEnablement::Disabled,
    }
}

fn workspace_scope() -> WorkspaceConfigScope {
    WorkspaceConfigScope::new(WorkspaceId::new("project").unwrap())
}

fn workspace_document(preferred_model: Option<ModelRef>) -> WorkspaceConfigDocument {
    let mcp_server = WorkspaceMcpServerConfig {
        id: McpServerId::new("workspace:project:mcp:github").unwrap(),
        display_name: "Project GitHub".into(),
        transport: McpTransportConfig::Stdio {
            command: "github-mcp".into(),
            args: Vec::new(),
        },
        enablement: McpServerEnablement::Enabled,
    };
    let skill_source = SkillSourceConfig {
        id: SkillSourceId::new("workspace:project:skill-source:review").unwrap(),
        root_reference: "workspace:skill-root:review".into(),
        enablement: SkillSourceEnablement::Enabled,
    };
    let plugin_id = PluginId::new("acme/code-review").unwrap();
    WorkspaceConfigDocument {
        agent: WorkspaceAgentConfig { preferred_model },
        mcp: WorkspaceMcpConfig {
            servers: BTreeMap::from([(mcp_server.id.clone(), mcp_server)]),
        },
        plugin_requests: WorkspacePluginRequests {
            requests: BTreeMap::from([(
                plugin_id.clone(),
                WorkspacePluginRequest {
                    plugin_id,
                    version: PluginVersion::new("1.2.3").unwrap(),
                    requested_scope: WorkspacePluginRequestScope::Workspace,
                },
            )]),
        },
        skills: WorkspaceSkillsConfig {
            sources: BTreeMap::from([(skill_source.id.clone(), skill_source)]),
        },
    }
}

#[test]
fn authority_uses_one_file_and_survives_reopen() {
    let path = config_path("single-authority");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let updated = store
        .apply(update_preferences(
            "select-model",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
            Patch::Value(Theme::Dark),
        ))
        .unwrap();

    let reopened = ConfigStore::open(&path).unwrap();
    let snapshot = reopened.read_snapshot().unwrap();
    assert_eq!(updated.revision, ConfigRevision::new(2));
    assert_eq!(snapshot.revision, updated.revision);
    assert_eq!(snapshot.generation.get(), 2);
    assert_eq!(snapshot.values.theme, Some(Theme::Dark));
    assert_eq!(
        snapshot.values.preferred_model,
        Some(model_ref("openai", "model"))
    );
    assert_eq!(
        snapshot.values.selected_provider().unwrap().provider,
        provider_id("openai")
    );
    assert!(!path.with_extension("authority.json").exists());
    remove_config_files(&path);
}

#[test]
fn preference_patches_preserve_missing_fields_and_clear_null_fields() {
    let path = config_path("patch-semantics");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let first = store
        .apply(update_preferences(
            "initial",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
            Patch::Value(Theme::Dark),
        ))
        .unwrap();
    store
        .apply(update_preferences(
            "clear-theme",
            first.revision.get(),
            Patch::Missing,
            Patch::Null,
        ))
        .unwrap();

    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(
        snapshot.values.preferred_model,
        Some(model_ref("openai", "model"))
    );
    assert_eq!(snapshot.values.theme, None);
    remove_config_files(&path);
}

#[test]
fn selected_model_must_reference_a_configured_provider() {
    let path = config_path("selected-provider");
    let store = ConfigStore::open(&path).unwrap();
    let error = store
        .apply(update_preferences(
            "select-missing-provider",
            0,
            Patch::Value(model_ref("openai", "model")),
            Patch::Missing,
        ))
        .unwrap_err();

    assert!(matches!(error, ConfigCommandError::Config(_)));
    assert_eq!(
        store.read_snapshot().unwrap().revision,
        ConfigRevision::INITIAL
    );
    remove_config_files(&path);
}

#[test]
fn approval_review_model_is_explicit_and_keeps_its_provider_configured() {
    let path = config_path("approval-review-model");
    let store = ConfigStore::open(&path).unwrap();
    assert_eq!(
        store.read_snapshot().unwrap().values.approval_review_model,
        ApprovalReviewModelSelection::Automatic
    );

    let missing_provider = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("select-missing-review-provider").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                preferred_model: Patch::Missing,
                approval_review_model: Patch::Value(ApprovalReviewModelSelection::Explicit {
                    model: model_ref("openai", "codex-auto-review"),
                }),
                theme: Patch::Missing,
            }),
        })
        .unwrap_err();
    assert!(matches!(missing_provider, ConfigCommandError::Config(_)));

    let configured = configure_provider(&store, 0, "openai");
    let selected = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("select-review-model").unwrap(),
            expected_revision: configured.revision,
            command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                preferred_model: Patch::Missing,
                approval_review_model: Patch::Value(ApprovalReviewModelSelection::Explicit {
                    model: model_ref("openai", "codex-auto-review"),
                }),
                theme: Patch::Missing,
            }),
        })
        .unwrap();
    assert_eq!(
        store.read_snapshot().unwrap().values.approval_review_model,
        ApprovalReviewModelSelection::Explicit {
            model: model_ref("openai", "codex-auto-review")
        }
    );

    let remove_error = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("remove-review-provider").unwrap(),
            expected_revision: selected.revision,
            command: UserConfigCommand::RemoveProvider {
                provider: provider_id("openai"),
            },
        })
        .unwrap_err();
    assert!(matches!(remove_error, ConfigCommandError::Config(_)));
    remove_config_files(&path);
}

#[test]
fn automatic_approval_review_follows_the_selected_model_provider() {
    let resolved = ResolvedConfig {
        preferred_model: Some(model_ref("anthropic", "claude-main")),
        providers: BTreeMap::from([(
            provider_id("anthropic"),
            ModelProviderConfig::new(provider_id("anthropic")),
        )]),
        ..ResolvedConfig::default()
    };

    assert_eq!(
        resolved
            .selected_approval_review_provider()
            .map(|provider| provider.provider.clone()),
        Some(provider_id("anthropic"))
    );
    assert_eq!(
        resolved
            .resolve_approval_review_model(&ProviderConfigRegistry::builtin())
            .unwrap(),
        model_ref("anthropic", "claude-sonnet-4-20250514")
    );
}

#[test]
fn provider_entries_validate_their_key_and_static_settings() {
    let path = config_path("provider-validation");
    let store = ConfigStore::open(&path).unwrap();
    let error = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("bad-provider").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::ConfigureProvider {
                provider: provider_id("openai"),
                config: ModelProviderConfig {
                    provider: provider_id("anthropic"),
                    base_url: Some("file:///tmp/provider".into()),
                    max_output_tokens: Some(0),
                },
            },
        })
        .unwrap_err();

    assert!(matches!(error, ConfigCommandError::Config(_)));
    assert!(!path.exists());
    remove_config_files(&path);
}

#[test]
fn command_replay_returns_its_original_revision_without_copying_a_snapshot() {
    let path = config_path("command-replay");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let first = update_preferences(
        "first",
        configured.revision.get(),
        Patch::Missing,
        Patch::Value(Theme::Dark),
    );
    let first_result = store.apply(first.clone()).unwrap();
    let second = store
        .apply(update_preferences(
            "second",
            first_result.revision.get(),
            Patch::Missing,
            Patch::Value(Theme::Light),
        ))
        .unwrap();

    let replayed = ConfigStore::open(&path).unwrap().apply(first).unwrap();
    assert_eq!(first_result.disposition, ConfigCommandDisposition::Updated);
    assert_eq!(replayed.disposition, ConfigCommandDisposition::Replayed);
    assert_eq!(replayed.revision, first_result.revision);
    assert_eq!(second.revision, ConfigRevision::new(3));
    assert_eq!(
        store.read_snapshot().unwrap().values.theme,
        Some(Theme::Light)
    );

    let persisted = std::fs::read_to_string(&path).unwrap();
    assert!(persisted.contains("resultRevision"));
    assert!(!persisted.contains("\"result\""));
    remove_config_files(&path);
}

#[test]
fn no_op_command_keeps_the_resolved_snapshot_generation() {
    let path = config_path("no-op-generation");
    let store = ConfigStore::open(&path).unwrap();
    let first = store
        .apply(update_preferences(
            "set-dark",
            0,
            Patch::Missing,
            Patch::Value(Theme::Dark),
        ))
        .unwrap();
    let no_op = store
        .apply(update_preferences(
            "set-dark-again",
            first.revision.get(),
            Patch::Missing,
            Patch::Value(Theme::Dark),
        ))
        .unwrap();

    assert_eq!(no_op.disposition, ConfigCommandDisposition::Updated);
    assert_eq!(no_op.revision, first.revision);
    assert_eq!(no_op.generation, first.generation);
    assert_eq!(store.read_snapshot().unwrap().generation, first.generation);
    remove_config_files(&path);
}

#[test]
fn command_rejects_stale_revisions_and_conflicting_retries() {
    let path = config_path("revision-conflict");
    let store = ConfigStore::open(&path).unwrap();
    let first = update_preferences("first", 0, Patch::Missing, Patch::Value(Theme::Dark));
    store.apply(first.clone()).unwrap();

    assert_eq!(
        store
            .apply(update_preferences(
                "stale",
                0,
                Patch::Missing,
                Patch::Value(Theme::Light),
            ))
            .unwrap_err(),
        ConfigCommandError::RevisionConflict {
            expected: ConfigRevision::INITIAL,
            actual: ConfigRevision::new(1),
        }
    );
    assert_eq!(
        store
            .apply(update_preferences(
                "first",
                1,
                Patch::Missing,
                Patch::Value(Theme::Light),
            ))
            .unwrap_err(),
        ConfigCommandError::CommandConflict
    );
    remove_config_files(&path);
}

#[test]
fn a_preferred_provider_cannot_be_removed_until_the_model_is_cleared() {
    let path = config_path("remove-provider");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let selected = store
        .apply(update_preferences(
            "select",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
            Patch::Missing,
        ))
        .unwrap();
    assert!(matches!(
        store.apply(ConfigCommandRequest {
            command_id: CommandId::new("remove-in-use").unwrap(),
            expected_revision: selected.revision,
            command: UserConfigCommand::RemoveProvider {
                provider: provider_id("openai"),
            },
        }),
        Err(ConfigCommandError::Config(_))
    ));
    let cleared = store
        .apply(update_preferences(
            "clear-model",
            selected.revision.get(),
            Patch::Null,
            Patch::Missing,
        ))
        .unwrap();
    store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("remove-after-clear").unwrap(),
            expected_revision: cleared.revision,
            command: UserConfigCommand::RemoveProvider {
                provider: provider_id("openai"),
            },
        })
        .unwrap();
    assert!(store.read_snapshot().unwrap().values.providers.is_empty());
    remove_config_files(&path);
}

#[test]
fn mcp_and_skill_declarations_are_durable_desired_config() {
    let path = config_path("mcp-and-skills");
    let store = ConfigStore::open(&path).unwrap();
    let mcp = mcp_server();
    let added_mcp = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("add-mcp").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::UpsertMcpServer {
                server: mcp.clone(),
            },
        })
        .unwrap();
    let source = skill_source();
    let added_source = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("add-skill-source").unwrap(),
            expected_revision: added_mcp.revision,
            command: UserConfigCommand::AddSkillSource {
                source: source.clone(),
            },
        })
        .unwrap();
    let enabled = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-mcp").unwrap(),
            expected_revision: added_source.revision,
            command: UserConfigCommand::SetMcpServerEnablement {
                server_id: mcp.id.clone(),
                enablement: McpServerEnablement::Enabled,
            },
        })
        .unwrap();

    let snapshot = ConfigStore::open(&path).unwrap().read_snapshot().unwrap();
    assert_eq!(snapshot.revision, enabled.revision);
    assert_eq!(
        snapshot.values.mcp.servers[&mcp.id].enablement,
        McpServerEnablement::Enabled
    );
    assert_eq!(
        snapshot.values.skills.sources[&source.id].root_reference,
        "user:skill-root:personal"
    );
    let persisted = std::fs::read_to_string(&path).unwrap();
    assert!(persisted.contains("credentialRef"));
    assert!(!persisted.contains("secretValue"));
    remove_config_files(&path);
}

#[test]
fn mcp_and_skill_declarations_reject_invalid_identity_or_missing_target() {
    assert!(McpServerId::new("github").is_err());
    assert!(SkillSourceId::new("personal").is_err());

    let path = config_path("mcp-and-skills-validation");
    let store = ConfigStore::open(&path).unwrap();
    let error = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-missing-mcp").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetMcpServerEnablement {
                server_id: McpServerId::new("user:mcp:missing").unwrap(),
                enablement: McpServerEnablement::Enabled,
            },
        })
        .unwrap_err();

    assert!(matches!(error, ConfigCommandError::Config(_)));
    assert_eq!(
        store.read_snapshot().unwrap().revision,
        ConfigRevision::INITIAL
    );
    remove_config_files(&path);
}

#[test]
fn workspace_document_is_namespaced_and_cannot_bind_credentials() {
    let path = workspace_config_path("declared-intent");
    std::fs::write(
        &path,
        serde_json::json!({
            "agent": {"preferredModel": {"provider": "openai", "model": "gpt-5.6"}},
            "mcp": {
                "servers": {
                    "workspace:project:mcp:github": {
                        "id": "workspace:project:mcp:github",
                        "displayName": "Project GitHub",
                        "transport": {
                            "type": "streamableHttp",
                            "url": "https://mcp.github.example"
                        },
                        "enablement": "enabled"
                    }
                }
            },
            "pluginRequests": {
                "requests": {
                    "acme/code-review": {
                        "pluginId": "acme/code-review",
                        "version": "1.2.3",
                        "requestedScope": "workspace"
                    }
                }
            },
            "skills": {
                "sources": {
                    "workspace:project:skill-source:review": {
                        "id": "workspace:project:skill-source:review",
                        "rootReference": "workspace:skill-root:review",
                        "enablement": "enabled"
                    }
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let scope = WorkspaceConfigScope::new(WorkspaceId::new("project").unwrap());
    let document = WorkspaceConfigStore::open(&path, scope)
        .read_document()
        .unwrap();
    assert_eq!(document.mcp.servers.len(), 1);
    assert_eq!(document.skills.sources.len(), 1);
    assert_eq!(document.plugin_requests.requests.len(), 1);
    assert_eq!(
        document
            .plugin_requests
            .requests
            .values()
            .next()
            .unwrap()
            .version
            .as_str(),
        "1.2.3"
    );
    remove_config_files(&path);
}

#[test]
fn workspace_document_rejects_foreign_namespace_and_unknown_fields() {
    let path = workspace_config_path("invalid");
    let scope = WorkspaceConfigScope::new(WorkspaceId::new("project").unwrap());
    std::fs::write(
        &path,
        serde_json::json!({
            "mcp": {
                "servers": {
                    "workspace:other:mcp:github": {
                        "id": "workspace:other:mcp:github",
                        "displayName": "Other GitHub",
                        "transport": {"type": "stdio", "command": "github-mcp", "args": []}
                    }
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    assert!(
        WorkspaceConfigStore::open(&path, scope.clone())
            .read_document()
            .is_err()
    );

    std::fs::write(&path, r#"{"unknown":true}"#).unwrap();
    assert!(
        WorkspaceConfigStore::open(&path, scope)
            .read_document()
            .is_err()
    );
    remove_config_files(&path);
}

#[test]
fn workspace_resolution_overrides_only_a_user_configured_model_provider() {
    let path = config_path("workspace-resolution");
    let store = ConfigStore::open(&path).unwrap();
    configure_provider(&store, 0, "openai");
    let user = store.read_snapshot().unwrap();
    let scope = workspace_scope();
    let document = workspace_document(Some(model_ref("openai", "gpt-5.6")));

    let resolved = resolve_scoped_config(
        &user,
        Some(WorkspaceConfigInput::new(
            &scope,
            WorkspaceConfigRevision::new(7),
            &document,
        )),
    )
    .unwrap();

    assert_eq!(resolved.user_revision, user.revision);
    assert_eq!(
        resolved.workspace_revision,
        Some(WorkspaceConfigRevision::new(7))
    );
    assert_eq!(
        resolved.values.preferred_model,
        Some(model_ref("openai", "gpt-5.6"))
    );
    assert_eq!(
        resolved.provenance.preferred_model,
        Some(ConfigValueSource::Workspace(
            WorkspaceId::new("project").unwrap()
        ))
    );
    assert_eq!(
        resolved
            .values
            .workspace
            .as_ref()
            .unwrap()
            .plugin_requests
            .requests
            .len(),
        1
    );
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ConfigDiagnosticCode::WorkspaceMcpPendingTrust
            && diagnostic.subject == "workspace:project:mcp:github"
    }));
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ConfigDiagnosticCode::WorkspaceSkillPendingTrust
            && diagnostic.subject == "workspace:project:skill-source:review"
    }));
    remove_config_files(&path);
}

#[test]
fn workspace_resolution_keeps_user_model_when_the_workspace_provider_is_unconfigured() {
    let path = config_path("workspace-model-rejected");
    let store = ConfigStore::open(&path).unwrap();
    let user = store.read_snapshot().unwrap();
    let scope = workspace_scope();
    let document = workspace_document(Some(model_ref("anthropic", "claude")));

    let resolved = resolve_scoped_config(
        &user,
        Some(WorkspaceConfigInput::new(
            &scope,
            WorkspaceConfigRevision::INITIAL,
            &document,
        )),
    )
    .unwrap();

    assert_eq!(resolved.values.preferred_model, None);
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ConfigDiagnosticCode::WorkspacePreferredModelProviderUnconfigured
            && diagnostic.subject == "anthropic"
    }));
    remove_config_files(&path);
}
