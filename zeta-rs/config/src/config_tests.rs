use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_model_provider_config::{ModelProviderConfig, ProviderConfigRegistry};
use zeta_protocol::{CommandId, Patch, ProviderId};
use zeta_workspace::{WorkspaceTrustDecision, WorkspaceTrustSource};

fn config_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-config-{label}-{}-{}.sqlite3",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn workspace_config_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-workspace-config-{label}-{}-{}.toml",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn remove_config_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("toml"));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
}

fn persisted_config_document(path: &Path) -> String {
    std::fs::read_to_string(path.with_extension("toml")).unwrap()
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

#[test]
fn tool_search_defaults_to_lexical_and_requires_a_configured_embedding_model() {
    let default_document = toml::from_str::<UserConfigDocument>("").unwrap();
    assert_eq!(
        default_document.tool_search.mode,
        ToolSearchModeConfig::Lexical
    );

    let invalid_hybrid =
        toml::from_str::<UserConfigDocument>("[toolSearch]\nmode = \"hybridEmbedding\"\n").unwrap();
    assert!(invalid_hybrid.validate().is_err());

    let provider = provider_id("ollama");
    let embedding_model = model_ref("ollama", "nomic-embed-text");
    let mut hybrid_document = UserConfigDocument::default();
    hybrid_document
        .providers
        .insert(provider.clone(), ModelProviderConfig::new(provider));
    hybrid_document.tool_search = ToolSearchConfig {
        mode: ToolSearchModeConfig::HybridEmbedding,
        embedding_model: Some(embedding_model.clone()),
    };
    hybrid_document.validate().unwrap();
    assert_eq!(
        ResolvedConfig::from(&hybrid_document).tool_search,
        ToolSearchConfig {
            mode: ToolSearchModeConfig::HybridEmbedding,
            embedding_model: Some(embedding_model),
        }
    );
}

#[test]
fn tool_search_command_persists_the_exact_embedding_model() {
    let path = config_path("tool-search-command");
    let store = ConfigStore::open(&path).unwrap();
    let provider = configure_provider(&store, 0, "ollama");
    let embedding_model = model_ref("ollama", "nomic-embed-text");

    let configured = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("configure-tool-search").unwrap(),
            expected_revision: provider.revision,
            command: UserConfigCommand::ConfigureToolSearch {
                config: ToolSearchConfig {
                    mode: ToolSearchModeConfig::HybridEmbedding,
                    embedding_model: Some(embedding_model.clone()),
                },
            },
        })
        .unwrap();

    assert_eq!(configured.revision, ConfigRevision::new(2));
    assert_eq!(
        store.read_snapshot().unwrap().values.tool_search,
        ToolSearchConfig {
            mode: ToolSearchModeConfig::HybridEmbedding,
            embedding_model: Some(embedding_model),
        }
    );
    drop(store);
    remove_config_files(&path);
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
) -> ConfigCommandRequest {
    ConfigCommandRequest {
        command_id: CommandId::new(command_id).unwrap(),
        expected_revision: ConfigRevision::new(revision),
        command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
            preferred_model,
            approval_review_model: Patch::Missing,
            tool_mode: Patch::Missing,
        }),
    }
}

fn workspace_trust_id() -> WorkspaceTrustId {
    format!("sha256:{}", "12".repeat(32)).parse().unwrap()
}

#[test]
fn tool_mode_defaults_to_direct_and_updates_durably() {
    let store = ConfigStore::open(&config_path("tool-mode")).unwrap();
    assert_eq!(
        store.read_snapshot().unwrap().values.tool_mode,
        zeta_protocol::ToolMode::Direct
    );

    store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("select-code-mode-only").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                preferred_model: Patch::Missing,
                approval_review_model: Patch::Missing,
                tool_mode: Patch::Value(zeta_protocol::ToolMode::CodeModeOnly),
            }),
        })
        .unwrap();

    assert_eq!(
        store.read_snapshot().unwrap().values.tool_mode,
        zeta_protocol::ToolMode::CodeModeOnly
    );
}

