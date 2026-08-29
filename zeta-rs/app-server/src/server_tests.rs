use super::*;
use base64::Engine;
use std::io::Cursor;
use std::io::Write;
use std::net::Shutdown;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use zeta_action_policy::ActionDigest;
use zeta_action_policy::ActionKind;
use zeta_action_policy::ActionPolicyRevision;
use zeta_action_policy::ActionProvenance;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ActionSource;
use zeta_action_policy::Capability;
use zeta_action_policy::CapabilityKind;
use zeta_action_policy::CapabilitySet;
use zeta_action_policy::ExecutionDecision;
use zeta_action_policy::ProcessInvocationKind;
use zeta_action_policy::ResolvedAction;
use zeta_action_policy::SandboxCompatibility;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandArgumentModeDto;
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_async_utils::CancellationToken;
use zeta_config::ConfigStore;
use zeta_core::ActionPolicyService;
use zeta_core::CoreError;
use zeta_core::InMemorySessionStore;
use zeta_core::InMemoryThreadStore;
use zeta_core::ModelService;
use zeta_core::RequestTurnInteraction;
use zeta_core::SessionCoordinator;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_core::ToolAuthorization;
use zeta_core::ToolOutputSink;
use zeta_core::ToolService;
use zeta_file_system::LocalFileSystem;
use zeta_login::AccountRef;
use zeta_login::AccountSnapshot;
use zeta_login::AccountStatus;
use zeta_login::BeginLogin;
use zeta_login::BeginLoginRequest;
use zeta_login::CancelLoginOutcome;
use zeta_login::CompleteLogin;
use zeta_login::InteractiveLoginDriver;
use zeta_login::LoginCompletionOutcome;
use zeta_login::LoginError;
use zeta_login::LoginId;
use zeta_login::LoginService;
use zeta_model_provider::EchoModel;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::AgentRequest;
use zeta_protocol::CommandId;
use zeta_protocol::ContentPart;
use zeta_protocol::InputItem;
use zeta_protocol::InteractionDeadline;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ProviderId;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInput;
use zeta_protocol::ResponseItem;
use zeta_protocol::StableTurnErrorCode;
use zeta_protocol::StopReason;
use zeta_protocol::ToolCall;
use zeta_protocol::ToolDefinition;
use zeta_protocol::ToolExecutionOutput;
use zeta_protocol::ToolOutputStream;
use zeta_protocol::TurnStatus;
use zeta_protocol::UserInput;
use zeta_sandboxing::FileSystemAccess;
use zeta_sandboxing::NetworkAccess;
use zeta_sandboxing::SandboxPolicy;
use zeta_secrets::MemorySecretStore;
use zeta_uds::UnixStream;
use zeta_workspace::TrustedWorkspace;
use zeta_workspace::WorkspaceCapability;
use zeta_workspace::WorkspaceRoot;
use zeta_workspace::WorkspaceTrustDecision;
use zeta_workspace::WorkspaceTrustSource;

fn server_with_model(model: Arc<dyn ModelService>) -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    AppServer::new(sessions, model).with_ephemeral_workspace_state()
}

fn server() -> AppServer {
    server_with_model(Arc::new(crate::local::ProviderModelService::new(Arc::new(
        EchoModel,
    ))))
}

#[derive(Clone)]
struct CapturingWriter {
    output: Arc<(Mutex<Vec<u8>>, Condvar)>,
}

struct BlockingWriter {
    started: Option<Sender<()>>,
    release: Receiver<()>,
    released: bool,
}

#[derive(Default)]
struct TestLoginDriver;

impl InteractiveLoginDriver for TestLoginDriver {
    fn provider_id(&self) -> &'static str {
        "openai-chatgpt"
    }

    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError> {
        Ok(None)
    }

    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError> {
        Ok(BeginLogin::Browser {
            login_id: request.login_id,
            authorization_url: "https://auth.example.test/start".into(),
        })
    }

    fn cancel(&self, _: &LoginId) -> Result<CancelLoginOutcome, LoginError> {
        Ok(CancelLoginOutcome::Cancelled)
    }

    fn logout(&self, _: &AccountRef) -> Result<(), LoginError> {
        Ok(())
    }
}

impl Write for CapturingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let (output, changed) = self.output.as_ref();
        output
            .lock()
            .map_err(|_| std::io::Error::other("capture lock poisoned"))?
            .extend_from_slice(bytes);
        changed.notify_all();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for BlockingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if !self.released {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
            }
            self.release
                .recv()
                .map_err(|_| std::io::Error::other("blocking writer release channel closed"))?;
            self.released = true;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn wait_for_captured(output: &Arc<(Mutex<Vec<u8>>, Condvar)>, needle: &str) {
    let deadline = Instant::now() + Duration::from_secs(2);
    let (bytes, changed) = output.as_ref();
    let mut bytes = bytes.lock().unwrap();
    loop {
        if String::from_utf8_lossy(&bytes).contains(needle) {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(!remaining.is_zero(), "timed out waiting for {needle}");
        let (next, _) = changed.wait_timeout(bytes, remaining).unwrap();
        bytes = next;
    }
}

#[derive(Clone)]
struct FixedModelCatalog {
    models: Vec<zeta_app_server_protocol::protocol::model::ModelCatalogEntry>,
    default: ModelRef,
}

impl crate::model_catalog::ModelCatalog for FixedModelCatalog {
    fn list(
        &self,
    ) -> Result<Vec<zeta_app_server_protocol::protocol::model::ModelCatalogEntry>, CoreError> {
        Ok(self.models.clone())
    }

    fn configured_default(&self) -> Result<Option<ModelRef>, CoreError> {
        Ok(Some(self.default.clone()))
    }

    fn validate(&self, model: &ModelRef) -> Result<(), CoreError> {
        self.models
            .iter()
            .any(|entry| &entry.model == model)
            .then_some(())
            .ok_or_else(|| CoreError::Model("model is not in the catalog".into()))
    }
}

fn model_ref(model: &str) -> ModelRef {
    ModelRef::new(
        ProviderId::new("openai").unwrap(),
        ModelId::new(model).unwrap(),
    )
}

#[test]
fn provider_rpc_lists_the_backend_catalog_and_stores_api_keys_without_projecting_values() {
    let secrets = Arc::new(MemorySecretStore::default());
    let server = server().with_provider_credentials(Arc::new(
        zeta_model_provider::ProviderCredentialService::new(
            zeta_model_provider_config::ProviderConfigRegistry::builtin(),
            secrets,
        ),
    ));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let initial = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"provider/list","params":{}
        }),
    );
    assert_eq!(initial["result"]["providers"].as_array().unwrap().len(), 13);

    let saved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"provider/apiKey/set",
            "params":{"provider":"openai","apiKey":"secret-provider-key"}
        }),
    );
    assert_eq!(saved["result"]["provider"], "openai");
    assert!(!saved.to_string().contains("secret-provider-key"));

    let updated = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"provider/list","params":{}
        }),
    );
    assert!(
        updated["result"]["providers"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |provider| provider["provider"] == "openai" && provider["apiKeyConfigured"] == true
            )
    );
}

fn call(
    server: &AppServer,
    connection: &mut ConnectionState,
    request: serde_json::Value,
) -> serde_json::Value {
    serde_json::from_str(&server.handle_json(connection, &request.to_string())).unwrap()
}

fn initialize(server: &AppServer, connection: &mut ConnectionState) {
    initialize_with_capabilities(server, connection, serde_json::json!({}));
}

fn initialize_with_capabilities(
    server: &AppServer,
    connection: &mut ConnectionState,
    capabilities: serde_json::Value,
) {
    let response = call(
        server,
        connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":capabilities}
        }),
    );
    assert_eq!(response["result"]["capabilities"]["sessions"], true);
    assert_eq!(response["result"]["capabilities"]["codebase"], false);
    assert_eq!(response["result"]["capabilities"]["typst"], true);
    assert_eq!(response["result"]["capabilities"]["updateReplay"], true);
}

#[test]
fn attachment_upload_is_chunked_connection_owned_and_returns_an_attachment_reference() {
    let server = server();
    let mut owner = server.connection();
    let mut other = server.connection();
    initialize(&server, &mut owner);
    initialize(&server, &mut other);
    let image = image::DynamicImage::new_rgba8(2, 1);
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();

    let started = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":10,
            "method":"attachment/upload/start",
            "params":{"mediaType":"png","encodedBytes":bytes.len(),"detail":"auto"}
        }),
    );
    let upload_id = started["result"]["uploadId"].as_str().unwrap();
    let rejected = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":11,
            "method":"attachment/upload/write",
            "params":{"uploadId":upload_id,"offset":0,"dataBase64":base64::engine::general_purpose::STANDARD.encode(&bytes)}
        }),
    );
    assert_eq!(rejected["error"]["message"], "InvalidParams");

    let written = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":12,
            "method":"attachment/upload/write",
            "params":{"uploadId":upload_id,"offset":0,"dataBase64":base64::engine::general_purpose::STANDARD.encode(&bytes)}
        }),
    );
    assert_eq!(written["result"]["nextOffset"], bytes.len());
    let finished = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":13,
            "method":"attachment/upload/finish",
            "params":{"uploadId":upload_id}
        }),
    );
    let attachment = &finished["result"]["attachment"];
    assert_eq!(attachment["mediaType"], "png");
    assert_eq!(attachment["width"], 2);
    assert_eq!(attachment["height"], 1);
    assert!(
        attachment["contentDigest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(!finished.to_string().contains("data:image/"));
}

#[test]
fn document_collaboration_orders_updates_and_returns_rebase_history() {
    let server = server();
    let mut first = server.connection();
    let mut second = server.connection();
    initialize(&server, &mut first);
    initialize(&server, &mut second);
    let document = collaboration_document("initial");
    let opened = call(
        &server,
        &mut first,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"document/collaboration/open",
            "params":{"clientId":"client-a","schemaId":"stanza-document-v1","document":document}
        }),
    );
    let room_id = opened["result"]["snapshot"]["roomId"]
        .as_str()
        .expect("opening a room must return its identifier")
        .to_string();
    assert_eq!(room_id.len(), "document-".len() + 32);
    assert!(room_id.starts_with("document-"));
    assert!(
        room_id["document-".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert_eq!(opened["result"]["snapshot"]["version"], 0);
    let joined = call(
        &server,
        &mut second,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"document/collaboration/open",
            "params":{"roomId":room_id,"clientId":"client-b","schemaId":"stanza-document-v1","document":"{}"}
        }),
    );
    assert_eq!(joined["result"]["snapshot"]["document"], document);
    let presence = call(
        &server,
        &mut first,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"document/collaboration/presence/publish",
            "params":{"roomId":room_id,"clientId":"client-a","selection":"{\"kind\":\"text\",\"anchor\":{\"nodeId\":\"text-1\",\"offset\":0},\"head\":{\"nodeId\":\"text-1\",\"offset\":1}}"}
        }),
    );
    assert_eq!(presence["result"]["generation"], 1);
    assert_eq!(presence["result"]["presences"][0]["clientId"], "client-a");
    let notifications = server.drain_notifications(&mut second);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("document/collaboration/presence"))
    );
    let read_presence = call(
        &server,
        &mut second,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"document/collaboration/presence/read",
            "params":{"roomId":room_id}
        }),
    );
    assert_eq!(
        read_presence["result"]["presences"][0]["clientId"],
        "client-a"
    );
    let accepted = call(
        &server,
        &mut first,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"document/collaboration/submit",
            "params":{"roomId":room_id,"clientId":"client-a","sequence":1,"baseVersion":0,"transaction":collaboration_transaction(),"document":collaboration_document("first")}
        }),
    );
    assert_eq!(accepted["result"]["status"], "accepted");
    assert_eq!(accepted["result"]["update"]["version"], 1);
    let notifications = server.drain_notifications(&mut second);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("document/collaboration/update"))
    );
    let conflict = call(
        &server,
        &mut second,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"document/collaboration/submit",
            "params":{"roomId":room_id,"clientId":"client-b","sequence":1,"baseVersion":0,"transaction":collaboration_transaction(),"document":"{}"}
        }),
    );
    assert_eq!(conflict["result"]["status"], "conflict");
    assert_eq!(conflict["result"]["updates"][0]["clientId"], "client-a");
    assert_eq!(conflict["result"]["updates"][0]["version"], 1);
}

fn collaboration_document(value: &str) -> String {
    format!(
        r#"{{"format":"zeta.document","version":1,"document":{{"id":"document-1","type":"doc","attrs":{{}},"marks":[],"content":[{{"id":"text-1","type":"text","attrs":{{}},"marks":[],"content":[],"text":"{value}"}}]}}}}"#
    )
}

fn collaboration_transaction() -> String {
    r#"{"format":"zeta.document.transaction","version":1,"transaction":{"steps":[],"addToHistory":true,"selectionSet":false,"storedMarksSet":false,"metadata":[]}}"#.into()
}

fn create_session(
    server: &AppServer,
    connection: &mut ConnectionState,
    request_id: u64,
    command_id: &str,
) -> serde_json::Value {
    call(
        server,
        connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":request_id,
            "method":"session/create",
            "params":{"commandId":command_id,"title":"task"}
        }),
    )
}

fn create_thread(
    server: &AppServer,
    connection: &mut ConnectionState,
    request_id: u64,
    command_id: &str,
    session_id: &str,
    expected_sequence: u64,
) -> serde_json::Value {
    call(
        server,
        connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":request_id,
            "method":"session/request",
            "params":{
                "commandId":command_id,
                "sessionId":session_id,
                "expectedSequence":expected_sequence,
                "request":{"type":"createThread","title":"root"}
            }
        }),
    )
}

