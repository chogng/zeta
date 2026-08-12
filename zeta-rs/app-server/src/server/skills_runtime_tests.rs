use super::BuiltInSkillSource;
use super::SkillCatalogReload;
use super::SkillConfigSnapshotProvider;
use super::SkillRuntime;
use super::event_affects_catalog;
use crate::AppServer;
use crate::server::update_broker::UpdateBroker;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_async_utils::CancellationToken;
use zeta_config::SkillEnablement;
use zeta_config::SkillsConfig;
use zeta_core::CoreError;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelSelection;
use zeta_core::ModelService;
use zeta_core::SessionCoordinator;
use zeta_core::SkillInstructionRetention;
use zeta_core::SkillInstructionsProvider;
use zeta_core::ThreadController;
use zeta_protocol::ContentPart;
use zeta_protocol::InputItem;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ResponseItem;
use zeta_protocol::SkillActivationReason;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_protocol::StopReason;
use zeta_protocol::TurnStatus;

struct TestConfig {
    skills: Mutex<SkillsConfig>,
}

impl TestConfig {
    fn new() -> Self {
        Self {
            skills: Mutex::new(SkillsConfig::default()),
        }
    }

    fn set_enablement(&self, skill_id: &SkillId, enablement: SkillEnablement) {
        self.skills
            .lock()
            .unwrap()
            .enablement
            .entry(skill_id.source.clone())
            .or_default()
            .insert(skill_id.name.clone(), enablement);
    }
}

impl SkillConfigSnapshotProvider for TestConfig {
    fn snapshot(&self) -> Result<SkillsConfig, String> {
        Ok(self.skills.lock().unwrap().clone())
    }
}

