use super::*;
use crate::local::ProviderModelService;
use crate::local_tools::LocalToolComposition;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use zeta_async_utils::CancellationToken;
use zeta_config::{
    ConfigCommandRequest, ConfigRevision, ConfigStore, UserConfigCommand, WorkspaceTrustSetting,
};
use zeta_core::{
    CoreError, CreateSessionRequest, CreateSessionThreadRequest, InMemorySessionStore,
    InMemoryThreadStore, NoTools, PolicyService, SequenceExpectation, SessionCoordinator,
    StartTurnRequest, ThreadController,
};
use zeta_model_provider::EchoModel;
use zeta_policy::{ActionReviewRequest, ExecutionDecision};
use zeta_protocol::{CommandId, UserInput};
use zeta_shell_command::RipgrepExecutable;
use zeta_workspace::WorkspaceTrustSource;

#[test]
fn workspace_runtime_replaces_authority_without_replacing_connection_owned_services() {
    let first = TestWorkspace::new("first", "first.txt");
    let second = TestWorkspace::new("second", "second.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();

    server
        .commit_trusted_workspace_runtime(first.authorization(), test_local_tools(), host)
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
        .commit_trusted_workspace_runtime(second.authorization(), test_local_tools(), host)
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
fn restricted_workspace_installs_only_non_executable_services() {
    let workspace = TestWorkspace::new("restricted", "readable.txt");
    let server = server()
        .with_local_workspace_host(None, WorkspaceSwitchTrustPolicy::Restricted)
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(workspace.root().canonical_path().to_path_buf())
    );
    assert!(server.file_system_service().is_ok());
    assert!(server.git_runtime_service().is_err());
    assert!(server.workspace_search_service().is_err());
    assert!(server.terminal_service().is_err());
}

#[test]
fn user_config_trust_is_resolved_for_each_client_requested_root() {
    let workspace = TestWorkspace::new("config-trust", "readable.txt");
    let root = WorkspaceRoot::open(&workspace.path).unwrap();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let server = server()
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.file_system_service().is_ok());
    assert!(server.terminal_service().is_err());

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-config-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
            },
        })
        .unwrap();

    assert_eq!(
        server.switch_local_workspace_root(workspace.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.git_runtime_service().is_ok());
    assert!(server.workspace_search_service().is_ok());
    assert!(server.terminal_service().is_ok());
}

#[test]
fn user_config_revocation_removes_executable_services_but_keeps_file_access() {
    let workspace = TestWorkspace::new("config-revocation", "readable.txt");
    let root = workspace.root();
    let config = Arc::new(ConfigStore::open(workspace.path.join("trust.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("trust-revoked-workspace").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Trusted,
            },
        })
        .unwrap();
    let server = server()
        .with_local_workspace_host(
            None,
            WorkspaceSwitchTrustPolicy::UserConfig(Arc::clone(&config)),
        )
        .unwrap();
    server
        .switch_local_workspace_root(workspace.path.clone())
        .unwrap();
    assert!(server.terminal_service().is_ok());
    let session = server
        .sessions
        .create_session(CreateSessionRequest {
            command_id: CommandId::new("create-revocation-session").unwrap(),
            title: "revocation".into(),
            model: None,
        })
        .unwrap();
    let thread = server
        .sessions
        .create_thread(CreateSessionThreadRequest {
            command_id: CommandId::new("create-revocation-thread").unwrap(),
            session_id: session.session_id,
            expected_sequence: SequenceExpectation::Exact(session.sequence),
            title: "revocation".into(),
        })
        .unwrap();
    let turn = server
        .sessions
        .threads()
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                command_id: CommandId::new("start-revocation-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                input: vec![UserInput::Text {
                    text: "must be interrupted".into(),
                }],
            },
        )
        .unwrap();

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-revoked-workspace").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetWorkspaceTrust {
                workspace: root.trust_id(),
                setting: WorkspaceTrustSetting::Restricted,
            },
        })
        .unwrap();
    server
        .workspace_runtime_control()
        .unwrap()
        .reconcile_user_trust(&config.read_snapshot().unwrap().values)
        .unwrap();

    let Ok(file_system) = server.file_system_service() else {
        panic!("restricted filesystem should remain installed after trust revocation");
    };
    assert_eq!(
        file_system
            .read_file(Path::new("readable.txt"), 1024)
            .unwrap(),
        b"config-revocation"
    );
    assert!(server.git_runtime_service().is_err());
    assert!(server.workspace_search_service().is_err());
    assert!(server.terminal_service().is_err());
    assert_eq!(
        server
            .sessions
            .threads()
            .read_thread(&thread.thread_id)
            .unwrap()
            .turns
            .iter()
            .find(|candidate| candidate.turn_id == turn.turn_id)
            .unwrap()
            .status,
        TurnStatus::Interrupted
    );
    assert!(
        server
            .local_workspace_host
            .as_ref()
            .unwrap()
            .tools
            .reloadable
            .tools()
            .definitions()
            .is_empty()
    );
}

#[test]
fn active_turn_blocks_workspace_switch_without_changing_authority() {
    let first = TestWorkspace::new("busy-first", "first.txt");
    let second = TestWorkspace::new("busy-second", "second.txt");
    let server = server()
        .with_local_workspace_host(None, host_trust())
        .unwrap();
    let host = server.local_workspace_host.as_ref().unwrap();
    server
        .commit_trusted_workspace_runtime(first.authorization(), test_local_tools(), host)
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

fn host_trust() -> WorkspaceSwitchTrustPolicy {
    WorkspaceSwitchTrustPolicy::TrustHostSelectedRoots(WorkspaceTrustSource::HostConfiguration)
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

    fn authorization(&self) -> WorkspaceAuthorization {
        WorkspaceAuthorization::new(
            self.root(),
            WorkspaceTrustDecision::Trusted(WorkspaceTrustSource::HostConfiguration),
        )
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