fn wait_for_latest_turn(server: &AppServer, thread_id: &str, expected: TurnStatus) {
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
        if snapshot
            .turns
            .last()
            .is_some_and(|turn| turn.status == expected)
        {
            return;
        }
        assert!(Instant::now() < deadline, "Turn did not reach {expected:?}");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn thread_goal_rpc_round_trips_and_publishes_scoped_updates() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "goal-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "goal-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":1}
        }),
    );
    server.drain_notifications(&mut connection);

    let set = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"thread/goal/set",
            "params":{"threadId":thread_id,"objective":"finish the implementation","tokenBudget":100}
        }),
    );
    assert_eq!(set["result"]["goal"]["status"], "active");
    assert_eq!(set["result"]["goal"]["tokensUsed"], 0);
    assert_eq!(set["result"]["goal"]["tokenBudget"], 100);
    let updates = server.drain_notifications(&mut connection);
    assert!(updates.iter().any(|notification| {
        notification.contains("\"method\":\"thread/goal/updated\"")
            && notification.contains("finish the implementation")
    }));

    let get = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":6,
            "method":"thread/goal/get",
            "params":{"threadId":thread_id}
        }),
    );
    assert_eq!(
        get["result"]["goal"]["goalId"],
        set["result"]["goal"]["goalId"]
    );

    let clear = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":7,
            "method":"thread/goal/clear",
            "params":{"threadId":thread_id}
        }),
    );
    assert_eq!(clear["result"]["cleared"], true);
    assert!(
        server
            .drain_notifications(&mut connection)
            .iter()
            .any(|notification| notification.contains("\"method\":\"thread/goal/cleared\""))
    );

    let missing = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":8,
            "method":"thread/goal/get",
            "params":{"threadId":thread_id}
        }),
    );
    assert!(missing["result"]["goal"].is_null());
}

#[test]
fn initialize_is_required_and_request_ids_are_connection_unique() {
    let server = server();
    let mut connection = server.connection();
    let gated = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"session/list","params":{}}),
    );
    assert_eq!(gated["error"]["message"], "NotInitialized");

    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let duplicate = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"session/list","params":{}}),
    );
    assert_eq!(duplicate["error"]["message"], "InvalidRequest");
}

#[test]
fn closed_connections_reject_future_requests() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    server.close_connection(connection.clone());

    let rejected = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"session/list","params":{}}),
    );
    assert_eq!(rejected["error"]["message"], "RequestCancelled");
}

#[test]
fn dynamic_interaction_capability_requires_exact_hosted_tool_names() {
    let server = server();
    let mut connection = server.connection();
    let missing_names = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{
                "clientInfo":{"name":"test","version":"1"},
                "capabilities":{
                    "agentInteractions":{"version":1,"kinds":["dynamicTool"]}
                }
            }
        }),
    );
    assert_eq!(missing_names["error"]["message"], "InvalidParams");

    let mut connection = server.connection();
    let accepted = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{
                "clientInfo":{"name":"test","version":"1"},
                "capabilities":{
                    "agentInteractions":{
                        "version":1,
                        "kinds":["dynamicTool"],
                        "dynamicTools":["client_lookup"]
                    }
                }
            }
        }),
    );
    assert_eq!(
        accepted["result"]["capabilities"]["agentInteractions"],
        true
    );
}

#[test]
fn workspace_search_requires_an_installed_backend() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"workspace/search/start",
            "params":{
                "query":"needle",
                "patternKind":"literal",
                "caseSensitivity":"smart",
                "includePatterns":[],
                "excludePatterns":[],
                "maxResults":100
            }
        }),
    );

    assert_eq!(response["error"]["message"], "SearchUnavailable");
}

#[test]
fn terminal_requires_an_installed_backend() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/create",
            "params":{"rows":24,"cols":80,"profile":{"type":"default"},"lifecycle":{"type":"connectionOwned"}}
        }),
    );

    assert_eq!(response["error"]["message"], "TerminalUnavailable");
}

#[test]
fn terminal_profiles_are_server_owned_and_reject_unknown_ids() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-terminal-profiles-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let server = server()
        .with_terminal_root(trusted_workspace(
            &root,
            WorkspaceCapability::ExecuteProcess,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let profiles = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/profile/list",
            "params":{}
        }),
    );
    let profiles = profiles["result"]["profiles"].as_array().unwrap();
    assert!(!profiles.is_empty());
    assert_eq!(
        profiles
            .iter()
            .filter(|profile| profile["isDefault"] == true)
            .count(),
        1
    );
    assert!(
        profiles
            .iter()
            .all(|profile| profile.get("program").is_none())
    );

    let rejected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"terminal/create",
            "params":{
                "rows":24,
                "cols":80,
                "profile":{"type":"profile","profileId":"client-program"},
                "lifecycle":{"type":"connectionOwned"}
            }
        }),
    );
    assert_eq!(rejected["error"]["message"], "InvalidParams");

    let environment_injection = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"terminal/create",
            "params":{
                "rows":24,
                "cols":80,
                "profile":{"type":"default"},
                "lifecycle":{"type":"connectionOwned"},
                "environment":{"OPENAI_API_KEY":"injected"}
            }
        }),
    );
    assert_eq!(environment_injection["error"]["message"], "InvalidParams");

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn terminal_rpc_drives_a_workspace_rooted_pty_to_exit() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-terminal-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let server = server()
        .with_terminal_root(trusted_workspace(
            &root,
            WorkspaceCapability::ExecuteProcess,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let created = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/create",
            "params":{"rows":24,"cols":80,"profile":{"type":"default"},"lifecycle":{"type":"connectionOwned"}}
        }),
    );
    let terminal_id = created["result"]["terminalId"].as_str().unwrap();
    #[cfg(windows)]
    let input = "echo zeta-terminal-ready\r\nexit\r\n";
    #[cfg(not(windows))]
    let input = "printf 'zeta-terminal-ready\\n'\nexit\n";
    let written = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"terminal/write",
            "params":{"terminalId":terminal_id,"data":input}
        }),
    );
    assert!(written["error"].is_null());

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut request_id = 4;
    let mut after_sequence = 0;
    let mut after_command_sequence = 0;
    let mut output = Vec::new();
    #[cfg(windows)]
    let mut command_statuses = Vec::new();
    let exit_code = loop {
        let read = call(
            &server,
            &mut connection,
            serde_json::json!({
                "jsonrpc":"2.0",
                "id":request_id,
                "method":"terminal/read",
                "params":{
                    "terminalId":terminal_id,
                    "afterSequence":after_sequence,
                    "afterCommandSequence":after_command_sequence,
                    "maxChunks":128
                }
            }),
        );
        request_id += 1;
        after_sequence = read["result"]["nextSequence"].as_u64().unwrap();
        after_command_sequence = read["result"]["nextCommandSequence"].as_u64().unwrap();
        for chunk in read["result"]["chunks"].as_array().unwrap() {
            output.extend(
                base64::engine::general_purpose::STANDARD
                    .decode(chunk["dataBase64"].as_str().unwrap())
                    .unwrap(),
            );
        }
        #[cfg(windows)]
        for event in read["result"]["commandEvents"].as_array().unwrap() {
            command_statuses.push(event["status"].as_str().unwrap().to_owned());
        }
        if read["result"]["exited"] == true {
            break read["result"]["exitCode"].as_i64().unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "terminal did not exit; output={:?}; exit_code={:?}",
            String::from_utf8_lossy(&output),
            read["result"]["exitCode"]
        );
        thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(exit_code, 0);
    assert!(String::from_utf8_lossy(&output).contains("zeta-terminal-ready"));
    #[cfg(windows)]
    {
        assert!(command_statuses.iter().any(|status| status == "running"));
        assert!(
            command_statuses
                .iter()
                .any(|status| matches!(status.as_str(), "completed" | "succeeded"))
        );
    }
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn terminal_rpc_enforces_connection_ownership_and_close() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-terminal-owner-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let server = server()
        .with_terminal_root(trusted_workspace(
            &root,
            WorkspaceCapability::ExecuteProcess,
        ))
        .unwrap();
    let mut owner = server.connection();
    let mut other = server.connection();
    initialize(&server, &mut owner);
    initialize(&server, &mut other);
    let created = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/create",
            "params":{"rows":24,"cols":80,"profile":{"type":"default"},"lifecycle":{"type":"connectionOwned"}}
        }),
    );
    let terminal_id = created["result"]["terminalId"].as_str().unwrap();

    let rejected = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/read",
            "params":{
                "terminalId":terminal_id,
                "afterSequence":0,
                "afterCommandSequence":0,
                "maxChunks":1
            }
        }),
    );
    assert_eq!(rejected["error"]["message"], "TerminalNotOwner");

    let closed = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"terminal/close",
            "params":{"terminalId":terminal_id}
        }),
    );
    assert!(closed["error"].is_null());
    let missing = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"terminal/read",
            "params":{
                "terminalId":terminal_id,
                "afterSequence":0,
                "afterCommandSequence":0,
                "maxChunks":1
            }
        }),
    );
    assert_eq!(missing["error"]["message"], "TerminalNotFound");

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reconnectable_terminal_detaches_rotates_its_token_and_rejects_replay() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-terminal-attach-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let server = server()
        .with_terminal_root(trusted_workspace(
            &root,
            WorkspaceCapability::ExecuteProcess,
        ))
        .unwrap();
    let mut owner = server.connection();
    initialize(&server, &mut owner);
    let created = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/create",
            "params":{"rows":24,"cols":80,"profile":{"type":"default"},"lifecycle":{"type":"reconnectable"}}
        }),
    );
    let terminal_id = created["result"]["terminalId"].as_str().unwrap().to_owned();
    let first_token = created["result"]["reconnect"]["reconnectToken"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(first_token.len(), 64);
    assert_eq!(
        created["result"]["reconnect"]["reconnectGracePeriodMillis"],
        30_000
    );

    server.close_connection(owner);
    let mut replacement = server.connection();
    initialize(&server, &mut replacement);
    let rejected = call(
        &server,
        &mut replacement,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/attach",
            "params":{"terminalId":terminal_id,"reconnectToken":"0".repeat(64),"rows":30,"cols":100}
        }),
    );
    assert_eq!(rejected["error"]["message"], "TerminalAttachRejected");

    let attached = call(
        &server,
        &mut replacement,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"terminal/attach",
            "params":{"terminalId":terminal_id,"reconnectToken":first_token,"rows":30,"cols":100}
        }),
    );
    let second_token = attached["result"]["reconnect"]["reconnectToken"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(second_token, first_token);

    server.close_connection(replacement);
    let mut final_connection = server.connection();
    initialize(&server, &mut final_connection);
    let replayed = call(
        &server,
        &mut final_connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"terminal/attach",
            "params":{"terminalId":terminal_id,"reconnectToken":first_token,"rows":30,"cols":100}
        }),
    );
    assert_eq!(replayed["error"]["message"], "TerminalAttachRejected");
    let final_attach = call(
        &server,
        &mut final_connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"terminal/attach",
            "params":{"terminalId":terminal_id,"reconnectToken":second_token,"rows":30,"cols":100}
        }),
    );
    assert_eq!(final_attach["result"]["terminalId"], terminal_id);
    let closed = call(
        &server,
        &mut final_connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"terminal/close",
            "params":{"terminalId":terminal_id}
        }),
    );
    assert!(closed["error"].is_null());

    server.close_connection(final_connection);
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn debug_adapter_rpc_enforces_connection_ownership_and_connection_cleanup() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-debug-adapter-owner-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let server = server()
        .with_debug_adapter_root(
            trusted_workspace(&root, WorkspaceCapability::LoadExecutableConfiguration),
            trusted_workspace(&root, WorkspaceCapability::ExecuteProcess),
        )
        .unwrap();
    let mut owner = server.connection();
    let mut other = server.connection();
    initialize(&server, &mut owner);
    initialize(&server, &mut other);
    #[cfg(windows)]
    let (program, arguments) = (
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()),
        vec!["/D", "/Q", "/C", "more"],
    );
    #[cfg(not(windows))]
    let (program, arguments) = ("/bin/sh".to_owned(), vec!["-c", "cat"]);
    let started = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"debug/adapter/start",
            "params":{"program":program,"arguments":arguments}
        }),
    );
    let session_id = started["result"]["sessionId"].as_str().unwrap();

    let rejected = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"debug/adapter/read",
            "params":{"sessionId":session_id,"afterSequence":0,"maxMessages":1}
        }),
    );
    assert_eq!(rejected["error"]["message"], "DebugAdapterNotOwner");

    server.close_connection(owner);
    let missing = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"debug/adapter/read",
            "params":{"sessionId":session_id,"afterSequence":0,"maxMessages":1}
        }),
    );
    assert_eq!(missing["error"]["message"], "DebugAdapterNotFound");

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn initialize_advertises_the_server_slash_command_snapshot() {
    let catalog = SlashCommandCatalog::new([SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    }])
    .unwrap();
    let server = server().with_slash_command_catalog(catalog);
    let mut connection = server.connection();

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );

    assert_eq!(
        response["result"]["slashCommands"],
        serde_json::json!([{
            "name": "diagnose",
            "description": "inspect the current workspace",
            "argumentMode": "optional"
        }])
    );
    assert_eq!(
        response["result"]["protocolVersion"],
        serde_json::json!({
            "major": zeta_app_server_protocol::protocol::initialize::APP_SERVER_PROTOCOL_MAJOR,
            "revision": zeta_app_server_protocol::protocol::initialize::APP_SERVER_PROTOCOL_REVISION,
        })
    );
    assert_eq!(response["result"]["capabilities"]["sessions"], true);
    assert_eq!(
        response["result"]["capabilities"]["contracts"]["sessions"]["version"],
        zeta_app_server_protocol::protocol::initialize::APP_SERVER_CAPABILITY_VERSION
    );
}

