use super::*;
use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zeta_app_server::AppServer;
use zeta_app_server::SlashCommandCatalog;
use zeta_app_server_protocol::protocol::common::{ClientCapabilities, ClientInfo};
use zeta_app_server_protocol::protocol::fs::{
    FsFileType, FsGetMetadataParams, FsReadDirectoryParams, FsReadFileParams, FsWriteFileParams,
};
use zeta_app_server_protocol::protocol::initialize::InitializeParams;
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionRequest, SessionRequestParams, SessionRequestResult,
    SessionThreadReadParams,
};
use zeta_app_server_protocol::protocol::skills::{
    SkillCatalogReloadDto, SkillEnablementDto, SkillListParams, SkillSetEnablementParams,
};
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_app_server_protocol::schema_hash;
use zeta_async_utils::CancellationToken;
use zeta_core::{
    CoreError, InMemorySessionStore, InMemoryThreadStore, ModelService, SessionCoordinator,
    ThreadController,
};
use zeta_protocol::{
    CommandId, ContentPart, InputItem as ModelInputItem, ModelRequest, ModelResponse, ResponseItem,
    StopReason, ThreadEvent, ThreadItem, ThreadUpdate, TurnStatus,
};

struct MockTransport(VecDeque<String>);

impl JsonRpcTransport for MockTransport {
    fn round_trip(&mut self, _: &str) -> Result<String, ClientError> {
        self.0
            .pop_front()
            .ok_or_else(|| ClientError::Transport("no response".into()))
    }
}

fn app_server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    AppServer::new(
        Arc::new(SessionCoordinator::with_store(
            Arc::new(InMemorySessionStore::default()),
            threads,
        )),
        Arc::new(TestModel),
    )
}

struct TestModel;