#[test]
fn semantic_code_index_egress_grants_are_bound_to_workspace_models_and_provider_config() {
    let workspace = workspace_trust_id();
    let provider = provider_id("openai-compatible");
    let mut provider_config = ModelProviderConfig::new(provider.clone());
    provider_config.base_url = Some("https://models.example.test/v1".into());
    let mut providers = BTreeMap::from([(provider.clone(), provider_config.clone())]);
    let first_models = SemanticCodeIndexModelSelection {
        embedding_model: model_ref("openai-compatible", "embed-v1"),
        rerank_model: Some(model_ref("openai-compatible", "rerank-v1")),
    };
    let mut config = SemanticCodeIndexConfig::default();
    config.replace_selection(SemanticCodeIndexSelection::Remote {
        models: first_models.clone(),
    });
    config.authorize(workspace.clone(), &providers).unwrap();
    assert_eq!(
        config.authorized_remote_models(&workspace, &providers),
        Some(&first_models)
    );

    providers.get_mut(&provider).unwrap().base_url =
        Some("https://different.example.test/v1".into());
    assert_eq!(
        config.authorized_remote_models(&workspace, &providers),
        None
    );

    providers.insert(provider, provider_config);
    config.replace_selection(SemanticCodeIndexSelection::Remote {
        models: SemanticCodeIndexModelSelection {
            embedding_model: model_ref("openai-compatible", "embed-v2"),
            rerank_model: None,
        },
    });
    assert_eq!(
        config.authorized_remote_models(&workspace, &providers),
        None
    );

    config.authorize(workspace.clone(), &providers).unwrap();
    assert!(
        config
            .authorized_remote_models(&workspace, &providers)
            .is_some()
    );
    config.revoke(&workspace);
    assert_eq!(
        config.authorized_remote_models(&workspace, &providers),
        None
    );
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

fn plugin_request() -> PluginRequest {
    PluginRequest {
        plugin_id: PluginId::new("acme/code-review").unwrap(),
        version: PluginVersion::new("1.2.3").unwrap(),
        enablement: PluginRequestEnablement::Disabled,
    }
}

fn hook(id: &str) -> HookConfig {
    HookConfig {
        id: HookId::new(id).unwrap(),
        event: HookEvent::BeforeTool,
        matcher: HookMatcher {
            tool_names: BTreeSet::from(["shell_command".into()]),
        },
        action: HookAction::Process {
            program: "review-hook".into(),
            args: vec!["--check".into()],
        },
        enablement: HookEnablement::Disabled,
    }
}

fn built_in_skill() -> SkillId {
    SkillId::new(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        SkillName::new("skill-creator").unwrap(),
    )
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
        hooks: HooksConfig {
            hooks: BTreeMap::from([(
                HookId::new("workspace:project:hook:review").unwrap(),
                hook("workspace:project:hook:review"),
            )]),
        },
        exec_policy: WorkspaceExecPolicyConfig::default(),
    }
}

#[test]
fn toml_authority_and_sqlite_metadata_survive_reopen() {
    let path = config_path("single-authority");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let updated = store
        .apply(update_preferences(
            "select-model",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
        ))
        .unwrap();

    let reopened = ConfigStore::open(&path).unwrap();
    let snapshot = reopened.read_snapshot().unwrap();
    assert_eq!(updated.revision, ConfigRevision::new(2));
    assert_eq!(snapshot.revision, updated.revision);
    assert_eq!(snapshot.generation.get(), 2);
    assert_eq!(
        snapshot.values.preferred_model,
        Some(model_ref("openai", "model"))
    );
    assert_eq!(
        snapshot.values.selected_provider().unwrap().provider,
        provider_id("openai")
    );
    assert!(path.with_extension("toml").exists());
    let columns: Vec<String> = rusqlite::Connection::open(&path)
        .unwrap()
        .prepare("PRAGMA table_info(config_metadata)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!columns.iter().any(|column| column == "document_json"));
    remove_config_files(&path);
}

#[test]
fn legacy_sqlite_document_is_migrated_once_into_toml() {
    let path = config_path("legacy-document-migration");
    let provider = ModelProviderConfig::new(provider_id("openai"));
    let document = UserConfigDocument {
        providers: BTreeMap::from([(provider_id("openai"), provider)]),
        ..UserConfigDocument::default()
    };
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE zeta_schema_migrations (
                 component TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );
             INSERT INTO zeta_schema_migrations (component, version) VALUES ('config', 1);
             CREATE TABLE config_authority (
                 authority_id INTEGER PRIMARY KEY,
                 schema_version INTEGER NOT NULL,
                 revision INTEGER NOT NULL,
                 generation INTEGER NOT NULL,
                 document_json TEXT NOT NULL
             );
             CREATE TABLE config_command_receipts (
                 command_id TEXT PRIMARY KEY,
                 expected_revision INTEGER NOT NULL,
                 command_json TEXT NOT NULL,
                 result_revision INTEGER NOT NULL,
                 result_generation INTEGER NOT NULL
             );",
        )
        .unwrap();
    let mut legacy_document = serde_json::to_value(&document).unwrap();
    legacy_document
        .as_object_mut()
        .unwrap()
        .remove("languageServers");
    connection
        .execute(
            "INSERT INTO config_authority VALUES (1, 7, 7, 9, ?1)",
            [serde_json::to_string(&legacy_document).unwrap()],
        )
        .unwrap();
    drop(connection);

    let store = ConfigStore::open(&path).unwrap();
    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(snapshot.revision, ConfigRevision::new(7));
    assert_eq!(snapshot.generation, ConfigGeneration::new(9));
    assert!(
        snapshot
            .values
            .providers
            .contains_key(&provider_id("openai"))
    );
    assert!(store.config_path().exists());
    let old_table: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table' AND name = 'config_authority'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old_table, 0);
    drop(store);
    remove_config_files(&path);
}