#[test]
fn session_request_starts_and_replays_manual_context_compaction() {
    let backend = Arc::new(CountingStartBackend::default());
    let server = server().with_turn_backend(backend.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "compact-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "compact-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let request = |id| {
        serde_json::json!({
            "jsonrpc":"2.0","id":id,"method":"session/request",
            "params":{
                "commandId":"manual-compact",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{
                    "type":"compactContext",
                    "threadId":thread_id,
                    "retentionPrompt":"preserve the deployment decision"
                }
            }
        })
    };

    let started = call(&server, &mut connection, request(4));
    let replayed = call(&server, &mut connection, request(5));
    let protocol_thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&protocol_thread_id)
        .unwrap();

    assert_eq!(started["result"]["type"], "turn");
    assert_eq!(replayed["result"], started["result"]);
    assert_eq!(backend.starts.load(Ordering::Relaxed), 1);
    assert_eq!(snapshot.turns.len(), 1);
    assert!(matches!(
        &snapshot.commands[0].receipt.command,
        zeta_protocol::ThreadCommand::CompactContext {
            retention_prompt: Some(prompt),
            ..
        } if prompt == "preserve the deployment decision"
    ));
}

#[test]
fn session_first_flow_exposes_canonical_session_and_thread_models() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "create-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    assert_eq!(session["result"]["session"]["sequence"], 1);
    let thread = create_thread(&server, &mut connection, 3, "create-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    assert_eq!(
        thread["result"]["value"]["session"]["threads"][0]["status"],
        "active"
    );
    assert_eq!(
        thread["result"]["value"]["session"]["currentThreadId"],
        thread_id
    );
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/read",
            "params":{"sessionId":session_id,"threadId":thread_id}
        }),
    );
    assert_eq!(read["result"]["thread"]["sessionId"], session_id);
    assert_eq!(read["result"]["thread"]["sequence"], 1);
}

#[test]
fn session_stop_archives_the_session_and_blocks_new_turns() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "create-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "create-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    let stopped = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/request",
            "params":{
                "commandId":"stop-session",
                "sessionId":session_id,
                "expectedSequence":3,
                "request":{"type":"stop"}
            }
        }),
    );
    assert_eq!(stopped["result"]["value"]["session"]["status"], "archived");

    let rejected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"turn-after-stop",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"after stop"}]}
            }
        }),
    );
    assert_eq!(rejected["error"]["message"], "CoreOperationFailed");
}

#[test]
fn model_selection_is_catalog_backed_and_session_scoped() {
    let default = model_ref("gpt-default");
    let alternate = model_ref("gpt-alternate");
    let mut default_info = zeta_protocol::ModelInfo::new(default.model.clone(), "Default");
    default_info.access = zeta_protocol::ModelAccess::ApiKey;
    let mut alternate_info = zeta_protocol::ModelInfo::new(alternate.model.clone(), "Alternate");
    alternate_info.access = zeta_protocol::ModelAccess::ApiKey;
    let catalog = FixedModelCatalog {
        models: vec![
            zeta_app_server_protocol::protocol::model::ModelCatalogEntry::from_info(
                default.clone(),
                &default_info,
                zeta_protocol::ModelOutputTransport::Unary,
            ),
            zeta_app_server_protocol::protocol::model::ModelCatalogEntry::from_info(
                alternate.clone(),
                &alternate_info,
                zeta_protocol::ModelOutputTransport::Unary,
            ),
        ],
        default,
    };
    let server = server().with_model_catalog(Arc::new(catalog));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let listed = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"model/list","params":{}}),
    );
    assert_eq!(listed["result"]["models"].as_array().unwrap().len(), 2);
    let first = create_session(&server, &mut connection, 3, "first-session");
    let second = create_session(&server, &mut connection, 4, "second-session");
    let first_id = first["result"]["session"]["sessionId"].as_str().unwrap();
    let second_id = second["result"]["session"]["sessionId"].as_str().unwrap();
    assert_eq!(first["result"]["session"]["model"]["model"], "gpt-default");
    assert_eq!(second["result"]["session"]["model"]["model"], "gpt-default");

    let changed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"set-first-model",
                "sessionId":first_id,
                "expectedSequence":1,
                "request":{"type":"setModel","model":{"provider":"openai","model":"gpt-alternate"}}
            }
        }),
    );
    assert_eq!(
        changed["result"]["value"]["session"]["model"]["model"],
        "gpt-alternate"
    );
    let unchanged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":6,
            "method":"session/read",
            "params":{"sessionId":second_id}
        }),
    );
    assert_eq!(
        unchanged["result"]["session"]["model"]["model"],
        "gpt-default"
    );
}

#[test]
fn typed_commands_replay_and_reject_payload_conflicts() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let first = create_session(&server, &mut connection, 2, "same-command");
    let replayed = create_session(&server, &mut connection, 3, "same-command");
    assert_eq!(
        replayed["result"]["session"]["sessionId"],
        first["result"]["session"]["sessionId"]
    );
    let conflict = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/create",
            "params":{"commandId":"same-command","title":"different"}
        }),
    );
    assert_eq!(conflict["error"]["message"], "CommandConflict");
}

#[test]
fn fork_preserves_parent_context_without_calling_the_model() {
    let model = Arc::new(RecordingModel::default());
    let server = server_with_model(model.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let root = create_thread(&server, &mut connection, 3, "root", session_id, 1);
    let root_id = root["result"]["value"]["threadId"].as_str().unwrap();
    call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"parent-turn","sessionId":session_id,"expectedSequence":1,
                "request":{"type":"startTurn","threadId":root_id,"input":[{"type":"text","text":"parent prompt"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, root_id, TurnStatus::Completed);
    assert_eq!(model.requests().len(), 1);

    let fork = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"fork",
                "sessionId":session_id,
                "expectedSequence":3,
                "request":{"type":"forkThread","parentThreadId":root_id,"title":"branch"}
            }
        }),
    );
    assert_eq!(model.requests().len(), 1);

    assert_eq!(
        fork["result"]["value"]["session"]["threads"][1]["origin"]["type"],
        "fork"
    );
    assert_eq!(
        fork["result"]["value"]["session"]["threads"][1]["origin"]["parentSequence"],
        server
            .sessions()
            .threads()
            .read_thread(&zeta_protocol::ThreadId::new(root_id).unwrap())
            .unwrap()
            .sequence
    );
    let child_id = fork["result"]["value"]["threadId"].as_str().unwrap();
    let child = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/thread/read",
            "params":{"sessionId":session_id,"threadId":child_id}
        }),
    );
    assert_eq!(
        child["result"]["thread"]["turns"].as_array().unwrap().len(),
        1
    );
    call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"session/request",
            "params":{
                "commandId":"child-turn","sessionId":session_id,
                "expectedSequence":child["result"]["thread"]["sequence"],
                "request":{"type":"startTurn","threadId":child_id,"input":[{"type":"text","text":"child prompt"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, child_id, TurnStatus::Completed);
    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    assert!(model_request_contains_text(&requests[1], "parent prompt"));
    assert!(model_request_contains_text(&requests[1], "done"));
    assert!(model_request_contains_text(&requests[1], "child prompt"));
    assert_eq!(requests[0].prompt_cache_key.as_deref(), Some(session_id));
    assert_eq!(requests[1].prompt_cache_key, requests[0].prompt_cache_key);
    let prefix_end = requests[1].prompt_cache_prefix_end.unwrap() as usize;
    assert!(prefix_end < requests[1].input.len() - 1);
    assert!(requests[1].input[..=prefix_end].iter().any(|item| {
        matches!(
            item,
            zeta_protocol::InputItem::Message(message)
                if message.content.iter().any(|content| {
                    matches!(content, zeta_protocol::ContentPart::Text(text) if text == "done")
                })
        )
    }));
}

#[test]
fn rewind_endpoint_imports_only_history_before_the_selected_turn() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let root = create_thread(&server, &mut connection, 3, "root", session_id, 1);
    let root_id = root["result"]["value"]["threadId"].as_str().unwrap();

    let first = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"turn-first","sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":root_id,"input":[{"type":"text","text":"first"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, root_id, TurnStatus::Completed);
    let after_first = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/thread/read","params":{"sessionId":session_id,"threadId":root_id}
        }),
    );
    let second = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/request",
            "params":{
                "commandId":"turn-second","sessionId":session_id,
                "expectedSequence":after_first["result"]["thread"]["sequence"],
                "request":{"type":"startTurn","threadId":root_id,"input":[{"type":"text","text":"second"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, root_id, TurnStatus::Completed);
    let rewound = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"session/request",
            "params":{
                "commandId":"rewind","sessionId":session_id,"expectedSequence":3,
                "request":{"type":"rewindThread","parentThreadId":root_id,"beforeTurnId":second["result"]["value"]["turnId"],"title":"rewound"}
            }
        }),
    );
    let child_id = rewound["result"]["value"]["threadId"].as_str().unwrap();
    let child = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":8,"method":"session/thread/read","params":{"sessionId":session_id,"threadId":child_id}
        }),
    );

    assert_eq!(
        child["result"]["thread"]["turns"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        child["result"]["thread"]["turns"][0]["turnId"],
        first["result"]["value"]["turnId"]
    );
    assert_eq!(
        rewound["result"]["value"]["session"]["threads"][1]["origin"]["type"],
        "rewind"
    );
    assert_eq!(
        rewound["result"]["value"]["session"]["currentThreadId"],
        child_id
    );
}

#[test]
fn rewrite_endpoint_replays_one_child_and_one_replacement_turn() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let root = create_thread(&server, &mut connection, 3, "root", session_id, 1);
    let root_id = root["result"]["value"]["threadId"].as_str().unwrap();

    let first = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"turn-first","sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":root_id,"input":[{"type":"text","text":"first"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, root_id, TurnStatus::Completed);
    let root_snapshot = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/thread/read",
            "params":{"sessionId":session_id,"threadId":root_id}
        }),
    );
    let second = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/request",
            "params":{
                "commandId":"turn-second","sessionId":session_id,
                "expectedSequence":root_snapshot["result"]["thread"]["sequence"],
                "request":{"type":"startTurn","threadId":root_id,"input":[{"type":"text","text":"second"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, root_id, TurnStatus::Completed);

    let rewrite_request = serde_json::json!({
        "jsonrpc":"2.0","id":7,"method":"session/request",
        "params":{
            "commandId":"rewrite-operation","sessionId":session_id,"expectedSequence":3,
            "request":{
                "type":"rewriteThread",
                "parentThreadId":root_id,
                "beforeTurnId":second["result"]["value"]["turnId"],
                "title":"rewritten",
                "input":[{"type":"text","text":"replacement"}]
            }
        }
    });
    let rewritten = call(&server, &mut connection, rewrite_request.clone());
    assert!(rewritten.get("error").is_none(), "{rewritten}");
    let child_id = rewritten["result"]["value"]["threadId"].as_str().unwrap();
    let replacement_turn_id = rewritten["result"]["value"]["turn"]["turnId"]
        .as_str()
        .unwrap();
    wait_for_latest_turn(&server, child_id, TurnStatus::Completed);

    let mut replay_request = rewrite_request;
    replay_request["id"] = serde_json::json!(8);
    let replayed = call(&server, &mut connection, replay_request);
    assert!(replayed.get("error").is_none(), "{replayed}");
    assert_eq!(replayed["result"]["value"]["threadId"], child_id);
    assert_eq!(
        replayed["result"]["value"]["turn"]["turnId"],
        replacement_turn_id
    );
    assert_eq!(
        replayed["result"]["value"]["session"]["currentThreadId"],
        child_id
    );
    assert_eq!(
        replayed["result"]["value"]["session"]["threads"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let child = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(child_id).unwrap())
        .unwrap();
    assert_eq!(child.turns.len(), 2);
    assert_eq!(
        child.turns[0].turn_id.as_str(),
        first["result"]["value"]["turnId"]
    );
    assert_eq!(child.turns[1].turn_id.as_str(), replacement_turn_id);
    assert_eq!(
        child
            .commands
            .iter()
            .filter(|command| {
                command.receipt.command_id.as_str() == "session-rewrite/start/rewrite-operation"
            })
            .count(),
        1
    );
    let session = server
        .sessions()
        .read_session(&zeta_protocol::SessionId::new(session_id).unwrap())
        .unwrap();
    assert!(session.commands.iter().any(|command| {
        command.receipt.command_id.as_str() == "session-rewrite/rewind/rewrite-operation"
    }));

    let conflict = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":9,"method":"session/request",
            "params":{
                "commandId":"rewrite-operation","sessionId":session_id,"expectedSequence":3,
                "request":{
                    "type":"rewriteThread",
                    "parentThreadId":root_id,
                    "beforeTurnId":second["result"]["value"]["turnId"],
                    "title":"rewritten",
                    "input":[{"type":"text","text":"different replacement"}]
                }
            }
        }),
    );
    assert_eq!(conflict["error"]["message"], "CommandConflict");
}

