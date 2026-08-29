use super::*;
use crate::ConnectionState;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_async_utils::CancellationSource;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::PreferencesUpdate;
use zeta_config::ResolvedConfig;
use zeta_config::UserConfigCommand;
use zeta_config::WorkspaceConfigScope;
use zeta_config::WorkspaceConfigStore;
use zeta_config::WorkspaceId;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelId;
use zeta_model_provider::ModelInvoker;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider::ProviderId;
use zeta_model_provider_config::ApiProfile;
use zeta_model_provider_config::EndpointPolicy;
use zeta_model_provider_config::ModelCatalogPolicy;
use zeta_model_provider_config::ModelContextConfig;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_model_provider_config::ProviderAdapter;
use zeta_model_provider_config::ProviderDefinition;
use zeta_plugins::LocalPluginPackage;
use zeta_plugins::PluginAuthorityCommand;
use zeta_plugins::PluginAuthorityCommandId;
use zeta_plugins::PluginAuthorityCommandRequest;
use zeta_plugins::PluginPackageStore;
use zeta_protocol::CommandId;
use zeta_protocol::ImageDetail;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::Patch;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;
use zeta_secrets::MemorySecretStore;
use zeta_web_search_extension::WebSearchBackend;
use zeta_web_search_extension::WebSearchError;
use zeta_web_search_extension::WebSearchRequest;
use zeta_web_search_extension::WebSearchResponse;

fn config_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-app-server-{label}-{}-{}.authority.json",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn workspace_config_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "zeta-app-server-workspace-{label}-{}-{}.toml",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn connector_plugin(root: &Path) -> LocalPluginPackage {
    std::fs::create_dir_all(root.join(".zeta-plugin")).unwrap();
    std::fs::create_dir_all(root.join("mcp")).unwrap();
    std::fs::write(
        root.join(".zeta-plugin/plugin.json"),
        r#"{
            "schemaVersion": 1,
            "id": "acme/live",
            "version": "1.0.0",
            "displayName": "Live Plugin",
            "compatibility": {"zeta": ">=0.1.0"},
            "contributions": {
                "mcpServers": [{"id": "live", "definition": "mcp/live.json"}],
                "connectors": [{
                    "id": "account",
                    "displayName": "Live account",
                    "description": "A live activation test connector.",
                    "mcpServer": "live"
                }]
            },
            "permissions": [{"type": "network", "hosts": ["example.com"]}],
            "credentialSlots": [{
                "name": "token",
                "kind": "secretText",
                "requiredFor": ["connector:account", "mcp:live"]
            }]
        }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("mcp/live.json"),
        r#"{"transport":{"type":"streamableHttp","url":"https://example.com/mcp"}}"#,
    )
    .unwrap();
    LocalPluginPackage::load(root).unwrap()
}

fn plugin_request(
    authority: &PluginActivationAuthority,
    command_id: &str,
    command: PluginAuthorityCommand,
) -> PluginAuthorityCommandRequest {
    PluginAuthorityCommandRequest {
        command_id: PluginAuthorityCommandId::new(command_id).unwrap(),
        expected_revision: authority.snapshot().revision(),
        command,
    }
}

fn connector_count(server: &AppServer, connection: &mut ConnectionState, request_id: u64) -> usize {
    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": "connector/list",
                "params": {}
            })
            .to_string(),
        ),
    )
    .unwrap();
    response["result"]["connectors"].as_array().unwrap().len()
}

fn wait_for_connector_count(
    server: &AppServer,
    connection: &mut ConnectionState,
    expected: usize,
    request_id: &mut u64,
) {
    for _ in 0..100 {
        *request_id += 1;
        if connector_count(server, connection, *request_id) == expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("Connector projection did not reach {expected} entries");
}

#[test]
fn local_composition_reads_empty_subscription_accounts_before_sign_in() {
    let profile = tempfile::tempdir().unwrap();
    let server = open_local_app_server(
        LocalAppServerOptions::new(profile.path())
            .without_built_in_skills()
            .with_session_state_mode(SessionStateMode::Ephemeral),
    )
    .unwrap();
    let mut connection = server.connection();
    let initialized = server.handle_json(
        &mut connection,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}}"#,
    );
    assert!(initialized.contains("\"result\""));

    let account = server.handle_json(
        &mut connection,
        r#"{"jsonrpc":"2.0","id":2,"method":"account/read","params":{}}"#,
    );
    let account: serde_json::Value = serde_json::from_str(&account).unwrap();
    assert_eq!(account["result"]["accounts"], serde_json::json!([]));
}

