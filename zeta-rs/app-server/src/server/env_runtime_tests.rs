use super::*;
use crate::local::ProviderModelService;
use crate::local_tools::LocalToolComposition;
use std::num::NonZeroU64;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use zeta_action_policy::ActionReviewRequest;
use zeta_action_policy::ExecutionDecision;
use zeta_async_utils::CancellationToken;
use zeta_cloud_codebase::CloudCodebaseCapabilities;
use zeta_cloud_codebase::CloudCodebaseDeletionSupport;
use zeta_cloud_codebase::CloudCodebaseDestination;
use zeta_cloud_codebase::CloudCodebaseGrant;
use zeta_cloud_codebase::CloudCodebaseGrantId;
use zeta_cloud_codebase::CloudCodebaseId;
use zeta_cloud_codebase::CloudCodebaseProvider;
use zeta_cloud_codebase::CloudCodebaseProviderError;
use zeta_cloud_codebase::CloudCodebaseProviderId;
use zeta_cloud_codebase::CloudCodebaseProviderRegistry;
use zeta_cloud_codebase::CloudCodebasePublication;
use zeta_cloud_codebase::CloudCodebasePublicationRequest;
use zeta_cloud_codebase::CloudCodebaseSelection;
use zeta_cloud_codebase::CloudCodebaseState;
use zeta_config::ConfigCommandRequest;
use zeta_config::ConfigRevision;
use zeta_config::ConfigStore;
use zeta_config::ToolSearchConfig;
use zeta_config::ToolSearchModeConfig;
use zeta_config::UserConfigCommand;
use zeta_core::ActionPolicyService;
use zeta_core::CoreError;
use zeta_core::InMemoryThreadStore;
use zeta_core::NoTools;
use zeta_core::SequenceExpectation;
use zeta_core::StartThreadRequest;
use zeta_core::StartTurnRequest;
use zeta_core::ThreadController;
use zeta_file_access::GrantSource;
use zeta_model_provider::EchoModel;
use zeta_model_provider::EmbeddingInvoker;
use zeta_model_provider::EmbeddingRequest;
use zeta_model_provider::EmbeddingResponse;
use zeta_model_provider::EmbeddingVector;
use zeta_model_provider::ModelProviderError;
use zeta_model_provider_config::ModelProviderConfig;
use zeta_protocol::CommandId;
use zeta_protocol::ModelId;
use zeta_protocol::ModelRef;
use zeta_protocol::ProviderId;
use zeta_protocol::UserInput;
use zeta_shell_command::RipgrepExecutable;

struct PermissionBoundSemanticEmbedding;