#[test]
fn thread_read_returns_a_bounded_latest_history_window() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let root = create_thread(&server, &mut connection, 3, "root", session_id, 1);
    let thread_id = root["result"]["value"]["threadId"].as_str().unwrap();

    let first = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"turn-first","sessionId":session_id,"expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"first"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    let full_after_first = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/thread/read",
            "params":{"sessionId":session_id,"threadId":thread_id}
        }),
    );
    let second = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"session/request",
            "params":{
                "commandId":"turn-second","sessionId":session_id,
                "expectedSequence":full_after_first["result"]["thread"]["sequence"],
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"second"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);

    let page = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"session/thread/read",
            "params":{
                "sessionId":session_id,"threadId":thread_id,
                "history":{"type":"latest","turnLimit":1}
            }
        }),
    );

    assert_eq!(
        page["result"]["thread"]["turns"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        page["result"]["thread"]["turns"][0]["turnId"],
        second["result"]["value"]["turnId"]
    );
    assert_eq!(page["result"]["history"]["hasOlderTurns"], true);
    assert_eq!(
        page["result"]["history"]["oldestTurnId"],
        second["result"]["value"]["turnId"]
    );
    assert_ne!(
        page["result"]["history"]["oldestTurnId"],
        first["result"]["value"]["turnId"]
    );

    let older_page = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7_1,"method":"session/thread/read",
            "params":{
                "sessionId":session_id,"threadId":thread_id,
                "history":{
                    "type":"before",
                    "turnId":second["result"]["value"]["turnId"],
                    "turnLimit":1
                }
            }
        }),
    );
    assert_eq!(
        older_page["result"]["thread"]["turns"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        older_page["result"]["thread"]["turns"][0]["turnId"],
        first["result"]["value"]["turnId"]
    );
    assert_eq!(older_page["result"]["history"]["hasOlderTurns"], false);

    let subscription = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":8,"method":"session/thread/subscribe",
            "params":{
                "sessionId":session_id,"threadId":thread_id,"afterSequence":0,
                "history":{"type":"latest","turnLimit":1}
            }
        }),
    );
    assert_eq!(
        subscription["result"]["thread"]["turns"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        subscription["result"]["updates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(subscription["result"]["history"]["hasOlderTurns"], true);

    let invalid = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":9,"method":"session/thread/read",
            "params":{
                "sessionId":session_id,"threadId":thread_id,
                "history":{"type":"latest","turnLimit":0}
            }
        }),
    );
    assert_eq!(invalid["error"]["message"], "InvalidParams");
}

#[derive(Default)]
struct CountingModel {
    calls: AtomicUsize,
}

#[derive(Default)]
struct SteeringModelState {
    requests: Vec<ModelRequest>,
    first_call_started: bool,
    release_first_call: bool,
}

#[derive(Default)]
struct AppServerSteeringModel {
    state: Mutex<SteeringModelState>,
    changed: Condvar,
}

impl AppServerSteeringModel {
    fn wait_for_first_call(&self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut state = self.state.lock().unwrap();
        while !state.first_call_started {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(!remaining.is_zero(), "model invocation did not start");
            let (next, _) = self.changed.wait_timeout(state, remaining).unwrap();
            state = next;
        }
    }

    fn release_first_call(&self) {
        let mut state = self.state.lock().unwrap();
        state.release_first_call = true;
        self.changed.notify_all();
    }

    fn requests(&self) -> Vec<ModelRequest> {
        self.state.lock().unwrap().requests.clone()
    }
}

struct ReleaseSteeringModel(Arc<AppServerSteeringModel>);

impl Drop for ReleaseSteeringModel {
    fn drop(&mut self) {
        self.0.release_first_call();
    }
}

#[derive(Default)]
struct FailingSteerBackend {
    steers: AtomicUsize,
}

#[derive(Default)]
struct CountingStartBackend {
    starts: AtomicUsize,
}

impl zeta_core::TurnExecutionBackend for CountingStartBackend {
    fn start(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        self.starts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn resume(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

impl zeta_core::TurnExecutionBackend for FailingSteerBackend {
    fn start(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn resume(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    fn steer(
        &self,
        _: &zeta_protocol::ThreadId,
        _: &zeta_protocol::TurnId,
        _: &CommandId,
        _: &[UserInput],
    ) -> Result<(), CoreError> {
        self.steers.fetch_add(1, Ordering::Relaxed);
        Err(CoreError::Execution("backend steer failed".into()))
    }
}

struct ShellTestTool;

impl ToolService for ShellTestTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: zeta_protocol::ToolName::new("shell-command").unwrap(),
            description: "run a test shell command".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "program": {"type": "string"},
                    "arguments": {"type": "array"},
                    "working_directory": {"type": "string"}
                },
                "required": ["program", "arguments", "working_directory"]
            }),
            strict: true,
        }]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(call.arguments.to_string()),
                ActionKind::LocalProcess(ProcessInvocationKind::Shell),
                "run test shell command",
                CapabilitySet::new([Capability::new(CapabilityKind::ProcessSpawn, "/bin/sh")]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "shell-command"),
            SandboxCompatibility::Supported(shell_test_sandbox()),
            ActionPolicyRevision::new("test-shell-v1"),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Ok(ToolExecutionOutput::Success(
            serde_json::json!({
                "exit_code": 0,
                "stdout": "shell output\n",
                "stderr": "",
                "stdout_truncated": false,
                "stderr_truncated": false,
            })
            .to_string(),
        ))
    }

    fn execute_streaming(
        &self,
        call: &ToolCall,
        authorization: &ToolAuthorization,
        cancellation: &CancellationToken,
        sink: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        sink.emit(ToolOutputStream::Stdout, "shell output\n".into())?;
        self.execute(call, authorization, cancellation)
    }
}

struct ShellTestPolicy;

impl ActionPolicyService for ShellTestPolicy {
    fn revision(&self) -> String {
        "test-policy-v1".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Ok(ExecutionDecision::RunSandboxed(shell_test_sandbox()))
    }
}

fn shell_test_sandbox() -> SandboxPolicy {
    SandboxPolicy::new(FileSystemAccess::WorkspaceWrite, NetworkAccess::Denied)
}

#[test]
fn shell_turn_runs_without_a_model_and_publishes_typed_output() {
    let model = Arc::new(CountingModel::default());
    let server = server_with_model(model.clone())
        .with_tool_service(Arc::new(ShellTestTool), Arc::new(ShellTestPolicy));
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":1}
        }),
    );

    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"shell-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startShellTurn","threadId":thread_id,"command":"printf shell output","workingDirectory":"."}
            }
        }),
    );

    assert!(response["result"]["value"]["turnId"].is_string());
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    assert_eq!(model.calls.load(Ordering::Relaxed), 0);
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(thread_id).unwrap())
        .unwrap();
    assert!(snapshot.items.iter().any(|item| {
        matches!(
            item,
            zeta_protocol::ThreadItem::ToolCall {
                name,
                binding: Some(binding),
                ..
            } if name.as_str() == "shell-command" && binding.caller == zeta_protocol::ToolCallCaller::Direct
        )
    }));
    assert!(snapshot.items.iter().any(|item| {
        matches!(
            item,
            zeta_protocol::ThreadItem::ToolResult { text, is_error: false, .. }
                if text.contains("shell output")
        )
    }));
    assert!(
        server
            .drain_notifications(&mut connection)
            .iter()
            .any(|notification| notification.contains("\"toolOutputDelta\""))
    );
}

impl ModelService for CountingModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let prompt = request
            .input
            .iter()
            .find_map(|item| match item {
                InputItem::Message(message) => message.content.iter().find_map(|content| {
                    let ContentPart::Text(text) = content else {
                        return None;
                    };
                    Some(text.as_str())
                }),
                InputItem::ToolResult(_) => None,
            })
            .unwrap_or_default();
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("answer: {prompt}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

impl ModelService for AppServerSteeringModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        request: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let mut state = self.state.lock().unwrap();
        let call = state.requests.len();
        state.requests.push(request.clone());
        if call == 0 {
            state.first_call_started = true;
            self.changed.notify_all();
            while !state.release_first_call {
                state = self.changed.wait(state).unwrap();
            }
        }
        drop(state);
        Ok(ModelResponse {
            output: vec![ResponseItem::Text(format!("response-{call}"))],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

#[test]
fn completed_turn_replays_without_invoking_the_model_twice() {
    let model = Arc::new(CountingModel::default());
    let server = server_with_model(model.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let request = |id| {
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":id,
            "method":"session/request",
            "params":{
                "commandId":"turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{
                    "type":"startTurn",
                    "threadId":thread_id,
                    "input":[{"type":"text","text":"hello"}]
                }
            }
        })
    };
    let first = call(&server, &mut connection, request(4));
    let replayed = call(&server, &mut connection, request(5));

    assert_eq!(
        first["result"]["value"]["turnId"],
        replayed["result"]["value"]["turnId"]
    );
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    assert_eq!(model.calls.load(Ordering::Relaxed), 1);
    let notifications = server.drain_notifications(&mut connection);
    assert!(notifications.iter().any(|notification| {
        notification.contains("\"method\":\"session/thread/update\"")
            && notification.contains("\"agentMessage\"")
    }));
}

#[derive(Default)]
struct RecordingModel {
    requests: Mutex<Vec<ModelRequest>>,
}

impl RecordingModel {
    fn requests(&self) -> Vec<ModelRequest> {
        self.requests.lock().unwrap().clone()
    }
}

struct EmptySkillConfig;

impl zeta_skills_extension::SkillConfigSnapshotProvider for EmptySkillConfig {
    fn snapshot(&self) -> Result<zeta_config::SkillsConfig, String> {
        Ok(zeta_config::SkillsConfig::default())
    }
}

impl ModelService for RecordingModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
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

fn model_request_contains_text(request: &ModelRequest, expected: &str) -> bool {
    request.input.iter().any(|item| {
        let InputItem::Message(message) = item else {
            return false;
        };
        message
            .content
            .iter()
            .any(|part| matches!(part, ContentPart::Text(text) if text == expected))
    })
}

#[test]
fn review_turn_freezes_review_rubric_and_renders_the_requested_target() {
    let model = Arc::new(RecordingModel::default());
    let server = server_with_model(model.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "review-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "review-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"review-turn","sessionId":session_id,"expectedSequence":1,
                "request":{
                    "type":"startReview","threadId":thread_id,
                    "target":{"type":"baseBranch","branch":"main"}
                }
            }
        }),
    );

    assert!(started["result"]["value"]["turnId"].is_string());
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    let requests = model.requests();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .instructions
            .as_deref()
            .is_some_and(|body| body.contains("overall_correctness"))
    );
    assert!(model_request_contains_text(
        &requests[0],
        "Review the changes against base branch `main`. Determine the merge base with HEAD, then inspect the diff from that merge base."
    ));
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(thread_id).unwrap())
        .unwrap();
    let instructions = snapshot.turns[0].instructions.as_ref().unwrap();
    assert_eq!(snapshot.turns[0].kind, zeta_protocol::TurnKind::Review);
    assert_eq!(instructions.owner(), "prompts");
    assert_eq!(instructions.id(), "review/code");
}

#[test]
fn explicit_skill_flows_through_core_extension_lifecycle() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-extension-skill-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let skill_root = root.join("skill-creator");
    std::fs::create_dir_all(&skill_root).unwrap();
    std::fs::write(
        skill_root.join("SKILL.md"),
        "---\nname: skill-creator\ndescription: Create skills\n---\n\nExtension instructions.\n",
    )
    .unwrap();
    let model = Arc::new(RecordingModel::default());
    let server = server_with_model(model.clone())
        .with_skill_runtime(
            zeta_skills_extension::BuiltInSkillSource::Root(root.clone()),
            Arc::new(EmptySkillConfig),
            None,
        )
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "skill-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "skill-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"skill-turn","sessionId":session_id,"expectedSequence":1,
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
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(thread_id).unwrap())
        .unwrap();
    assert_eq!(snapshot.turns[0].activated_skills.len(), 1);
    let requests = model.requests.lock().unwrap();
    assert!(
        requests[0]
            .tools
            .iter()
            .any(|tool| tool.name.as_str() == zeta_skills_extension::SKILLS_READ_TOOL_NAME)
    );
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .filter(|tool| tool.name.as_str() == zeta_skills_extension::SKILLS_READ_TOOL_NAME)
            .count(),
        1
    );
    assert!(requests[0].input.iter().any(|item| {
        let InputItem::Message(message) = item else {
            return false;
        };
        message.content.iter().any(|part| {
            matches!(part, ContentPart::Text(text) if text.contains("<skill-instructions") && text.contains("Extension instructions."))
        })
    }));
    assert!(requests[0].input.iter().any(|item| {
        let InputItem::Message(message) = item else {
            return false;
        };
        message.content.iter().any(|part| {
            matches!(part, ContentPart::Text(text) if text.contains("<available-skills") && text.contains("name=\"skill-creator\""))
        })
    }));
    assert!(
        requests[0]
            .tools
            .iter()
            .any(|tool| tool.name.as_str() == zeta_skills_extension::SKILLS_READ_TOOL_NAME)
    );
    drop(requests);
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn session_request_routes_typed_mutations_through_the_session_boundary() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();

    let thread = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"session/request",
            "params":{
                "commandId":"create-thread",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"createThread","title":"thread"}
            }
        }),
    );
    assert_eq!(thread["result"]["type"], "thread");
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let session_sequence = thread["result"]["value"]["session"]["sequence"]
        .as_u64()
        .unwrap();

    let approval_mode = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/request",
            "params":{
                "commandId":"set-next-approval-mode",
                "sessionId":session_id,
                "expectedSequence":session_sequence,
                "request":{"type":"setNextApprovalMode","approvalMode":"autoReview"}
            }
        }),
    );
    assert_eq!(
        approval_mode["result"]["value"]["session"]["nextApprovalMode"],
        "autoReview"
    );

    let turn = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"start-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{
                    "type":"startTurn",
                    "threadId":thread_id,
                    "input":[{"type":"text","text":"hello"}]
                }
            }
        }),
    );
    assert_eq!(turn["result"]["type"], "turn");
    assert!(turn["result"]["value"]["turnId"].is_string());
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(thread_id).unwrap())
        .unwrap();
    assert_eq!(
        snapshot.turns.last().unwrap().approval_mode,
        zeta_protocol::ApprovalMode::AutoReview
    );
}