#[test]
fn additive_document_schema_upgrade_keeps_revision_and_generation() {
    let path = config_path("document-schema-upgrade");
    let config_path = path.with_extension("toml");
    std::fs::write(
        &config_path,
        toml::to_string_pretty(&UserConfigDocument::default()).unwrap(),
    )
    .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE zeta_schema_migrations (
                 component TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );
             INSERT INTO zeta_schema_migrations (component, version) VALUES ('config', 2);
             CREATE TABLE config_metadata (
                 authority_id INTEGER PRIMARY KEY,
                 document_schema_version INTEGER NOT NULL,
                 revision INTEGER NOT NULL,
                 generation INTEGER NOT NULL,
                 content_digest TEXT NOT NULL
             );
             INSERT INTO config_metadata VALUES (1, 7, 7, 9, 'legacy-digest');
             CREATE TABLE config_command_receipts (
                 command_id TEXT PRIMARY KEY,
                 expected_revision INTEGER NOT NULL,
                 command_json TEXT NOT NULL,
                 result_revision INTEGER NOT NULL,
                 result_generation INTEGER NOT NULL
             );",
        )
        .unwrap();
    drop(connection);

    let store = ConfigStore::open(&path).unwrap();
    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(snapshot.revision, ConfigRevision::new(7));
    assert_eq!(snapshot.generation, ConfigGeneration::new(9));
    let document_schema_version: u32 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT document_schema_version FROM config_metadata WHERE authority_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(document_schema_version, 9);
    drop(store);
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
        ))
        .unwrap();
    store
        .apply(update_preferences(
            "clear-model",
            first.revision.get(),
            Patch::Null,
        ))
        .unwrap();

    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(snapshot.values.preferred_model, None);
    remove_config_files(&path);
}

#[test]
fn workspace_trust_commands_persist_user_owned_decisions() {
    let path = config_path("workspace-trust");
    let store = ConfigStore::open(&path).unwrap();
    let workspace = workspace_trust_id();
    let display_root = std::path::PathBuf::from("/tmp/zeta-workspace-trust");
    let trusted = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: workspace.clone(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: Some(display_root.clone()),
            },
        })
        .unwrap();

    assert_eq!(
        store
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .decision_for(&workspace),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::ExplicitUserDecision)
    );
    assert_eq!(
        store
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .explicit_root_path_for(&workspace),
        Some(display_root.as_path())
    );
    assert!(persisted_config_document(&path).contains("[workspaceTrust.roots]"));

    store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("forget-workspace").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::ForgetWorkspaceTrust {
                workspace: workspace.clone(),
            },
        })
        .unwrap();
    assert_eq!(
        store
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .decision_for(&workspace),
        WorkspaceTrustDecision::Restricted
    );
    assert_eq!(
        store
            .read_snapshot()
            .unwrap()
            .values
            .workspace_trust
            .explicit_root_path_for(&workspace),
        None
    );
    remove_config_files(&path);
}