impl ModelService for TestModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let prompt = request
            .input
            .iter()
            .rev()
            .find_map(|item| match item {
                ModelInputItem::Message(message) => {
                    message.content.iter().find_map(|content| match content {
                        ContentPart::Text(text) => Some(text.as_str()),
                        ContentPart::ImageUrl { .. } => None,
                    })
                }
                ModelInputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("Zeta: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

#[test]
fn client_rejects_response_for_another_request() {
    let mut client = AppServerClient::new(MockTransport(VecDeque::from([
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{}}".into(),
    ])));
    let result: Result<serde_json::Value, _> =
        client.call(ClientMethod::Initialize, serde_json::json!({}));
    assert!(matches!(result, Err(ClientError::Protocol(_))));
}

#[test]
fn client_reads_workspace_directories_through_the_typed_contract() {
    let mut client = AppServerClient::new(MockTransport(VecDeque::from([
        r#"{"jsonrpc":"2.0","id":1,"result":{"entries":[{"name":"src","fileType":"directory"}]}}"#
            .into(),
    ])));

    let result = client
        .read_directory(FsReadDirectoryParams {
            path: "nested".into(),
        })
        .unwrap();

    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "src");
    assert_eq!(result.entries[0].file_type, FsFileType::Directory);
}

#[test]
fn client_reads_writes_and_versions_workspace_files_through_typed_contracts() {
    let mut client = AppServerClient::new(MockTransport(VecDeque::from([
        r#"{"jsonrpc":"2.0","id":1,"result":{"content":"fn main() {}\n"}}"#.into(),
        r#"{"jsonrpc":"2.0","id":2,"result":{"fileType":"file","sizeBytes":13,"readonly":false,"modifiedAtMillis":41}}"#.into(),
        r#"{"jsonrpc":"2.0","id":3,"result":{"metadata":{"fileType":"file","sizeBytes":14,"readonly":false,"modifiedAtMillis":42}}}"#.into(),
    ])));

    let read = client
        .read_file(FsReadFileParams {
            path: "src/main.rs".into(),
        })
        .unwrap();
    let metadata = client
        .get_file_metadata(FsGetMetadataParams {
            path: "src/main.rs".into(),
        })
        .unwrap();
    let written = client
        .write_file(FsWriteFileParams {
            path: "src/main.rs".into(),
            content: "fn main() { }\n".into(),
        })
        .unwrap();

    assert_eq!(read.content, "fn main() {}\n");
    assert_eq!(metadata.modified_at_millis, Some(41));
    assert_eq!(written.metadata.modified_at_millis, Some(42));
}

#[test]
fn in_process_client_uses_session_first_contract_and_canonical_updates() {
    let mut client = AppServerClient::new(InProcessTransport::from_server(app_server()));
    let initialized = client
        .initialize(InitializeParams {
            client_info: ClientInfo {
                name: "test-client".into(),
                version: "1".into(),
            },
            capabilities: ClientCapabilities::default(),
        })
        .expect("in-process client initializes");
    assert_eq!(initialized.schema_hash.0, schema_hash());
    assert_eq!(client.initialization().unwrap(), &initialized);
    let session = client
        .create_session(SessionCreateParams {
            command_id: CommandId::new("session-one").expect("test ID is non-empty"),
            title: "test".into(),
        })
        .expect("Session is created");
    let thread = client
        .request_session(SessionRequestParams {
            command_id: CommandId::new("thread-one").expect("test ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: session.session.sequence,
            request: SessionRequest::CreateThread {
                title: "root".into(),
            },
        })
        .expect("Thread is created");
    let thread = match thread {
        SessionRequestResult::Thread(thread) => thread,
        result => panic!("unexpected create-thread result: {result:?}"),
    };
    let turn = client
        .request_session(SessionRequestParams {
            command_id: CommandId::new("turn-one").expect("test ID is non-empty"),
            session_id: session.session.session_id.clone(),
            expected_sequence: 1,
            request: SessionRequest::StartTurn {
                thread_id: thread.thread_id.clone(),
                input: vec![
                    InputItem::Text {
                        text: "hello".into(),
                    },
                    InputItem::Image {
                        url: "data:image/png;base64,iVBORw0KGgpwYXlsb2Fk".into(),
                    },
                ],
            },
        })
        .expect("Turn starts");
    let turn = match turn {
        SessionRequestResult::Turn(turn) => turn,
        result => panic!("unexpected start-turn result: {result:?}"),
    };
    let deadline = Instant::now() + Duration::from_secs(1);
    let snapshot = loop {
        let snapshot = client
            .read_session_thread(SessionThreadReadParams {
                session_id: session.session.session_id.clone(),
                thread_id: thread.thread_id.clone(),
            })
            .expect("Thread remains readable");
        if snapshot.thread.turns[0].status == TurnStatus::Completed {
            break snapshot;
        }
        assert!(Instant::now() < deadline, "Turn did not complete");
        thread::sleep(Duration::from_millis(1));
    };
    let notifications = client.drain_notifications().expect("notifications decode");
    let output = notifications
        .iter()
        .find_map(|notification| match notification {
            ServerNotification::SessionThreadUpdate(update) => match &update.update {
                ThreadUpdate::Committed {
                    event:
                        ThreadEvent::ItemCompleted {
                            item: ThreadItem::AgentMessage { text, .. },
                            ..
                        },
                } => Some(text.as_str()),
                _ => None,
            },
            _ => None,
        });
    assert_eq!(output, Some("Zeta: hello"));
    assert_eq!(snapshot.thread.turns[0].turn_id, turn.turn_id);
    assert_eq!(snapshot.thread.turns[0].status, TurnStatus::Completed);
    assert!(matches!(
        &snapshot.thread.turns[0].items[1],
        ThreadItem::UserImage { url, .. }
            if url == "data:image/png;base64,iVBORw0KGgpwYXlsb2Fk"
    ));
}

#[test]
fn client_preserves_the_initialized_slash_command_snapshot() {
    let definition = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    };
    let server = app_server()
        .with_slash_command_catalog(SlashCommandCatalog::new([definition.clone()]).unwrap());
    let mut client = AppServerClient::new(InProcessTransport::from_server(server));

    client
        .initialize(InitializeParams {
            client_info: ClientInfo {
                name: "test-client".into(),
                version: "1".into(),
            },
            capabilities: ClientCapabilities::default(),
        })
        .unwrap();

    assert_eq!(
        client.initialization().unwrap().slash_commands,
        [definition]
    );
}

#[test]
fn initialization_snapshot_is_unavailable_before_handshake() {
    let client = AppServerClient::new(MockTransport(VecDeque::new()));

    assert!(matches!(
        client.initialization(),
        Err(ClientError::Protocol(message)) if message.contains("initialize handshake")
    ));
}

#[test]
fn embedded_startup_propagates_the_host_slash_command_catalog() {
    let definition = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    };
    let state_root = std::env::temp_dir().join(format!(
        "zeta-app-server-client-slash-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "test-client".into(),
                version: "1".into(),
            },
        )
        .with_slash_command_catalog(SlashCommandCatalog::new([definition.clone()]).unwrap()),
    )
    .unwrap();

    assert_eq!(
        client.initialization().unwrap().slash_commands,
        [definition]
    );
    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn shared_embedded_host_opens_independent_initialized_connections() {
    let state_root = std::env::temp_dir().join(format!(
        "zeta-app-server-client-shared-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let host = open_in_process_app_server(InProcessClientOptions::new(
        &state_root,
        ClientInfo {
            name: "shared-client".into(),
            version: "1".into(),
        },
    ))
    .unwrap();
    let first = host.connect().unwrap();
    let second = host.connect().unwrap();

    assert_eq!(
        first.initialization().unwrap().schema_hash,
        second.initialization().unwrap().schema_hash
    );

    drop((first, second, host));
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn ephemeral_session_state_ignores_and_does_not_append_durable_history() {
    let state_root = unique_directory("ephemeral-session-state");
    let mut durable = start_in_process_client(InProcessClientOptions::new(
        &state_root,
        ClientInfo {
            name: "durable-seed".into(),
            version: "1".into(),
        },
    ))
    .unwrap();
    durable
        .create_session(SessionCreateParams {
            command_id: CommandId::new("durable-seed-session").unwrap(),
            title: "durable seed".into(),
        })
        .unwrap();
    drop(durable);

    let mut ephemeral = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "ephemeral".into(),
                version: "1".into(),
            },
        )
        .with_session_state_mode(SessionStateMode::Ephemeral),
    )
    .unwrap();
    assert!(ephemeral.list_sessions().unwrap().sessions.is_empty());
    ephemeral
        .create_session(SessionCreateParams {
            command_id: CommandId::new("ephemeral-session").unwrap(),
            title: "ephemeral session".into(),
        })
        .unwrap();
    drop(ephemeral);

    let mut durable_again = start_in_process_client(InProcessClientOptions::new(
        &state_root,
        ClientInfo {
            name: "durable-check".into(),
            version: "1".into(),
        },
    ))
    .unwrap();
    let sessions = durable_again.list_sessions().unwrap().sessions;
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "durable seed");

    drop(durable_again);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn embedded_skill_catalog_lists_built_ins_and_persists_enablement() {
    let state_root = unique_directory("skills-state");
    let skills_root = unique_directory("skills-root");
    write_skill(&skills_root, "skill-creator", "Create or update a Skill");
    let mut client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "skills-client".into(),
                version: "1".into(),
            },
        )
        .with_built_in_skill_root(&skills_root),
    )
    .unwrap();

    let listed = client
        .list_skills(SkillListParams {
            reload: SkillCatalogReloadDto::Refresh,
        })
        .unwrap();
    assert_eq!(listed.skills.len(), 1);
    assert_eq!(listed.skills[0].id.name.as_str(), "skill-creator");
    assert_eq!(listed.skills[0].enablement, SkillEnablementDto::Enabled);

    let revision = client.read_config().unwrap().revision;
    client
        .set_skill_enablement(SkillSetEnablementParams {
            command_id: CommandId::new("disable-skill-creator").unwrap(),
            expected_revision: revision,
            skill_id: listed.skills[0].id.clone(),
            enablement: SkillEnablementDto::Disabled,
        })
        .unwrap();
    let disabled = client.list_skills(SkillListParams::default()).unwrap();
    assert_eq!(disabled.generation, listed.generation + 1);
    assert_eq!(disabled.skills[0].enablement, SkillEnablementDto::Disabled);
    assert!(
        client
            .drain_notifications()
            .unwrap()
            .iter()
            .any(|notification| matches!(
                notification,
                ServerNotification::SkillsChanged(changed)
                    if changed.generation == disabled.generation
            ))
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
    let _ = fs::remove_dir_all(skills_root);
}