#[test]
fn session_request_steers_a_running_turn_retry_safely_and_replans() {
    let model = Arc::new(AppServerSteeringModel::default());
    let _release_on_drop = ReleaseSteeringModel(Arc::clone(&model));
    let server = server_with_model(model.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "steer-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "steer-thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"start-steered-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"initial"}]}
            }
        }),
    );
    let turn_id = started["result"]["value"]["turnId"].as_str().unwrap();
    let start_sequence = started["result"]["value"]["sequence"].as_u64().unwrap();
    model.wait_for_first_call();

    let steer_request = |id| {
        serde_json::json!({
            "jsonrpc":"2.0","id":id,"method":"session/request",
            "params":{
                "commandId":"steer-running-turn",
                "sessionId":session_id,
                "expectedSequence":start_sequence,
                "request":{
                    "type":"steerTurn",
                    "threadId":thread_id,
                    "turnId":turn_id,
                    "input":[{"type":"text","text":"focus on the failing test"}]
                }
            }
        })
    };
    let steered = call(&server, &mut connection, steer_request(5));
    assert_eq!(steered["result"]["type"], "turnSteer");
    assert_eq!(steered["result"]["value"]["turnId"], turn_id);
    assert_eq!(
        steered["result"]["value"]["sequence"].as_u64().unwrap(),
        start_sequence + 3
    );

    let replayed = call(&server, &mut connection, steer_request(6));
    assert_eq!(replayed["result"], steered["result"]);
    let conflict = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"session/request",
            "params":{
                "commandId":"steer-running-turn",
                "sessionId":session_id,
                "expectedSequence":start_sequence,
                "request":{
                    "type":"steerTurn",
                    "threadId":thread_id,
                    "turnId":turn_id,
                    "input":[{"type":"text","text":"different payload"}]
                }
            }
        }),
    );
    assert_eq!(conflict["error"]["message"], "CommandConflict");

    let protocol_thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&protocol_thread_id)
        .unwrap();
    assert_eq!(
        snapshot
            .items
            .iter()
            .filter(|item| matches!(
                item,
                zeta_protocol::ThreadItem::UserMessage { text, .. }
                    if text == "focus on the failing test"
            ))
            .count(),
        1
    );

    model.release_first_call();
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);
    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].input.iter().any(|item| {
        matches!(item, InputItem::Message(message) if message.content.iter().any(
            |part| matches!(part, ContentPart::Text(text) if text == "focus on the failing test")
        ))
    }));
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&protocol_thread_id)
        .unwrap();
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        zeta_protocol::ThreadItem::AgentMessage { text, .. } if text == "response-1"
    )));
    assert!(!snapshot.items.iter().any(|item| matches!(
        item,
        zeta_protocol::ThreadItem::AgentMessage { text, .. } if text == "response-0"
    )));
}

#[test]
fn failed_backend_steer_is_not_replayed_across_rpc_retry() {
    let backend = Arc::new(FailingSteerBackend::default());
    let server = server().with_turn_backend(backend.clone());
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let session = create_session(&server, &mut connection, 2, "failed-steer-session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(
        &server,
        &mut connection,
        3,
        "failed-steer-thread",
        session_id,
        1,
    );
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"start-failed-steer-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"initial"}]}
            }
        }),
    );
    let turn_id = started["result"]["value"]["turnId"].as_str().unwrap();
    let sequence = started["result"]["value"]["sequence"].as_u64().unwrap();
    let request = |id| {
        serde_json::json!({
            "jsonrpc":"2.0","id":id,"method":"session/request",
            "params":{
                "commandId":"failed-backend-steer",
                "sessionId":session_id,
                "expectedSequence":sequence,
                "request":{
                    "type":"steerTurn",
                    "threadId":thread_id,
                    "turnId":turn_id,
                    "input":[{"type":"text","text":"updated direction"}]
                }
            }
        })
    };

    let failed = call(&server, &mut connection, request(5));
    let replayed = call(&server, &mut connection, request(6));

    assert_eq!(failed["error"]["message"], "CoreOperationFailed");
    assert_eq!(replayed["error"]["message"], "CoreOperationFailed");
    assert_eq!(backend.steers.load(Ordering::Relaxed), 1);
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&zeta_protocol::ThreadId::new(thread_id).unwrap())
        .unwrap();
    assert_eq!(snapshot.turns[0].status, TurnStatus::Failed);
    assert!(
        !snapshot
            .steer_deliveries
            .contains_key(&CommandId::new("failed-backend-steer").unwrap())
    );
}

#[test]
fn updates_are_broadcast_to_other_subscribed_connections() {
    let server = server();
    let mut writer = server.connection();
    initialize(&server, &mut writer);
    let session = create_session(&server, &mut writer, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut writer, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    let mut observer = server.connection();
    initialize(&server, &mut observer);
    let session_subscription = call(
        &server,
        &mut observer,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"session/subscribe",
            "params":{"sessionId":session_id,"afterSequence":3}
        }),
    );
    assert_eq!(
        session_subscription["result"]["threadProjections"][0]["thread"]["threadId"],
        thread_id
    );
    assert_eq!(
        session_subscription["result"]["agentTree"]["roots"][0]["threadId"],
        thread_id
    );
    call(
        &server,
        &mut writer,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"session/request",
            "params":{
                "commandId":"fork",
                "sessionId":session_id,
                "expectedSequence":3,
                "request":{"type":"forkThread","parentThreadId":thread_id,"title":"branch"}
            }
        }),
    );
    call(
        &server,
        &mut writer,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"session/request",
            "params":{
                "commandId":"turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"hello"}]}
            }
        }),
    );
    wait_for_latest_turn(&server, thread_id, TurnStatus::Completed);

    let notifications = server.drain_notifications(&mut observer);
    assert!(
        notifications
            .iter()
            .any(|value| value.contains("\"method\":\"session/update\""))
    );
    assert!(notifications.iter().any(|value| {
        value.contains("\"method\":\"session/thread/update\"") && value.contains("\"agentMessage\"")
    }));
    assert!(notifications.iter().any(|value| {
        value.contains("\"method\":\"session/thread/update\"")
            && value.contains("\"itemDelta\"")
            && value.contains("\"streamCursor\"")
    }));
}

#[test]
fn subscribe_returns_durable_gap_for_reconnect() {
    let server = server();
    let mut first_connection = server.connection();
    initialize(&server, &mut first_connection);
    let session = create_session(&server, &mut first_connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut first_connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();

    let mut reconnected = server.connection();
    initialize(&server, &mut reconnected);
    let replay = call(
        &server,
        &mut reconnected,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":0}
        }),
    );
    assert_eq!(replay["result"]["updates"][0]["durableSequence"], 1);
    assert_eq!(
        replay["result"]["updates"][0]["update"]["type"],
        "committed"
    );
}

#[test]
fn resources_remain_connection_owned_and_chunked() {
    let server = server();
    let mut owner = server.connection();
    let mut other = server.connection();
    initialize(&server, &mut owner);
    initialize(&server, &mut other);
    let resource_id = server
        .create_resource(&owner, "text/plain".into(), b"hello".to_vec())
        .unwrap();
    let owner_read = call(
        &server,
        &mut owner,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":3}
        }),
    );
    assert_eq!(owner_read["result"]["decodedLength"], 3);
    let denied = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":3}
        }),
    );
    assert_eq!(denied["error"]["message"], "ResourceNotOwner");
}

#[test]
fn typst_compilation_returns_a_connection_owned_pdf_resource() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let compiled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"document/typst/compile",
            "params":{"source":"= Paper\n\nA formula: $x^2$."}
        }),
    );
    assert_eq!(compiled["result"]["status"], "success");
    assert_eq!(
        compiled["result"]["resource"]["mimeType"],
        "application/pdf"
    );
    let resource_id = compiled["result"]["resource"]["resourceId"]
        .as_str()
        .unwrap();

    let bytes = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"resource/read",
            "params":{"resourceId":resource_id,"offset":0,"maxBytes":16}
        }),
    );
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(bytes["result"]["dataBase64"].as_str().unwrap())
        .unwrap();
    assert!(decoded.starts_with(b"%PDF-"));
}

#[test]
fn typst_source_errors_are_typed_results_not_server_failures() {
    let server = server();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let compiled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"document/typst/compile",
            "params":{"source":"#let ="}
        }),
    );
    assert_eq!(compiled["result"]["status"], "failed");
    assert!(
        compiled["result"]["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| !diagnostics.is_empty())
    );
}

#[test]
fn config_updates_use_typed_command_ids() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-config-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server().with_config_store(Arc::new(ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    let mut observer = server.connection();
    initialize(&server, &mut connection);
    initialize(&server, &mut observer);
    let updated = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"config/update",
            "params":{
                "commandId":"config-noop","expectedRevision":0,
                "approvalReviewModel":{"type":"automatic"}
            }
        }),
    );
    assert_eq!(updated["result"]["revision"], 0);
    assert_eq!(updated["result"]["generation"], 0);
    assert_eq!(updated["result"]["disposition"], "updated");
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(read["result"]["revision"], 0);
    assert_eq!(
        read["result"]["approvalReviewModel"],
        serde_json::json!({"type":"automatic"})
    );
    let mcp = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"mcp/server/upsert",
            "params":{
                "commandId":"github-mcp","expectedRevision":0,
                "server":{
                    "id":"user:mcp:github",
                    "displayName":"GitHub",
                    "transport":{"type":"streamableHttp","url":"https://mcp.github.example"},
                    "credential":{"type":"reference","credentialRef":"user:credential:github"},
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(mcp["result"]["revision"], 1);
    let mcp_status = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":40,"method":"mcp/server/status","params":{}
        }),
    );
    assert_eq!(mcp_status["result"]["catalogGeneration"], 1);
    assert_eq!(mcp_status["result"]["servers"][0]["id"], "user:mcp:github");
    assert_eq!(
        mcp_status["result"]["servers"][0]["state"]["status"],
        "disabled"
    );
    let skill = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":5,"method":"skill/source/add",
            "params":{
                "commandId":"personal-skills","expectedRevision":1,
                "source":{
                    "id":"user:skill-source:personal",
                    "rootReference":"user:skill-root:personal",
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(skill["result"]["revision"], 2);
    let enabled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":6,"method":"mcp/server/enablement/set",
            "params":{
                "commandId":"enable-github-mcp","expectedRevision":2,
                "serverId":"user:mcp:github","enablement":"enabled"
            }
        }),
    );
    assert_eq!(enabled["result"]["revision"], 3);
    let stale = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":7,"method":"skill/source/enablement/set",
            "params":{
                "commandId":"stale-skill","expectedRevision":2,
                "sourceId":"user:skill-source:personal","enablement":"enabled"
            }
        }),
    );
    assert_eq!(stale["error"]["message"], "ConfigRevisionConflict");
    let plugin = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":8,"method":"plugin/request/upsert",
            "params":{
                "commandId":"request-review-plugin","expectedRevision":3,
                "request":{
                    "pluginId":"acme/code-review",
                    "version":"1.2.3",
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(plugin["result"]["revision"], 4);
    let hook = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":9,"method":"hook/upsert",
            "params":{
                "commandId":"add-review-hook","expectedRevision":4,
                "hook":{
                    "id":"user:hook:review",
                    "event":"beforeTool",
                    "matcher":{"toolNames":["shell_command"]},
                    "action":{"type":"process","program":"review-hook","args":["--check"]},
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(hook["result"]["revision"], 5);
    let plugin_enabled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":10,"method":"plugin/request/enablement/set",
            "params":{
                "commandId":"enable-review-plugin","expectedRevision":5,
                "pluginId":"acme/code-review","enablement":"enabled"
            }
        }),
    );
    assert_eq!(plugin_enabled["result"]["revision"], 6);
    let hook_enabled = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":11,"method":"hook/enablement/set",
            "params":{
                "commandId":"enable-review-hook","expectedRevision":6,
                "hookId":"user:hook:review","enablement":"enabled"
            }
        }),
    );
    assert_eq!(hook_enabled["result"]["revision"], 7);
    let executable = std::env::temp_dir()
        .join("rust-analyzer")
        .to_string_lossy()
        .into_owned();
    let language_server = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":12,"method":"languageServer/configure",
            "params":{
                "commandId":"configure-rust-analyzer","expectedRevision":7,
                "serverId":"rust-analyzer",
                "config":{"mode":"enabled","executable":executable}
            }
        }),
    );
    assert_eq!(language_server["result"]["revision"], 8);
    let configured = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":13,"method":"config/read","params":{}}),
    );
    assert_eq!(configured["result"]["revision"], 8);
    assert_eq!(
        configured["result"]["mcpServers"]["user:mcp:github"]["enablement"],
        "enabled"
    );
    assert_eq!(
        configured["result"]["skillSources"]["user:skill-source:personal"]["rootReference"],
        "user:skill-root:personal"
    );
    assert_eq!(
        configured["result"]["pluginRequests"]["acme/code-review"]["enablement"],
        "enabled"
    );
    assert_eq!(
        configured["result"]["hooks"]["user:hook:review"]["action"]["program"],
        "review-hook"
    );
    assert_eq!(
        configured["result"]["languageServers"]["rust-analyzer"]["mode"],
        "enabled"
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let observed = server
            .drain_notifications(&mut observer)
            .into_iter()
            .map(|notification| serde_json::from_str::<serde_json::Value>(&notification).unwrap())
            .any(|notification| {
                notification["method"] == "config/changed"
                    && notification["params"]["revision"] == 8
                    && notification["params"]["generation"] == 8
            });
        if observed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "config watcher did not publish the committed generation"
        );
        thread::sleep(Duration::from_millis(10));
    }
    drop(observer);
    drop(connection);
    drop(server);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
}