#[test]
fn shared_profile_runtime_projects_sessions_across_isolated_workspaces() {
    let profile = tempfile::tempdir().unwrap();
    let first_workspace = tempfile::tempdir().unwrap();
    let second_workspace = tempfile::tempdir().unwrap();
    let runtime = Arc::new(LocalProfileRuntime::open(profile.path()).unwrap());
    let open = |workspace: &Path| {
        open_local_app_server(
            LocalAppServerOptions::new(profile.path())
                .with_profile_runtime(Arc::clone(&runtime))
                .with_workspace_root(workspace)
                .without_built_in_skills(),
        )
        .unwrap()
    };
    let first = open(first_workspace.path());
    let second = open(second_workspace.path());
    let mut first_connection = first.connection();
    let mut second_connection = second.connection();
    for (server, connection) in [
        (&first, &mut first_connection),
        (&second, &mut second_connection),
    ] {
        let initialized = server.handle_json(
            connection,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}}"#,
        );
        assert!(initialized.contains("\"result\""));
    }

    let created: serde_json::Value = serde_json::from_str(&first.handle_json(
        &mut first_connection,
        r#"{"jsonrpc":"2.0","id":2,"method":"session/create","params":{"commandId":"create-shared","title":"Shared task"}}"#,
    ))
    .unwrap();
    let session_id = created["result"]["session"]["sessionId"].as_str().unwrap();
    let projected_root = Path::new(
        created["result"]["session"]["workspace"]["root"]
            .as_str()
            .unwrap(),
    )
    .canonicalize()
    .unwrap();
    assert_eq!(
        projected_root,
        first_workspace.path().canonicalize().unwrap()
    );

    let listed: serde_json::Value = serde_json::from_str(&second.handle_json(
        &mut second_connection,
        r#"{"jsonrpc":"2.0","id":2,"method":"session/list","params":{}}"#,
    ))
    .unwrap();
    assert_eq!(listed["result"]["sessions"][0]["sessionId"], session_id);
    let subscribed = second.handle_json(
        &mut second_connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "session/subscribe",
            "params": {"sessionId": session_id, "afterSequence": 1}
        })
        .to_string(),
    );
    assert!(subscribed.contains("\"result\""));
    let created_thread = first.handle_json(
        &mut first_connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "session/request",
            "params": {
                "commandId": "create-thread",
                "sessionId": session_id,
                "expectedSequence": 1,
                "request": {"type": "createThread", "title": "Shared thread"}
            }
        })
        .to_string(),
    );
    assert!(created_thread.contains("\"result\""));
    let notifications = second.drain_notifications(&mut second_connection);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("session/update"))
    );
    let rejected: serde_json::Value = serde_json::from_str(
        &second.handle_json(
            &mut second_connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "session/request",
                "params": {
                    "commandId": "wrong-workspace",
                    "sessionId": session_id,
                    "expectedSequence": 3,
                    "request": {"type": "complete"}
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(rejected["error"]["message"], "WorkspaceAuthorityMismatch");
}

#[test]
fn shared_profile_runtime_reuses_one_durable_secret_store_across_workspaces() {
    let profile = tempfile::tempdir().unwrap();
    let first_workspace = tempfile::tempdir().unwrap();
    let second_workspace = tempfile::tempdir().unwrap();
    let runtime = Arc::new(LocalProfileRuntime::open(profile.path()).unwrap());
    let open = |workspace: &Path| {
        open_local_app_server(
            LocalAppServerOptions::new(profile.path())
                .with_profile_runtime(Arc::clone(&runtime))
                .with_workspace_root(workspace)
                .without_built_in_skills(),
        )
        .unwrap()
    };
    let first = open(first_workspace.path());
    let second = open(second_workspace.path());
    let mut first_connection = first.connection();
    let mut second_connection = second.connection();
    for (server, connection) in [
        (&first, &mut first_connection),
        (&second, &mut second_connection),
    ] {
        let initialized = server.handle_json(
            connection,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}}"#,
        );
        assert!(initialized.contains("\"result\""));
    }

    let saved: serde_json::Value = serde_json::from_str(&first.handle_json(
        &mut first_connection,
        r#"{"jsonrpc":"2.0","id":2,"method":"provider/apiKey/set","params":{"provider":"openai","apiKey":"shared-secret"}}"#,
    ))
    .unwrap();
    assert_eq!(saved["result"]["apiKeyConfigured"], true);
    assert!(!saved.to_string().contains("shared-secret"));

    let listed: serde_json::Value = serde_json::from_str(&second.handle_json(
        &mut second_connection,
        r#"{"jsonrpc":"2.0","id":2,"method":"provider/list","params":{}}"#,
    ))
    .unwrap();
    assert!(
        listed["result"]["providers"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |provider| provider["provider"] == "openai" && provider["apiKeyConfigured"] == true
            )
    );
    assert_eq!(
        std::fs::read_dir(profile.path().join("secrets/values"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn shared_profile_runtime_rejects_a_second_secret_store_authority() {
    let profile = tempfile::tempdir().unwrap();
    let runtime = Arc::new(LocalProfileRuntime::open(profile.path()).unwrap());
    let authority = PluginActivationAuthority::open(profile.path().join("plugins")).unwrap();
    let options = LocalAppServerOptions::new(profile.path())
        .with_profile_runtime(runtime)
        .without_built_in_skills()
        .with_plugin_authority(authority, Arc::new(MemorySecretStore::default()))
        .unwrap();

    let error = match open_local_app_server(options) {
        Ok(_) => panic!("a second profile SecretStore authority must be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        error.0,
        "shared profile runtime and Connector runtime use different SecretStore authorities"
    );
}

#[test]
fn shared_profile_runtime_owns_exactly_one_marketplace_authority() {
    let profile = tempfile::tempdir().unwrap();
    let runtime = LocalProfileRuntime::open(profile.path()).unwrap();
    let first_config = zeta_marketplace_client::RemoteMarketplaceConfig::new(
        "https://marketplace.example/metadata/".parse().unwrap(),
        "https://marketplace.example/targets/".parse().unwrap(),
        vec![1],
        profile.path().join("cache-a"),
    )
    .unwrap();
    let first = runtime.marketplace_manager(first_config.clone()).unwrap();
    let reused = runtime.marketplace_manager(first_config).unwrap();
    assert!(Arc::ptr_eq(&first, &reused));

    let second_config = zeta_marketplace_client::RemoteMarketplaceConfig::new(
        "https://other.example/metadata/".parse().unwrap(),
        "https://other.example/targets/".parse().unwrap(),
        vec![2],
        profile.path().join("cache-b"),
    )
    .unwrap();
    let error = match runtime.marketplace_manager(second_config) {
        Ok(_) => panic!("a second Marketplace authority must be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        error.0,
        "one profile runtime cannot use multiple Marketplace authorities"
    );
}

#[test]
fn live_plugin_authority_reconciles_connector_projection() {
    let profile = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let plugin_root = profile.path().join("plugins");
    let store = PluginPackageStore::open(&plugin_root).unwrap();
    let installed = store
        .install_local(&connector_plugin(source.path()))
        .unwrap();
    let authority = PluginActivationAuthority::open(&plugin_root).unwrap();
    authority
        .apply(plugin_request(
            &authority,
            "install-live",
            PluginAuthorityCommand::Install {
                package: installed.clone(),
            },
        ))
        .unwrap();
    let options = LocalAppServerOptions::new(profile.path())
        .without_built_in_skills()
        .with_session_state_mode(SessionStateMode::Ephemeral)
        .with_plugin_authority(authority.clone(), Arc::new(MemorySecretStore::default()))
        .unwrap();
    let server = open_local_app_server(options).unwrap();
    let mut connection = server.connection();
    let initialize = server.handle_json(
        &mut connection,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}}"#,
    );
    assert!(initialize.contains("\"result\""));
    let mut request_id = 1;
    wait_for_connector_count(&server, &mut connection, 0, &mut request_id);

    authority
        .apply(plugin_request(
            &authority,
            "grant-live",
            PluginAuthorityCommand::Grant {
                package: installed.clone(),
            },
        ))
        .unwrap();
    authority
        .apply(plugin_request(
            &authority,
            "enable-live",
            PluginAuthorityCommand::Enable {
                package: installed.clone(),
            },
        ))
        .unwrap();
    wait_for_connector_count(&server, &mut connection, 1, &mut request_id);

    authority
        .apply(plugin_request(
            &authority,
            "disable-live",
            PluginAuthorityCommand::Disable { package: installed },
        ))
        .unwrap();
    wait_for_connector_count(&server, &mut connection, 0, &mut request_id);
}

struct LocalSemanticEmbedding;

impl EmbeddingInvoker for LocalSemanticEmbedding {
    fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse, ModelProviderError> {
        EmbeddingResponse::new(
            request
                .inputs()
                .iter()
                .map(|_| EmbeddingVector::new(vec![1.0, 0.0]))
                .collect::<Result<Vec<_>, _>>()?,
        )
    }
}

#[test]
fn local_composition_installs_semantic_models_before_workspace_activation() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::create_dir(workspace.path().join(".git")).unwrap();
    std::fs::write(workspace.path().join("lib.rs"), "pub fn indexed() {}\n").unwrap();
    let models = CodeIndexSemanticModels::new(
        zeta_code_index_semantic::CodeIndexEmbeddingModelId::new("local-test-v1").unwrap(),
        Arc::new(LocalSemanticEmbedding),
    );
    let options = LocalAppServerOptions::new(profile.path())
        .with_workspace_root(workspace.path())
        .without_built_in_skills()
        .with_session_state_mode(SessionStateMode::Ephemeral);

    let server = open_local_app_server_with_code_index_providers(
        options,
        LocalCodeIndexProviders::new().with_semantic_models(models),
    )
    .unwrap();

    assert!(server.code_index_semantic_service().is_some());
}

#[test]
fn user_config_initial_workspace_fails_closed_without_host_trust() {
    let profile = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("readable.txt"), "restricted\n").unwrap();
    let options = LocalAppServerOptions::new(profile.path())
        .with_user_config_workspace_root(workspace.path())
        .without_built_in_skills()
        .with_session_state_mode(SessionStateMode::Ephemeral);

    let server = open_local_app_server(options).unwrap();

    assert!(!server.active_workspace_is_trusted());
}

struct UnusedSearchBackend;

impl WebSearchBackend for UnusedSearchBackend {
    fn service_name(&self) -> &str {
        "test search"
    }

    fn network_scopes(&self) -> Vec<String> {
        vec!["search.example.com".into()]
    }

    fn credential_reference(&self) -> Option<String> {
        None
    }

    fn search(
        &self,
        _: &WebSearchRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<WebSearchResponse, WebSearchError> {
        panic!("composition test does not execute search")
    }
}

#[test]
fn local_web_search_is_absent_by_default_and_registered_when_injected() {
    let profile = tempfile::tempdir().unwrap();
    let default_server = open_local_app_server(
        LocalAppServerOptions::new(profile.path())
            .without_built_in_skills()
            .with_session_state_mode(SessionStateMode::Ephemeral),
    )
    .unwrap();
    assert!(
        default_server
            .local_workspace_tool_ports()
            .unwrap()
            .definitions()
            .iter()
            .all(|definition| definition.name.as_str() != "web_search")
    );

    let injected_profile = tempfile::tempdir().unwrap();
    let injected_server = open_local_app_server(
        LocalAppServerOptions::new(injected_profile.path())
            .without_built_in_skills()
            .with_session_state_mode(SessionStateMode::Ephemeral)
            .with_web_search_backend(Arc::new(UnusedSearchBackend)),
    )
    .unwrap();
    assert!(
        injected_server
            .local_workspace_tool_ports()
            .unwrap()
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "web_search")
    );
}

fn remove_config_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("toml"));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
}