impl EmbeddingInvoker for PermissionBoundSemanticEmbedding {
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
fn unavailable_hybrid_tool_search_remains_gated_and_reports_status() {
    let tools = EnvToolPorts::new(
        ToolPort::host(Arc::new(NoTools), Arc::new(RejectPolicy)),
        None,
        None,
        None,
        &ToolSearchConfig::default(),
        &Default::default(),
        None,
    )
    .unwrap();
    let before = tools.state.lock().unwrap().registry_generation;
    let provider = ProviderId::new("ollama").unwrap();
    let model = ModelRef::new(provider.clone(), ModelId::new("nomic-embed-text").unwrap());
    let config = ToolSearchConfig {
        mode: ToolSearchModeConfig::HybridEmbedding,
        embedding_model: Some(model.clone()),
    };
    let providers =
        std::collections::BTreeMap::from([(provider.clone(), ModelProviderConfig::new(provider))]);

    tools
        .reconcile_user_config(None, &config, &providers)
        .unwrap();

    assert!(tools.state.lock().unwrap().registry_generation > before);
    assert_eq!(
        tools.tool_search_status(),
        ToolSearchEmbeddingStatus::Unavailable {
            model: Some(model),
            reason: "this App Server host does not provide semantic model invocation".into(),
        }
    );
}

#[test]
fn env_runtime_replaces_authority_without_replacing_connection_owned_services() {
    let first = TestDir::new("first", "first.txt");
    let second = TestDir::new("second", "second.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();

    server
        .commit_full_env_runtime(first.authorization(), test_local_tools(), host)
        .unwrap();
    let tool_names = host
        .tools
        .reloadable
        .tools()
        .definitions()
        .into_iter()
        .map(|definition| definition.name.to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(tool_names.contains(crate::server::multi_agent_tools::SPAWN_AGENT_TOOL_NAME));
    assert!(tool_names.contains(crate::server::multi_agent_tools::SEND_AGENT_MESSAGE_TOOL_NAME));
    assert!(tool_names.contains(crate::server::multi_agent_tools::WAIT_AGENT_TOOL_NAME));
    assert!(!tool_names.contains("browser_open"));
    host.tools.replace_host_available(true).unwrap();
    assert!(
        host.tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    let Ok(first_file_system) = server.file_system_service_for(None) else {
        panic!("first file system should be installed");
    };
    assert_eq!(
        first_file_system
            .read_file(Path::new("first.txt"), 1024)
            .unwrap(),
        b"first"
    );
    let Ok(first_search) = server.content_search_service_for(None) else {
        panic!("first search service should be installed");
    };
    let Ok(first_terminals) = server.terminal_service() else {
        panic!("first terminal service should be installed");
    };
    let Ok(first_git) = server.git_runtime_service() else {
        panic!("first Git runtime should be installed");
    };

    server
        .commit_full_env_runtime(second.authorization(), test_local_tools(), host)
        .unwrap();
    let Ok(second_file_system) = server.file_system_service_for(None) else {
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
    let Ok(second_search) = server.content_search_service_for(None) else {
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
fn local_env_host_rejects_an_unconfigured_state_mode() {
    let mut server = server();
    server.env_state = EnvStateMode::Unconfigured;

    let error = match server.with_local_env_host(None, host_policy()) {
        Ok(_) => panic!("unconfigured Directory state should be rejected"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "local Directory host requires an explicit Directory state mode"
    );
}

#[test]
fn env_cwd_set_rpc_requires_a_local_env_host() {
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
        "method": "env/cwd/set",
        "params": {
            "cwd": std::env::current_dir().unwrap()
        }
    });
    let response: serde_json::Value =
        serde_json::from_str(&server.handle_json(&mut connection, &request.to_string())).unwrap();

    assert_eq!(response["error"]["message"], "EnvCwdSetUnavailable");
}

#[test]
fn env_dirs_set_routes_services_by_stable_folder_id() {
    let first = TestDir::new("multi-root-first", "first.txt");
    let second = TestDir::new("multi-root-second", "second.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let mut connection = server.product_host_connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "desktop", "version": "1"},
                "capabilities": {"dirPermissionsHost": {"version": 1}}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "env/dirs/set",
                "params": {"dirs": [
                    {"id": "first", "path": first.path, "grant": {"type": "host", "permissions": ["readFiles", "writeFiles", "executeCommands", "watchFiles", "browseFiles", "searchFiles", "loadInstructions", "loadConfig", "discoverSkills", "discoverMcp", "useLanguageServices", "discoverHooks", "discoverPlugins", "inspectRepository", "mutateRepository"]}},
                    {"id": "second", "path": second.path, "grant": {"type": "host", "permissions": ["readFiles", "writeFiles", "executeCommands", "watchFiles", "browseFiles", "searchFiles", "loadInstructions", "loadConfig", "discoverSkills", "discoverMcp", "useLanguageServices", "discoverHooks", "discoverPlugins", "inspectRepository", "mutateRepository"]}}
                ]}
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(response["result"]["dirs"][0]["id"], "first");
    assert_eq!(response["result"]["dirs"][1]["id"], "second");
    let Ok(first_files) = server.file_system_service_for(Some("first")) else {
        panic!("first folder file service should be available");
    };
    let Ok(second_files) = server.file_system_service_for(Some("second")) else {
        panic!("second folder file service should be available");
    };
    assert_eq!(
        first_files.read_file(Path::new("first.txt"), 1024).unwrap(),
        b"multi-root-first"
    );
    assert_eq!(
        second_files
            .read_file(Path::new("second.txt"), 1024)
            .unwrap(),
        b"multi-root-second"
    );
    assert!(server.file_system_service_for(Some("missing")).is_err());
    let Ok(first_terminal) = server.terminal_service_for(Some("first")) else {
        panic!("first folder terminal service should be available");
    };
    let Ok(second_terminal) = server.terminal_service_for(Some("second")) else {
        panic!("second folder terminal service should be available");
    };
    assert!(!Arc::ptr_eq(&first_terminal, &second_terminal));
}

#[test]
fn dirs_are_session_scoped_and_removable() {
    let primary = TestDir::new("add-dir-primary", "primary.txt");
    let session_dir = TestDir::new("add-dir-extra", "extra.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(primary.authorization(), test_local_tools(), host)
        .unwrap();
    let first = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-add-dir-session").unwrap(),
            title: "first".into(),
        })
        .unwrap();
    let second = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-other-add-dir-session").unwrap(),
            title: "second".into(),
        })
        .unwrap();

    let (mutation, directories) = server
        .add_session_dir(
            &first.session_id,
            session_dir.path.clone(),
            host_dir_permissions(),
        )
        .unwrap();

    assert_eq!(mutation, Mutation::AddedDir);
    assert_eq!(directories.revision, 1);
    assert_eq!(directories.dirs.len(), 1);
    assert_eq!(
        directories.dirs[0].path,
        session_dir.root().canonical_path()
    );
    assert_eq!(
        server
            .list_session_dirs(&second.session_id)
            .unwrap()
            .dirs
            .len(),
        0
    );
    let (mutation, directories) = server
        .remove_session_dir(&first.session_id, &session_dir.path)
        .unwrap();
    assert_eq!(mutation, Mutation::RemovedDir);
    assert!(directories.dirs.is_empty());
}

#[test]
fn cwd_directory_can_be_added_explicitly() {
    let primary = TestDir::new("add-dir-duplicate-primary", "primary.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(primary.authorization(), test_local_tools(), host)
        .unwrap();
    let session = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-primary-add-dir-session").unwrap(),
            title: "session".into(),
        })
        .unwrap();

    let (mutation, snapshot) = server
        .add_session_dir(
            &session.session_id,
            primary.path.clone(),
            host_dir_permissions(),
        )
        .unwrap();

    assert_eq!(mutation, Mutation::AddedDir);
    assert_eq!(snapshot.dirs[0].path, primary.root().canonical_path());
}

#[test]
fn dir_mutation_requires_a_dir_permissions_host_connection() {
    let primary = TestDir::new("add-dir-capability-primary", "primary.txt");
    let session_dir = TestDir::new("add-dir-capability-extra", "extra.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(primary.authorization(), test_local_tools(), host)
        .unwrap();
    let session = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-capability-add-dir-session").unwrap(),
            title: "session".into(),
        })
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "renderer", "version": "1"},
                "capabilities": {}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "session/dirs/add",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert_eq!(response["error"]["message"], "PermissionRequired");
}

#[test]
fn dir_permissions_are_revision_bound_and_filter_capability_snapshots() {
    let primary = TestDir::new("add-dir-permission-primary", "primary.txt");
    let session_dir = TestDir::new("add-dir-permission-extra", "extra.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(primary.authorization(), test_local_tools(), host)
        .unwrap();
    let session = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-permission-add-dir-session").unwrap(),
            title: "session".into(),
        })
        .unwrap();
    let mut connection = server.product_host_connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "zeta-code", "version": "1"},
                "capabilities": {"dirPermissionsHost": {"version": 1}}
            }
        })
        .to_string(),
    );
    let added: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "session/dirs/add",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "permissions": ["readFiles", "writeFiles"]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(added["result"]["revision"], 1);

    let updated: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "session/dirs/permissions/set",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "expectedRevision": 1,
                    "permissions": ["readFiles"]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(updated["result"]["mutation"], "updated");
    assert_eq!(updated["result"]["revision"], 2);
    assert_eq!(
        updated["result"]["dirs"][0]["permissions"],
        serde_json::json!(["readFiles"])
    );
    let access = server
        .env_runtime
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .dir_grants
        .clone();
    assert!(
        access
            .snapshot_for(&session.session_id, Permission::MutateRepository)
            .unwrap()
            .unwrap()
            .authorizations()
            .is_empty()
    );

    let activated: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "session/dirs/permissions/set",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "expectedRevision": 2,
                    "permissions": ["readFiles", "executeCommands", "watchFiles", "loadInstructions"]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(activated["result"]["revision"], 3);
    {
        let runtime = server
            .env_runtime
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(runtime.session_dir_watchers.len(), 1);
        assert_eq!(
            runtime
                .dir_grants
                .snapshot_for(&session.session_id, Permission::ExecuteCommands)
                .unwrap()
                .unwrap()
                .authorizations()
                .len(),
            1
        );
    }
    let terminal: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "terminal/createInSessionDirectory",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "rows": 24,
                    "cols": 80,
                    "profile": {"type": "default"},
                    "lifecycle": {"type": "connectionOwned"}
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    let terminal_id = terminal["result"]["terminalId"]
        .as_str()
        .expect("authorized session-dir terminal should start")
        .to_owned();

    let deactivated: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "session/dirs/permissions/set",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "expectedRevision": 3,
                    "permissions": ["readFiles"]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(deactivated["result"]["revision"], 4);
    assert!(
        server
            .env_runtime
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .session_dir_watchers
            .is_empty()
    );
    let revoked_terminal: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "terminal/read",
                "params": {
                    "terminalId": terminal_id,
                    "afterSequence": 0,
                    "afterCommandSequence": 0,
                    "maxChunks": 1
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(revoked_terminal["error"]["message"], "TerminalNotFound");

    let stale: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "session/dirs/permissions/set",
                "params": {
                    "sessionId": session.session_id,
                    "path": session_dir.path,
                    "expectedRevision": 3,
                    "permissions": ["readFiles", "writeFiles"]
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(stale["error"]["message"], "RevisionConflict");
}

#[test]
fn env_cwd_set_does_not_require_a_directory_grant() {
    let dir = TestDir::new("rpc-cwd", "readable.txt");
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();
    let mut connection = server.connection();
    server.handle_json(
        &mut connection,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "clientInfo": {"name": "test", "version": "1"},
                "capabilities": {}
            }
        })
        .to_string(),
    );

    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "env/cwd/set",
                "params": {
                    "cwd": dir.path
                }
            })
            .to_string(),
        ),
    )
    .unwrap();

    assert!(response.get("error").is_none());
    assert!(response["result"]["cwd"].is_string());
    assert!(server.terminal_service().is_err());
}