#[test]
fn exec_policy_rule_rpc_round_trips_typed_user_rules() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-exec-policy-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server().with_config_store(Arc::new(ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let upserted = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"execPolicy/rule/upsert",
            "params":{
                "commandId":"allow-git-status","expectedRevision":0,
                "rule":{
                    "id":"allow-git-status",
                    "selector":{
                        "type":"commandPrefix",
                        "pattern":[
                            {"type":"literal","value":"git"},
                            {"type":"literal","value":"status"}
                        ]
                    },
                    "effect":{"type":"allowUnsandboxed"},
                    "justification":"explicit user rule"
                }
            }
        }),
    );
    assert_eq!(upserted["result"]["revision"], 1);

    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(
        read["result"]["execPolicyRules"],
        serde_json::json!([{
            "id":"allow-git-status",
            "selector":{
                "type":"commandPrefix",
                "pattern":[
                    {"type":"literal","value":"git"},
                    {"type":"literal","value":"status"}
                ]
            },
            "effect":{"type":"allowUnsandboxed"},
            "justification":"explicit user rule"
        }])
    );

    let removed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"execPolicy/rule/remove",
            "params":{
                "commandId":"remove-git-status","expectedRevision":1,
                "ruleId":"allow-git-status"
            }
        }),
    );
    assert_eq!(removed["result"]["revision"], 2);
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":5,"method":"config/read","params":{}}),
    );
    assert_eq!(read["result"]["execPolicyRules"], serde_json::json!([]));
}

#[test]
fn mcp_runtime_intent_rpc_does_not_mutate_config_revision() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-mcp-intent-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server()
        .with_config_store(Arc::new(ConfigStore::open(&path).unwrap()))
        .with_local_workspace_host(None, crate::server::WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();
    let mut connection = server.connection();
    let initialized = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );
    assert!(
        initialized["result"]["capabilities"]["sessions"]
            .as_bool()
            .unwrap()
    );

    let created = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"mcp/server/upsert",
            "params":{
                "commandId":"intent-server","expectedRevision":0,
                "server":{
                    "id":"user:mcp:intent",
                    "displayName":"Intent",
                    "transport":{"type":"streamableHttp","url":"https://mcp.example.test"},
                    "credential":{"type":"unauthenticated"},
                    "enablement":"disabled"
                }
            }
        }),
    );
    assert_eq!(created["result"]["revision"], 1);
    let connected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"mcp/server/connect",
            "params":{"serverId":"user:mcp:intent"}
        }),
    );
    assert_eq!(
        connected["result"],
        serde_json::json!({
            "serverId":"user:mcp:intent","intent":"connect"
        })
    );
    let disconnected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"mcp/server/disconnect",
            "params":{"serverId":"user:mcp:intent"}
        }),
    );
    assert_eq!(
        disconnected["result"],
        serde_json::json!({
            "serverId":"user:mcp:intent","intent":"disconnect"
        })
    );
    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":5,"method":"config/read","params":{}}),
    );
    assert_eq!(read["result"]["revision"], 1);
    assert_eq!(
        read["result"]["mcpServers"]["user:mcp:intent"]["enablement"],
        "disabled"
    );
}

#[test]
fn tool_search_configure_rejects_an_unavailable_embedding_before_commit() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-tool-search-config-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server().with_config_store(Arc::new(ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let rejected = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"toolSearch/configure",
            "params":{
                "commandId":"enable-tool-search","expectedRevision":0,
                "mode":"hybridEmbedding",
                "embeddingModel":{"provider":"ollama","model":"nomic-embed-text"}
            }
        }),
    );
    assert_eq!(rejected["error"]["message"], "ToolSearchUnavailable");

    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(read["result"]["revision"], 0);
    assert_eq!(read["result"]["toolSearch"]["mode"], "lexical");
    assert_eq!(
        read["result"]["toolSearch"]["embeddingStatus"]["type"],
        "disabled"
    );
}

#[test]
fn provider_context_budget_metadata_round_trips_through_the_rpc() {
    let path = std::env::temp_dir().join(format!(
        "zeta-app-server-model-context-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let server = server().with_config_store(Arc::new(ConfigStore::open(&path).unwrap()));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let configured = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"provider/configure",
            "params":{
                "commandId":"configure-openai-context","expectedRevision":0,
                "config":{
                    "provider":"openai",
                    "modelContext":{
                        "gpt-5.6":{
                            "contextWindow":200000,
                            "autoCompactTokenLimit":150000
                        }
                    }
                }
            }
        }),
    );
    assert_eq!(configured["result"]["revision"], 1);

    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"config/read","params":{}}),
    );
    assert_eq!(
        read["result"]["providers"]["openai"]["modelContext"]["gpt-5.6"],
        serde_json::json!({
            "contextWindow": 200000,
            "autoCompactTokenLimit": 150000
        })
    );

    drop(connection);
    drop(server);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("toml"));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
}

#[test]
fn interaction_resolution_uses_the_durable_identity_and_resumes_the_turn() {
    let server = server();
    let mut connection = server.connection();
    initialize_with_capabilities(
        &server,
        &mut connection,
        serde_json::json!({
            "agentInteractions":{"version":1,"kinds":["userInput"]}
        }),
    );
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("agent-turn").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "wait".into(),
                }],
            },
        )
        .unwrap();
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("input-1").unwrap(),
                item_id: None,
                request: AgentRequest::UserInput {
                    request: RequestUserInput {
                        questions: Vec::new(),
                    },
                },
                deadline: None,
            },
        )
        .unwrap();

    let subscribed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":0}
        }),
    );
    assert!(subscribed["result"]["thread"].is_object());
    let notifications = server.drain_notifications(&mut connection);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("\"method\":\"agent/request\""))
    );

    let resolved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"resolve-input-1",
                "sessionId":session_id,
                "expectedSequence":5,
                "request":{"type":"resolveInteraction","threadId":thread_id,"turnId":started.turn_id,"requestId":"input-1","response":{"type":"userInput", "response":{"answers":{}}}}
            }
        }),
    );

    assert_eq!(resolved["result"]["value"]["sequence"], 6);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
        assert!(snapshot.turns[0].pending_interaction.is_none());
        if snapshot.turns[0].status == zeta_core::TurnStatus::Completed {
            assert!(snapshot.items.iter().any(|item| {
                matches!(item, zeta_protocol::ThreadItem::AgentMessage { text, .. } if text == "Zeta: wait")
            }));
            break;
        }
        assert_eq!(snapshot.turns[0].status, zeta_core::TurnStatus::Running);
        assert!(
            Instant::now() < deadline,
            "resolved interaction did not resume the waiting Turn"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[derive(Default)]
struct AppServerInteractiveModel {
    calls: AtomicUsize,
}

impl ModelService for AppServerInteractiveModel {
    fn invoke(
        &self,
        _: zeta_core::ModelSelection<'_>,
        _: &ModelRequest,
        _: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        if call == 0 {
            return Ok(ModelResponse {
                output: vec![ResponseItem::ToolCall(ToolCall {
                    id: zeta_protocol::ToolCallId::new("app-server-interactive-call").unwrap(),
                    name: zeta_protocol::ToolName::new("app-server-interactive").unwrap(),
                    arguments: serde_json::json!({}),
                })],
                usage: None,
                stop_reason: StopReason::ToolUse,
            });
        }
        Ok(ModelResponse {
            output: vec![ResponseItem::Text("done".into())],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

struct AppServerInteractiveTool;

impl ToolService for AppServerInteractiveTool {
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: zeta_protocol::ToolName::new("app-server-interactive").unwrap(),
            description: "request test user input".into(),
            parameters: serde_json::json!({"type":"object"}),
            strict: true,
        }]
    }

    fn prepare(&self, call: &ToolCall) -> Result<ActionReviewRequest, CoreError> {
        Ok(ActionReviewRequest::new(
            ResolvedAction::new(
                ActionDigest::from_canonical_bytes(serde_json::to_vec(call).unwrap()),
                ActionKind::SystemOperation,
                "request test user input",
                CapabilitySet::new([]),
            ),
            ActionProvenance::new(ActionSource::BuiltInTool, "app-server-interactive"),
            SandboxCompatibility::Supported(shell_test_sandbox()),
            ActionPolicyRevision::new("app-server-interactive-v1"),
        ))
    }

    fn execute(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
    ) -> Result<ToolExecutionOutput, CoreError> {
        Err(CoreError::Execution(
            "interactive execution context is required".into(),
        ))
    }

    fn execute_streaming_with_facts_and_interactions(
        &self,
        _: &ToolCall,
        _: &ToolAuthorization,
        _: &CancellationToken,
        _: &zeta_core::ToolExecutionFacts,
        interactions: Arc<dyn zeta_core::ToolInteractionService>,
        _: &mut dyn ToolOutputSink,
    ) -> Result<ToolExecutionOutput, CoreError> {
        match interactions.request_user_input(RequestUserInput {
            questions: vec![zeta_protocol::UserInputQuestion {
                id: "city".into(),
                header: "City".into(),
                question: "Which city?".into(),
                options: Vec::new(),
                allow_free_form: true,
            }],
        })? {
            zeta_core::ToolUserInputOutcome::Answered(response) => Ok(
                ToolExecutionOutput::Success(response.answers["city"].value.clone()),
            ),
            zeta_core::ToolUserInputOutcome::Cancelled(reason) => Ok(ToolExecutionOutput::Failure(
                format!("interaction cancelled: {reason:?}"),
            )),
        }
    }
}

struct CountingResumeBackend {
    delegate: Arc<dyn zeta_core::TurnExecutionBackend>,
    resumes: Arc<AtomicUsize>,
}

impl zeta_core::TurnExecutionBackend for CountingResumeBackend {
    fn start(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        self.delegate.start(thread_id, turn_id)
    }

    fn resume(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
    ) -> Result<(), CoreError> {
        self.resumes.fetch_add(1, Ordering::Relaxed);
        self.delegate.resume(thread_id, turn_id)
    }

    fn steer(
        &self,
        thread_id: &zeta_protocol::ThreadId,
        turn_id: &zeta_protocol::TurnId,
        command_id: &CommandId,
        input: &[UserInput],
    ) -> Result<(), CoreError> {
        self.delegate.steer(thread_id, turn_id, command_id, input)
    }
}

#[test]
fn live_tool_interaction_resolution_wakes_execution_without_duplicate_backend_resume() {
    let model = Arc::new(AppServerInteractiveModel::default());
    let server = server_with_model(model.clone()).with_tool_service(
        Arc::new(AppServerInteractiveTool),
        Arc::new(ShellTestPolicy),
    );
    let resumes = Arc::new(AtomicUsize::new(0));
    let backend = Arc::new(CountingResumeBackend {
        delegate: server.turn_executor_backend(),
        resumes: resumes.clone(),
    });
    let server = server.with_turn_backend(backend);
    let mut connection = server.connection();
    initialize_with_capabilities(
        &server,
        &mut connection,
        serde_json::json!({
            "agentInteractions":{"version":1,"kinds":["userInput"]}
        }),
    );
    let session = create_session(&server, &mut connection, 2, "interactive-session");
    let session_id = session["result"]["session"]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    let thread = create_thread(
        &server,
        &mut connection,
        3,
        "interactive-thread",
        &session_id,
        1,
    );
    let thread_id = thread["result"]["value"]["threadId"]
        .as_str()
        .unwrap()
        .to_string();
    call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":1}
        }),
    );
    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"interactive-turn",
                "sessionId":session_id,
                "expectedSequence":1,
                "request":{"type":"startTurn","threadId":thread_id,"input":[{"type":"text","text":"ask"}]}
            }
        }),
    );
    let turn_id = started["result"]["value"]["turnId"]
        .as_str()
        .unwrap()
        .to_string();
    let protocol_thread_id = zeta_protocol::ThreadId::new(&thread_id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let (sequence, request_id) = loop {
        let snapshot = server
            .sessions()
            .threads()
            .read_thread(&protocol_thread_id)
            .unwrap();
        if let Some(interaction) = snapshot.turns[0].pending_interaction.as_ref() {
            break (snapshot.sequence, interaction.request_id.to_string());
        }
        assert!(
            Instant::now() < deadline,
            "interactive tool did not publish its durable request"
        );
        thread::sleep(Duration::from_millis(2));
    };
    let notifications = server.drain_notifications(&mut connection);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("\"method\":\"agent/request\""))
    );

    let resolved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":6,
            "method":"session/request",
            "params":{
                "commandId":"resolve-live-tool-input",
                "sessionId":session_id,
                "expectedSequence":sequence,
                "request":{
                    "type":"resolveInteraction",
                    "threadId":thread_id,
                    "turnId":turn_id,
                    "requestId":request_id,
                    "response":{"type":"userInput","response":{"answers":{"city":{"value":"Paris"}}}}
                }
            }
        }),
    );
    assert!(resolved["result"]["value"]["sequence"].is_number());
    wait_for_latest_turn(&server, &thread_id, TurnStatus::Completed);
    assert_eq!(resumes.load(Ordering::Relaxed), 0);
    assert_eq!(model.calls.load(Ordering::Relaxed), 2);
    let snapshot = server
        .sessions()
        .threads()
        .read_thread(&protocol_thread_id)
        .unwrap();
    assert!(snapshot.items.iter().any(|item| matches!(
        item,
        zeta_protocol::ThreadItem::ToolResult { text, is_error: false, .. } if text == "Paris"
    )));
}