fn model_ref(model: &str) -> ModelRef {
    ModelRef::new(
        ProviderId::new("test").unwrap(),
        ModelId::new(model).unwrap(),
    )
}

#[test]
fn configured_model_context_enables_core_managed_compaction() {
    let provider = ProviderId::new("openai").unwrap();
    let model = ModelId::new("gpt-5.6").unwrap();
    let mut provider_config = ModelProviderConfig::new(provider.clone());
    provider_config.max_output_tokens = Some(2_048);
    provider_config.model_context = BTreeMap::from([(
        model.clone(),
        ModelContextConfig {
            context_window: 20_000,
            auto_compact_token_limit: Some(15_000),
        },
    )]);
    let config = ResolvedConfig {
        preferred_model: Some(ModelRef::new(provider.clone(), model.clone())),
        providers: BTreeMap::from([(provider, provider_config)]),
        ..ResolvedConfig::default()
    };

    assert_eq!(
        context_budget_for_config(&config).unwrap(),
        ContextBudget::core_managed(
            ContextTokenCount::new(20_000),
            ContextTokenCount::new(2_048),
            ContextTokenCount::new(MODEL_CONTEXT_SAFETY_MARGIN_TOKENS),
            ContextCompactionLimit::Tokens(ContextTokenCount::new(15_000)),
        )
    );
    let entry = runtime_catalog_entry(
        zeta_app_server_protocol::protocol::model::ModelCatalogEntry::from_info(
            config.preferred_model.clone().unwrap(),
            &zeta_protocol::ModelInfo::new(model, "GPT 5.6"),
            zeta_protocol::ModelOutputTransport::Unary,
        ),
        &config,
    )
    .unwrap();
    assert_eq!(entry.context_window, Some(20_000));
    assert_eq!(entry.available_context_window, Some(11_928));
}

