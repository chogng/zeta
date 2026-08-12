use super::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_async_utils::CancellationSource;
use zeta_config::{
    ConfigCommandRequest, ConfigRevision, PreferencesUpdate, ResolvedConfig, UserConfigCommand,
    WorkspaceConfigScope, WorkspaceConfigStore, WorkspaceId,
};
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::{ModelId, ModelInvoker, ModelProviderError, ProviderId};
use zeta_model_provider_config::ModelContextConfig;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::{
    CommandId, ModelRef, ModelRequest, ModelResponse, Patch, ResponseItem, StopReason,
};
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
        preferred_model: Some(ModelRef::new(provider.clone(), model)),
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
    let model = Arc::new(ConfigBackedModelService {
        config: config.clone(),
        workspace: None,
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
    let model = ConfigBackedModelService {
        config: config.clone(),
        workspace: Some(workspace.clone()),
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