#[test]
fn restricted_dir_installs_only_non_executable_services() {
    let dir = TestDir::new("restricted", "readable.txt");
    let provider = Arc::new(GrantRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodebaseProvider> = provider;
    let providers = CloudCodebaseProviderRegistry::new([provider_trait]).unwrap();
    let server = server()
        .with_cloud_codebase_providers(providers)
        .with_local_env_host(None, DirGrantPolicy::InspectOnly)
        .unwrap();

    assert_eq!(
        server.switch_local_dir_root(dir.path.clone()),
        Ok(dir.root().canonical_path().to_path_buf())
    );
    assert!(server.file_system_service_for(None).is_ok());
    assert!(server.codebase_service().is_ok());
    assert!(server.cloud_codebase_service().is_err());
    assert!(server.git_runtime_service().is_ok());
    assert!(server.content_search_service_for(None).is_err());
    assert!(server.terminal_service().is_err());
    server
        .local_env_host
        .as_ref()
        .unwrap()
        .tools
        .replace_host_available(true)
        .unwrap();
    assert!(
        server
            .local_env_host
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
fn browser_tools_follow_capable_connection_lifecycle_with_explicit_permissions() {
    let dir = TestDir::new("browser-capability", "readable.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    assert_eq!(
        server.switch_local_dir_root(dir.path.clone()),
        Ok(dir.root().canonical_path().to_path_buf())
    );
    let tools = &server.local_env_host.as_ref().unwrap().tools;
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );

    let mut connection = server.connection();
    let response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "desktop-test", "version": "1" },
                    "capabilities": {
                        "browser": { "version": 1, "observe": true, "input": false }
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(response["result"]["capabilities"]["sessions"], true);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );

    let mut second_connection = server.connection();
    let second_response: serde_json::Value = serde_json::from_str(
        &server.handle_json(
            &mut second_connection,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": { "name": "desktop-test-2", "version": "1" },
                    "capabilities": {
                        "browser": { "version": 1, "observe": true, "input": true }
                    }
                }
            })
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(second_response["result"]["capabilities"]["sessions"], true);
    assert!(
        tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    server.close_connection(second_connection);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
    server.close_connection(connection);
    assert!(
        !tools
            .definitions()
            .iter()
            .any(|definition| definition.name.as_str() == "browser_open")
    );
}

#[test]
fn dir_activation_loads_dir_skill_source() {
    let dir = TestDir::new("dir-skills", "readable.txt");
    let skill_root = dir.path.join(".zeta/skills/review-dir");
    std::fs::create_dir_all(&skill_root).unwrap();
    std::fs::write(
        skill_root.join("SKILL.md"),
        "---\nname: review-dir\ndescription: Review this Directory\n---\n\nReview instructions.\n",
    )
    .unwrap();
    let server = server()
        .with_skill_runtime(
            zeta_skills_extension::BuiltInSkillSource::Omitted,
            Arc::new(EmptySkillConfig),
            None,
        )
        .unwrap()
        .with_local_env_host(None, DirGrantPolicy::InspectOnly)
        .unwrap();

    server.switch_local_dir_root(dir.path.clone()).unwrap();

    let snapshot = server
        .skills
        .as_ref()
        .unwrap()
        .list(zeta_skills_extension::SkillCatalogReload::Cached)
        .unwrap();
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(
        snapshot.entries[0].catalog_entry.source().kind(),
        zeta_skills::SkillSourceKind::Directory
    );
}

#[test]
fn user_config_permissions_are_resolved_for_each_client_requested_dir() {
    let dir = TestDir::new("config-permissions", "readable.txt");
    let root = Dir::open_local(&dir.path).unwrap();
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let server = server()
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();

    assert_eq!(
        server.switch_local_dir_root(dir.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.file_system_service_for(None).is_ok());
    assert!(server.terminal_service().is_err());

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("set-dir-permissions").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: host_dir_permissions(),
                display_path: None,
            },
        })
        .unwrap();

    assert_eq!(
        server.switch_local_dir_root(dir.path.clone()),
        Ok(root.canonical_path().to_path_buf())
    );
    assert!(server.git_runtime_service().is_ok());
    assert!(server.content_search_service_for(None).is_ok());
    assert!(server.terminal_service().is_ok());
}