#[test]
fn image_input_policy_tracks_the_selected_provider_and_original_detail_capability() {
    let providers = ProviderConfigRegistry::builtin();
    let openai = ResolvedConfig {
        preferred_model: Some(ModelRef::new(
            ProviderId::new("openai").unwrap(),
            ModelId::new("gpt-5.6").unwrap(),
        )),
        ..ResolvedConfig::default()
    };
    let anthropic = ResolvedConfig {
        preferred_model: Some(ModelRef::new(
            ProviderId::new("anthropic").unwrap(),
            ModelId::new("claude-sonnet-4-20250514").unwrap(),
        )),
        ..ResolvedConfig::default()
    };

    let openai_policy = image_input_policy_for_config(&openai, &providers);
    assert_eq!(
        openai_policy.limits_for(ImageDetail::Auto),
        ModelImageInputLimits::new(6_000, 10_000)
    );
    assert_eq!(
        openai_policy.limits_for(ImageDetail::High),
        ModelImageInputLimits::new(2_048, 2_440)
    );
    let anthropic_policy = image_input_policy_for_config(&anthropic, &providers);
    assert_eq!(
        anthropic_policy.limits_for(ImageDetail::Auto),
        ModelImageInputLimits::new(1_568, 1_120)
    );
    assert_eq!(
        anthropic_policy.limits_for(ImageDetail::Original),
        ModelImageInputLimits::new(1_568, 1_120)
    );
}