#[test]
fn legacy_restricted_workspace_entries_are_removed_when_config_opens() {
    let path = config_path("workspace-trust-legacy-restricted");
    let workspace = workspace_trust_id();
    std::fs::write(
        path.with_extension("toml"),
        format!(
            "[workspaceTrust.roots]\n\"{workspace}\" = \"restricted\"\n\n[workspaceTrust.rootPaths]\n\"{workspace}\" = \"/tmp/legacy-workspace\"\n"
        ),
    )
    .unwrap();

    let store = ConfigStore::open(&path).unwrap();
    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(
        snapshot
            .values
            .workspace_trust
            .explicit_setting_for(&workspace),
        None
    );
    assert_eq!(
        snapshot.values.workspace_trust.decision_for(&workspace),
        WorkspaceTrustDecision::Restricted
    );
    let persisted = persisted_config_document(&path);
    assert!(!persisted.contains("restricted"));
    assert!(!persisted.contains("legacy-workspace"));
    drop(store);
    remove_config_files(&path);
}

#[test]
fn setting_workspace_restricted_removes_the_allowlist_entry() {
    let path = config_path("workspace-trust-revoke");
    let store = ConfigStore::open(&path).unwrap();
    let workspace = workspace_trust_id();
    let trusted = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-workspace-for-revoke").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: workspace.clone(),
                setting: WorkspaceTrustSetting::Trusted,
                display_root: Some("/tmp/revoke-workspace".into()),
            },
        })
        .unwrap();

    let restricted = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-workspace-for-revoke").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: workspace.clone(),
                setting: WorkspaceTrustSetting::Restricted,
                display_root: None,
            },
        })
        .unwrap();

    assert_eq!(restricted.revision.get(), 2);
    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(
        snapshot
            .values
            .workspace_trust
            .explicit_setting_for(&workspace),
        None
    );
    assert_eq!(
        snapshot
            .values
            .workspace_trust
            .explicit_root_path_for(&workspace),
        None
    );
    assert!(!persisted_config_document(&path).contains("restricted"));
    drop(store);
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
                tool_mode: Patch::Missing,
                approval_review_model: Patch::Value(ApprovalReviewModelSelection::Explicit {
                    model: model_ref("openai", "codex-auto-review"),
                }),
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
                tool_mode: Patch::Missing,
                approval_review_model: Patch::Value(ApprovalReviewModelSelection::Explicit {
                    model: model_ref("openai", "codex-auto-review"),
                }),
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
                    model_context: BTreeMap::new(),
                },
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
fn command_replay_returns_its_original_revision_without_copying_a_snapshot() {
    let path = config_path("command-replay");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let first = update_preferences(
        "first",
        configured.revision.get(),
        Patch::Value(model_ref("openai", "model-a")),
    );
    let first_result = store.apply(first.clone()).unwrap();
    let second = store
        .apply(update_preferences(
            "second",
            first_result.revision.get(),
            Patch::Value(model_ref("openai", "model-b")),
        ))
        .unwrap();

    let replayed = ConfigStore::open(&path).unwrap().apply(first).unwrap();
    assert_eq!(first_result.disposition, ConfigCommandDisposition::Updated);
    assert_eq!(replayed.disposition, ConfigCommandDisposition::Replayed);
    assert_eq!(replayed.revision, first_result.revision);
    assert_eq!(second.revision, ConfigRevision::new(3));
    assert_eq!(
        store.read_snapshot().unwrap().values.preferred_model,
        Some(model_ref("openai", "model-b"))
    );

    let receipt_count: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM config_command_receipts", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(receipt_count, 3);
    remove_config_files(&path);
}

#[test]
fn no_op_command_keeps_the_resolved_snapshot_generation() {
    let path = config_path("no-op-generation");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let first = store
        .apply(update_preferences(
            "set-model",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
        ))
        .unwrap();
    let no_op = store
        .apply(update_preferences(
            "set-model-again",
            first.revision.get(),
            Patch::Value(model_ref("openai", "model")),
        ))
        .unwrap();

    assert_eq!(no_op.disposition, ConfigCommandDisposition::Updated);
    assert_eq!(no_op.revision, first.revision);
    assert_eq!(no_op.generation, first.generation);
    assert_eq!(store.read_snapshot().unwrap().generation, first.generation);
    remove_config_files(&path);
}

