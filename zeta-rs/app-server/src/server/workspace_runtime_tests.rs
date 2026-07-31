use super::*;
use crate::local::ProviderModelService;
use crate::local_tools::LocalToolComposition;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use zeta_async_utils::CancellationToken;
use zeta_core::{
    CoreError, CreateSessionRequest, CreateSessionThreadRequest, InMemorySessionStore,
    InMemoryThreadStore, NoTools, PolicyService, SequenceExpectation, SessionCoordinator,
    StartTurnRequest, ThreadController,
};
use zeta_model_provider::EchoModel;
use zeta_policy::{ActionReviewRequest, ExecutionDecision};
use zeta_protocol::{CommandId, UserInput};
use zeta_shell_command::RipgrepExecutable;

#[test]
fn workspace_runtime_replaces_authority_without_replacing_connection_owned_services() {
    let first = TestWorkspace::new("first", "first.txt");
    let second = TestWorkspace::new("second", "second.txt");
    let server = server().with_local_workspace_host(None).unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();

    server
        .commit_workspace_runtime(first.root(), test_local_tools(), host)
        .unwrap();
    let Ok(first_file_system) = server.file_system_service() else {
        panic!("first file system should be installed");
    };
    assert_eq!(
        first_file_system
            .read_file(Path::new("first.txt"), 1024)
            .unwrap(),
        b"first"
    );
    let Ok(first_search) = server.workspace_search_service() else {
        panic!("first search service should be installed");
    };
    let Ok(first_terminals) = server.terminal_service() else {
        panic!("first terminal service should be installed");
    };
    let Ok(first_git) = server.git_runtime_service() else {
        panic!("first Git runtime should be installed");
    };

    server
        .commit_workspace_runtime(second.root(), test_local_tools(), host)
        .unwrap();
    let Ok(second_file_system) = server.file_system_service() else {
        panic!("second file system should be installed");
    };
    assert_eq!(
        second_file_system
            .read_file(Path::new("second.txt"), 1024)
            .unwrap(),
        b"second"
    );
    assert!(
        second_file_system
            .read_file(Path::new("first.txt"), 1024)
            .is_err()
    );
    let Ok(second_search) = server.workspace_search_service() else {
        panic!("second search service should be installed");
    };
    let Ok(second_terminals) = server.terminal_service() else {
        panic!("second terminal service should be installed");
    };
    let Ok(second_git) = server.git_runtime_service() else {
        panic!("second Git runtime should be installed");
    };

    assert!(Arc::ptr_eq(&first_search, &second_search));
    assert!(Arc::ptr_eq(&first_terminals, &second_terminals));
    assert!(!Arc::ptr_eq(&first_git, &second_git));
}

#[test]
fn workspace_switch_rpc_requires_a_local_workspace_host() {
    let server = server();
    let mut connection = server.connection();
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {"name": "test", "version": "1"},
            "capabilities": {}
        }
    });
    server.handle_json(&mut connection, &initialized.to_string());
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "workspace/switch",
        "params": {"root": std::env::current_dir().unwrap()}
    });
    let response: serde_json::Value =
        serde_json::from_str(&server.handle_json(&mut connection, &request.to_string())).unwrap();

    assert_eq!(response["error"]["message"], "WorkspaceSwitchUnavailable");
}

#[test]
fn active_turn_blocks_workspace_switch_without_changing_authority() {
    let first = TestWorkspace::new("busy-first", "first.txt");
    let second = TestWorkspace::new("busy-second", "second.txt");
    let server = server().with_local_workspace_host(None).unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();
    server
        .commit_workspace_runtime(first.root(), test_local_tools(), host)
        .unwrap();
    let session = server
        .sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-session").unwrap(),
            title: "session".into(),
            model: None,
        })
        .unwrap();
    let thread = server
        .sessions
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-thread").unwrap(),
            session_id: session.session_id,
            expected_sequence: SequenceExpectation::Exact(session.sequence),
            title: "thread".into(),
        })
        .unwrap();
    server
        .sessions
        .threads()
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                input: vec![UserInput::Text {
                    text: "stay in the first Workspace".into(),
                }],
            },
        )
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(second.path.clone()),
        Err(WorkspaceRuntimeError::Busy)
    );
    let Ok(file_system) = server.file_system_service() else {
        panic!("first Workspace file system should remain installed");
    };
    assert_eq!(
        file_system.read_file(Path::new("first.txt"), 1024).unwrap(),
        b"busy-first"
    );
    assert!(
        file_system
            .read_file(Path::new("second.txt"), 1024)
            .is_err()
    );
}

fn server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    let sessions = Arc::new(SessionCoordinator::with_store(
        Arc::new(InMemorySessionStore::default()),
        threads,
    ));
    AppServer::new(
        sessions,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
}

fn test_local_tools() -> LocalToolComposition {
    LocalToolComposition {
        tools: Arc::new(NoTools),
        policy: Arc::new(RejectPolicy),
        ripgrep: RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap(),
    }
}

struct RejectPolicy;

impl PolicyService for RejectPolicy {
    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Err(CoreError::Policy("test policy rejects every action".into()))
    }
}

static NEXT_WORKSPACE: AtomicUsize = AtomicUsize::new(0);

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new(label: &str, file: &str) -> Self {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("workspace-runtime-tests")
            .join(format!("{}-{label}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(file), label).unwrap();
        Self { path }
    }

    fn root(&self) -> WorkspaceRoot {
        WorkspaceRoot::open(&self.path).unwrap()
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