#[test]
fn embedded_skill_watcher_invalidates_changed_content() {
    let state_root = unique_directory("watch-state");
    let skills_root = unique_directory("watch-root");
    write_skill(&skills_root, "review", "Review code");
    let mut client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "skills-watch-client".into(),
                version: "1".into(),
            },
        )
        .with_built_in_skill_root(&skills_root),
    )
    .unwrap();
    let initial = client.list_skills(SkillListParams::default()).unwrap();
    thread::sleep(Duration::from_millis(150));

    write_skill(&skills_root, "review", "Review code and tests");

    let deadline = Instant::now() + Duration::from_secs(3);
    let changed_generation = loop {
        let notification =
            client
                .drain_notifications()
                .unwrap()
                .into_iter()
                .find_map(|notification| match notification {
                    ServerNotification::SkillsChanged(changed) => Some(changed.generation),
                    _ => None,
                });
        if let Some(generation) = notification {
            break generation;
        }
        assert!(Instant::now() < deadline, "Skill watcher did not publish");
        thread::sleep(Duration::from_millis(10));
    };
    let refreshed = client.list_skills(SkillListParams::default()).unwrap();
    assert_eq!(refreshed.generation, changed_generation);
    assert_eq!(refreshed.skills[0].description, "Review code and tests");
    assert!(refreshed.generation > initial.generation);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
    let _ = fs::remove_dir_all(skills_root);
}

fn unique_directory(label: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "zeta-app-server-client-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    directory
}

fn write_skill(root: &std::path::Path, name: &str, description: &str) {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\nInstructions.\n"),
    )
    .unwrap();
}