#[test]
fn committed_changes_publish_after_the_sqlite_snapshot_advances() {
    let path = config_path("change-subscription");
    let store = ConfigStore::open(&path).unwrap();
    let changes = store.subscribe_changes();
    let configured = configure_provider(&store, 0, "openai");
    let _ = changes.recv_timeout(std::time::Duration::from_secs(1));
    let changed = store
        .apply(update_preferences(
            "set-model-and-notify",
            configured.revision.get(),
            Patch::Value(model_ref("openai", "model")),
        ))
        .unwrap();
    let notification = changes
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();

    assert_eq!(notification.revision, changed.revision);
    assert_eq!(notification.generation, changed.generation);
    assert_eq!(
        store.read_snapshot().unwrap().values.preferred_model,
        Some(model_ref("openai", "model"))
    );
    remove_config_files(&path);
}

#[test]
fn changes_committed_by_another_connection_publish_to_local_subscribers() {
    let path = config_path("cross-connection-subscription");
    let observing_store = ConfigStore::open(&path).unwrap();
    let writing_store = ConfigStore::open(&path).unwrap();
    let changes = observing_store.subscribe_changes();

    let changed = configure_provider(&writing_store, 0, "openai");
    let notification = changes
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();

    assert_eq!(notification.revision, changed.revision);
    assert_eq!(notification.generation, changed.generation);
    assert_eq!(
        observing_store
            .read_snapshot()
            .unwrap()
            .values
            .providers
            .len(),
        1
    );
    drop(writing_store);
    drop(observing_store);
    remove_config_files(&path);
}

#[test]
fn valid_external_toml_edits_advance_revision_and_publish() {
    let path = config_path("external-toml-edit");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let changes = store.subscribe_changes();
    let config_path = store.config_path().to_path_buf();
    let mut document: UserConfigDocument =
        toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    document.agent.preferred_model = Some(model_ref("openai", "external-model"));
    std::fs::write(&config_path, toml::to_string_pretty(&document).unwrap()).unwrap();

    let change = changes
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    assert_eq!(change.revision, configured.revision.next());
    assert_eq!(
        store.read_snapshot().unwrap().values.preferred_model,
        Some(model_ref("openai", "external-model"))
    );
    drop(store);
    remove_config_files(&path);
}

#[test]
fn invalid_external_toml_does_not_replace_the_last_valid_metadata() {
    let path = config_path("invalid-external-toml");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    std::fs::write(store.config_path(), "unknown = true").unwrap();

    assert!(store.read_snapshot().is_err());
    let metadata_revision: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT revision FROM config_metadata WHERE authority_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(metadata_revision as u64, configured.revision.get());
    drop(store);
    remove_config_files(&path);
}