#[test]
fn expired_interaction_is_cancelled_and_fails_the_turn() {
    let server = server();
    let mut connection = server.connection();
    initialize_with_capabilities(
        &server,
        &mut connection,
        serde_json::json!({
            "agentInteractions":{"version":1,"kinds":["userInput"]}
        }),
    );
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("deadline-turn").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "wait".into(),
                }],
            },
        )
        .unwrap();
    let expires_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 100;
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("deadline-input").unwrap(),
                item_id: None,
                request: AgentRequest::UserInput {
                    request: RequestUserInput {
                        questions: Vec::new(),
                    },
                },
                deadline: Some(InteractionDeadline { expires_at_unix_ms }),
            },
        )
        .unwrap();
    let subscribed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":0}
        }),
    );
    assert!(subscribed["result"]["thread"].is_object());

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = server.sessions().threads().read_thread(&thread_id).unwrap();
        if snapshot.turns[0].status == TurnStatus::Failed {
            assert!(snapshot.turns[0].pending_interaction.is_none());
            assert_eq!(
                snapshot.turns[0].failure.as_ref().unwrap().code,
                StableTurnErrorCode::InteractionDeadlineElapsed
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "interaction deadline watcher did not close the Turn"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn approval_interaction_resolves_through_the_typed_app_server_contract() {
    let server = server();
    let mut connection = server.connection();
    initialize_with_capabilities(
        &server,
        &mut connection,
        serde_json::json!({
            "agentInteractions":{"version":1,"kinds":["approval"]}
        }),
    );
    let session = create_session(&server, &mut connection, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut connection, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("approval-turn").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "approve".into(),
                }],
            },
        )
        .unwrap();
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("approval-1").unwrap(),
                item_id: None,
                request: AgentRequest::Approval {
                    request: ActionApprovalRequest {
                        action_digest: "a".repeat(64),
                        policy_revision: "policy-1".into(),
                        capabilities: vec![ActionApprovalCapability {
                            kind: ActionApprovalCapabilityKind::Network,
                            scope: "api.example.com".into(),
                        }],
                        reason: "network requires approval".into(),
                        sandbox_denial: None,
                    },
                },
                deadline: None,
            },
        )
        .unwrap();

    let subscribed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"session/thread/subscribe",
            "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":0}
        }),
    );
    assert!(subscribed["result"]["thread"].is_object());
    let notifications = server.drain_notifications(&mut connection);
    assert!(
        notifications
            .iter()
            .any(|notification| notification.contains("\"method\":\"agent/request\""))
    );

    let resolved = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"resolve-approval-1",
                "sessionId":session_id,
                "expectedSequence":5,
                "request":{"type":"resolveInteraction","threadId":thread_id,"turnId":started.turn_id,"requestId":"approval-1","response":{
                    "type":"approval",
                    "response":{"decision":"approveOnce"}
                }}
            }
        }),
    );

    assert_eq!(resolved["result"]["value"]["sequence"], 6);
    let events = server
        .sessions()
        .threads()
        .thread_updates_after(&thread_id, 5)
        .unwrap();
    assert!(matches!(
        &events[0].update,
        zeta_protocol::ThreadUpdate::Committed {
            event: zeta_protocol::ThreadEvent::InteractionResolved {
                response: zeta_protocol::AgentResponse::Approval { .. },
                ..
            }
        }
    ));
}

#[test]
fn interaction_response_is_rejected_from_a_capable_non_owner_connection() {
    let server = server();
    let mut owner = server.connection();
    let mut other = server.connection();
    let capabilities = serde_json::json!({
        "agentInteractions":{"version":1,"kinds":["approval"]}
    });
    initialize_with_capabilities(&server, &mut owner, capabilities.clone());
    initialize_with_capabilities(&server, &mut other, capabilities);
    let session = create_session(&server, &mut owner, 2, "session");
    let session_id = session["result"]["session"]["sessionId"].as_str().unwrap();
    let thread = create_thread(&server, &mut owner, 3, "thread", session_id, 1);
    let thread_id = thread["result"]["value"]["threadId"].as_str().unwrap();
    let thread_id = zeta_protocol::ThreadId::new(thread_id).unwrap();
    let session_id = zeta_protocol::SessionId::new(session_id).unwrap();
    let started = server
        .sessions()
        .threads()
        .start_turn(
            &thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("approval-turn-owner-check").unwrap(),
                expected_sequence: zeta_core::SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "approve".into(),
                }],
            },
        )
        .unwrap();
    server
        .sessions()
        .threads()
        .request_turn_interaction(
            &thread_id,
            &started.turn_id,
            RequestTurnInteraction {
                request_id: RequestId::new("approval-owner-check").unwrap(),
                item_id: None,
                request: AgentRequest::Approval {
                    request: ActionApprovalRequest {
                        action_digest: "b".repeat(64),
                        policy_revision: "policy-1".into(),
                        capabilities: vec![ActionApprovalCapability {
                            kind: ActionApprovalCapabilityKind::Network,
                            scope: "api.example.com".into(),
                        }],
                        reason: "network requires approval".into(),
                        sandbox_denial: None,
                    },
                },
                deadline: None,
            },
        )
        .unwrap();

    for connection in [&mut owner, &mut other] {
        let subscribed = call(
            &server,
            connection,
            serde_json::json!({
                "jsonrpc":"2.0",
                "id":4,
                "method":"session/thread/subscribe",
                "params":{"sessionId":session_id,"threadId":thread_id,"afterSequence":0}
            }),
        );
        assert!(subscribed["result"]["thread"].is_object());
    }
    assert!(
        server
            .drain_notifications(&mut owner)
            .iter()
            .any(|notification| notification.contains("\"method\":\"agent/request\""))
    );
    assert!(
        !server
            .drain_notifications(&mut other)
            .iter()
            .any(|notification| notification.contains("\"method\":\"agent/request\""))
    );

    let rejected = call(
        &server,
        &mut other,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"session/request",
            "params":{
                "commandId":"resolve-approval-from-other",
                "sessionId":session_id,
                "expectedSequence":5,
                "request":{"type":"resolveInteraction","threadId":thread_id,"turnId":started.turn_id,"requestId":"approval-owner-check","response":{
                    "type":"approval",
                    "response":{"decision":"approveOnce"}
                }}
            }
        }),
    );

    assert_eq!(rejected["error"]["message"], "AgentInteractionNotOwner");
    assert!(
        server
            .sessions()
            .threads()
            .read_thread(&thread_id)
            .unwrap()
            .turns[0]
            .pending_interaction
            .is_some()
    );
}

#[test]
fn jsonl_transport_writes_response_before_causal_updates() {
    let server = server();
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"},\"capabilities\":{}}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session/create\",\"params\":{\"commandId\":\"session\",\"title\":\"task\"}}\n"
    );
    let mut output = Vec::new();
    server
        .serve_jsonl(Cursor::new(input.as_bytes()), &mut output)
        .unwrap();
    let lines = String::from_utf8(output).unwrap();
    assert_eq!(lines.lines().count(), 2);
    assert!(lines.lines().all(|line| line.contains("\"id\":")));
}

#[test]
fn jsonl_transport_writes_notifications_without_another_request() {
    let server = Arc::new(server());
    let (mut client, host) = UnixStream::pair().unwrap();
    let output = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
    let served = Arc::clone(&server);
    let captured = Arc::clone(&output);
    let server_thread = thread::spawn(move || {
        served.serve_jsonl(
            std::io::BufReader::new(host),
            CapturingWriter { output: captured },
        )
    });

    client
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"},\"capabilities\":{}}}\n",
        )
        .unwrap();
    wait_for_captured(&output, "\"id\":1");
    server.publish_fs_changed_for_test(
        zeta_app_server_protocol::protocol::fs::FsChanged::PathsChanged {
            workspace_folder_id: None,
            paths: vec![std::path::PathBuf::from("src/lib.rs")],
        },
    );

    wait_for_captured(&output, "\"method\":\"fs/changed\"");
    client.shutdown(Shutdown::Write).unwrap();
    server_thread.join().unwrap().unwrap();
}

#[test]
fn slow_connection_writer_does_not_block_another_connection() {
    let server = Arc::new(server());
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"clientInfo\":{\"name\":\"slow\",\"version\":\"1\"},\"capabilities\":{}}}\n",
    );
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let served = Arc::clone(&server);
    let slow = thread::spawn(move || {
        served.serve_jsonl(
            Cursor::new(input.as_bytes()),
            BlockingWriter {
                started: Some(started_tx),
                release: release_rx,
                released: false,
            },
        )
    });
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let mut fast_connection = server.connection();
    initialize(&server, &mut fast_connection);

    release_tx.send(()).unwrap();
    slow.join().unwrap().unwrap();
    server.close_connection(fast_connection);
}

#[test]
fn account_rpc_projects_login_completion_without_credentials() {
    let login = Arc::new(LoginService::new(Arc::new(TestLoginDriver)).unwrap());
    let server = server().with_login_service(Arc::clone(&login));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let initial = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"account/read","params":{}}),
    );
    assert_eq!(initial["result"]["accounts"], serde_json::json!([]));
    let started = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"account/login/start",
            "params":{"method":{"type":"openAiChatGptBrowser"}}
        }),
    );
    let login_id = LoginId::new(started["result"]["loginId"].as_str().unwrap()).unwrap();
    login
        .complete(CompleteLogin {
            login_id,
            outcome: LoginCompletionOutcome::Succeeded {
                account: AccountSnapshot {
                    account: AccountRef {
                        provider: "openai-chatgpt".into(),
                        account_id: "acct_redacted".into(),
                    },
                    email: Some("person@example.test".into()),
                    display_name: None,
                    organization: None,
                    plan: Some("plus".into()),
                    status: AccountStatus::Ready,
                    credential_revision: 7,
                },
            },
        })
        .unwrap();

    let notifications = server.drain_notifications(&mut connection);
    assert!(notifications.iter().any(|value| {
        value.contains("\"method\":\"account/login/completed\"")
            && value.contains("\"accountId\":\"acct_redacted\"")
    }));
    assert!(
        notifications
            .iter()
            .any(|value| value.contains("\"method\":\"account/updated\""))
    );
    assert!(notifications.iter().all(|value| {
        !value.contains("accessToken")
            && !value.contains("refreshToken")
            && !value.contains("apiKey")
    }));

    let read = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"account/read","params":{}}),
    );
    assert_eq!(read["result"]["accounts"][0]["plan"], "plus");
    assert_eq!(read["result"]["accounts"][0]["credentialRevision"], 7);
}