#[test]
fn built_in_catalog_refreshes_and_preserves_monotonic_runtime_generation() {
    let root = test_directory("built-in-refresh");
    write_skill(&root, "skill-creator", "Create skills");
    let config = Arc::new(TestConfig::new());
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        config,
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let initial = runtime.list(SkillCatalogReload::Cached).unwrap();
    assert_eq!(initial.generation, 1);
    assert_eq!(
        initial.entries[0].catalog_entry.id().name.as_str(),
        "skill-creator"
    );

    write_skill(&root, "skill-creator", "Create and improve skills");
    let refreshed = runtime.list(SkillCatalogReload::Refresh).unwrap();
    assert_eq!(refreshed.generation, 2);
    assert_eq!(
        refreshed.entries[0].catalog_entry.metadata().description(),
        "Create and improve skills"
    );
    assert_eq!(
        runtime
            .list(SkillCatalogReload::Refresh)
            .unwrap()
            .generation,
        2
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn missing_auto_detected_built_in_root_is_visible_as_a_diagnostic() {
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Missing,
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let snapshot = runtime.list(SkillCatalogReload::Cached).unwrap();
    assert!(snapshot.entries.is_empty());
    assert_eq!(snapshot.diagnostics.len(), 1);
    assert_eq!(
        snapshot.diagnostics[0].source,
        "builtin:skill-source:zeta-release"
    );
    assert_eq!(
        snapshot.diagnostics[0].code,
        zeta_skills::SkillDiagnosticCode::SourceUnavailable
    );
}

#[test]
fn enablement_overlay_changes_projection_without_changing_skill_content() {
    let root = test_directory("enablement");
    write_skill(&root, "skill-creator", "Create skills");
    let config = Arc::new(TestConfig::new());
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        config.clone(),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();
    let skill_id = SkillId::new(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        SkillName::new("skill-creator").unwrap(),
    );

    config.set_enablement(&skill_id, SkillEnablement::Disabled);
    let disabled = runtime.list(SkillCatalogReload::Cached).unwrap();

    assert_eq!(disabled.generation, 2);
    assert_eq!(disabled.entries[0].enablement, SkillEnablement::Disabled);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn workspace_root_contributes_native_dot_zeta_skills() {
    let workspace = test_directory("workspace-source");
    let root = workspace.join(".zeta/skills");
    write_skill(&root, "workspace-review", "Reviews Workspace code");
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Omitted,
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();

    let snapshot = runtime.bind_workspace_root(workspace.clone()).unwrap();

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        snapshot.entries[0].catalog_entry.source().kind(),
        zeta_skills::SkillSourceKind::Workspace
    );
    assert_eq!(
        snapshot.entries[0].catalog_entry.id().source.as_str(),
        "workspace:skill-source:.zeta"
    );
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn explicit_activation_freezes_digest_and_later_reload_fails_closed_on_change() {
    let root = test_directory("activation");
    write_skill(&root, "skill-creator", "Create skills");
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();
    let selected = SkillRef::follow_latest(SkillId::new(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        SkillName::new("skill-creator").unwrap(),
    ));
    let activated = runtime.activate_explicit(&selected).unwrap();
    let frozen = activated.activation().clone();

    assert!(activated.body().contains("Instructions."));
    write_skill(&root, "skill-creator", "Changed description and body");
    assert!(runtime.resolve(std::slice::from_ref(&frozen)).is_err());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn activation_reason_maps_to_context_retention_at_the_adapter_boundary() {
    let root = test_directory("activation-retention");
    write_skill(&root, "skill-creator", "Create skills");
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Root(root.clone()),
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();
    let selected = SkillRef::follow_latest(SkillId::new(
        SkillSourceId::new("builtin:skill-source:zeta-release").unwrap(),
        SkillName::new("skill-creator").unwrap(),
    ));
    let explicit = runtime.activate_explicit(&selected).unwrap();
    let mut automatic = explicit.activation().clone();
    automatic.reason = SkillActivationReason::Automatic;

    let explicit_instruction = runtime
        .resolve(std::slice::from_ref(explicit.activation()))
        .unwrap();
    let automatic_instruction = runtime.resolve(&[automatic]).unwrap();

    assert_eq!(
        explicit_instruction[0].retention(),
        SkillInstructionRetention::Required
    );
    assert_eq!(
        automatic_instruction[0].retention(),
        SkillInstructionRetention::BestEffort
    );
    let _ = fs::remove_dir_all(root);
}

#[derive(Default)]
struct RecordingModel {
    requests: Mutex<Vec<ModelRequest>>,
}

impl ModelService for RecordingModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(ModelResponse {
            output: vec![ResponseItem::Text("done".into())],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

#[test]
fn explicit_skill_reaches_the_model_through_the_app_server_rpc() {
    let root = test_directory("rpc-activation");
    write_skill(&root, "skill-creator", "Create skills");
    let model = Arc::new(RecordingModel::default());
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    let server = AppServer::new(sessions, model.clone())
        .with_skill_runtime(
            BuiltInSkillSource::Root(root.clone()),
            Arc::new(TestConfig::new()),
        )
        .unwrap();
    let mut connection = server.connection();
    rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );
    let session = rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"session/create",
            "params":{"commandId":"create-session","title":"skill task"}
        }),
    );
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"session/request",
            "params":{
                "commandId":"create-thread","sessionId":session_id,"expectedSequence":1,
                "request":{"type":"createThread","title":"root"}
            }
        }),
    );
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let started = rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"start-with-skill","sessionId":session_id,"expectedSequence":1,
                "request":{
                    "type":"startTurn","threadId":thread_id,
                    "input":[
                        {"type":"skill","skill":{
                            "id":{"source":"builtin:skill-source:zeta-release","name":"skill-creator"},
                            "version":{"type":"followLatest"}
                        }},
                        {"type":"text","text":"create one"}
                    ]
                }
            }
        }),
    );
    assert!(started["result"]["value"]["turnId"].is_string());
    let started_turn_id = started["result"]["value"]["turnId"].clone();

    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
        if snapshot
            .turns
            .last()
            .is_some_and(|turn| turn.status == TurnStatus::Completed)
        {
            assert_eq!(snapshot.turns[0].activated_skills.len(), 1);
            assert_eq!(
                snapshot.turns[0].activated_skills[0].reason,
                SkillActivationReason::Explicit
            );
            break;
        }
        assert!(Instant::now() < deadline, "Skill Turn did not complete");
        std::thread::yield_now();
    }
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].input.iter().any(|item| {
        let InputItem::Message(message) = item else {
            return false;
        };
        message.content.iter().any(|part| {
            matches!(part, ContentPart::Text(text) if text.contains("<skill-instructions") && text.contains("Instructions."))
        })
    }));
    drop(requests);

    fs::remove_dir_all(root.join("skill-creator")).unwrap();
    let replayed = rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/request",
            "params":{
                "commandId":"start-with-skill","sessionId":session_id,"expectedSequence":1,
                "request":{
                    "type":"startTurn","threadId":thread_id,
                    "input":[
                        {"type":"skill","skill":{
                            "id":{"source":"builtin:skill-source:zeta-release","name":"skill-creator"},
                            "version":{"type":"followLatest"}
                        }},
                        {"type":"text","text":"create one"}
                    ]
                }
            }
        }),
    );
    assert_eq!(replayed["result"]["value"]["turnId"], started_turn_id);
    assert_eq!(model.requests.lock().unwrap().len(), 1);
    let conflict = rpc(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/request",
            "params":{
                "commandId":"start-with-skill","sessionId":session_id,"expectedSequence":1,
                "request":{
                    "type":"startTurn","threadId":thread_id,
                    "input":[
                        {"type":"skill","skill":{
                            "id":{"source":"builtin:skill-source:zeta-release","name":"skill-creator"},
                            "version":{"type":"followLatest"}
                        }},
                        {"type":"text","text":"different input"}
                    ]
                }
            }
        }),
    );
    assert_eq!(conflict["error"]["message"], "CommandConflict");
    drop(connection);
    drop(server);
    let _ = fs::remove_dir_all(root);
}

fn rpc(
    server: &AppServer,
    connection: &mut crate::ConnectionState,
    request: serde_json::Value,
) -> serde_json::Value {
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}

#[test]
fn unrelated_dot_zeta_runtime_files_do_not_refresh_workspace_skills() {
    let workspace = test_directory("workspace-event-filter");
    let root = workspace.join(".zeta/skills");
    write_skill(&root, "workspace-review", "Reviews Workspace code");
    let runtime = SkillRuntime::new(
        BuiltInSkillSource::Omitted,
        Arc::new(TestConfig::new()),
        Arc::new(UpdateBroker::default()),
    )
    .unwrap();
    runtime.bind_workspace_root(workspace.clone()).unwrap();

    assert!(!event_affects_catalog(
        &runtime,
        &zeta_file_watcher::FileWatcherEvent::PathsChanged {
            paths: vec![workspace.join(".zeta/streams/thread/runtime.rollout")],
        },
    ));
    assert!(event_affects_catalog(
        &runtime,
        &zeta_file_watcher::FileWatcherEvent::PathsChanged {
            paths: vec![workspace.join(".zeta/skills/workspace-review/SKILL.md")],
        },
    ));
    let _ = fs::remove_dir_all(workspace);
}

fn test_directory(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-skills-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_skill(root: &Path, name: &str, description: &str) {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\nInstructions.\n"),
    )
    .unwrap();
}