#[test]
fn concurrent_open_installs_one_config_schema() {
    let path = config_path("concurrent-open");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let threads = (0..2)
        .map(|_| {
            let path = path.clone();
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let store = ConfigStore::open(path).unwrap();
                store.read_snapshot().unwrap().revision
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();

    for thread in threads {
        assert_eq!(thread.join().unwrap(), ConfigRevision::INITIAL);
    }
    remove_config_files(&path);
}

#[test]
fn command_rejects_stale_revisions_and_conflicting_retries() {
    let path = config_path("revision-conflict");
    let store = ConfigStore::open(&path).unwrap();
    let configured = configure_provider(&store, 0, "openai");
    let first = update_preferences(
        "first",
        configured.revision.get(),
        Patch::Value(model_ref("openai", "model-a")),
    );
    store.apply(first.clone()).unwrap();

    assert_eq!(
        store
            .apply(update_preferences(
                "stale",
                configured.revision.get(),
                Patch::Value(model_ref("openai", "model-b")),
            ))
            .unwrap_err(),
        ConfigCommandError::RevisionConflict {
            expected: ConfigRevision::new(1),
            actual: ConfigRevision::new(2),
        }
    );
    assert_eq!(
        store
            .apply(update_preferences(
                "first",
                2,
                Patch::Value(model_ref("openai", "model-b")),
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
    let persisted = persisted_config_document(&path);
    assert!(persisted.contains("credentialRef"));
    assert!(!persisted.contains("secretValue"));
    remove_config_files(&path);
}

#[test]
fn plugin_and_hook_declarations_are_durable_desired_config() {
    let path = config_path("plugin-and-hooks");
    let store = ConfigStore::open(&path).unwrap();
    let plugin = plugin_request();
    let added_plugin = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("request-plugin").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::UpsertPluginRequest {
                request: plugin.clone(),
            },
        })
        .unwrap();
    let hook = hook("user:hook:review");
    let added_hook = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("add-hook").unwrap(),
            expected_revision: added_plugin.revision,
            command: UserConfigCommand::UpsertHook { hook: hook.clone() },
        })
        .unwrap();
    let enabled_plugin = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-plugin-request").unwrap(),
            expected_revision: added_hook.revision,
            command: UserConfigCommand::SetPluginRequestEnablement {
                plugin_id: plugin.plugin_id.clone(),
                enablement: PluginRequestEnablement::Enabled,
            },
        })
        .unwrap();
    let enabled_hook = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-hook").unwrap(),
            expected_revision: enabled_plugin.revision,
            command: UserConfigCommand::SetHookEnablement {
                hook_id: hook.id.clone(),
                enablement: HookEnablement::Enabled,
            },
        })
        .unwrap();

    let snapshot = ConfigStore::open(&path).unwrap().read_snapshot().unwrap();
    assert_eq!(snapshot.revision, enabled_hook.revision);
    assert_eq!(
        snapshot.values.plugins.requests[&plugin.plugin_id].enablement,
        PluginRequestEnablement::Enabled
    );
    assert_eq!(
        snapshot.values.hooks.hooks[&hook.id].enablement,
        HookEnablement::Enabled
    );
    let persisted = persisted_config_document(&path);
    assert!(persisted.contains("[plugins.requests"));
    assert!(persisted.contains("[hooks.hooks"));
    drop(store);
    remove_config_files(&path);
}

#[test]
fn plugin_and_hook_commands_reject_missing_targets_and_unsafe_shapes() {
    assert!(PluginVersion::new("latest").is_err());
    assert!(HookId::new("review").is_err());

    let path = config_path("plugin-hook-validation");
    let store = ConfigStore::open(&path).unwrap();
    let missing_plugin = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-missing-plugin").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetPluginRequestEnablement {
                plugin_id: PluginId::new("acme/review").unwrap(),
                enablement: PluginRequestEnablement::Enabled,
            },
        })
        .unwrap_err();
    assert!(matches!(missing_plugin, ConfigCommandError::Config(_)));

    let invalid_hook = HookConfig {
        id: HookId::new("user:hook:complete").unwrap(),
        event: HookEvent::TurnCompleted,
        matcher: HookMatcher {
            tool_names: BTreeSet::from(["shell_command".into()]),
        },
        action: HookAction::Process {
            program: "notify".into(),
            args: Vec::new(),
        },
        enablement: HookEnablement::Enabled,
    };
    let error = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("invalid-hook").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::UpsertHook { hook: invalid_hook },
        })
        .unwrap_err();
    assert!(matches!(error, ConfigCommandError::Config(_)));
    drop(store);
    remove_config_files(&path);
}