#[test]
fn filesystem_rpc_lists_and_describes_workspace_paths() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-files-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "hello").unwrap();
    std::fs::write(root.join("paper.pdf"), b"%PDF-1.7\n").unwrap();
    let server = server().with_file_system(Arc::new(LocalFileSystem::new(
        WorkspaceRoot::open(&root).unwrap(),
    )));
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let listed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"fs/readDirectory",
            "params":{"path":"src"}
        }),
    );
    let metadata = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"fs/getMetadata",
            "params":{"path":"src/lib.rs"}
        }),
    );
    let contents = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"fs/readFile",
            "params":{"path":"src/lib.rs"}
        }),
    );
    let written = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"fs/writeFile",
            "params":{"path":"src/lib.rs","content":"updated","expectedRevision":contents["result"]["revision"]}
        }),
    );
    let created = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":6,
            "method":"fs/writeFile",
            "params":{"path":"src/new.rs","content":"new"}
        }),
    );
    let stale = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":7,
            "method":"fs/writeFile",
            "params":{"path":"src/lib.rs","content":"stale","expectedRevision":contents["result"]["revision"]}
        }),
    );
    let binary = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":8,
            "method":"fs/readBinaryFile",
            "params":{"path":"paper.pdf"}
        }),
    );
    let binary_data = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":9,
            "method":"resource/read",
            "params":{"resourceId":binary["result"]["resource"]["resourceId"],"offset":0,"maxBytes":262144}
        }),
    );

    assert_eq!(
        listed["result"]["entries"],
        serde_json::json!([{"name":"lib.rs","fileType":"file"}]),
    );
    assert_eq!(metadata["result"]["fileType"], "file");
    assert_eq!(metadata["result"]["sizeBytes"], 5);
    assert_eq!(contents["result"]["content"], "hello");
    assert!(contents["result"]["revision"].is_string());
    assert_eq!(written["result"]["metadata"]["sizeBytes"], 7);
    assert!(written["result"]["revision"].is_string());
    assert_eq!(created["result"]["metadata"]["sizeBytes"], 3);
    assert_eq!(stale["error"]["code"], -32042);
    assert_eq!(stale["error"]["message"], "FileSystemRevisionConflict");
    assert_eq!(
        binary["result"]["resource"]["mimeType"],
        "application/octet-stream"
    );
    assert_eq!(binary["result"]["resource"]["size"], 9);
    assert!(binary["result"]["revision"].is_string());
    assert_eq!(binary_data["result"]["dataBase64"], "JVBERi0xLjcK");
    assert_eq!(binary_data["result"]["decodedLength"], 9);
    assert_eq!(binary_data["result"]["eof"], true);
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        "updated"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/new.rs")).unwrap(),
        "new"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn filesystem_watcher_publishes_only_workspace_relative_paths() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-files-watch-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    let workspace = WorkspaceRoot::open(&root).unwrap();
    let server = server()
        .with_file_system(Arc::new(LocalFileSystem::new(workspace.clone())))
        .with_file_system_watcher(workspace)
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);

    let mut observed = None;
    for attempt in 0..60 {
        std::fs::write(root.join("changed.txt"), format!("external {attempt}\n")).unwrap();
        thread::sleep(Duration::from_millis(50));
        for raw in server.drain_notifications(&mut connection) {
            let notification: serde_json::Value = serde_json::from_str(&raw).unwrap();
            if notification["method"] == "fs/changed" {
                observed = Some(notification);
                break;
            }
        }
        if observed.is_some() {
            break;
        }
    }

    let observed = observed.expect("filesystem watcher should publish an invalidation hint");
    assert_eq!(observed["params"]["type"], "pathsChanged");
    let paths = observed["params"]["paths"].as_array().unwrap();
    assert!(paths.iter().any(|path| path == "changed.txt"));
    assert!(
        paths
            .iter()
            .all(|path| path.as_str().is_some_and(|path| !path.starts_with('/')))
    );
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_status_rpc_projects_workspace_repository_state() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-git-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    run_git(&root, &["init"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.test"]);
    std::fs::write(workspace.join("tracked.txt"), "first\n").unwrap();
    std::fs::write(root.join("outside.txt"), "outside\n").unwrap();
    run_git(&root, &["add", "workspace/tracked.txt", "outside.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    std::fs::write(workspace.join("tracked.txt"), "changed\n").unwrap();
    std::fs::write(workspace.join("new.txt"), "new\n").unwrap();
    std::fs::write(root.join("outside.txt"), "outside changed\n").unwrap();

    let server = server()
        .with_git_root(trusted_workspace(
            &workspace,
            WorkspaceCapability::MutateRepository,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let repositories = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"git/repositories",
            "params":{}
        }),
    );
    let repository_id = repositories["result"]["repositories"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(repositories["result"]["repositories"][0]["path"], "");
    let response = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"git/status",
            "params":{"repositoryId":repository_id}
        }),
    );
    assert_eq!(response["result"]["repositoryId"], repository_id);
    assert_eq!(response["result"]["head"]["type"], "branch");
    assert_eq!(response["result"]["changes"].as_array().unwrap().len(), 2);
    assert!(
        response["result"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| {
                change["path"] == "tracked.txt" && change["worktreeStatus"] == "modified"
            })
    );
    assert!(
        response["result"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| {
                change["path"] == "new.txt" && change["worktreeStatus"] == "untracked"
            })
    );

    let staged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"git/stage",
            "params":{"repositoryId":repository_id,"paths":["new.txt"]}
        }),
    );
    assert!(
        staged["result"]["status"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| { change["path"] == "new.txt" && change["indexStatus"] == "added" })
    );
    let unstaged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"git/unstage",
            "params":{"repositoryId":repository_id,"paths":["new.txt"]}
        }),
    );
    assert!(
        unstaged["result"]["status"]["changes"].is_array(),
        "unexpected unstage response: {unstaged}"
    );
    assert!(
        unstaged["result"]["status"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| {
                change["path"] == "new.txt" && change["worktreeStatus"] == "untracked"
            })
    );
    let invalid = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":6,
            "method":"git/stage",
            "params":{"repositoryId":repository_id,"paths":["../outside.txt"]}
        }),
    );
    assert_eq!(invalid["error"]["message"], "InvalidParams");
    let _ = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":7,
            "method":"git/stage",
            "params":{"repositoryId":repository_id,"paths":["new.txt"]}
        }),
    );
    let committed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":8,
            "method":"git/commit",
            "params":{"repositoryId":repository_id,"message":"add workspace file"}
        }),
    );
    assert!(!committed["result"]["objectId"].as_str().unwrap().is_empty());
    let discarded = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":9,
            "method":"git/discardWorktree",
            "params":{"repositoryId":repository_id,"paths":["tracked.txt"]}
        }),
    );
    assert!(
        discarded["result"]["status"]["changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn restricted_workspace_exposes_git_status_but_rejects_mutations() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-restricted-git-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init", "--initial-branch=main"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.test"]);
    std::fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    std::fs::write(root.join("tracked.txt"), "changed\n").unwrap();

    let server = server()
        .with_local_workspace_host(None, crate::server::WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();
    assert_eq!(
        server.switch_local_workspace_root(root.clone()),
        Ok(dunce::canonicalize(&root).unwrap())
    );
    let mut connection = server.connection();
    let initialized = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"clientInfo":{"name":"test","version":"1"},"capabilities":{}}
        }),
    );
    assert_eq!(initialized["result"]["capabilities"]["git"], true);

    let status = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"git/status","params":{}}),
    );
    assert_eq!(status["result"]["changes"].as_array().unwrap().len(), 1);
    assert_eq!(status["result"]["changes"][0]["path"], "tracked.txt");

    let staged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"git/stage",
            "params":{"paths":["tracked.txt"]}
        }),
    );
    assert_eq!(staged["error"]["message"], "GitUnavailable");

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_remote_rpcs_fetch_pull_and_push_against_a_local_bare_remote() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-git-remote-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(
        &root,
        &["init", "--bare", "--initial-branch=main", "origin.git"],
    );
    run_git(&root, &["clone", "origin.git", "workspace"]);
    let workspace = root.join("workspace");
    run_git(&workspace, &["config", "user.name", "Zeta Test"]);
    run_git(&workspace, &["config", "user.email", "zeta@example.test"]);
    run_git(&workspace, &["config", "core.autocrlf", "false"]);
    std::fs::write(workspace.join("shared.txt"), "initial\n").unwrap();
    run_git(&workspace, &["add", "shared.txt"]);
    run_git(&workspace, &["commit", "-m", "initial"]);
    run_git(&workspace, &["push", "--set-upstream", "origin", "main"]);
    run_git(&root, &["clone", "origin.git", "peer"]);
    let peer = root.join("peer");
    run_git(&peer, &["config", "user.name", "Zeta Test"]);
    run_git(&peer, &["config", "user.email", "zeta@example.test"]);
    run_git(&peer, &["config", "core.autocrlf", "false"]);

    let server = server()
        .with_git_root(trusted_workspace(
            &workspace,
            WorkspaceCapability::MutateRepository,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    std::fs::write(peer.join("shared.txt"), "from peer\n").unwrap();
    run_git(&peer, &["add", "shared.txt"]);
    run_git(&peer, &["commit", "-m", "peer update"]);
    run_git(&peer, &["push"]);

    let fetched = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"git/fetch","params":{}}),
    );
    assert!(fetched.get("error").is_none(), "{fetched}");
    let pulled = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"git/pull","params":{}}),
    );
    assert!(pulled.get("error").is_none(), "{pulled}");
    assert_eq!(
        std::fs::read_to_string(workspace.join("shared.txt")).unwrap(),
        "from peer\n"
    );

    std::fs::write(workspace.join("local.txt"), "from app server\n").unwrap();
    let staged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":4,
            "method":"git/stage",
            "params":{"paths":["local.txt"]}
        }),
    );
    assert!(staged.get("error").is_none(), "{staged}");
    let committed = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":5,
            "method":"git/commit",
            "params":{"message":"app server update"}
        }),
    );
    assert!(committed.get("error").is_none(), "{committed}");
    let pushed = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":6,"method":"git/push","params":{}}),
    );
    assert!(pushed.get("error").is_none(), "{pushed}");
    run_git(&peer, &["pull", "--ff-only"]);
    assert_eq!(
        std::fs::read_to_string(peer.join("local.txt")).unwrap(),
        "from app server\n"
    );
    run_git(
        &workspace,
        &[
            "remote",
            "set-url",
            "origin",
            "https://github.com/example/zeta.git",
        ],
    );
    let graph = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":7,"method":"git/graph","params":{"limit":50}}),
    );
    assert!(graph.get("error").is_none(), "{graph}");
    assert!(
        graph["result"]["references"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reference| {
                reference["name"] == "origin/main"
                    && reference["kind"] == "remoteBranch"
                    && reference["remoteName"] == "origin"
            })
    );
    assert_eq!(graph["result"]["remotes"][0]["name"], "origin");
    assert_eq!(
        graph["result"]["remotes"][0]["identity"]["provider"],
        "github"
    );
    let graph_page = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":8,"method":"git/graph","params":{"limit":1}}),
    );
    assert!(graph_page.get("error").is_none(), "{graph_page}");
    assert_eq!(graph_page["result"]["commits"].as_array().unwrap().len(), 1);
    assert_eq!(graph_page["result"]["hasMore"], true);
    let cursor = graph_page["result"]["nextCursor"]
        .as_str()
        .expect("graph continuation cursor");
    let graph_tail = call(
        &server,
        &mut connection,
        serde_json::json!({"jsonrpc":"2.0","id":9,"method":"git/graph","params":{"limit":50,"cursor":cursor}}),
    );
    assert!(graph_tail.get("error").is_none(), "{graph_tail}");
    assert_eq!(graph_tail["result"]["hasMore"], false);
    assert!(graph_tail["result"]["nextCursor"].is_null());

    let committed_object_id = committed["result"]["objectId"]
        .as_str()
        .expect("committed object id");
    let commit_changes = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":10,
            "method":"git/commitChanges",
            "params":{"objectId":committed_object_id}
        }),
    );
    assert!(commit_changes.get("error").is_none(), "{commit_changes}");
    assert!(commit_changes["result"]["parentObjectId"].is_string());
    assert!(
        commit_changes["result"]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|change| change["path"] == "local.txt" && change["status"] == "added")
    );
    let commit_file = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":11,
            "method":"git/commitFile",
            "params":{"objectId":committed_object_id,"path":"local.txt"}
        }),
    );
    assert!(commit_file.get("error").is_none(), "{commit_file}");
    assert_eq!(commit_file["result"]["original"]["kind"], "missing");
    assert_eq!(commit_file["result"]["modified"]["kind"], "text");
    assert_eq!(
        commit_file["result"]["modified"]["text"],
        "from app server\n"
    );

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_change_file_rpc_preserves_head_index_and_worktree_sides() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-git-change-file-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.test"]);
    std::fs::write(root.join("tracked.txt"), "from head\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    std::fs::write(root.join("tracked.txt"), "from index\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    std::fs::write(root.join("tracked.txt"), "from working tree\n").unwrap();

    let server = server()
        .with_git_root(trusted_workspace(
            &root,
            WorkspaceCapability::InspectRepository,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let staged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"git/changeFile",
            "params":{"path":"tracked.txt","comparison":"staged"}
        }),
    );
    assert!(staged.get("error").is_none(), "{staged}");
    assert_eq!(staged["result"]["original"]["text"], "from head\n");
    assert_eq!(staged["result"]["modified"]["text"], "from index\n");

    let unstaged = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":3,
            "method":"git/changeFile",
            "params":{"path":"tracked.txt","comparison":"unstaged"}
        }),
    );
    assert!(unstaged.get("error").is_none(), "{unstaged}");
    assert_eq!(unstaged["result"]["original"]["text"], "from index\n");
    assert_eq!(
        unstaged["result"]["modified"]["text"],
        "from working tree\n"
    );

    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_watcher_publishes_external_workspace_changes() {
    let root = std::env::temp_dir().join(format!(
        "zeta-app-server-git-watch-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    run_git(&root, &["init"]);
    run_git(&root, &["config", "user.name", "Zeta Test"]);
    run_git(&root, &["config", "user.email", "zeta@example.test"]);
    std::fs::write(root.join("tracked.txt"), "initial\n").unwrap();
    run_git(&root, &["add", "tracked.txt"]);
    run_git(&root, &["commit", "-m", "initial"]);
    let server = server()
        .with_git_root(trusted_workspace(
            &root,
            WorkspaceCapability::MutateRepository,
        ))
        .unwrap();
    let mut connection = server.connection();
    initialize(&server, &mut connection);
    let initial = call(
        &server,
        &mut connection,
        serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"git/status",
            "params":{}
        }),
    );
    let initial_revision = initial["result"]["revision"].as_u64().unwrap();
    server.drain_notifications(&mut connection);

    let mut observed = None;
    for attempt in 0..60 {
        std::fs::write(root.join("tracked.txt"), format!("external {attempt}\n")).unwrap();
        thread::sleep(Duration::from_millis(50));
        for raw in server.drain_notifications(&mut connection) {
            let notification: serde_json::Value = serde_json::from_str(&raw).unwrap();
            if notification["method"] == "git/statusChanged"
                && notification["params"]["status"]["revision"]
                    .as_u64()
                    .is_some_and(|revision| revision > initial_revision)
            {
                observed = Some(notification);
                break;
            }
        }
        if observed.is_some() {
            break;
        }
    }

    let observed = observed.expect("Git watcher should publish a changed workspace status");
    assert_eq!(
        observed["params"]["status"]["changes"][0]["path"],
        "tracked.txt"
    );
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn trusted_workspace(root: &std::path::Path, capability: WorkspaceCapability) -> TrustedWorkspace {
    TrustedWorkspace::require(
        WorkspaceRoot::open(root).unwrap(),
        WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::HostConfiguration),
        capability,
    )
    .unwrap()
}