fn configure_test_provider(config: &ConfigStore, revision: ConfigRevision) -> ConfigRevision {
    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new(format!("configure-test-{}", revision.get())).unwrap(),
            expected_revision: revision,
            command: UserConfigCommand::ConfigureProvider {
                provider: ProviderId::new("test").unwrap(),
                config: ModelProviderConfig::new(ProviderId::new("test").unwrap()),
            },
        })
        .unwrap()
        .revision
}

fn select_model(
    config: &ConfigStore,
    command_id: &str,
    revision: ConfigRevision,
    model: &str,
) -> ConfigRevision {
    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new(command_id).unwrap(),
            expected_revision: revision,
            command: UserConfigCommand::UpdatePreferences(PreferencesUpdate {
                preferred_model: Patch::Value(model_ref(model)),
                approval_review_model: Patch::Missing,
                tool_mode: Patch::Missing,
                grep_backend: Patch::Missing,
            }),
        })
        .unwrap()
        .revision
}

#[derive(Default)]
struct GateState {
    entered: bool,
    released: bool,
}

#[derive(Default)]
struct ResponseGate {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl ResponseGate {
    fn wait_until_entered(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.entered {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn wait_until_released(&self) {
        let mut state = self.state.lock().unwrap();
        state.entered = true;
        self.changed.notify_all();
        while !state.released {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.released = true;
        self.changed.notify_all();
    }
}

struct RecordingSnapshotResolver {
    gate: Arc<ResponseGate>,
}

#[derive(Default)]
struct RecordingModelProvider {
    request: Mutex<Option<ModelRuntimeRequest>>,
}

impl ModelProvider for RecordingModelProvider {
    fn runtime(
        &self,
        request: ModelRuntimeRequest,
    ) -> Result<Arc<dyn ModelInvoker>, ModelProviderError> {
        *self.request.lock().unwrap() = Some(request);
        Ok(Arc::new(UnavailableModel::new("recorded")))
    }
}

#[test]
fn subscription_model_resolution_does_not_require_an_api_key_provider_config() {
    let provider = Arc::new(RecordingModelProvider::default());
    let resolver = ModelProviderSnapshotResolver {
        model_provider: provider.clone(),
    };
    let config = ResolvedConfig {
        preferred_model: Some(ModelRef::new(
            ProviderId::new("openai").unwrap(),
            ModelId::new("gpt-5.6-sol").unwrap(),
        )),
        ..ResolvedConfig::default()
    };

    let _ = resolver.resolve(&config);

    let request = provider.request.lock().unwrap().clone().unwrap();
    assert_eq!(request.model, config.preferred_model.unwrap());
    assert_eq!(request.config.provider.as_str(), "openai");
    assert_eq!(request.config.base_url, None);
}

impl ModelSnapshotResolver for RecordingSnapshotResolver {
    fn resolve(&self, config: &ResolvedConfig) -> Arc<dyn ModelInvoker> {
        Arc::new(SnapshotModel {
            model: config
                .preferred_model
                .as_ref()
                .map(|model| model.model.as_str().to_owned())
                .unwrap_or_else(|| "unconfigured".into()),
            gate: self.gate.clone(),
        })
    }
}

struct SnapshotModel {
    model: String,
    gate: Arc<ResponseGate>,
}

impl ModelInvoker for SnapshotModel {
    fn invoke(&self, _: &ModelRequest) -> Result<ModelResponse, ModelProviderError> {
        self.gate.wait_until_released();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(self.model.clone())],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

#[test]
fn model_invocations_use_latest_config_without_mutating_an_in_flight_snapshot() {
    let path = config_path("model-snapshot");
    let config = Arc::new(ConfigStore::open(&path).unwrap());
    let configured = configure_test_provider(&config, ConfigRevision::INITIAL);
    let before_update = select_model(&config, "select-before", configured, "before-update");
    let gate = Arc::new(ResponseGate::default());
    let provider_configs = test_provider_registry();
    let model = Arc::new(ConfigBackedModelService {
        config: config.clone(),
        workspace: None,
        provider_configs: provider_configs.clone(),
        models_manager: ModelsManager::new(provider_configs),
        resolver: Arc::new(RecordingSnapshotResolver { gate: gate.clone() }),
    });
    let in_flight_model = model.clone();
    let in_flight = thread::spawn(move || invoke_text(in_flight_model.as_ref(), "first"));
    gate.wait_until_entered();
    select_model(&config, "select-after", before_update, "after-update");
    gate.release();

    assert_eq!(in_flight.join().unwrap(), "before-update");
    assert_eq!(invoke_text(model.as_ref(), "second"), "after-update");
    remove_config_files(&path);
}

#[test]
fn local_model_resolution_applies_workspace_model_at_the_next_safe_point() {
    let config_path = config_path("workspace-model");
    let config = Arc::new(ConfigStore::open(&config_path).unwrap());
    let configured = configure_test_provider(&config, ConfigRevision::INITIAL);
    select_model(&config, "select-user", configured, "user-model");

    let workspace_path = workspace_config_path("workspace-model");
    std::fs::write(
        &workspace_path,
        r#"
[agent.preferredModel]
provider = "test"
model = "workspace-model"
"#,
    )
    .unwrap();
    let workspace = Arc::new(WorkspaceConfigTracker::new(WorkspaceConfigStore::open(
        &workspace_path,
        WorkspaceConfigScope::new(WorkspaceId::new("project").unwrap()),
    )));
    let provider_configs = test_provider_registry();
    let model = ConfigBackedModelService {
        config: config.clone(),
        workspace: Some(workspace.clone()),
        provider_configs: provider_configs.clone(),
        models_manager: ModelsManager::new(provider_configs),
        resolver: Arc::new(RecordingSnapshotResolver {
            gate: Arc::new(ResponseGate::default()),
        }),
    };

    let user = config.read_snapshot().unwrap();
    assert_eq!(
        model
            .resolve_config(&user)
            .unwrap()
            .preferred_model
            .unwrap()
            .model
            .as_str(),
        "workspace-model"
    );
    let (_, initial_revision) = workspace.read().unwrap();
    std::fs::write(&workspace_path, "").unwrap();
    let (_, changed_revision) = workspace.read().unwrap();
    assert_eq!(changed_revision.get(), initial_revision.get() + 1);

    remove_config_files(&config_path);
    let _ = std::fs::remove_file(workspace_path);
}

#[test]
fn local_catalog_projects_static_models_without_runtime_availability() {
    let path = config_path("models-manager-catalog");
    let config = Arc::new(ConfigStore::open(&path).unwrap());
    let configured = configure_test_provider(&config, ConfigRevision::INITIAL);
    select_model(&config, "select-custom", configured, "custom-model");
    let provider_configs = test_provider_registry();
    let model = ConfigBackedModelService {
        config,
        workspace: None,
        provider_configs: provider_configs.clone(),
        models_manager: ModelsManager::new(provider_configs),
        resolver: Arc::new(RecordingSnapshotResolver {
            gate: Arc::new(ResponseGate::default()),
        }),
    };

    let models = model.list().unwrap();

    let custom = models
        .iter()
        .find(|entry| entry.model == model_ref("custom-model"))
        .unwrap();
    assert_eq!(custom.display_name, "custom-model");
    assert_eq!(custom.access, zeta_protocol::ModelAccess::Unknown);
    assert_eq!(custom.context_window, None);
    assert_eq!(
        custom.capabilities,
        zeta_protocol::ModelCapabilities::UNKNOWN
    );
    let openai = models
        .iter()
        .find(|entry| {
            entry.model.provider.as_str() == "openai" && entry.model.model.as_str() == "gpt-5.6"
        })
        .unwrap();
    assert_eq!(openai.access, zeta_protocol::ModelAccess::ApiKey);
    assert_eq!(openai.context_window, None);
    assert_eq!(
        openai.capabilities.image_detail_original,
        zeta_protocol::CapabilitySupport::Supported
    );
    model.validate(&openai.model).unwrap();
    remove_config_files(&path);
}

fn invoke_text(model: &dyn ModelService, prompt: &str) -> String {
    model
        .invoke(
            zeta_core::ModelSelection::ConfiguredDefault,
            &ModelRequest::text(prompt),
            &CancellationSource::new().token(),
        )
        .unwrap()
        .text()
}

fn test_provider_registry() -> ProviderConfigRegistry {
    let mut registry = ProviderConfigRegistry::builtin();
    registry
        .register(ProviderDefinition::new(
            ProviderId::new("test").unwrap(),
            "Test",
            ProviderAdapter::OpenAiCompatible,
            ApiProfile::OpenAiChatCompletions,
            EndpointPolicy::ConfiguredOnly,
            ModelCatalogPolicy::AllowUnlisted,
        ))
        .unwrap();
    registry
}