#[test]
fn user_config_permissions_reactivate_an_active_restricted_dir() {
    let dir = TestDir::new("config-promotion", "readable.txt");
    let root = dir.root();
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let server = server()
        .with_config_store(Arc::clone(&config))
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();
    server.switch_local_dir_root(dir.path.clone()).unwrap();
    assert!(server.terminal_service().is_err());
    assert!(server.content_search_service_for(None).is_err());

    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("promote-config-dir").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: host_dir_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    assert!(server.reconcile_active_dir_permissions().is_ok());

    assert!(server.terminal_service().is_ok());
    assert!(server.content_search_service_for(None).is_ok());
    assert_eq!(trusted.revision.get(), 1);
}

#[test]
fn user_config_revocation_removes_executable_services_but_keeps_file_access() {
    let dir = TestDir::new("config-revocation", "readable.txt");
    let root = dir.root();
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("grant-revoked-dir").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: host_dir_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    let server = server()
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();
    server.switch_local_dir_root(dir.path.clone()).unwrap();
    assert!(server.terminal_service().is_ok());
    let thread = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-revocation-thread").unwrap(),
            title: "revocation".into(),
        })
        .unwrap();
    let turn = server
        .threads
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("start-revocation-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "must be interrupted".into(),
                }],
            },
        )
        .unwrap();

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-revoked-dir").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: inspection_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    server
        .env_runtime_control()
        .unwrap()
        .reconcile_user_dir_permissions(&config.read_snapshot().unwrap().values)
        .unwrap();

    let Ok(file_system) = server.file_system_service_for(None) else {
        panic!("restricted filesystem should remain installed after permission revocation");
    };
    assert_eq!(
        file_system
            .read_file(Path::new("readable.txt"), 1024)
            .unwrap(),
        b"config-revocation"
    );
    assert!(server.codebase_service().is_ok());
    assert!(server.git_runtime_service().is_ok());
    assert!(server.content_search_service_for(None).is_err());
    assert!(server.terminal_service().is_err());
    assert_eq!(
        server
            .threads
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
            .local_env_host
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
fn user_config_revocation_removes_local_semantic_model_access() {
    let dir = TestDir::new("semantic-revocation", "source.rs");
    let root = dir.root();
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("grant-semantic-dir").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: host_dir_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    let models = crate::CodebaseModels::new(
        zeta_codebase::EmbeddingIndexKey::new("permissions-test-v1").unwrap(),
        Arc::new(PermissionBoundSemanticEmbedding),
    );
    let server = server()
        .with_codebase_models(models)
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();
    server.switch_local_dir_root(dir.path.clone()).unwrap();
    assert!(server.codebase_semantic_service().is_some());

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-semantic-dir").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: inspection_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    server
        .env_runtime_control()
        .unwrap()
        .reconcile_user_dir_permissions(&config.read_snapshot().unwrap().values)
        .unwrap();

    assert!(server.codebase_semantic_service().is_none());
    assert!(server.codebase_service().is_ok());
}

#[test]
fn user_config_revocation_deletes_cloud_grant_and_removes_cloud_runtime() {
    let dir = TestDir::new("cloud-revocation", "source.rs");
    let root = dir.root();
    let config = Arc::new(ConfigStore::open(dir.path.join("permissions.sqlite3")).unwrap());
    let trusted = config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("grant-cloud-dir").unwrap(),
            expected_revision: ConfigRevision::INITIAL,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: host_dir_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    let provider = Arc::new(GrantRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodebaseProvider> = provider.clone();
    let providers = CloudCodebaseProviderRegistry::new([provider_trait]).unwrap();
    let server = server()
        .with_cloud_codebase_providers(providers)
        .with_local_env_host(None, DirGrantPolicy::UserConfig(Arc::clone(&config)))
        .unwrap();
    server.switch_local_dir_root(dir.path.clone()).unwrap();
    let Ok(codebase) = server.codebase_service() else {
        panic!("local Codebase should be installed under explicit permissions");
    };
    codebase.rebuild().unwrap();
    let Ok(controller) = server.cloud_codebase_service() else {
        panic!("cloud codebase controller should be installed under explicit permissions");
    };
    let grant = CloudCodebaseGrant {
        id: CloudCodebaseGrantId::new("grant-revocation-grant").unwrap(),
        codebase_id: CloudCodebaseId::new("grant-revocation-codebase").unwrap(),
        root_id: controller.root_id().as_str().to_owned(),
        destination: CloudCodebaseDestination::new(
            CloudCodebaseProviderId::new("grant-revocation").unwrap(),
            "tenant-a",
            "dir-index",
        )
        .unwrap(),
        selection: CloudCodebaseSelection::EntireIndex,
        max_egress_bytes: NonZeroU64::new(1024 * 1024).unwrap(),
    };
    assert_eq!(
        controller.authorize(grant).unwrap().state,
        CloudCodebaseState::Granted
    );

    config
        .apply(ConfigCommandRequest {
            command_id: CommandId::new("restrict-cloud-dir").unwrap(),
            expected_revision: trusted.revision,
            command: UserConfigCommand::SetDirPermissions {
                dir: root.id(),
                permissions: inspection_permissions(),
                display_path: None,
            },
        })
        .unwrap();
    server
        .env_runtime_control()
        .unwrap()
        .reconcile_user_dir_permissions(&config.read_snapshot().unwrap().values)
        .unwrap();

    assert!(server.cloud_codebase_service().is_err());
    assert_eq!(provider.deletions.load(Ordering::SeqCst), 1);
}

#[test]
fn restricted_activation_retries_a_persisted_pending_cloud_deletion() {
    let dir = TestDir::new("pending-cloud-deletion", "source.rs");
    let storage = tempfile::tempdir().unwrap();
    let provider = Arc::new(GrantRevocationProvider::new());
    let provider_trait: Arc<dyn CloudCodebaseProvider> = provider.clone();
    let providers = CloudCodebaseProviderRegistry::new([provider_trait]).unwrap();
    let first_server = server()
        .with_cloud_codebase_storage_root(storage.path())
        .with_cloud_codebase_providers(providers)
        .with_local_env_host(None, host_policy())
        .unwrap();
    first_server
        .switch_local_dir_root(dir.path.clone())
        .unwrap();
    let Ok(codebase) = first_server.codebase_service() else {
        panic!("local Codebase should be installed");
    };
    codebase.rebuild().unwrap();
    let Ok(controller) = first_server.cloud_codebase_service() else {
        panic!("cloud controller should be installed");
    };
    controller.authorize(cloud_grant(&controller)).unwrap();
    provider.fail_deletions.store(true, Ordering::SeqCst);
    assert!(controller.revoke().is_err());
    drop(controller);
    drop(first_server);

    provider.fail_deletions.store(false, Ordering::SeqCst);
    let provider_trait: Arc<dyn CloudCodebaseProvider> = provider.clone();
    let providers = CloudCodebaseProviderRegistry::new([provider_trait]).unwrap();
    let restricted_server = server()
        .with_cloud_codebase_storage_root(storage.path())
        .with_cloud_codebase_providers(providers)
        .with_local_env_host(None, DirGrantPolicy::InspectOnly)
        .unwrap();
    restricted_server
        .switch_local_dir_root(dir.path.clone())
        .unwrap();

    assert!(restricted_server.cloud_codebase_service().is_err());
    assert_eq!(provider.deletions.load(Ordering::SeqCst), 2);
}

#[test]
fn active_turn_blocks_env_cwd_set_without_changing_authority() {
    let first = TestDir::new("busy-first", "first.txt");
    let second = TestDir::new("busy-second", "second.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(first.authorization(), test_local_tools(), host)
        .unwrap();
    let thread = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-thread").unwrap(),
            title: "thread".into(),
        })
        .unwrap();
    server
        .threads
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("start-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "stay in the first Directory".into(),
                }],
            },
        )
        .unwrap();

    assert_eq!(
        server.switch_local_dir_root(second.path.clone()),
        Err(EnvRuntimeError::Busy)
    );
    let Ok(file_system) = server.file_system_service_for(None) else {
        panic!("first Directory file system should remain installed");
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

#[test]
fn active_turn_accepts_session_access_changes_and_revokes_old_snapshots() {
    let primary = TestDir::new("active-turn-add-dir-primary", "primary.txt");
    let session_dir = TestDir::new("active-turn-add-dir-extra", "extra.txt");
    let server = server().with_local_env_host(None, host_policy()).unwrap();
    let host = server.local_env_host.as_ref().unwrap();
    server
        .commit_full_env_runtime(primary.authorization(), test_local_tools(), host)
        .unwrap();
    let thread = server
        .start_thread(StartThreadRequest {
            command_id: CommandId::new("create-active-add-dir-thread").unwrap(),
            title: "thread".into(),
        })
        .unwrap();
    server
        .threads
        .start_turn(
            &thread.thread_id,
            StartTurnRequest {
                kind: zeta_protocol::TurnKind::Coding,
                instructions: zeta_models_manager::BASE_INSTRUCTIONS.freeze(),
                command_id: CommandId::new("start-active-add-dir-turn").unwrap(),
                expected_sequence: SequenceExpectation::Exact(1),
                model: None,
                policy_revision: "test-policy-v1".into(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                tool_mode: zeta_protocol::ToolMode::Direct,
                tool_profile: None,
                activated_skills: Vec::new(),
                input: vec![UserInput::Text {
                    text: "continue after the access scope changes".into(),
                }],
            },
        )
        .unwrap();

    let (mutation, _) = server
        .add_session_dir(
            &thread.session_id,
            session_dir.path.clone(),
            host_dir_permissions(),
        )
        .unwrap();
    assert_eq!(mutation, Mutation::AddedDir);
    let access = {
        let runtime = server
            .env_runtime
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(&runtime.dir_grants)
    };
    let snapshot = access
        .snapshot_for(&thread.session_id, Permission::MutateRepository)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.revision().get(), 1);
    assert_eq!(snapshot.authorizations().len(), 1);
    let frozen_token = snapshot.authorizations()[0].clone();

    let (mutation, directories) = server
        .remove_session_dir(&thread.session_id, &session_dir.path)
        .unwrap();
    assert_eq!(mutation, Mutation::RemovedDir);
    assert!(directories.dirs.is_empty());
    assert!(frozen_token.ensure_active().is_err());
    let empty_snapshot = access
        .snapshot_for(&thread.session_id, Permission::MutateRepository)
        .unwrap()
        .unwrap();
    assert_eq!(empty_snapshot.revision().get(), 2);
    assert!(empty_snapshot.authorizations().is_empty());
}

fn server() -> AppServer {
    let threads = Arc::new(ThreadController::with_store(Arc::new(
        InMemoryThreadStore::default(),
    )));
    AppServer::new(
        threads,
        Arc::new(ProviderModelService::new(Arc::new(EchoModel))),
    )
    .with_ephemeral_env_state()
}

fn test_local_tools() -> LocalToolComposition {
    LocalToolComposition::without_executors(
        Arc::new(NoTools),
        Arc::new(RejectPolicy),
        RipgrepExecutable::from_path(std::env::current_exe().unwrap()).unwrap(),
    )
}

fn host_policy() -> DirGrantPolicy {
    DirGrantPolicy::HostSelectedDirs(GrantSource::HostConfiguration)
}

struct RejectPolicy;

struct EmptySkillConfig;

struct GrantRevocationProvider {
    id: CloudCodebaseProviderId,
    deletions: AtomicUsize,
    fail_deletions: AtomicBool,
}

impl GrantRevocationProvider {
    fn new() -> Self {
        Self {
            id: CloudCodebaseProviderId::new("grant-revocation").unwrap(),
            deletions: AtomicUsize::new(0),
            fail_deletions: AtomicBool::new(false),
        }
    }
}

impl CloudCodebaseProvider for GrantRevocationProvider {
    fn id(&self) -> &CloudCodebaseProviderId {
        &self.id
    }

    fn capabilities(&self) -> CloudCodebaseCapabilities {
        CloudCodebaseCapabilities {
            deletion: CloudCodebaseDeletionSupport::IdempotentGrantDeletion,
        }
    }

    fn publish(
        &self,
        _request: CloudCodebasePublicationRequest,
    ) -> Result<CloudCodebasePublication, CloudCodebaseProviderError> {
        Ok(CloudCodebasePublication {
            remote_generation: "dir-index-ready".into(),
        })
    }

    fn query(
        &self,
        _request: zeta_cloud_codebase::CloudCodebaseQueryRequest,
    ) -> Result<zeta_cloud_codebase::CloudCodebaseQueryResult, CloudCodebaseProviderError> {
        Err(CloudCodebaseProviderError::new("query not configured"))
    }

    fn delete_grant(&self, _grant: &CloudCodebaseGrant) -> Result<(), CloudCodebaseProviderError> {
        self.deletions.fetch_add(1, Ordering::SeqCst);
        if self.fail_deletions.load(Ordering::SeqCst) {
            return Err(CloudCodebaseProviderError::new("delete failed"));
        }
        Ok(())
    }
}

fn cloud_grant(controller: &zeta_cloud_codebase::CloudCodebaseController) -> CloudCodebaseGrant {
    CloudCodebaseGrant {
        id: CloudCodebaseGrantId::new("pending-deletion-grant").unwrap(),
        codebase_id: CloudCodebaseId::new("pending-deletion-codebase").unwrap(),
        root_id: controller.root_id().as_str().to_owned(),
        destination: CloudCodebaseDestination::new(
            CloudCodebaseProviderId::new("grant-revocation").unwrap(),
            "tenant-a",
            "dir-index",
        )
        .unwrap(),
        selection: CloudCodebaseSelection::EntireIndex,
        max_egress_bytes: NonZeroU64::new(1024 * 1024).unwrap(),
    }
}

impl zeta_skills_extension::SkillConfigSnapshotProvider for EmptySkillConfig {
    fn snapshot(&self) -> Result<zeta_config::SkillsConfig, String> {
        Ok(zeta_config::SkillsConfig::default())
    }
}

impl ActionPolicyService for RejectPolicy {
    fn revision(&self) -> String {
        "test-policy-v1".into()
    }

    fn decide(
        &self,
        _: &ActionReviewRequest,
        _: &CancellationToken,
    ) -> Result<ExecutionDecision, CoreError> {
        Err(CoreError::Policy("test policy rejects every action".into()))
    }
}

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str, file: &str) -> Self {
        let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("dir-runtime-tests")
            .join(format!("{}-{label}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(file), label).unwrap();
        Self { path }
    }

    fn root(&self) -> Dir {
        Dir::open_local(&self.path).unwrap()
    }

    fn authorization(&self) -> Grant {
        Grant::for_environment(
            self.root(),
            GrantSource::HostConfiguration,
            host_dir_permissions(),
        )
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
