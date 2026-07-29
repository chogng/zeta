use super::*;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_async_utils::CancellationSource;
use zeta_config::{
    ConfigCommandRequest, ConfigRevision, PreferencesUpdate, ResolvedConfig, UserConfigCommand,
    WorkspaceConfigScope, WorkspaceConfigStore, WorkspaceId,
};
use zeta_model_provider::{ModelId, ModelInvoker, ModelProviderError, ProviderId};
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::{
    CommandId, ModelRef, ModelRequest, ModelResponse, Patch, ResponseItem, StopReason,
};

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
        "zeta-app-server-workspace-{label}-{}-{}.json",
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

fn model_ref(model: &str) -> ModelRef {
    ModelRef::new(
        ProviderId::new("test").unwrap(),
        ModelId::new(model).unwrap(),
    )
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
                theme: Patch::Missing,
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
        serde_json::json!({
            "agent": {
                "preferredModel": {"provider": "test", "model": "workspace-model"}
            }
        })
        .to_string(),
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
    std::fs::write(&workspace_path, "{}").unwrap();
    let (_, changed_revision) = workspace.read().unwrap();
    assert_eq!(changed_revision.get(), initial_revision.get() + 1);

    remove_config_files(&config_path);
    let _ = std::fs::remove_file(workspace_path);
}

fn invoke_text(model: &dyn ModelService, prompt: &str) -> String {
    model
        .invoke(
            &ModelRequest::text(prompt),
            &CancellationSource::new().token(),
        )
        .unwrap()
        .text()
}