#[test]
fn per_skill_enablement_is_durable_and_enabled_removes_the_override() {
    let path = config_path("skill-enablement");
    let store = ConfigStore::open(&path).unwrap();
    let skill_id = built_in_skill();
    let disabled = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("disable-built-in-skill").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetSkillEnablement {
                skill_id: skill_id.clone(),
                enablement: SkillEnablement::Disabled,
            },
        })
        .unwrap();

    let reopened = ConfigStore::open(&path).unwrap();
    assert_eq!(
        reopened
            .read_snapshot()
            .unwrap()
            .values
            .skills
            .skill_enablement(&skill_id),
        SkillEnablement::Disabled
    );
    reopened
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("enable-built-in-skill").unwrap(),
            expected_revision: disabled.revision,
            command: UserConfigCommand::SetSkillEnablement {
                skill_id: skill_id.clone(),
                enablement: SkillEnablement::Enabled,
            },
        })
        .unwrap();

    let skills = ConfigStore::open(&path)
        .unwrap()
        .read_snapshot()
        .unwrap()
        .values
        .skills;
    assert_eq!(skills.skill_enablement(&skill_id), SkillEnablement::Enabled);
    assert!(skills.enablement.is_empty());
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
        toml::to_string_pretty(&workspace_document(Some(model_ref("openai", "gpt-5.6")))).unwrap(),
    )
    .unwrap();

    let scope = WorkspaceConfigScope::new(WorkspaceId::new("project").unwrap());
    let document = WorkspaceConfigStore::open(&path, scope)
        .read_document()
        .unwrap();
    assert_eq!(document.mcp.servers.len(), 1);
    assert_eq!(document.skills.sources.len(), 1);
    assert_eq!(document.plugin_requests.requests.len(), 1);
    assert_eq!(document.hooks.hooks.len(), 1);
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
        r#"
[mcp.servers."workspace:other:mcp:github"]
id = "workspace:other:mcp:github"
displayName = "Other GitHub"
enablement = "disabled"

[mcp.servers."workspace:other:mcp:github".transport]
type = "stdio"
command = "github-mcp"
args = []
"#,
    )
    .unwrap();
    assert!(
        WorkspaceConfigStore::open(&path, scope.clone())
            .read_document()
            .is_err()
    );

    std::fs::write(&path, "unknown = true").unwrap();
    assert!(
        WorkspaceConfigStore::open(&path, scope.clone())
            .read_document()
            .is_err()
    );

    std::fs::write(
        &path,
        format!(
            "[workspaceTrust.roots]\n\"{}\" = \"trusted\"\n",
            workspace_trust_id()
        ),
    )
    .unwrap();
    assert!(
        WorkspaceConfigStore::open(&path, scope.clone())
            .read_document()
            .is_err()
    );

    std::fs::write(
        &path,
        r#"
[hooks.hooks."workspace:project:hook:review"]
id = "workspace:project:hook:review"
event = "beforeTool"
enablement = "disabled"

[hooks.hooks."workspace:project:hook:review".matcher]
toolNames = []

[hooks.hooks."workspace:project:hook:review".action]
type = "process"
program = "review-hook"
args = []
shell = true
"#,
    )
    .unwrap();
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
        diagnostic.code == ConfigDiagnosticCode::WorkspacePluginPendingTrust
            && diagnostic.subject == "acme/code-review"
    }));
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == ConfigDiagnosticCode::WorkspaceHookPendingTrust
            && diagnostic.subject == "workspace:project:hook:review"
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

#[test]
fn language_server_preferences_are_typed_persisted_and_revision_safe() {
    let path = config_path("language-server-preference");
    let executable = std::env::temp_dir().join("rust-analyzer");
    let store = ConfigStore::open(&path).unwrap();
    let server_id = LanguageServerId::new("rust-analyzer").unwrap();

    let result = store
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("configure-rust-analyzer").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::ConfigureLanguageServer {
                server_id: server_id.clone(),
                config: LanguageServerConfig {
                    mode: LanguageServerModeConfig::Enabled,
                    executable: Some(executable.clone()),
                },
            },
        })
        .unwrap();

    assert_eq!(result.revision, ConfigRevision::new(1));
    let snapshot = store.read_snapshot().unwrap();
    assert_eq!(
        snapshot.values.language_servers.servers.get(&server_id),
        Some(&LanguageServerConfig {
            mode: LanguageServerModeConfig::Enabled,
            executable: Some(executable),
        })
    );
    assert!(persisted_config_document(&path).contains("rust-analyzer"));

    let invalid = store.apply(ConfigCommandRequest {
        command_id: CommandId::new("configure-relative-rust-analyzer").unwrap(),
        expected_revision: ConfigRevision::new(1),
        command: UserConfigCommand::ConfigureLanguageServer {
            server_id,
            config: LanguageServerConfig {
                mode: LanguageServerModeConfig::Automatic,
                executable: Some("relative/rust-analyzer".into()),
            },
        },
    });
    assert!(matches!(invalid, Err(ConfigCommandError::Config(_))));
    remove_config_files(&path);
}
